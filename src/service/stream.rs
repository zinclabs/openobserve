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

use std::io::Error;

use actix_web::{http, http::StatusCode, HttpResponse};
use arrow_schema::DataType;
use config::{
    is_local_disk_storage,
    meta::{
        promql,
        stream::{
            DistinctField, StreamParams, StreamSettings, StreamStats, StreamType,
            UpdateStreamSettings,
        },
    },
    utils::{json, time::now_micros},
    SIZE_IN_MB, SQL_FULL_TEXT_SEARCH_FIELDS,
};
use datafusion::arrow::datatypes::Schema;
use hashbrown::HashMap;
use infra::{
    cache::stats,
    schema::{
        unwrap_partition_time_level, unwrap_stream_settings, STREAM_RECORD_ID_GENERATOR,
        STREAM_SCHEMAS, STREAM_SCHEMAS_COMPRESSED, STREAM_SCHEMAS_LATEST, STREAM_SETTINGS,
    },
    table::distinct_values::{check_field_use, DistinctFieldRecord, OriginType},
};

use crate::{
    common::meta::{
        authz::Authz,
        http::HttpResponse as MetaHttpResponse,
        stream::{Stream, StreamProperty},
    },
    service::{db, db::distinct_values, metrics::get_prom_metadata_from_schema},
};

const LOCAL: &str = "disk";
const S3: &str = "s3";

pub async fn get_stream(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    page_size_and_idx: Option<(usize, usize)>,
) -> Result<HttpResponse, Error> {
    let schema = infra::schema::get(org_id, stream_name, stream_type)
        .await
        .unwrap();

    let mut stats = stats::get_stream_stats(org_id, stream_name, stream_type);
    transform_stats(&mut stats);
    if schema != Schema::empty() {
        let stream = stream_res(
            stream_name,
            stream_type,
            schema,
            Some(stats),
            page_size_and_idx,
        );
        Ok(HttpResponse::Ok().json(stream))
    } else {
        Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            StatusCode::NOT_FOUND.into(),
            "stream not found".to_string(),
        )))
    }
}

pub async fn get_streams(
    org_id: &str,
    stream_type: Option<StreamType>,
    fetch_schema: bool,
    permitted_streams: Option<Vec<String>>,
) -> Vec<Stream> {
    let indices = db::schema::list(org_id, stream_type, fetch_schema)
        .await
        .unwrap_or_default();

    let filtered_indices = if let Some(s_type) = stream_type {
        let s_type = match s_type {
            StreamType::EnrichmentTables => "enrichment_table",
            _ => s_type.as_str(),
        };
        match permitted_streams {
            Some(permitted_streams) => {
                if permitted_streams.contains(&format!("{}:_all_{}", s_type, org_id)) {
                    indices
                } else {
                    indices
                        .into_iter()
                        .filter(|stream_loc| {
                            permitted_streams
                                .contains(&format!("{}:{}", s_type, stream_loc.stream_name))
                        })
                        .collect::<Vec<_>>()
                }
            }
            None => indices,
        }
    } else {
        indices
    };
    let mut indices_res = Vec::with_capacity(filtered_indices.len());
    for stream_loc in filtered_indices {
        let mut stats = stats::get_stream_stats(
            org_id,
            stream_loc.stream_name.as_str(),
            stream_loc.stream_type,
        );
        if stats.eq(&StreamStats::default()) {
            indices_res.push(stream_res(
                stream_loc.stream_name.as_str(),
                stream_loc.stream_type,
                stream_loc.schema,
                None,
                None,
            ));
        } else {
            transform_stats(&mut stats);
            indices_res.push(stream_res(
                stream_loc.stream_name.as_str(),
                stream_loc.stream_type,
                stream_loc.schema,
                Some(stats),
                None,
            ));
        }
    }
    indices_res
}

