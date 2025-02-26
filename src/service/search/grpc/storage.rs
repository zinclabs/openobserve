// Copyright 2025 OpenObserve Inc.
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

use std::{collections::HashSet, sync::Arc};

use anyhow::Context;
use arrow_schema::Schema;
use config::{
    get_config, is_local_disk_storage,
    meta::{
        bitvec::BitVec,
        inverted_index::{InvertedIndexOptimizeMode, InvertedIndexTantivyMode},
        search::{ScanStats, StorageType},
        stream::FileKey,
    },
    utils::{
        file::is_exists,
        inverted_index::convert_parquet_idx_file_name_to_tantivy_file,
        tantivy::tokenizer::{o2_tokenizer_build, O2_TOKENIZER},
        time::BASE_TIME,
    },
    FILE_EXT_TANTIVY, FILE_EXT_TANTIVY_FOLDER, INDEX_FIELD_NAME_FOR_ALL,
};
use datafusion::execution::cache::cache_manager::FileStatisticsCache;
use futures::future::try_join_all;
use hashbrown::HashMap;
use infra::{
    cache::file_data,
    errors::{Error, ErrorCodes},
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use tantivy::Directory;
use tokio::sync::Semaphore;
use tracing::Instrument;

static DOWNLOAD_SEMAPHORE: Lazy<std::sync::Arc<Semaphore>> = Lazy::new(|| {
    let cfg = get_config();
    std::sync::Arc::new(Semaphore::new(cfg.limit.file_download_thread_num))
});

use crate::service::{
    db, file_list,
    search::{
        datafusion::exec,
        generate_search_schema_diff,
        index::IndexCondition,
        tantivy::puffin_directory::{
            caching_directory::CachingDirectory,
            convert_puffin_file_to_tantivy_dir,
            footer_cache::FooterCache,
            reader::{warm_up_terms, PuffinDirReader},
            reader_cache,
        },
    },
};

/// search in remote object storage
#[tracing::instrument(name = "service:search:grpc:storage", skip_all, fields(org_id = query.org_id, stream_name = query.stream_name))]
#[allow(clippy::too_many_arguments)]
pub async fn search(
    query: Arc<super::QueryParams>,
    schema: Arc<Schema>,
    file_list: &[FileKey],
    sorted_by_time: bool,
    file_stat_cache: Option<FileStatisticsCache>,
    mut index_condition: Option<IndexCondition>,
    mut fst_fields: Vec<String>,
    idx_optimize_rule: Option<InvertedIndexOptimizeMode>,
) -> super::SearchTable {
    let enter_span = tracing::span::Span::current();
    log::info!("[trace_id {}] search->storage: enter", query.trace_id);
    // fetch all schema versions, group files by version
    let schema_versions = match infra::schema::get_versions(
        &query.org_id,
        &query.stream_name,
        query.stream_type,
        query.time_range,
    )
    .instrument(enter_span.clone())
    .await
    {
        Ok(versions) => versions,
        Err(err) => {
            log::error!("[trace_id {}] get schema error: {}", query.trace_id, err);
            return Err(Error::ErrorCode(ErrorCodes::SearchStreamNotFound(
                query.stream_name.clone(),
            )));
        }
    };
    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, get schema versions num {}",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        schema_versions.len()
    );
    if schema_versions.is_empty() {
        return Ok((vec![], ScanStats::new()));
    }
    let latest_schema_id = schema_versions.len() - 1;

    // get file list
    let mut files = file_list.to_vec();
    if files.is_empty() {
        return Ok((vec![], ScanStats::default()));
    }
    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, load file_list num {}",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        files.len(),
    );

    // check inverted index
    let cfg = get_config();
    #[allow(deprecated)]
    let inverted_index_type = cfg.common.inverted_index_search_format.clone();
    let use_inverted_index = query.use_inverted_index && inverted_index_type == "tantivy";
    if use_inverted_index {
        log::info!(
            "[trace_id {}] flight->search: use_inverted_index with tantivy format {}",
            query.trace_id,
            use_inverted_index
        );
    }

    let mut idx_took = 0;
    let mut is_add_filter_back = false;
    if use_inverted_index {
        (idx_took, is_add_filter_back, _) = filter_file_list_by_tantivy_index(
            query.clone(),
            &mut files,
            index_condition.clone(),
            idx_optimize_rule,
        )
        .await?;
        log::info!(
            "[trace_id {}] search->storage: stream {}/{}/{}, {} inverted index reduced file_list num to {} in {} ms",
            query.trace_id,
            query.org_id,
            query.stream_type,
            query.stream_name,
            inverted_index_type,
            files.len(),
            idx_took
        );
    }

    if !is_add_filter_back {
        index_condition = None;
        fst_fields = vec![];
    }

    let cfg = get_config();
    let mut files_group: HashMap<usize, Vec<FileKey>> =
        HashMap::with_capacity(schema_versions.len());
    let mut scan_stats = ScanStats::new();
    if schema_versions.len() == 1 {
        let files = files.to_vec();
        scan_stats = match file_list::calculate_files_size(&files).await {
            Ok(size) => size,
            Err(err) => {
                log::error!(
                    "[trace_id {}] calculate files size error: {}",
                    query.trace_id,
                    err
                );
                return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                    "calculate files size error".to_string(),
                )));
            }
        };
        files_group.insert(latest_schema_id, files);
    } else {
        scan_stats.files = files.len() as i64;
        for file in files.iter() {
            // calculate scan size
            scan_stats.records += file.meta.records;
            scan_stats.original_size += file.meta.original_size;
            scan_stats.compressed_size += file.meta.compressed_size;
            // check schema version
            let schema_ver_id = match db::schema::filter_schema_version_id(
                &schema_versions,
                file.meta.min_ts,
                file.meta.max_ts,
            ) {
                Some(id) => id,
                None => {
                    log::error!(
                        "[trace_id {}] search->storage: file {} schema version not found, will use the latest schema, min_ts: {}, max_ts: {}",
                        query.trace_id,
                        &file.key,
                        file.meta.min_ts,
                        file.meta.max_ts
                    );
                    // HACK: use the latest version if not found in schema versions
                    latest_schema_id
                }
            };
            let group = files_group.entry(schema_ver_id).or_default();
            group.push(file.clone());
        }
    }

    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, load files {}, scan_size {}, compressed_size {}",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        scan_stats.files,
        scan_stats.original_size,
        scan_stats.compressed_size
    );

    if cfg.common.memory_circuit_breaker_enable {
        super::check_memory_circuit_breaker(&query.trace_id, &scan_stats)?;
    }

    // load files to local cache
    let cache_start = std::time::Instant::now();
    let cache_type = cache_files(
        &query.trace_id,
        &files.iter().map(|f| f.key.as_ref()).collect_vec(),
        &mut scan_stats,
        "parquet",
    )
    .instrument(enter_span.clone())
    .await?;

    scan_stats.idx_took = idx_took as i64;
    scan_stats.querier_files = scan_stats.files;
    let download_msg = if cache_type == file_data::CacheType::None {
        "".to_string()
    } else {
        format!("downloading others into {:?} in background,", cache_type)
    };
    log::info!(
        "[trace_id {}] search->storage: stream {}/{}/{}, load files {}, memory cached {}, disk cached {}, {download_msg} took: {} ms",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        scan_stats.querier_files,
        scan_stats.querier_memory_cached_files,
        scan_stats.querier_disk_cached_files,
        cache_start.elapsed().as_millis()
    );

    // set target partitions based on cache type
    let target_partitions = if cache_type == file_data::CacheType::None {
        cfg.limit.query_thread_num
    } else {
        cfg.limit.cpu_num
    };

    // construct latest schema map
    let latest_schema = Arc::new(
        schema
            .as_ref()
            .clone()
            .with_metadata(std::collections::HashMap::new()),
    );
    let mut latest_schema_map = HashMap::with_capacity(latest_schema.fields().len());
    for field in latest_schema.fields() {
        latest_schema_map.insert(field.name(), field);
    }

    let mut tables = Vec::new();
    for (ver, files) in files_group {
        if files.is_empty() {
            continue;
        }
        if files.is_empty() {
            continue;
        }
        let schema = schema_versions[ver].clone();
        let schema = schema.with_metadata(std::collections::HashMap::new());

        let session = config::meta::search::Session {
            id: format!("{}-storage-{ver}", query.trace_id),
            storage_type: StorageType::Memory,
            work_group: query.work_group.clone(),
            target_partitions,
        };

        let diff_fields = generate_search_schema_diff(&schema, &latest_schema_map);
        let table = exec::create_parquet_table(
            &session,
            latest_schema.clone(),
            &files,
            diff_fields,
            sorted_by_time,
            file_stat_cache.clone(),
            index_condition.clone(),
            fst_fields.clone(),
        )
        .await?;
        tables.push(table);
    }

    Ok((tables, scan_stats))
}

