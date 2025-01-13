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

use std::{path::Path, time::Duration};

use config::{
    get_config,
    meta::{cluster::Role, stream::StreamType},
    metrics,
    metrics::{NAMESPACE, SPAN_METRICS_BUCKET},
    utils::file::scan_files,
};
use hashbrown::HashMap;
use infra::{cache, db::get_db};
use once_cell::sync::Lazy;
use opentelemetry::{global, metrics::Histogram, KeyValue};
use opentelemetry_sdk::{
    metrics::{
        new_view, reader::DefaultTemporalitySelector, Aggregation, Instrument, PeriodicReader,
        SdkMeterProvider, Stream,
    },
    runtime, Resource,
};
use tokio::{sync::RwLock, time};

use crate::{
    common::infra::{
        cluster::get_cached_online_nodes,
        config::{ORG_USERS, USERS},
    },
    service::{
        db,
        exporter::otlp_metrics_exporter::{O2MetricsClient, O2MetricsExporter},
    },
};

#[derive(Debug)]
pub struct TraceMetricsItem {
    pub organization: String,
    pub traces_stream_name: String,
    pub service_name: String,
    pub span_name: String,
    pub span_status: String,
    pub span_kind: String,
    pub duration: f64,
    pub span_id: String,
}

pub type TraceMetricsChan = (
    tokio::sync::mpsc::Sender<TraceMetricsItem>,
    RwLock<tokio::sync::mpsc::Receiver<TraceMetricsItem>>,
);

pub static TRACE_METRICS_CHAN: Lazy<TraceMetricsChan> = Lazy::new(|| {
    let (tx, rx) =
        tokio::sync::mpsc::channel(get_config().common.traces_span_metrics_channel_buffer);
    (tx, RwLock::new(rx))
});

pub static TRACE_METRICS_SPAN_HISTOGRAM: Lazy<Histogram<f64>> = Lazy::new(|| {
    let meter = opentelemetry::global::meter("o2");
    meter
        .f64_histogram(format!("{}_span_duration_milliseconds", NAMESPACE))
        .with_unit("ms")
        .with_description("span duration milliseconds")
        .init()
});

pub async fn run() -> Result<(), anyhow::Error> {
    // load metrics
    load_query_cache_limit_bytes().await?;
    load_ingest_wal_used_bytes().await?;
    if config::cluster::LOCAL_NODE.is_ingester() {
        tokio::spawn(async {
            if let Err(e) = traces_metrics_collect().await {
                log::error!("Error traces_metrics_collect metrics: {}", e);
            }
        });
    }

    // update metrics every 60 seconds
    let mut interval = time::interval(time::Duration::from_secs(60));
    interval.tick().await; // trigger the first run
    loop {
        if let Err(e) = update_metadata_metrics().await {
            log::error!("Error update metadata metrics: {}", e);
        }
        if let Err(e) = update_storage_metrics().await {
            log::error!("Error update storage metrics: {}", e);
        }
        if let Err(e) = update_memory_usage().await {
            log::error!("Error update memory_usage metrics: {}", e);
        }
        interval.tick().await;
    }
}

async fn load_query_cache_limit_bytes() -> Result<(), anyhow::Error> {
    let cfg = get_config();
    metrics::QUERY_MEMORY_CACHE_LIMIT_BYTES
        .with_label_values(&[])
        .set(cfg.memory_cache.max_size as i64);
    metrics::QUERY_DISK_CACHE_LIMIT_BYTES
        .with_label_values(&[])
        .set(cfg.disk_cache.max_size as i64);
    Ok(())
}

async fn load_ingest_wal_used_bytes() -> Result<(), anyhow::Error> {
    let cfg = get_config();
    let data_dir = match Path::new(&cfg.common.data_wal_dir).canonicalize() {
        Ok(path) => path,
        Err(_) => return Ok(()),
    };
    let pattern = format!("{}files/", &cfg.common.data_wal_dir);
    let files = scan_files(pattern, "parquet", None).unwrap_or_default();
    let mut sizes = HashMap::new();
    for file in files {
        let local_file = file.to_owned();
        let Ok(local_path) = Path::new(&file).canonicalize() else {
            continue;
        };
        let file_path = local_path
            .strip_prefix(&data_dir)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        let columns = file_path.split('/').collect::<Vec<&str>>();
        let _ = columns[0].to_string();
        let org_id = columns[1].to_string();
        let stream_type = columns[2].to_string();
        let entry = sizes.entry((org_id, stream_type)).or_insert(0);
        *entry += match std::fs::metadata(local_file) {
            Ok(metadata) => metadata.len(),
            Err(_) => 0,
        };
    }
    for ((org_id, stream_type), size) in sizes {
        metrics::INGEST_WAL_USED_BYTES
            .with_label_values(&[&org_id, &stream_type])
            .set(size as i64);
    }
    Ok(())
}