pub fn stream_res(
    stream_name: &str,
    stream_type: StreamType,
    schema: Schema,
    stats: Option<StreamStats>,
    page_size_and_idx: Option<(usize, usize)>,
) -> Stream {
    let storage_type = if is_local_disk_storage() { LOCAL } else { S3 };
    let mappings = {
        let iter = schema.fields().iter().map(|field| StreamProperty {
            prop_type: field.data_type().to_string(),
            name: field.name().to_string(),
        });
        if let Some((page_size, page_idx)) = page_size_and_idx {
            iter.skip(page_idx * page_size).take(page_size).collect()
        } else {
            iter.collect()
        }
    };

    let mut stats = stats.unwrap_or_default();
    stats.created_at = stream_created(&schema).unwrap_or_default();

    let metrics_meta = if stream_type == StreamType::Metrics {
        let mut meta = get_prom_metadata_from_schema(&schema).unwrap_or(promql::Metadata {
            metric_type: promql::MetricType::Empty,
            metric_family_name: stream_name.to_string(),
            help: stream_name.to_string(),
            unit: "".to_string(),
        });
        if meta.metric_type == promql::MetricType::Empty
            && (stream_name.ends_with("_bucket")
                || stream_name.ends_with("_sum")
                || stream_name.ends_with("_count"))
        {
            meta.metric_type = promql::MetricType::Counter;
        }
        Some(meta)
    } else {
        None
    };

    let mut settings = unwrap_stream_settings(&schema).unwrap_or_default();
    settings.partition_time_level = Some(unwrap_partition_time_level(
        settings.partition_time_level,
        stream_type,
    ));

    Stream {
        name: stream_name.to_string(),
        storage_type: storage_type.to_string(),
        stream_type,
        schema: mappings,
        stats,
        settings,
        metrics_meta,
    }
}

#[tracing::instrument(skip(settings))]
pub async fn save_stream_settings(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    mut settings: StreamSettings,
) -> Result<HttpResponse, Error> {
    let cfg = config::get_config();
    // check if we are allowed to ingest
    if db::compact::retention::is_deleting_stream(org_id, stream_type, stream_name, None) {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                format!("stream [{stream_name}] is being deleted"),
            )),
        );
    }

    // only allow setting user defined schema for logs stream
    if stream_type != StreamType::Logs
        && settings.defined_schema_fields.is_some()
        && !settings.defined_schema_fields.as_ref().unwrap().is_empty()
    {
        return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            "only logs stream can have user defined schema".to_string(),
        )));
    }

    // _all field can't setting for inverted index & index field
    for key in settings.full_text_search_keys.iter() {
        if key == &cfg.common.column_all {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("field [{}] can't be used for full text search", key),
            )));
        }
    }
    for key in settings.index_fields.iter() {
        if key == &cfg.common.column_all {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("field [{}] can't be used for secondary index", key),
            )));
        }
    }

    for key in settings.partition_keys.iter() {
        if SQL_FULL_TEXT_SEARCH_FIELDS.contains(&key.field) || key.field == cfg.common.column_all {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("field [{}] can't be used for partition key", key.field),
            )));
        }
    }

    // get schema
    let schema = match infra::schema::get(org_id, stream_name, stream_type).await {
        Ok(schema) => schema,
        Err(e) => {
            return Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                    format!("error in getting schema : {e}"),
                )),
            );
        }
    };
    let schema_fields = schema
        .fields()
        .iter()
        .map(|f| (f.name(), f))
        .collect::<HashMap<_, _>>();

    // check the full text search keys must be text field
    for key in settings.full_text_search_keys.iter() {
        let Some(field) = schema_fields.get(key) else {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("field [{}] not found in schema", key),
            )));
        };
        if field.data_type() != &DataType::Utf8 {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                format!("full text search field [{}] must be text field", key),
            )));
        }
    }

    // we need to keep the old partition information, because the hash bucket num can't be changed
    // get old settings and then update partition_keys
    let mut old_partition_keys = unwrap_stream_settings(&schema)
        .unwrap_or_default()
        .partition_keys;
    // first disable all old partition keys
    for v in old_partition_keys.iter_mut() {
        v.disabled = true;
    }
    // then update new partition keys
    for v in settings.partition_keys.iter() {
        if let Some(old_field) = old_partition_keys.iter_mut().find(|k| k.field == v.field) {
            if old_field.types != v.types {
                return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                    http::StatusCode::BAD_REQUEST.into(),
                    format!("field [{}] partition types can't be changed", v.field),
                )));
            }
            old_field.disabled = v.disabled;
        } else {
            old_partition_keys.push(v.clone());
        }
    }
    settings.partition_keys = old_partition_keys;

    for range in settings.extended_retention_days.iter() {
        if range.start > range.end {
            return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "start day should be less than end day".to_string(),
            )));
        }
    }

    let mut metadata = schema.metadata.clone();
    metadata.insert("settings".to_string(), json::to_string(&settings).unwrap());
    if !metadata.contains_key("created_at") {
        metadata.insert(
            "created_at".to_string(),
            chrono::Utc::now().timestamp_micros().to_string(),
        );
    }
    db::schema::update_setting(org_id, stream_name, stream_type, metadata)
        .await
        .unwrap();

    Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
        http::StatusCode::OK.into(),
        "".to_string(),
    )))
}