#[tracing::instrument(name = "service:search:grpc:storage:cache_files", skip_all)]
async fn cache_files(
    trace_id: &str,
    files: &[&str],
    scan_stats: &mut ScanStats,
    file_type: &str,
) -> Result<file_data::CacheType, Error> {
    // check how many files already cached
    for file in files.iter() {
        if file_data::memory::exist(file).await {
            scan_stats.querier_memory_cached_files += 1;
        } else if file_data::disk::exist(file).await {
            scan_stats.querier_disk_cached_files += 1;
        }
    }
    let files_num = files.len() as i64;
    if files_num == scan_stats.querier_memory_cached_files + scan_stats.querier_disk_cached_files {
        // all files are cached
        return Ok(file_data::CacheType::None);
    }

    // check cache size
    let cfg = get_config();
    let cache_type = if cfg.memory_cache.enabled
        && scan_stats.compressed_size < cfg.memory_cache.skip_size as i64
    {
        // if scan_compressed_size < ZO_MEMORY_CACHE_SKIP_SIZE, use memory cache
        file_data::CacheType::Memory
    } else if !is_local_disk_storage()
        && cfg.disk_cache.enabled
        && scan_stats.compressed_size < cfg.disk_cache.skip_size as i64
    {
        // if scan_compressed_size < ZO_DISK_CACHE_SKIP_SIZE, use disk cache
        file_data::CacheType::Disk
    } else {
        // no cache, the files are too big than cache size
        return Ok(file_data::CacheType::None);
    };

    let trace_id = trace_id.to_string();
    let files = files.iter().map(|f| f.to_string()).collect_vec();
    let file_type = file_type.to_string();
    tokio::spawn(async move {
        let start = std::time::Instant::now();
        let files = files.iter().map(|f| f.as_str()).collect_vec();
        match cache_files_inner(&trace_id, &files, cache_type).await {
            Err(e) => {
                log::error!(
                    "[trace_id {}] search->storage: cache {} files in background error: {:?}",
                    trace_id,
                    file_type,
                    e
                );
            }
            Ok(cache_type) => {
                log::info!(
                    "[trace_id {}] search->storage: cache {} files in background into {:?} cache done, downloaded {} files, took: {} ms",
                    trace_id,
                    file_type,
                    cache_type,
                    files.len(),
                    start.elapsed().as_millis()
                );
            }
        }
    });

    // if cached file less than 50% of the total files, return None
    if scan_stats.querier_memory_cached_files + scan_stats.querier_disk_cached_files < files_num / 2
    {
        Ok(file_data::CacheType::None)
    } else {
        Ok(cache_type)
    }
}

