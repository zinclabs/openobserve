// Copyright 2024 Zinc Labs Inc.
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

use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{cluster::get_internal_grpc_token, stream::StreamType},
    metrics::get_registry,
    utils::{prom_json_encoder::JsonEncoder, util::zero_or},
};
use hashbrown::HashSet;
use once_cell::sync::Lazy;
use proto::cluster_rpc::{
    ingest_client::IngestClient, IngestionData, IngestionRequest, IngestionType,
};
use serde_json::Value;
use tokio::time::{self, Duration};
use tonic::{
    codec::CompressionEncoding,
    metadata::{MetadataKey, MetadataValue},
    Request,
};

use crate::service::{self, grpc::get_ingester_channel};

static METRICS_WHITELIST: Lazy<HashSet<String>> = Lazy::new(|| {
    config::get_config()
        .common
        .self_metrics_consumption_whitelist
        .split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.trim().is_empty())
        .collect()
});

async fn send_metrics(config: &config::Config, metrics: Vec<Value>) -> Result<(), tonic::Status> {
    let org = config.common.usage_org.as_str();
    let req = IngestionRequest {
        org_id: org.to_owned(),
        stream_name: "".to_owned(),
        stream_type: StreamType::Metrics.to_string(),
        data: Some(IngestionData::from(metrics)),
        ingestion_type: Some(IngestionType::Json.into()),
        metadata: None,
    };
    let org_header_key: MetadataKey<_> = config.grpc.org_header_key.parse().unwrap();
    let token: MetadataValue<_> = get_internal_grpc_token().parse().unwrap();
    let (_, channel) = get_ingester_channel().await?;
    let mut client = IngestClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        req.metadata_mut()
            .insert(org_header_key.clone(), org.parse().unwrap());
        Ok(req)
    });
    client = client
        .send_compressed(CompressionEncoding::Gzip)
        .accept_compressed(CompressionEncoding::Gzip)
        .max_decoding_message_size(config.grpc.max_message_size * 1024 * 1024)
        .max_encoding_message_size(config.grpc.max_message_size * 1024 * 1024);
    client.ingest(req).await?;
    Ok(())
}

pub async fn run() -> Result<(), anyhow::Error> {
    let config = get_config();
    let org = config.common.usage_org.as_str();

    log::debug!(
        "self-metrics consumption enabled status : {}",
        config.common.self_metrics_consumption_enabled
    );

    if !config.common.self_metrics_consumption_enabled {
        return Ok(());
    }
    if METRICS_WHITELIST.is_empty() {
        log::warn!("metrics self-consumption whitelist is empty, no metrics will be consumed");
        // no point in scraping if there are no metrics enabled
        return Ok(());
    }

    let registry = get_registry();

    // Set up the interval timer for periodic fetching
    let timeout = zero_or(config.common.self_metrics_consumption_interval, 60);
    let mut interval = time::interval(Duration::from_secs(timeout));
    interval.tick().await; // Trigger the first run

    loop {
        // Wait for the interval before running the task again
        interval.tick().await;
        let prom_data: Vec<_> = registry
            .gather()
            .into_iter()
            .filter(|mf| {
                let name = mf.get_name();
                METRICS_WHITELIST.contains(name)
            })
            .collect();

        // ingester can ingest its own metrics, others need to send to one of the ingesters
        if LOCAL_NODE.is_ingester() {
            let metrics = JsonEncoder::new().encode_to_string(&prom_data).unwrap();
            let bytes = bytes::Bytes::from(metrics);
            match service::metrics::json::ingest(org, bytes).await {
                Ok(_) => {
                    log::debug!("successfully ingested self-metrics");
                }
                Err(e) => {
                    log::error!("error in ingesting self-metrics : {:?}", e)
                }
            }
        } else {
            let metrics = JsonEncoder::new().encode_to_json(&prom_data);
            match send_metrics(&config, metrics).await {
                Ok(_) => {
                    log::debug!("successfully sent self-metrics for ingestion");
                }
                Err(e) => {
                    log::error!("error in sending self-metrics : {:?}", e)
                }
            }
        }
    }
}
