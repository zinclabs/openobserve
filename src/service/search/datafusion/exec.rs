// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::{str::FromStr, sync::Arc};

use arrow_schema::Field;
use config::{
    get_config,
    meta::{
        search::{Session as SearchSession, StorageType},
        stream::{FileKey, FileMeta, StreamType},
    },
    utils::{parquet::new_parquet_writer, schema_ext::SchemaExt},
    PARQUET_BATCH_SIZE,
};
use datafusion::{
    arrow::datatypes::{DataType, Schema},
    catalog::TableProvider,
    common::Column,
    datasource::{
        file_format::parquet::ParquetFormat,
        listing::{ListingOptions, ListingTableConfig, ListingTableUrl},
        object_store::{DefaultObjectStoreRegistry, ObjectStoreRegistry},
    },
    error::{DataFusionError, Result},
    execution::{
        cache::cache_manager::{CacheManagerConfig, FileStatisticsCache},
        context::SessionConfig,
        memory_pool::{FairSpillPool, GreedyMemoryPool},
        runtime_env::{RuntimeConfig, RuntimeEnv},
        session_state::SessionStateBuilder,
    },
    logical_expr::AggregateUDF,
    optimizer::OptimizerRule,
    physical_plan::execute_stream,
    prelude::{Expr, SessionContext},
};
use futures::TryStreamExt;
use hashbrown::HashMap;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::{
    common::infra::config::get_config as get_o2_config, search::WorkGroup,
};

use super::{
    file_type::{FileType, GetExt},
    optimizer::join_reorder::JoinReorderRule,
    planner::extension_planner::OpenobserveQueryPlanner,
    storage::file_list,
    table_provider::{uniontable::NewUnionTable, NewListingTable},
    udf::transform_udf::get_all_transform,
};
use crate::service::{
    metadata::distinct_values::DISTINCT_STREAM_PREFIX, search::index::IndexCondition,
};

const DATAFUSION_MIN_MEM: usize = 1024 * 1024 * 256; // 256MB
const DATAFUSION_MIN_PARTITION: usize = 2; // CPU cores

pub async fn merge_parquet_files(
    stream_type: StreamType,
    stream_name: &str,
    schema: Arc<Schema>,
    tables: Vec<Arc<dyn TableProvider>>,
    bloom_filter_fields: &[String],
    metadata: &FileMeta,
) -> Result<(Arc<Schema>, Vec<u8>)> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    // get all sorted data
    let sql = if stream_type == StreamType::Index {
        format!(
            "SELECT * FROM tbl WHERE file_name NOT IN (SELECT file_name FROM tbl WHERE deleted IS TRUE ORDER BY {} DESC) ORDER BY {} DESC",
            cfg.common.column_timestamp, cfg.common.column_timestamp
        )
    } else if cfg.limit.distinct_values_hourly
        && stream_type == StreamType::Metadata
        && stream_name.starts_with(DISTINCT_STREAM_PREFIX)
    {
        let fields = schema
            .fields()
            .iter()
            .filter(|f| f.name() != &cfg.common.column_timestamp && f.name() != "count")
            .map(|x| x.name().to_string())
            .collect::<Vec<_>>();
        let fields_str = fields.join(", ");
        format!(
            "SELECT MIN({}) AS {}, SUM(count) as count, {} FROM tbl GROUP BY {} ORDER BY {} DESC",
            cfg.common.column_timestamp,
            cfg.common.column_timestamp,
            fields_str,
            fields_str,
            cfg.common.column_timestamp
        )
    } else {
        format!(
            "SELECT * FROM tbl ORDER BY {} DESC",
            cfg.common.column_timestamp
        )
    };
    log::debug!("merge_parquet_files sql: {}", sql);

    // create datafusion context
    let sort_by_timestamp_desc = true;
    let target_partitions = cfg.limit.cpu_num;
    let ctx =
        prepare_datafusion_context(None, vec![], sort_by_timestamp_desc, target_partitions).await?;
    // register union table
    let union_table = Arc::new(NewUnionTable::try_new(schema.clone(), tables)?);
    ctx.register_table("tbl", union_table)?;

    let plan = ctx.state().create_logical_plan(&sql).await?;
    let physical_plan = ctx.state().create_physical_plan(&plan).await?;
    let schema = physical_plan.schema();

    // write result to parquet file
    let mut buf = Vec::new();
    let mut writer = new_parquet_writer(&mut buf, &schema, bloom_filter_fields, metadata);
    let mut batch_stream = execute_stream(physical_plan, ctx.task_ctx())?;
    loop {
        match batch_stream.try_next().await {
            Ok(Some(batch)) => {
                if let Err(e) = writer.write(&batch).await {
                    log::error!("merge_parquet_files write Error: {}", e);
                    return Err(e.into());
                }
            }
            Ok(None) => {
                break;
            }
            Err(e) => {
                log::error!("merge_parquet_files execute stream Error: {}", e);
                return Err(e);
            }
        }
    }
    writer.close().await?;

    ctx.deregister_table("tbl")?;
    drop(ctx);

    log::debug!(
        "merge_parquet_files took {} ms",
        start.elapsed().as_millis()
    );

    Ok((schema, buf))
}