#[tracing::instrument(name = "service:search:grpc:storage:cache_files_inner", skip_all)]
async fn cache_files_inner(
    trace_id: &str,
    files: &[&str],
    cache_type: file_data::CacheType,
) -> Result<file_data::CacheType, Error> {
    let mut tasks = Vec::new();
    for file in files.iter() {
        let trace_id = trace_id.to_string();
        let file_name = file.to_string();
        let permit = DOWNLOAD_SEMAPHORE.clone().acquire_owned().await.unwrap();
        let task: tokio::task::JoinHandle<()> = tokio::task::spawn(async move {
            let cfg = get_config();
            let ret = match cache_type {
                file_data::CacheType::Memory => {
                    let mut disk_exists = false;
                    let mem_exists = file_data::memory::exist(&file_name).await;
                    if !mem_exists && !cfg.memory_cache.skip_disk_check {
                        // when skip_disk_check = false, need to check disk cache
                        disk_exists = file_data::disk::exist(&file_name).await;
                    }
                    if !mem_exists && (cfg.memory_cache.skip_disk_check || !disk_exists) {
                        file_data::memory::download(&trace_id, &file_name)
                            .await
                            .err()
                    } else {
                        None
                    }
                }
                file_data::CacheType::Disk => {
                    if !file_data::disk::exist(&file_name).await {
                        file_data::disk::download(&trace_id, &file_name).await.err()
                    } else {
                        None
                    }
                }
                _ => None,
            };
            // return file_name if download failed
            if let Some(e) = ret {
                log::warn!(
                        "[trace_id {trace_id}] search->storage: download file to cache err: {}, file: {}",
                        e,
                        file_name
                    );
            }
            drop(permit);
        });
        tasks.push(task);
    }

    for task in tasks {
        if let Err(e) = task.await {
            log::error!(
                "[trace_id {trace_id}] search->storage: load file task err: {}",
                e
            );
        }
    }

    Ok(cache_type)
}