#[tracing::instrument(skip(new_settings))]
pub async fn update_stream_settings(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    new_settings: UpdateStreamSettings,
) -> Result<HttpResponse, Error> {
    let cfg = config::get_config();
    match infra::schema::get_settings(org_id, stream_name, stream_type).await {
        Some(mut settings) => {
            if let Some(max_query_range) = new_settings.max_query_range {
                settings.max_query_range = max_query_range;
            }
            if let Some(store_original_data) = new_settings.store_original_data {
                settings.store_original_data = store_original_data;
            }
            if let Some(approx_partition) = new_settings.approx_partition {
                settings.approx_partition = approx_partition;
            }

            if let Some(flatten_level) = new_settings.flatten_level {
                settings.flatten_level = Some(flatten_level);
            }

            if let Some(data_retention) = new_settings.data_retention {
                settings.data_retention = data_retention;
            }

            // check for user defined schema
            if !new_settings.defined_schema_fields.add.is_empty() {
                settings.defined_schema_fields =
                    if let Some(mut schema_fields) = settings.defined_schema_fields {
                        schema_fields.extend(new_settings.defined_schema_fields.add);
                        Some(schema_fields)
                    } else {
                        Some(new_settings.defined_schema_fields.add)
                    }
            }
            if !new_settings.defined_schema_fields.remove.is_empty() {
                if let Some(schema_fields) = settings.defined_schema_fields.as_mut() {
                    schema_fields
                        .retain(|field| !new_settings.defined_schema_fields.remove.contains(field));
                }
            }
            if let Some(schema_fields) = settings.defined_schema_fields.as_ref() {
                if schema_fields.len() > cfg.limit.user_defined_schema_max_fields {
                    return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
                        http::StatusCode::BAD_REQUEST.into(),
                        format!(
                            "user defined schema fields count exceeds the limit: {}",
                            cfg.limit.user_defined_schema_max_fields
                        ),
                    )));
                }
            }

            // check for bloom filter fields
            if !new_settings.bloom_filter_fields.add.is_empty() {
                settings
                    .bloom_filter_fields
                    .extend(new_settings.bloom_filter_fields.add);
            }
            if !new_settings.bloom_filter_fields.remove.is_empty() {
                settings
                    .bloom_filter_fields
                    .retain(|field| !new_settings.bloom_filter_fields.remove.contains(field));
            }

            // check for index fields
            if !new_settings.index_fields.add.is_empty() {
                settings.index_fields.extend(new_settings.index_fields.add);
                settings.index_updated_at = now_micros();
            }
            if !new_settings.index_fields.remove.is_empty() {
                settings
                    .index_fields
                    .retain(|field| !new_settings.index_fields.remove.contains(field));
            }

            if !new_settings.extended_retention_days.add.is_empty() {
                settings
                    .extended_retention_days
                    .extend(new_settings.extended_retention_days.add);
            }

            if !new_settings.extended_retention_days.remove.is_empty() {
                settings
                    .extended_retention_days
                    .retain(|range| !new_settings.extended_retention_days.remove.contains(range));
            }

            if !new_settings.distinct_value_fields.add.is_empty() {
                for f in &new_settings.distinct_value_fields.add {
                    // we ignore full text search fields
                    if settings.full_text_search_keys.contains(f)
                        || new_settings.full_text_search_keys.add.contains(f)
                    {
                        continue;
                    }
                    let record = DistinctFieldRecord::new(
                        OriginType::Stream,
                        stream_name,
                        org_id,
                        stream_name,
                        stream_type.to_string(),
                        f,
                    );
                    if let Err(e) = distinct_values::add(record).await {
                        return Ok(HttpResponse::InternalServerError().json(
                            MetaHttpResponse::error(
                                http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                                format!("error in updating settings : {e}"),
                            ),
                        ));
                    }
                    // we cannot allow duplicate entries here
                    let temp = DistinctField {
                        name: f.to_owned(),
                        added_ts: chrono::Utc::now().timestamp_micros(),
                    };
                    if !settings.distinct_value_fields.contains(&temp) {
                        settings.distinct_value_fields.push(temp);
                    }
                }
            }

            if !new_settings.distinct_value_fields.remove.is_empty() {
                for f in &new_settings.distinct_value_fields.remove {
                    let usage =
                        match check_field_use(org_id, stream_name, stream_type.as_str(), f).await {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(HttpResponse::InternalServerError().json(
                                    MetaHttpResponse::error(
                                        http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                                        format!("error in updating settings : {e}"),
                                    ),
                                ));
                            }
                        };
                    // if there are multiple uses, we cannot allow it to be removed
                    if usage.len() > 1 {
                        return Ok(HttpResponse::BadRequest().json(
                            MetaHttpResponse::error(
                                http::StatusCode::BAD_REQUEST.into(),
                                format!("error in removing distinct field : field {f} if used in dashboards/reports"),
                            ),
                        ));
                    }
                    // here we can be sure that usage is at most 1 record
                    if let Some(entry) = usage.first() {
                        if entry.origin != OriginType::Stream {
                            return Ok(HttpResponse::BadRequest().json(
                                MetaHttpResponse::error(
                                    http::StatusCode::BAD_REQUEST.into(),
                                    format!("error in removing distinct field : field {f} if used in dashboards/reports"),
                                ),
                            ));
                        }
                    }
                }
                // here we are sure that all fields to be removed can be removed,
                // so we bulk filter
                settings.distinct_value_fields.retain(|field| {
                    !new_settings
                        .distinct_value_fields
                        .remove
                        .contains(&field.name)
                });
            }

            if !new_settings.full_text_search_keys.add.is_empty() {
                settings
                    .full_text_search_keys
                    .extend(new_settings.full_text_search_keys.add);
                settings.index_updated_at = now_micros();
            }

            if !new_settings.full_text_search_keys.remove.is_empty() {
                settings
                    .full_text_search_keys
                    .retain(|field| !new_settings.full_text_search_keys.remove.contains(field));
            }

            if !new_settings.partition_keys.add.is_empty() {
                settings
                    .partition_keys
                    .extend(new_settings.partition_keys.add);
            }

            if !new_settings.partition_keys.remove.is_empty() {
                settings
                    .partition_keys
                    .retain(|field| !new_settings.partition_keys.remove.contains(field));
            }

            if let Some(partition_time_level) = new_settings.partition_time_level {
                settings.partition_time_level = Some(partition_time_level);
            }
            save_stream_settings(org_id, stream_name, stream_type, settings).await
        }
        None => Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            "stream settings could not be found".to_string(),
        ))),
    }
}