pub fn create_session_config(
    sorted_by_time: bool,
    target_partitions: usize,
) -> Result<SessionConfig> {
    let cfg = get_config();
    let mut target_partitions = if target_partitions == 0 {
        cfg.limit.cpu_num
    } else {
        std::cmp::max(cfg.limit.datafusion_min_partition_num, target_partitions)
    };
    if target_partitions == 0 {
        target_partitions = DATAFUSION_MIN_PARTITION;
    }
    let mut config = SessionConfig::from_env()?
        .with_batch_size(PARQUET_BATCH_SIZE)
        .with_target_partitions(target_partitions)
        .with_information_schema(true);
    config
        .options_mut()
        .execution
        .listing_table_ignore_subdirectory = false;
    config.options_mut().sql_parser.dialect = "PostgreSQL".to_string();

    // based on data distributing, it only works for the data on a few records
    // config = config.set_bool("datafusion.execution.parquet.pushdown_filters", true);
    // config = config.set_bool("datafusion.execution.parquet.reorder_filters", true);

    if cfg.common.bloom_filter_enabled {
        config = config.set_bool("datafusion.execution.parquet.bloom_filter_on_read", true);
    }
    if cfg.common.bloom_filter_disabled_on_search {
        config = config.set_bool("datafusion.execution.parquet.bloom_filter_on_read", false);
    }
    if sorted_by_time {
        config = config.set_bool("datafusion.execution.split_file_groups_by_statistics", true);
    }

    // When set to true, skips verifying that the schema produced by planning the input of
    // `LogicalPlan::Aggregate` exactly matches the schema of the input plan.
    config = config.set_bool(
        "datafusion.execution.skip_physical_aggregate_schema_check",
        true,
    );

    Ok(config)
}

pub async fn create_runtime_env(memory_limit: usize) -> Result<RuntimeEnv> {
    let object_store_registry = DefaultObjectStoreRegistry::new();

    let memory = super::storage::memory::FS::new();
    let memory_url = url::Url::parse("memory:///").unwrap();
    object_store_registry.register_store(&memory_url, Arc::new(memory));

    let wal = super::storage::wal::FS::new();
    let wal_url = url::Url::parse("wal:///").unwrap();
    object_store_registry.register_store(&wal_url, Arc::new(wal));

    let tmpfs = super::storage::tmpfs::Tmpfs::new();
    let tmpfs_url = url::Url::parse("tmpfs:///").unwrap();
    object_store_registry.register_store(&tmpfs_url, Arc::new(tmpfs));

    let cfg = get_config();
    let mut rn_config =
        RuntimeConfig::new().with_object_store_registry(Arc::new(object_store_registry));
    if cfg.limit.datafusion_file_stat_cache_max_entries > 0 {
        let cache_config = CacheManagerConfig::default();
        let cache_config = cache_config.with_files_statistics_cache(Some(
            super::storage::file_statistics_cache::GLOBAL_CACHE.clone(),
        ));
        rn_config = rn_config.with_cache_manager(cache_config);
    }

    let memory_size = std::cmp::max(DATAFUSION_MIN_MEM, memory_limit);
    let mem_pool = super::MemoryPoolType::from_str(&cfg.memory_cache.datafusion_memory_pool)
        .map_err(|e| {
            DataFusionError::Execution(format!("Invalid datafusion memory pool type: {}", e))
        })?;
    match mem_pool {
        super::MemoryPoolType::Greedy => {
            rn_config = rn_config.with_memory_pool(Arc::new(GreedyMemoryPool::new(memory_size)))
        }
        super::MemoryPoolType::Fair => {
            rn_config = rn_config.with_memory_pool(Arc::new(FairSpillPool::new(memory_size)))
        }
        super::MemoryPoolType::None => {}
    };
    RuntimeEnv::try_new(rn_config)
}