/// Filter file list using inverted index
/// This function will load the index file corresponding to each file in the file list.
/// FSTs in those files are used to match the incoming query in `SearchRequest`.
/// If the query does not match any FST in the index file, the file will be filtered out.
/// If the query does match then the segment IDs for the file will be updated.
/// If the query not find corresponding index file, the file will *not* be filtered out.
pub async fn filter_file_list_by_tantivy_index(
    query: Arc<super::QueryParams>,
    file_list: &mut Vec<FileKey>,
    index_condition: Option<IndexCondition>,
    idx_optimize_rule: Option<InvertedIndexOptimizeMode>,
) -> Result<(usize, bool, usize), Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    // Cache the corresponding Index files
    let mut scan_stats = ScanStats::new();
    let mut file_list_map = file_list
        .drain(..)
        .map(|f| (f.key.clone(), f))
        .collect::<HashMap<_, _>>();
    let index_file_names = file_list_map
        .iter()
        .filter_map(|(_, f)| {
            scan_stats.compressed_size += f.meta.index_size;
            if f.meta.index_size > 0 {
                convert_parquet_idx_file_name_to_tantivy_file(&f.key)
                    .map(|ttv_file| (ttv_file, f.clone()))
            } else {
                None
            }
        })
        .collect_vec();
    scan_stats.querier_files = index_file_names.len() as i64;
    let cache_type = cache_files(
        &query.trace_id,
        &index_file_names
            .iter()
            .map(|(ttv_file, _)| ttv_file.as_str())
            .collect_vec(),
        &mut scan_stats,
        "index",
    )
    .await?;

    let download_msg = if cache_type == file_data::CacheType::None {
        "".to_string()
    } else {
        format!("downloading others into {:?} in background,", cache_type)
    };
    log::info!(
        "[trace_id {}] search->tantivy: stream {}/{}/{}, load puffin index files {}, memory cached {}, disk cached {}, {download_msg} took: {} ms",
        query.trace_id,
        query.org_id,
        query.stream_type,
        query.stream_name,
        scan_stats.querier_files,
        scan_stats.querier_memory_cached_files,
        scan_stats.querier_disk_cached_files,
        start.elapsed().as_millis()
    );

    // set target partitions based on cache type
    let target_partitions = if cache_type == file_data::CacheType::None {
        cfg.limit.query_thread_num
    } else {
        cfg.limit.cpu_num
    };

    let search_start = std::time::Instant::now();
    let mut is_add_filter_back = file_list_map.len() != index_file_names.len();
    let time_range = query.time_range.unwrap_or((0, 0));
    let index_parquet_files = index_file_names.into_iter().map(|(_, f)| f).collect_vec();
    let (mut index_parquet_files, query_limit) =
        if let Some(InvertedIndexOptimizeMode::SimpleSelect(limit, _ascend)) = idx_optimize_rule {
            if limit > 0 {
                (
                    group_files_by_time_range(index_parquet_files, target_partitions),
                    limit,
                )
            } else {
                (
                    index_parquet_files.into_iter().map(|f| vec![f]).collect(),
                    0,
                )
            }
        } else {
            (
                index_parquet_files.into_iter().map(|f| vec![f]).collect(),
                0,
            )
        };

    let mut no_more_files = false;
    let mut total_hits = 0;
    let group_num = index_parquet_files.len();
    let max_group_len = index_parquet_files
        .iter()
        .map(|g| g.len())
        .max()
        .unwrap_or(0);
    for _ in 0..max_group_len {
        if no_more_files {
            // delete the rest of the files
            for i in 0..group_num {
                let Some(file) = index_parquet_files.get_mut(i).and_then(|g| {
                    if g.is_empty() {
                        None
                    } else {
                        Some(g.remove(g.len() - 1))
                    }
                }) else {
                    continue;
                };
                file_list_map.remove(&file.key);
            }
            continue;
        }

        // Spawn a task for each group of files get row_id from index
        let mut tasks = Vec::new();
        let semaphore = std::sync::Arc::new(Semaphore::new(target_partitions));
        for i in 0..group_num {
            let Some(file) = index_parquet_files.get_mut(i).and_then(|g| {
                if g.is_empty() {
                    None
                } else {
                    Some(g.remove(g.len() - 1))
                }
            }) else {
                continue;
            };
            let trace_id = query.trace_id.to_string();
            // Spawn a task for each file, wherein full text search and
            // secondary index search queries are executed
            let index_condition_clone = index_condition.clone();
            let idx_optimize_rule_clone = idx_optimize_rule.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let task = tokio::task::spawn(async move {
                let ret = search_tantivy_index(
                    &trace_id,
                    time_range,
                    index_condition_clone,
                    idx_optimize_rule_clone,
                    &file,
                )
                .await;
                drop(permit);
                ret
            });
            tasks.push(task)
        }

        // Wait for all tasks to complete
        for result in try_join_all(tasks)
            .await
            .map_err(|e| Error::ErrorCode(ErrorCodes::ServerInternalError(e.to_string())))?
        {
            let result: anyhow::Result<(String, Option<BitVec>, usize)> = result;
            // Each result corresponds to a file in the file list
            match result {
                Ok((file_name, bitvec, hits_in_file)) => {
                    total_hits += hits_in_file;
                    if file_name.is_empty() && bitvec.is_none() {
                        // no need inverted index for this file, need add filter back
                        is_add_filter_back = true;
                        continue;
                    }
                    if let Some(res) = bitvec {
                        log::debug!(
                            "[trace_id {}] search->tantivy: hits for index_condition: {:?} found {} in {}",
                            query.trace_id,
                            index_condition,
                            hits_in_file,
                            file_name
                        );
                        if hits_in_file == 0 {
                            // if the bitmap is empty then we remove the file from the list
                            file_list_map.remove(&file_name);
                        } else {
                            // Replace the segment IDs in the existing `FileKey` with the found
                            let file = file_list_map.get_mut(&file_name).unwrap();
                            file.with_segment_ids(res);
                        }
                    } else {
                        // if the bitmap is empty then we remove the file from the list
                        log::debug!(
                            "[trace_id {}] search->tantivy: no match found in index for file {}",
                            query.trace_id,
                            file_name
                        );
                        file_list_map.remove(&file_name);
                    }
                }
                Err(e) => {
                    log::error!(
                        "[trace_id {}] search->tantivy: error filtering via index. Keep file to search, error: {}",
                        query.trace_id,
                        e
                    );
                    is_add_filter_back = true;
                    continue;
                }
            }
        }
        // if limit is set and total hits exceed the limit, we stop searching
        if query_limit > 0 && total_hits > query_limit {
            no_more_files = true;
        }
    }

    log::info!(
        "[trace_id {}] search->tantivy: total hits for index_condition: {:?} found {} rows, is_add_filter_back: {}, file_num: {}, took: {} ms",
        query.trace_id,
        index_condition,
        total_hits,
        is_add_filter_back,
        file_list_map.len(),
        search_start.elapsed().as_millis()
    );
    file_list.extend(file_list_map.into_values());
    Ok((
        start.elapsed().as_millis() as usize,
        is_add_filter_back,
        total_hits,
    ))
}

