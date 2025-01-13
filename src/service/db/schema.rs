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

use std::sync::Arc;

use arrow_schema::{Field, Schema};
use bytes::Bytes;
use config::{
    cluster::LOCAL_NODE_ID,
    get_config,
    ider::SnowflakeIdGenerator,
    is_local_disk_storage,
    meta::{cluster::RoleGroup, stream::StreamType},
    utils::json,
};
use hashbrown::{HashMap, HashSet};
use infra::{
    cache,
    schema::{
        unwrap_stream_settings, SchemaCache, STREAM_RECORD_ID_GENERATOR, STREAM_SCHEMAS,
        STREAM_SCHEMAS_COMPRESSED, STREAM_SCHEMAS_LATEST, STREAM_SETTINGS,
    },
};
#[cfg(feature = "enterprise")]
use {
    infra::{errors::Error, schema::mk_key},
    o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config,
};

use crate::{
    common::{
        infra::{
            cluster::get_cached_online_querier_nodes,
            config::{ENRICHMENT_TABLES, ORGANIZATIONS},
        },
        meta::stream::StreamSchema,
    },
    service::{db, enrichment::StreamTable, organization::check_and_create_org},
};

pub async fn merge(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    schema: &Schema,
    min_ts: Option<i64>,
) -> Result<Option<(Schema, Vec<Field>)>, anyhow::Error> {
    let ret = infra::schema::merge(org_id, stream_name, stream_type, schema, min_ts).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        let key = mk_key(org_id, stream_type, stream_name);
        o2_enterprise::enterprise::super_cluster::queue::schema_merge(
            &key,
            json::to_vec(&schema).unwrap().into(),
            infra::db::NEED_WATCH,
            min_ts,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(ret)
}

pub async fn update_setting(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    metadata: std::collections::HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    infra::schema::update_setting(org_id, stream_name, stream_type, metadata.clone()).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        let key = mk_key(org_id, stream_type, stream_name);
        o2_enterprise::enterprise::super_cluster::queue::schema_setting(
            &key,
            json::to_vec(&metadata).unwrap().into(),
            infra::db::NEED_WATCH,
            None,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(())
}

pub async fn delete_fields(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    deleted_fields: Vec<String>,
) -> Result<(), anyhow::Error> {
    infra::schema::delete_fields(org_id, stream_name, stream_type, deleted_fields.clone()).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        let key = mk_key(org_id, stream_type, stream_name);
        o2_enterprise::enterprise::super_cluster::queue::schema_delete_fields(
            &key,
            json::to_vec(&deleted_fields).unwrap().into(),
            infra::db::NEED_WATCH,
            None,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(())
}

pub async fn delete(
    org_id: &str,
    stream_name: &str,
    stream_type: Option<StreamType>,
) -> Result<(), anyhow::Error> {
    let stream_type = stream_type.unwrap_or(StreamType::Logs);
    infra::schema::delete(org_id, stream_type, stream_name, None).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        let key = mk_key(org_id, stream_type, stream_name);
        o2_enterprise::enterprise::super_cluster::queue::delete(
            &key,
            false,
            infra::db::NEED_WATCH,
            None,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(())
}

async fn list_stream_schemas(
    org_id: &str,
    stream_type: Option<StreamType>,
    fetch_schema: bool,
) -> Vec<StreamSchema> {
    let r = STREAM_SCHEMAS_LATEST.read().await;
    if r.is_empty() {
        return vec![];
    }

    let prefix = match stream_type {
        None => format!("{org_id}/"),
        Some(stream_type) => format!("{org_id}/{stream_type}/"),
    };
    r.iter()
        .filter_map(|(key, val)| {
            key.strip_prefix(&prefix).map(|key| {
                let (stream_type, stream_name) = match stream_type {
                    Some(stream_type) => (stream_type, key.into()),
                    None => {
                        let columns = key.split('/').take(2).collect::<Vec<_>>();
                        assert_eq!(columns.len(), 2, "BUG");
                        (columns[0].into(), columns[1].into())
                    }
                };
                StreamSchema {
                    stream_name,
                    stream_type,
                    schema: if fetch_schema {
                        val.schema().as_ref().clone()
                    } else {
                        Schema::empty()
                    },
                }
            })
        })
        .collect()
}

pub async fn list(
    org_id: &str,
    stream_type: Option<StreamType>,
    fetch_schema: bool,
) -> Result<Vec<StreamSchema>, anyhow::Error> {
    let r = STREAM_SCHEMAS_LATEST.read().await;
    if !r.is_empty() {
        drop(r);
        return Ok(list_stream_schemas(org_id, stream_type, fetch_schema).await);
    }

    let db_key = match stream_type {
        None => format!("/schema/{org_id}/"),
        Some(stream_type) => format!("/schema/{org_id}/{stream_type}/"),
    };
    let items = db::list(&db_key).await?;
    let mut schemas: HashMap<(String, StreamType), Vec<(Bytes, i64)>> =
        HashMap::with_capacity(items.len());
    for (key, val) in items {
        let key = key.strip_prefix(&db_key).unwrap();
        let (stream_type, stream_name, start_dt) = match stream_type {
            Some(stream_type) => {
                let columns = key.split('/').take(2).collect::<Vec<_>>();
                assert_eq!(columns.len(), 2, "BUG");
                (stream_type, columns[0].into(), columns[1].parse().unwrap())
            }
            None => {
                let columns = key.split('/').take(3).collect::<Vec<_>>();
                assert_eq!(columns.len(), 3, "BUG");
                (
                    columns[0].into(),
                    columns[1].into(),
                    columns[2].parse().unwrap(),
                )
            }
        };
        let entry = schemas
            .entry((stream_name, stream_type))
            .or_insert(Vec::new());
        entry.push((val, start_dt));
    }
    Ok(schemas
        .into_iter()
        .map(|((stream_name, stream_type), versions)| StreamSchema {
            stream_name,
            stream_type,
            schema: if fetch_schema {
                versions
                    .iter()
                    .max_by_key(|(_, start_dt)| *start_dt)
                    .map(|(val, _)| {
                        if fetch_schema {
                            let mut schema: Vec<Schema> = json::from_slice(val).unwrap();
                            if !schema.is_empty() {
                                schema.remove(schema.len() - 1)
                            } else {
                                Schema::empty()
                            }
                        } else {
                            Schema::empty()
                        }
                    })
                    .unwrap()
            } else {
                Schema::empty()
            },
        })
        .collect())
}

pub async fn watch() -> Result<(), anyhow::Error> {
    #[cfg(feature = "enterprise")]
    let audit_enabled = get_o2_config().common.audit_enabled;
    #[cfg(not(feature = "enterprise"))]
    let audit_enabled = false;
    let key = "/schema/";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching stream schema");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_stream_schema: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let key_columns = ev.key.split('/').collect::<Vec<&str>>();
                let (ev_key, ev_start_dt) = if key_columns.len() > 5 {
                    (
                        key_columns[..5].join("/"),
                        key_columns[5].parse::<i64>().unwrap_or(0),
                    )
                } else {
                    (ev.key.to_string(), ev.start_dt.unwrap_or_default())
                };

                let item_key = ev_key.strip_prefix(key).unwrap();
                let r = STREAM_SCHEMAS.read().await;
                let prev_start_dt = if let Some(schemas) = r.get(&item_key.to_owned()) {
                    let idx = if schemas.len() >= 2 {
                        schemas.len() - 2
                    } else {
                        0
                    };
                    schemas[idx].0
                } else {
                    0
                };
                drop(r);
                let ts_range = if ev_start_dt == 0 && prev_start_dt == 0 {
                    None
                } else if ev_start_dt == 0 || (prev_start_dt > 0 && ev_start_dt > prev_start_dt) {
                    Some((prev_start_dt, chrono::Utc::now().timestamp_micros()))
                } else {
                    Some((ev_start_dt, chrono::Utc::now().timestamp_micros()))
                };

                let mut schema_versions =
                    match db::list_values_by_start_dt(&format!("{ev_key}/"), ts_range).await {
                        Ok(val) => val,
                        Err(e) => {
                            log::error!("Error getting value: {}", e);
                            continue;
                        }
                    };
                if schema_versions.is_empty() {
                    continue;
                }
                let latest_start_dt = schema_versions.last().unwrap().0;
                let mut latest_schema: Vec<Schema> =
                    match json::from_slice(&schema_versions.last().unwrap().1) {
                        Ok(val) => val,
                        Err(e) => {
                            log::error!("Error parsing schema, key: {}, error: {}", item_key, e);
                            continue;
                        }
                    };
                if latest_schema.is_empty() {
                    continue;
                }
                let latest_schema = latest_schema.pop().unwrap();
                let settings = unwrap_stream_settings(&latest_schema).unwrap_or_default();
                if settings.store_original_data {
                    if let dashmap::Entry::Vacant(entry) =
                        STREAM_RECORD_ID_GENERATOR.entry(item_key.to_string())
                    {
                        entry.insert(SnowflakeIdGenerator::new(unsafe { LOCAL_NODE_ID }));
                    }
                }
                let mut w = STREAM_SETTINGS.write().await;
                w.insert(item_key.to_string(), settings);
                drop(w);
                let mut w = STREAM_SCHEMAS_LATEST.write().await;
                w.insert(
                    item_key.to_string(),
                    SchemaCache::new(latest_schema.clone()),
                );
                drop(w);
                let cfg = get_config();
                if cfg.common.schema_cache_compress_enabled {
                    let schema_versions = schema_versions
                        .into_iter()
                        .map(|(start_dt, data)| {
                            let en_data = zstd::encode_all(data.as_ref(), 3).unwrap();
                            (start_dt, en_data.into())
                        })
                        .collect::<Vec<_>>();
                    let mut w = STREAM_SCHEMAS_COMPRESSED.write().await;
                    w.entry(item_key.to_string())
                        .and_modify(|existing_vec| {
                            existing_vec
                                .retain(|(v, _)| schema_versions.iter().all(|(v1, _)| v1 != v));
                            existing_vec.extend(schema_versions.clone())
                        })
                        .or_insert(schema_versions);
                    w.shrink_to_fit();
                    drop(w);
                } else {
                    // remove latest, already parsed it
                    _ = schema_versions.pop().unwrap();
                    // parse other versions
                    let schema_versions = itertools::chain(
                        schema_versions.into_iter().map(|(start_dt, data)| {
                            (
                                start_dt,
                                json::from_slice::<Vec<Schema>>(&data)
                                    .unwrap()
                                    .pop()
                                    .unwrap(),
                            )
                        }),
                        // add latest version here
                        vec![(latest_start_dt, latest_schema)],
                    )
                    .collect::<Vec<_>>();
                    let mut w = STREAM_SCHEMAS.write().await;
                    w.entry(item_key.to_string())
                        .and_modify(|existing_vec| {
                            existing_vec
                                .retain(|(v, _)| schema_versions.iter().all(|(v1, _)| v1 != v));
                            existing_vec.extend(schema_versions.clone())
                        })
                        .or_insert(schema_versions);
                    drop(w);
                }

                let keys = item_key.split('/').collect::<Vec<&str>>();
                let org_id = keys[0];
                let stream_type = StreamType::from(keys[1]);
                let stream_name = keys[2];

                if stream_type.eq(&StreamType::EnrichmentTables) {
                    let data = super::enrichment_table::get(org_id, stream_name)
                        .await
                        .unwrap();
                    ENRICHMENT_TABLES.insert(
                        item_key.to_owned(),
                        StreamTable {
                            org_id: org_id.to_string(),
                            stream_name: stream_name.to_string(),
                            data,
                        },
                    );
                }

                // if create_org_through_ingestion is enabled, we need to create the org
                // if it doesn't exist. Hence, we need to check if the org exists in the cache
                if (cfg.common.create_org_through_ingestion
                    || cfg.common.usage_enabled
                    || audit_enabled)
                    && !ORGANIZATIONS.read().await.contains_key(org_id)
                {
                    if let Err(e) = check_and_create_org(org_id).await {
                        log::error!("Failed to save organization in database: {}", e);
                    }
                }
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let columns = item_key.split('/').collect::<Vec<&str>>();
                let org_id = columns[0];
                let stream_type = StreamType::from(columns[1]);
                let stream_name = columns[2];
                let start_dt = match columns.get(3) {
                    Some(start_dt) => start_dt.parse::<i64>().unwrap_or_default(),
                    None => 0,
                };
                if start_dt > 0 {
                    // delete only one version
                    continue;
                }
                let mut w = STREAM_SCHEMAS.write().await;
                w.remove(item_key);
                w.shrink_to_fit();
                drop(w);
                let mut w = STREAM_SCHEMAS_COMPRESSED.write().await;
                w.remove(item_key);
                w.shrink_to_fit();
                drop(w);
                let mut w = STREAM_SCHEMAS_LATEST.write().await;
                w.remove(item_key);
                w.shrink_to_fit();
                drop(w);
                {
                    STREAM_RECORD_ID_GENERATOR.remove(item_key);
                    STREAM_RECORD_ID_GENERATOR.shrink_to_fit();
                }
                let mut w = STREAM_SETTINGS.write().await;
                w.remove(item_key);
                w.shrink_to_fit();
                drop(w);
                cache::stats::remove_stream_stats(org_id, stream_name, stream_type);
                if let Err(e) =
                    super::compact::files::del_offset(org_id, stream_type, stream_name).await
                {
                    log::error!("del_offset: {}", e);
                }

                if stream_type.eq(&StreamType::EnrichmentTables) && is_local_disk_storage() {
                    let data_dir = format!(
                        "{}files/{org_id}/{stream_type}/{stream_name}",
                        get_config().common.data_wal_dir
                    );
                    let path = std::path::Path::new(&data_dir);
                    if path.exists() {
                        if let Err(e) = tokio::fs::remove_dir_all(path).await {
                            log::error!("remove_dir_all: {}", e);
                        };
                    }
                }
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let db_key = "/schema/";
    let items = db::list(db_key).await?;
    let mut schemas: HashMap<String, Vec<(i64, Bytes)>> = HashMap::with_capacity(items.len());
    for (key, val) in items {
        let key = key.strip_prefix(db_key).unwrap();
        let columns = key.split('/').take(4).collect::<Vec<_>>();
        assert_eq!(columns.len(), 4, "BUG");
        let item_key = format!("{}/{}/{}", columns[0], columns[1], columns[2]);
        let start_dt: i64 = columns[3].parse().unwrap();
        let entry = schemas.entry(item_key).or_insert(Vec::new());
        entry.push((start_dt, val));
    }
    let keys = schemas.keys().map(|k| k.to_string()).collect::<Vec<_>>();
    for item_key in keys.iter() {
        let Some(mut schema_versions) = schemas.remove(item_key) else {
            continue;
        };
        if schema_versions.is_empty() {
            continue;
        }
        schema_versions.sort_by(|a, b| a.0.cmp(&b.0));
        let latest_schema: Vec<Schema> = json::from_slice(&schema_versions.last().unwrap().1)
            .map_err(|e| {
                anyhow::anyhow!("Error parsing schema, key: {}, error: {}", item_key, e)
            })?;
        if latest_schema.is_empty() {
            continue;
        }
        let latest_schema = latest_schema.last().unwrap();
        let settings = unwrap_stream_settings(latest_schema).unwrap_or_default();
        if settings.store_original_data {
            if let dashmap::Entry::Vacant(entry) =
                STREAM_RECORD_ID_GENERATOR.entry(item_key.to_string())
            {
                entry.insert(SnowflakeIdGenerator::new(unsafe { LOCAL_NODE_ID }));
            }
        }
        let mut w = STREAM_SETTINGS.write().await;
        w.insert(item_key.to_string(), settings);
        drop(w);
        let mut w = STREAM_SCHEMAS_LATEST.write().await;
        w.insert(
            item_key.to_string(),
            SchemaCache::new(latest_schema.clone()),
        );
        drop(w);
        if get_config().common.schema_cache_compress_enabled {
            let schema_versions = schema_versions
                .into_iter()
                .map(|(start_dt, data)| {
                    let en_data = zstd::encode_all(data.as_ref(), 3).unwrap();
                    (start_dt, en_data.into())
                })
                .collect::<Vec<_>>();
            let mut w = STREAM_SCHEMAS_COMPRESSED.write().await;
            w.insert(item_key.to_string(), schema_versions);
            drop(w);
        } else {
            let schema_versions = schema_versions
                .into_iter()
                .map(|(start_dt, data)| {
                    (
                        start_dt,
                        json::from_slice::<Vec<Schema>>(&data)
                            .unwrap()
                            .pop()
                            .unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            let mut w = STREAM_SCHEMAS.write().await;
            w.insert(item_key.to_string(), schema_versions);
            drop(w);
        }
    }
    log::info!("Stream schemas Cached");
    Ok(())
}

pub async fn cache_enrichment_tables() -> Result<(), anyhow::Error> {
    let r = STREAM_SCHEMAS_LATEST.read().await;
    let mut tables = HashMap::new();
    for schema_key in r.keys() {
        if !schema_key.contains(format!("/{}/", StreamType::EnrichmentTables).as_str()) {
            continue;
        }
        let columns = schema_key.split('/').collect::<Vec<&str>>();
        let org_id = columns[0];
        let stream_type = StreamType::from(columns[1]);
        let stream_name = columns[2];
        if !stream_type.eq(&StreamType::EnrichmentTables) {
            continue;
        }
        tables.insert(
            schema_key.to_owned(),
            StreamTable {
                org_id: org_id.to_string(),
                stream_name: stream_name.to_string(),
                data: vec![],
            },
        );
    }
    drop(r);
    if tables.is_empty() {
        log::info!("EnrichmentTables Cached");
        return Ok(());
    }

    // waiting for querier to be ready
    let expect_querier_num = get_config().limit.starting_expect_querier_num;
    loop {
        let nodes = get_cached_online_querier_nodes(Some(RoleGroup::Interactive))
            .await
            .unwrap_or_default();
        if nodes.len() >= expect_querier_num {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        log::info!("Waiting for querier to be ready");
    }

    // fill data
    for (key, tbl) in tables {
        let data = super::enrichment_table::get(&tbl.org_id, &tbl.stream_name).await?;
        ENRICHMENT_TABLES.insert(
            key,
            StreamTable {
                org_id: tbl.org_id,
                stream_name: tbl.stream_name,
                data,
            },
        );
    }
    log::info!("EnrichmentTables Cached");
    Ok(())
}

pub fn filter_schema_version_id(schemas: &[Schema], _start_dt: i64, end_dt: i64) -> Option<usize> {
    let versions = schemas.len();
    for (i, schema) in schemas.iter().enumerate() {
        let metadata = schema.metadata();
        let schema_end_dt: i64 = metadata
            .get("end_dt")
            .unwrap_or(&"0".to_string())
            .parse()
            .unwrap();
        if end_dt < schema_end_dt {
            return Some(i);
        }
    }
    if versions > 0 {
        Some(versions - 1)
    } else {
        None
    }
}

pub async fn list_organizations_from_cache() -> Vec<String> {
    let mut names = HashSet::new();
    let r = STREAM_SCHEMAS_LATEST.read().await;
    for schema_key in r.keys() {
        if !schema_key.contains('/') {
            continue;
        }
        let name = schema_key.split('/').collect::<Vec<&str>>()[0].to_string();
        if !names.contains(&name) {
            names.insert(name);
        }
    }
    names.into_iter().collect::<Vec<String>>()
}

pub async fn list_streams_from_cache(org_id: &str, stream_type: StreamType) -> Vec<String> {
    let mut names = HashSet::new();
    let r = STREAM_SCHEMAS_LATEST.read().await;
    for schema_key in r.keys() {
        if !schema_key.contains('/') {
            continue;
        }
        let columns = schema_key.split('/').collect::<Vec<&str>>();
        let cur_org_id = columns[0];
        if !org_id.eq(cur_org_id) {
            continue;
        }
        let cur_stream_type = StreamType::from(columns[1]);
        if !stream_type.eq(&cur_stream_type) {
            continue;
        }
        let cur_stream_name = columns[2].to_string();
        names.insert(cur_stream_name);
    }
    names.into_iter().collect::<Vec<String>>()
}