pub async fn prepare_datafusion_context(
    _work_group: Option<String>,
    optimizer_rules: Vec<Arc<dyn OptimizerRule + Send + Sync>>,
    sorted_by_time: bool,
    target_partitions: usize,
) -> Result<SessionContext, DataFusionError> {
    let cfg = get_config();
    #[cfg(not(feature = "enterprise"))]
    let (memory_size, target_partition) = (cfg.memory_cache.datafusion_max_size, target_partitions);
    #[cfg(feature = "enterprise")]
    let (target_partition, memory_size) = (target_partitions, cfg.memory_cache.datafusion_max_size);
    #[cfg(feature = "enterprise")]
    let (target_partition, memory_size) =
        get_cpu_and_mem_limit(_work_group.clone(), target_partition, memory_size).await?;

    let session_config = create_session_config(sorted_by_time, target_partition)?;
    let runtime_env = Arc::new(create_runtime_env(memory_size).await?);
    let mut builder = SessionStateBuilder::new()
        .with_config(session_config)
        .with_runtime_env(runtime_env)
        .with_default_features();
    if !optimizer_rules.is_empty() {
        builder = builder
            .with_optimizer_rules(optimizer_rules)
            .with_physical_optimizer_rule(Arc::new(JoinReorderRule::new()));
    }
    if cfg.common.feature_join_match_one_enabled {
        builder = builder.with_query_planner(Arc::new(OpenobserveQueryPlanner::new()));
    }
    Ok(SessionContext::new_with_state(builder.build()))
}