pub async fn get_tantivy_directory(
    _trace_id: &str,
    file_name: &str,
    file_size: i64,
) -> anyhow::Result<PuffinDirReader> {
    let source = object_store::ObjectMeta {
        location: file_name.into(),
        last_modified: *BASE_TIME,
        size: file_size as usize,
        e_tag: None,
        version: None,
    };
    Ok(PuffinDirReader::from_path(source).await?)
}

async fn search_tantivy_index(
    trace_id: &str,
    time_range: (i64, i64),
    index_condition: Option<IndexCondition>,
    idx_optimize_rule: Option<InvertedIndexOptimizeMode>,
    parquet_file: &FileKey,
) -> anyhow::Result<(String, Option<BitVec>, usize)> {
    let Some(ttv_file_name) = convert_parquet_idx_file_name_to_tantivy_file(&parquet_file.key)
    else {
        return Err(anyhow::anyhow!(
            "[trace_id {trace_id}] search->storage: Unable to find tantivy index files for parquet file {}",
            parquet_file.key.clone()
        ));
    };

    // cache the indexer and reader
    let cfg = get_config();
    let indexer = if cfg.common.inverted_index_cache_enabled {
        reader_cache::GLOBAL_CACHE.get(&ttv_file_name)
    } else {
        None
    };
    let (tantivy_index, tantivy_reader) = match indexer {
        Some((indexer, reader)) => (indexer, reader),
        None => {
            log::debug!("init cache for puffin file: {}", ttv_file_name);
            let reader_directory: Box<dyn Directory> = if cfg.common.inverted_index_tantivy_mode
                == InvertedIndexTantivyMode::Mmap.to_string()
            {
                let puffin_dir_path = format!(
                    "{}{}",
                    cfg.common.data_cache_dir,
                    ttv_file_name.replace(FILE_EXT_TANTIVY, FILE_EXT_TANTIVY_FOLDER)
                );
                if !is_exists(&puffin_dir_path) {
                    let read_dir = get_tantivy_directory(
                        trace_id,
                        &ttv_file_name,
                        parquet_file.meta.index_size,
                    )
                    .await?;
                    let _ = convert_puffin_file_to_tantivy_dir(read_dir, &puffin_dir_path).await?;
                }
                Box::new(tantivy::directory::MmapDirectory::open(&puffin_dir_path)?)
            } else {
                let puffin_dir = Arc::new(
                    get_tantivy_directory(trace_id, &ttv_file_name, parquet_file.meta.index_size)
                        .await?,
                );
                let footer_cache = FooterCache::from_directory(puffin_dir.clone()).await?;
                let cache_dir =
                    CachingDirectory::new_with_cacher(puffin_dir, Arc::new(footer_cache));
                Box::new(cache_dir)
            };

            let index = tantivy::Index::open(reader_directory)?;
            index
                .tokenizers()
                .register(O2_TOKENIZER, o2_tokenizer_build());
            let reader = index
                .reader_builder()
                .reload_policy(tantivy::ReloadPolicy::Manual)
                .num_warming_threads(0)
                .try_into()?;
            let index = Arc::new(index);
            let reader = Arc::new(reader);
            if cfg.common.inverted_index_cache_enabled {
                reader_cache::GLOBAL_CACHE
                    .put(ttv_file_name.to_string(), (index.clone(), reader.clone()));
            }
            (index, reader)
        }
    };

    let tantivy_searcher = tantivy_reader.searcher();
    let tantivy_schema = tantivy_index.schema();
    let fts_field = tantivy_schema.get_field(INDEX_FIELD_NAME_FOR_ALL).ok();

    // generate the tantivy query
    let condition: IndexCondition = index_condition.ok_or(anyhow::anyhow!(
        "[trace_id {trace_id}] search->storage: IndexCondition not found"
    ))?;
    let query = condition.to_tantivy_query(tantivy_schema.clone(), fts_field)?;

    // warm up the terms in the query
    if cfg.common.inverted_index_tantivy_mode == InvertedIndexTantivyMode::Puffin.to_string() {
        let mut warm_terms: HashMap<tantivy::schema::Field, HashMap<tantivy::Term, bool>> =
            HashMap::new();
        query.query_terms(&mut |term, need_position| {
            let field = term.field();
            let entry = warm_terms.entry(field).or_default();
            entry.insert(term.clone(), need_position);
        });
        // if no terms are found in the query, warm up all fields
        if warm_terms.is_empty() {
            for field in condition.get_tantivy_fields() {
                let field = tantivy_schema.get_field(&field).unwrap();
                warm_terms.insert(field, HashMap::new());
            }
        }
        warm_up_terms(&tantivy_searcher, &warm_terms).await?;
    }

    // search the index
    let file_in_range =
        parquet_file.meta.min_ts <= time_range.1 && parquet_file.meta.max_ts >= time_range.0;
    let idx_optimize_rule_clone = idx_optimize_rule.clone();
    let matched_docs =
        tokio::task::spawn_blocking(move || match (file_in_range, idx_optimize_rule_clone) {
            (false, _) | (true, None) => tantivy_searcher
                .search(&query, &tantivy::collector::DocSetCollector)
                .map(|ret| (ret, 0)),
            (true, Some(InvertedIndexOptimizeMode::SimpleSelect(limit, ascend))) => {
                tantivy_searcher
                    .search(
                        &query,
                        &tantivy::collector::TopDocs::with_limit(limit).tweak_score(
                            move |_segment_reader: &tantivy::SegmentReader| {
                                move |doc_id: tantivy::DocId, _original_score: tantivy::Score| {
                                    if ascend {
                                        doc_id as i64
                                    } else {
                                        -(doc_id as i64)
                                    }
                                }
                            },
                        ),
                    )
                    .map(|ret| {
                        (
                            ret.into_iter().map(|(_, doc)| doc).collect::<HashSet<_>>(),
                            0,
                        )
                    })
            }
            (true, Some(InvertedIndexOptimizeMode::SimpleCount)) => tantivy_searcher
                .search(&query, &tantivy::collector::Count)
                .map(|ret| (HashSet::new(), ret)),
        })
        .await??;

    // return early if no matches in tantivy
    let (matched_docs, total_hits) = matched_docs;
    if total_hits > 0 {
        return Ok((parquet_file.key.to_string(), None, total_hits));
    }
    if matched_docs.is_empty() {
        return Ok((parquet_file.key.to_string(), None, 0));
    }
    // return early if the number of matched docs is too large
    if cfg.limit.inverted_index_skip_threshold > 0
        && matched_docs.len()
            > (parquet_file.meta.records as usize / 100 * cfg.limit.inverted_index_skip_threshold)
        && !matches!(
            idx_optimize_rule,
            Some(InvertedIndexOptimizeMode::SimpleCount)
        )
    {
        log::debug!(
            "matched docs over [{}/100] in tantivy index, skip this file: {}",
            cfg.limit.inverted_index_skip_threshold,
            parquet_file.key
        );
        return Ok(("".to_string(), None, 0));
    }

    // Prepare a vec of segment offsets
    // this is useful when there are more than one segments
    let seg_metas = tantivy_index
        .searchable_segment_metas()
        .context("Count segments")?;
    if seg_metas.len() > 1 {
        return Err(anyhow::anyhow!(
            "[trace_id {trace_id}] search->storage: Multiple segments in tantivy index not supported"
        ));
    }
    let mut res = BitVec::repeat(false, parquet_file.meta.records as usize);
    let matched_num = matched_docs.len();
    for doc in matched_docs {
        res.set(doc.doc_id as usize, true);
    }
    Ok((parquet_file.key.to_string(), Some(res), matched_num))
}

