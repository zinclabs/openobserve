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

use std::time::Duration;

use anyhow::Result;
use config::{get_config, meta::alerts::destinations::HTTPType, utils::json::Value};
use tokio::time;

use crate::service::alerts::destinations;

/// Two approaches for external ingestion task:
///
/// 1. channel to queue up tasks:
///  a. producer/sender(pipeline_node): enqueue tasks
///  b. consumer/receiver(new struct): dequeue task and persist to disk locally
///  c. a background job loads the data from local disk and make external http requests
///  Pro:
///      - fast pipeline execution, minimal work
///  cons:
///      - Risk of data loss if system crashes before consumer persists
///      - Queue needs enough memory for large payloads
///      - more complex consumer (2 separate background jobs)
///  
/// 2. no channel: simply pipeline + a background job
///   a. pipeline http node handles persisting received records to disk locally
///   b. a background job loads the data from local disk and make external http requests
///  Pro:
///      - data guaranteed to be persisted to disk
///      - lower memory usage, especially comparing to when lots of tasks queued in memory for
///        approach 1
///      - simpler
///  cons:
///      - Higher latency on incoming requests due to persistence
///
/// However, neither approach addresses the potential data loss issue when the running node
/// crashes/removed, since both persist data locally. This would require more advanced solution.
/// Team think it's fine.

async fn read_from_pipeline_local_records() -> Result<Vec<Value>> {
    todo!()
}

pub async fn run_external_ingestion_job() {
    let interval = get_config().limit.alert_schedule_interval;
    let mut interval = time::interval(time::Duration::from_secs(interval as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        match read_from_pipeline_local_batches().await {
            Ok(batches) => {
                for batch in batches {
                    let org_id = batch["org_id"].as_str().unwrap();
                    let destinations = batch["destinations"].as_array().unwrap();
                    let records = batch["records"].as_array().unwrap().to_vec();
                    for dest in destinations {
                        let dest = dest.as_str().unwrap();
                        if let Err(e) = send_external_http_request(org_id, dest, &records).await {
                            log::error!("[PIPELINE] run external ingestion jobs error: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("[PIPELINE] run external ingestion jobs error: {}", e);
            }
        }
        if let Err(e) = read {
            log::error!(
                "[PIPELINE] run external destination ingestion jobs error: {}",
                e
            );
        }
    }
}

async fn send_external_http_request(org_id: &str, dest: &str, records: &Vec<Value>) -> Result<()> {
    let cfg = get_config();
    let dest = destinations::get(org_id, dest).await?;
    let client = reqwest::Client::builder().timeout(Duration::from_secs(cfg.limit.request_timeout));
    let client = if dest.skip_tls_verify {
        client.danger_accept_invalid_certs(true).build()?
    } else {
        client.build()?
    };
    let url = url::Url::parse(&dest.url)?;
    let mut req = match dest.method {
        HTTPType::POST => client.post(url).json(records),
        HTTPType::PUT => client.put(url).json(records),
        HTTPType::GET => client.get(url),
    };

    // Add additional headers if any from destination description
    let mut has_context_type = false;
    if let Some(headers) = &dest.headers {
        for (key, value) in headers.iter() {
            if !key.is_empty() && !value.is_empty() {
                if key.to_lowercase().trim() == "content-type" {
                    has_context_type = true;
                }
                req = req.header(key, value);
            }
        }
    };
    // set default content type
    if !has_context_type {
        req = req.header("Content-type", "application/json");
    }

    let resp = req.send().await?;
    let resp_status = resp.status();
    let resp_body = resp.text().await?;
    log::debug!(
        "Records sent to destination {} with status: {}, body: {:?}",
        dest.url,
        resp_status,
        resp_body,
    );
    if !resp_status.is_success() {
        log::error!(
            "External routing to {} failed with status: {}, body: {}",
            dest.url,
            resp_status,
            resp_body
        );
        return Err(anyhow::anyhow!(
            "sent error status: {}, err: {}",
            resp_status,
            resp_body
        ));
    }
    Ok(())
}