pub fn register_udf(ctx: &SessionContext, org_id: &str) -> Result<()> {
    ctx.register_udf(super::udf::str_match_udf::STR_MATCH_UDF.clone());
    ctx.register_udf(super::udf::str_match_udf::STR_MATCH_IGNORE_CASE_UDF.clone());
    ctx.register_udf(super::udf::fuzzy_match_udf::FUZZY_MATCH_UDF.clone());
    ctx.register_udf(super::udf::regexp_udf::REGEX_MATCH_UDF.clone());
    ctx.register_udf(super::udf::regexp_udf::REGEX_NOT_MATCH_UDF.clone());
    ctx.register_udf(super::udf::regexp_udf::REGEXP_MATCH_TO_FIELDS_UDF.clone());
    ctx.register_udf(super::udf::regexp_matches_udf::REGEX_MATCHES_UDF.clone());
    ctx.register_udf(super::udf::time_range_udf::TIME_RANGE_UDF.clone());
    ctx.register_udf(super::udf::date_format_udf::DATE_FORMAT_UDF.clone());
    ctx.register_udf(super::udf::string_to_array_v2_udf::STRING_TO_ARRAY_V2_UDF.clone());
    ctx.register_udf(super::udf::arrzip_udf::ARR_ZIP_UDF.clone());
    ctx.register_udf(super::udf::arrindex_udf::ARR_INDEX_UDF.clone());
    ctx.register_udf(super::udf::arr_descending_udf::ARR_DESCENDING_UDF.clone());
    ctx.register_udf(super::udf::arrjoin_udf::ARR_JOIN_UDF.clone());
    ctx.register_udf(super::udf::arrcount_udf::ARR_COUNT_UDF.clone());
    ctx.register_udf(super::udf::arrsort_udf::ARR_SORT_UDF.clone());
    ctx.register_udf(super::udf::cast_to_arr_udf::CAST_TO_ARR_UDF.clone());
    ctx.register_udf(super::udf::spath_udf::SPATH_UDF.clone());
    ctx.register_udf(super::udf::to_arr_string_udf::TO_ARR_STRING.clone());
    ctx.register_udf(super::udf::histogram_udf::HISTOGRAM_UDF.clone());
    ctx.register_udf(super::udf::match_all_udf::MATCH_ALL_RAW_UDF.clone());
    ctx.register_udf(super::udf::match_all_udf::MATCH_ALL_RAW_IGNORE_CASE_UDF.clone());
    ctx.register_udf(super::udf::match_all_udf::MATCH_ALL_UDF.clone());
    #[cfg(feature = "enterprise")]
    ctx.register_udf(super::udf::cipher_udf::DECRYPT_UDF.clone());
    #[cfg(feature = "enterprise")]
    ctx.register_udf(super::udf::cipher_udf::ENCRYPT_UDF.clone());
    ctx.register_udf(super::udf::match_all_udf::FUZZY_MATCH_ALL_UDF.clone());
    ctx.register_udaf(AggregateUDF::from(
        super::udaf::percentile_cont::PercentileCont::new(),
    ));

    let udf_list = get_all_transform(org_id)?;
    for udf in udf_list {
        ctx.register_udf(udf.clone());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn register_table(
    session: &SearchSession,
    schema: Arc<Schema>,
    table_name: &str,
    files: &[FileKey],
    rules: HashMap<String, DataType>,
    sort_key: &[(String, bool)],
) -> Result<SessionContext> {
    let cfg = get_config();
    // only sort by timestamp desc
    let sorted_by_time =
        sort_key.len() == 1 && sort_key[0].0 == cfg.common.column_timestamp && sort_key[0].1;

    let ctx = prepare_datafusion_context(
        session.work_group.clone(),
        vec![],
        sorted_by_time,
        session.target_partitions,
    )
    .await?;

    let table = create_parquet_table(
        session,
        schema.clone(),
        files,
        rules.clone(),
        sorted_by_time,
        ctx.runtime_env().cache_manager.get_file_statistic_cache(),
        None,
        vec![],
    )
    .await?;
    ctx.register_table(table_name, table)?;

    Ok(ctx)
}

#[allow(clippy::too_many_arguments)]
pub async fn create_parquet_table(
    session: &SearchSession,
    schema: Arc<Schema>,
    files: &[FileKey],
    rules: HashMap<String, DataType>,
    sorted_by_time: bool,
    file_stat_cache: Option<FileStatisticsCache>,
    index_condition: Option<IndexCondition>,
    fst_fields: Vec<String>,
) -> Result<Arc<dyn TableProvider>> {
    let cfg = get_config();
    let target_partitions = if session.target_partitions == 0 {
        cfg.limit.cpu_num
    } else {
        std::cmp::max(
            cfg.limit.datafusion_min_partition_num,
            session.target_partitions,
        )
    };

    #[cfg(feature = "enterprise")]
    let (target_partitions, _) =
        get_cpu_and_mem_limit(session.work_group.clone(), target_partitions, 0).await?;

    let target_partitions = if target_partitions == 0 {
        DATAFUSION_MIN_PARTITION
    } else {
        target_partitions
    };

    // Configure listing options
    let file_format = ParquetFormat::default();
    let mut listing_options = ListingOptions::new(Arc::new(file_format))
        .with_file_extension(FileType::PARQUET.get_ext())
        .with_target_partitions(target_partitions)
        .with_collect_stat(true);

    if sorted_by_time {
        // specify sort columns for parquet file
        listing_options =
            listing_options.with_file_sort_order(vec![vec![datafusion::logical_expr::SortExpr {
                expr: Expr::Column(Column::new_unqualified(cfg.common.column_timestamp.clone())),
                asc: false,
                nulls_first: false,
            }]]);
    }

    let schema_key = schema.hash_key();
    let prefix = if session.storage_type == StorageType::Memory {
        file_list::set(&session.id, &schema_key, files).await;
        format!("memory:///{}/schema={}/", session.id, schema_key)
    } else if session.storage_type == StorageType::Wal {
        file_list::set(&session.id, &schema_key, files).await;
        format!("wal:///{}/schema={}/", session.id, schema_key)
    } else if session.storage_type == StorageType::Tmpfs {
        format!("tmpfs:///{}/", session.id)
    } else {
        return Err(DataFusionError::Execution(format!(
            "Unsupported storage_type {:?}",
            session.storage_type,
        )));
    };
    let prefix = match ListingTableUrl::parse(prefix) {
        Ok(url) => url,
        Err(e) => {
            return Err(datafusion::error::DataFusionError::Execution(format!(
                "ListingTableUrl error: {e}",
            )));
        }
    };

    let mut config = ListingTableConfig::new(prefix).with_listing_options(listing_options);
    let timestamp_field = schema.field_with_name(&cfg.common.column_timestamp);
    let schema = if timestamp_field.is_ok() && timestamp_field.unwrap().is_nullable() {
        let new_fields = schema
            .fields()
            .iter()
            .map(|x| {
                if x.name() == &cfg.common.column_timestamp {
                    Arc::new(Field::new(
                        cfg.common.column_timestamp.clone(),
                        DataType::Int64,
                        false,
                    ))
                } else {
                    x.clone()
                }
            })
            .collect::<Vec<_>>();
        Arc::new(Schema::new(new_fields))
    } else {
        schema
    };
    config = config.with_schema(schema);
    let mut table = NewListingTable::try_new(config, rules, index_condition, fst_fields)?;
    if session.storage_type != StorageType::Tmpfs && file_stat_cache.is_some() {
        table = table.with_cache(file_stat_cache);
    }
    Ok(Arc::new(table))
}

#[cfg(feature = "enterprise")]
async fn get_cpu_and_mem_limit(
    work_group: Option<String>,
    mut target_partitions: usize,
    mut memory_size: usize,
) -> Result<(usize, usize)> {
    if let Some(wg) = work_group {
        if let Ok(wg) = WorkGroup::from_str(&wg) {
            let (cpu, mem) = wg.get_dynamic_resource().await.map_err(|e| {
                DataFusionError::Execution(format!("Failed to get dynamic resource: {}", e))
            })?;
            if get_o2_config().search_group.cpu_limit_enabled {
                target_partitions = target_partitions * cpu as usize / 100;
            }
            memory_size = memory_size * mem as usize / 100;
            log::debug!(
                "[datafusion:{}] target_partition: {}, memory_size: {}",
                wg,
                target_partitions,
                memory_size
            );
        }
    }
    Ok((target_partitions, memory_size))
}