// Group files by time range
// use the min_ts & max_ts of the file.meta to group files and each group can't contains crossing
// time range files
fn group_files_by_time_range(mut files: Vec<FileKey>, partition_num: usize) -> Vec<Vec<FileKey>> {
    // sort files by max_ts in ascending order
    files.sort_unstable_by(|a, b| a.meta.max_ts.cmp(&b.meta.max_ts));
    // group by time range
    let mut file_groups_indices: Vec<Vec<FileKey>> = vec![];
    for file in files {
        let file_group_to_insert = file_groups_indices.iter_mut().find(|group| {
            file.meta.min_ts
                > group
                    .last()
                    .expect("groups should be nonempty at construction")
                    .meta
                    .max_ts
        });
        match file_group_to_insert {
            Some(group) => group.push(file),
            None => file_groups_indices.push(vec![file]),
        }
    }
    // regroup if the number of groups is less than expect partitions
    if file_groups_indices.len() >= partition_num {
        file_groups_indices
    } else {
        repartition_sorted_groups(file_groups_indices, partition_num)
    }
}

// 1. first get larger group
// 2. split larger groups based on odd and even numbers
// 3. loop until the group reaches the number of partitions
fn repartition_sorted_groups(
    mut groups: Vec<Vec<FileKey>>,
    partition_num: usize,
) -> Vec<Vec<FileKey>> {
    if groups.is_empty() {
        return groups;
    }

    while groups.len() < partition_num {
        let max_index = find_max_group_index(&groups);
        let max_group = groups.remove(max_index);

        // if the max group has less than 1 files, we don't split it further
        if max_group.len() <= 1 {
            groups.push(max_group);
            break;
        }

        // split max_group into odd and even groups
        let group_cap = (max_group.len() + 1) / 2;
        let mut odd_group = Vec::with_capacity(group_cap);
        let mut even_group = Vec::with_capacity(group_cap);

        for (idx, file) in max_group.into_iter().enumerate() {
            if idx % 2 == 0 {
                even_group.push(file);
            } else {
                odd_group.push(file);
            }
        }

        if !odd_group.is_empty() {
            groups.push(odd_group);
        }
        if !even_group.is_empty() {
            groups.push(even_group);
        }
    }

    groups
}