async fn update_metadata_metrics() -> Result<(), anyhow::Error> {
    let db = get_db().await;
    let stats = db.stats().await?;
    metrics::META_STORAGE_BYTES
        .with_label_values(&[])
        .set(stats.bytes_len);
    metrics::META_STORAGE_KEYS
        .with_label_values(&[])
        .set(stats.keys_count);

    if get_config().common.local_mode {
        metrics::META_NUM_NODES.with_label_values(&["all"]).set(1);
    } else {
        metrics::META_NUM_NODES.reset();
        let nodes = get_cached_online_nodes().await;
        if nodes.is_some() {
            for node in nodes.unwrap() {
                if node.is_ingester() {
                    metrics::META_NUM_NODES
                        .with_label_values(&[Role::Ingester.to_string().as_str()])
                        .inc();
                }
                if node.is_querier() {
                    metrics::META_NUM_NODES
                        .with_label_values(&[Role::Querier.to_string().as_str()])
                        .inc();
                }
                if node.is_compactor() {
                    metrics::META_NUM_NODES
                        .with_label_values(&[Role::Compactor.to_string().as_str()])
                        .inc();
                }
                if node.is_router() {
                    metrics::META_NUM_NODES
                        .with_label_values(&[Role::Router.to_string().as_str()])
                        .inc();
                }
                if node.is_alert_manager() {
                    metrics::META_NUM_NODES
                        .with_label_values(&[Role::AlertManager.to_string().as_str()])
                        .inc();
                }
            }
        }
    }

    let stream_types = [StreamType::Logs, StreamType::Metrics, StreamType::Traces];
    let orgs = db::schema::list_organizations_from_cache().await;
    metrics::META_NUM_ORGANIZATIONS
        .with_label_values(&[])
        .set(orgs.len() as i64);
    for org_id in &orgs {
        for stream_type in stream_types {
            let streams = db::schema::list_streams_from_cache(org_id, stream_type).await;
            if !streams.is_empty() {
                metrics::META_NUM_STREAMS
                    .with_label_values(&[org_id.as_str(), stream_type.to_string().as_str()])
                    .set(streams.len() as i64);
            }
        }
    }

    // let users = db.count("/user/").await?;
    let users = USERS.len();

    metrics::META_NUM_USERS_TOTAL
        .with_label_values(&[])
        .set(users as i64);
    for org_id in &orgs {
        let mut count: i64 = 0;
        for user in ORG_USERS.iter() {
            if user.key().starts_with(&format!("{org_id}/")) {
                count += 1;
            }
        }
        metrics::META_NUM_USERS
            .with_label_values(&[org_id.as_str()])
            .set(count);
    }

    metrics::META_NUM_FUNCTIONS.reset();
    let functions = db.list_keys("/function/").await?;
    for key in functions {
        let key = key.strip_prefix("/function/").unwrap();
        let columns = key.split('/').collect::<Vec<&str>>();
        if columns.len() <= 2 {
            // query functions
            metrics::META_NUM_FUNCTIONS
                .with_label_values(&[columns[0], "", "", "query"])
                .inc();
        } else {
            // ingest functions
            metrics::META_NUM_FUNCTIONS
                .with_label_values(&[columns[0], columns[2], columns[1], "ingest"])
                .inc();
        }
    }

    // TODO alert
    // TODO dashboard

    Ok(())
}

async fn update_storage_metrics() -> Result<(), anyhow::Error> {
    let stats = cache::stats::get_stats();
    for (key, stat) in stats {
        let columns = key.split('/').collect::<Vec<&str>>();
        metrics::STORAGE_ORIGINAL_BYTES
            .with_label_values(&[columns[0], columns[2], columns[1]])
            .set(stat.storage_size as i64);
        metrics::STORAGE_COMPRESSED_BYTES
            .with_label_values(&[columns[0], columns[2], columns[1]])
            .set(stat.compressed_size as i64);
        metrics::STORAGE_FILES
            .with_label_values(&[columns[0], columns[2], columns[1]])
            .set(stat.file_num);
        metrics::STORAGE_RECORDS
            .with_label_values(&[columns[0], columns[2], columns[1]])
            .set(stat.doc_num);
    }
    Ok(())
}

async fn update_memory_usage() -> Result<(), anyhow::Error> {
    if let Some(cur_memory) = memory_stats::memory_stats() {
        metrics::MEMORY_USAGE
            .with_label_values(&[])
            .set(cur_memory.physical_mem as i64);
    }
    Ok(())
}

pub async fn init_meter_provider() -> Result<SdkMeterProvider, anyhow::Error> {
    let exporter = O2MetricsExporter::new(
        O2MetricsClient::new(),
        Box::new(DefaultTemporalitySelector::new()),
    );
    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(Duration::from_secs(
            get_config().common.traces_span_metrics_export_interval,
        ))
        .build();
    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            "openobserve",
        )]))
        .with_view(new_view(
            Instrument::new().name(format!("{}_span_duration_milliseconds", NAMESPACE)),
            Stream::new().aggregation(Aggregation::ExplicitBucketHistogram {
                boundaries: SPAN_METRICS_BUCKET.to_vec(),
                record_min_max: false,
            }),
        )?)
        .build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

async fn traces_metrics_collect() -> Result<(), anyhow::Error> {
    let mut receiver = TRACE_METRICS_CHAN.1.write().await;

    while let Some(item) = receiver.recv().await {
        // Record measurements using the histogram instrument.
        // log::info!("receive span metrics: {}", item.span_id);
        TRACE_METRICS_SPAN_HISTOGRAM.record(
            item.duration,
            &[
                KeyValue::new("organization", item.organization),
                KeyValue::new("traces_stream_name", item.traces_stream_name),
                KeyValue::new("service_name", item.service_name),
                KeyValue::new("operation_name", item.span_name),
                KeyValue::new("status_code", item.span_status),
                KeyValue::new("span_kind", item.span_kind),
            ],
        );
    }

    Ok(())
}