#[tracing::instrument]
pub async fn delete_stream(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
) -> Result<HttpResponse, Error> {
    let schema = infra::schema::get_versions(org_id, stream_name, stream_type, None)
        .await
        .unwrap();
    if schema.is_empty() {
        return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            StatusCode::NOT_FOUND.into(),
            "stream not found".to_string(),
        )));
    }

    // create delete for compactor
    if let Err(e) =
        db::compact::retention::delete_stream(org_id, stream_type, stream_name, None).await
    {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR.into(),
                format!("failed to delete stream: {e}"),
            )),
        );
    }

    // delete stream schema
    if let Err(e) = db::schema::delete(org_id, stream_name, Some(stream_type)).await {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR.into(),
                format!("failed to delete stream: {e}"),
            )),
        );
    }

    // delete stream schema cache
    let key = format!("{org_id}/{stream_type}/{stream_name}");
    let mut w = STREAM_SCHEMAS.write().await;
    w.remove(&key);
    drop(w);
    let mut w = STREAM_SCHEMAS_COMPRESSED.write().await;
    w.remove(&key);
    drop(w);
    let mut w = STREAM_SCHEMAS_LATEST.write().await;
    w.remove(&key);
    drop(w);

    // delete stream settings cache
    let mut w = STREAM_SETTINGS.write().await;
    w.remove(&key);
    drop(w);

    // delete stream record id generator cache
    {
        STREAM_RECORD_ID_GENERATOR.remove(&key);
    }

    // delete stream compaction offset
    if let Err(e) = db::compact::files::del_offset(org_id, stream_type, stream_name).await {
        return Ok(
            HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                StatusCode::INTERNAL_SERVER_ERROR.into(),
                format!("failed to delete stream: {e}"),
            )),
        );
    };

    // delete associated pipelines
    if let Some(pipeline) =
        db::pipeline::get_by_stream(&StreamParams::new(org_id, stream_name, stream_type)).await
    {
        if let Err(e) = db::pipeline::delete(&pipeline.id).await {
            return Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    format!(
                        "Stream deletion fail: failed to delete the associated pipeline {}: {e}",
                        pipeline.name
                    ),
                )),
            );
        }
    }

    crate::common::utils::auth::remove_ownership(
        org_id,
        stream_type.as_str(),
        Authz::new(stream_name),
    )
    .await;

    Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
        StatusCode::OK.into(),
        "stream deleted".to_string(),
    )))
}

fn transform_stats(stats: &mut StreamStats) {
    stats.storage_size /= SIZE_IN_MB;
    stats.compressed_size /= SIZE_IN_MB;
    stats.index_size /= SIZE_IN_MB;
}

pub fn stream_created(schema: &Schema) -> Option<i64> {
    schema
        .metadata()
        .get("created_at")
        .map(|v| v.parse().unwrap())
}

pub async fn delete_fields(
    org_id: &str,
    stream_name: &str,
    stream_type: Option<StreamType>,
    fields: &[String],
) -> Result<(), anyhow::Error> {
    if fields.is_empty() {
        return Ok(());
    }
    db::schema::delete_fields(
        org_id,
        stream_name,
        stream_type.unwrap_or_default(),
        fields.to_vec(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use datafusion::arrow::datatypes::{DataType, Field};

    use super::*;

    #[test]
    fn test_stream_res() {
        let stats = StreamStats::default();
        let schema = Schema::new(vec![Field::new("f.c", DataType::Int32, false)]);
        let res = stream_res("Test", StreamType::Logs, schema, Some(stats.clone()), None);
        assert_eq!(res.stats, stats);
    }
}