// find the index of the group with the most files
fn find_max_group_index(groups: &[Vec<FileKey>]) -> usize {
    groups
        .iter()
        .enumerate()
        .fold(0, |max_index, (idx, group)| {
            if group.len() > groups[max_index].len() {
                idx
            } else {
                max_index
            }
        })
}

#[cfg(test)]
mod tests {
    use config::meta::stream::FileMeta;

    use super::*;

    fn create_file_key(min_ts: i64, max_ts: i64) -> FileKey {
        FileKey {
            key: format!("file_{}_{}", min_ts, max_ts),
            meta: FileMeta {
                min_ts,
                max_ts,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_group_files_by_time_range() {
        let files = vec![
            create_file_key(1, 10),
            create_file_key(11, 20),
            create_file_key(21, 30),
            create_file_key(31, 40),
            create_file_key(41, 50),
        ];
        let partition_num = 3;
        let groups = group_files_by_time_range(files, partition_num);
        assert_eq!(groups.len(), 3);
    }

    #[test]
    fn test_group_files_by_time_range_with_overlap() {
        let files = vec![
            create_file_key(1, 10),
            create_file_key(5, 15),
            create_file_key(11, 20),
            create_file_key(18, 30),
            create_file_key(31, 40),
            create_file_key(41, 50),
        ];
        let partition_num = 2;
        let groups = group_files_by_time_range(files, partition_num);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_group_files_by_time_range_with_less_partitions() {
        let files = vec![create_file_key(1, 10), create_file_key(11, 20)];
        let partition_num = 3;
        let groups = group_files_by_time_range(files, partition_num);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_repartition_sorted_groups() {
        let groups = vec![
            vec![create_file_key(1, 10), create_file_key(11, 20)],
            vec![create_file_key(21, 30), create_file_key(31, 40)],
        ];
        let partition_num = 4;
        let repartitioned_groups = repartition_sorted_groups(groups, partition_num);
        assert_eq!(repartitioned_groups.len(), 4);
    }

    #[test]
    fn test_repartition_sorted_groups_with_large_group() {
        let groups = vec![vec![
            create_file_key(1, 10),
            create_file_key(11, 20),
            create_file_key(21, 30),
            create_file_key(31, 40),
            create_file_key(41, 50),
        ]];
        let partition_num = 3;
        let repartitioned_groups = repartition_sorted_groups(groups, partition_num);
        assert_eq!(repartitioned_groups.len(), 3);
    }

    #[test]
    fn test_find_max_group_index() {
        let groups = vec![
            vec![create_file_key(1, 10)],
            vec![create_file_key(11, 20), create_file_key(21, 30)],
            vec![create_file_key(31, 40)],
        ];
        let max_index = find_max_group_index(&groups);
        assert_eq!(max_index, 1);
    }
}
