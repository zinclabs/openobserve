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

use anyhow::{anyhow, Result};
use config::{
    cluster::LOCAL_NODE,
    get_config,
    meta::{
        search::SearchEventType,
        self_reporting::{
            usage::{AggregatedData, GroupKey, UsageData, UsageEvent, USAGE_STREAM},
            ReportingData,
        },
        stream::{StreamParams, StreamType},
    },
    utils::json,
};
use hashbrown::HashMap;
use proto::cluster_rpc;

use crate::{common::meta::ingestion, service};

pub(super) async fn ingest_usages(mut curr_usages: Vec<UsageData>) {
    if curr_usages.is_empty() {
        log::info!("[SELF-REPORTING] Returning as no usages reported ");
        return;
    }
    let mut groups: HashMap<GroupKey, AggregatedData> = HashMap::new();
    let mut search_events = vec![];
    for usage_data in curr_usages.iter_mut() {
        // Skip aggregation for usage_data with event "Search"
        if usage_data.event == UsageEvent::Search {
            // enrich dashboard search usage with more context if `usage_data` only has id's, but
            // not names
            if matches!(usage_data.search_type, Some(SearchEventType::Dashboards)) {
                if let Some((Some(dashboard_id), None)) = usage_data
                    .search_event_context
                    .as_ref()
                    .map(|ctx| (&ctx.dashboard_id, &ctx.dashboard_name))
                {
                    if let Ok((folder, dashboard)) = service::dashboards::get_folder_and_dashboard(
                        &usage_data.org_id,
                        dashboard_id,
                    )
                    .await
                    {
                        if let Some(ctx) = usage_data.search_event_context.as_mut() {
                            ctx.enrich_for_dashboard(
                                dashboard.title().unwrap().to_string(),
                                folder.name,
                                folder.folder_id,
                            )
                        };
                    }
                }
            }
            search_events.push(usage_data.clone());
            continue;
        }
        let node = usage_data.node_name.clone().unwrap_or_default();
        let key = GroupKey {
            stream_name: usage_data.stream_name.clone(),
            org_id: usage_data.org_id.clone(),
            stream_type: usage_data.stream_type,
            day: usage_data.day,
            hour: usage_data.hour,
            event: usage_data.event,
            email: usage_data.user_email.clone(),
            node,
        };

        let is_new = groups.contains_key(&key);

        let entry = groups.entry(key).or_insert_with(|| AggregatedData {
            count: 1,
            usage_data: usage_data.clone(),
        });
        if !is_new {
            continue;
        } else {
            entry.usage_data.num_records += usage_data.num_records;
            entry.usage_data.size += usage_data.size;
            entry.usage_data.response_time += usage_data.response_time;
            entry.count += 1;
        }
    }

    let mut report_data = vec![];
    for (_, data) in groups {
        let mut usage_data = data.usage_data;
        usage_data.response_time /= data.count as f64;
        report_data.push(usage_data);
    }

    // Push all the search events
    report_data.append(&mut search_events);
    let cfg = get_config();
    if &cfg.common.usage_reporting_mode != "local"
        && !cfg.common.usage_reporting_url.is_empty()
        && !cfg.common.usage_reporting_creds.is_empty()
    {
        let url = url::Url::parse(&cfg.common.usage_reporting_url).unwrap();
        let creds = if cfg.common.usage_reporting_creds.starts_with("Basic") {
            cfg.common.usage_reporting_creds.to_string()
        } else {
            format!("Basic {}", &cfg.common.usage_reporting_creds)
        };
        match reqwest::Client::builder()
            .build()
            .unwrap()
            .post(url)
            .header("Content-Type", "application/json")
            .header(reqwest::header::AUTHORIZATION, creds)
            .json(&report_data)
            .send()
            .await
        {
            Ok(resp) => {
                let resp_status = resp.status();
                if !resp_status.is_success() {
                    log::error!(
                        "[SELF-REPORTING] Error in ingesting usage data to external URL: {}",
                        resp.text()
                            .await
                            .unwrap_or_else(|_| resp_status.to_string())
                    );
                    if &cfg.common.usage_reporting_mode != "both" {
                        // on error in ingesting usage data, push back the data
                        let curr_usages = curr_usages.clone();
                        for usage_data in curr_usages {
                            if let Err(e) = super::queues::USAGE_QUEUE
                                .enqueue(ReportingData::Usage(Box::new(usage_data)))
                                .await
                            {
                                log::error!(
                                    "[SELF-REPORTING] Error in pushing back un-ingested Usage data to UsageQueuer: {e}"
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!(
                    "[SELF-REPORTING] Error in ingesting usage data to external URL {:?}",
                    e
                );
                if &cfg.common.usage_reporting_mode != "both" {
                    // on error in ingesting usage data, push back the data
                    let curr_usages = curr_usages.clone();
                    for usage_data in curr_usages {
                        if let Err(e) = super::queues::USAGE_QUEUE
                            .enqueue(ReportingData::Usage(Box::new(usage_data)))
                            .await
                        {
                            log::error!(
                                "[SELF-REPORTING] Error in pushing back un-ingested Usage data to UsageQueuer: {e}"
                            );
                        }
                    }
                }
            }
        }
    }

    if &cfg.common.usage_reporting_mode != "remote" {
        let report_data = report_data
            .iter_mut()
            .map(|usage| json::to_value(usage).unwrap())
            .collect::<Vec<_>>();
        // report usage data
        let usage_stream = StreamParams::new(&cfg.common.usage_org, USAGE_STREAM, StreamType::Logs);
        if ingest_reporting_data(report_data, usage_stream)
            .await
            .is_err()
            && &cfg.common.usage_reporting_mode != "both"
        {
            // on error in ingesting usage data, push back the data
            for usage_data in curr_usages {
                if let Err(e) = super::queues::USAGE_QUEUE
                    .enqueue(ReportingData::Usage(Box::new(usage_data)))
                    .await
                {
                    log::error!(
                        "[SELF-REPORTING] Error in pushing back un-ingested Usage data to UsageQueuer: {e}"
                    );
                }
            }
        }
    }
}

pub(super) async fn ingest_reporting_data(
    reporting_data_json: Vec<json::Value>,
    stream_params: StreamParams,
) -> Result<()> {
    if reporting_data_json.is_empty() {
        log::info!("[SELF-REPORTING] Returning as no errors reported");
        return Ok(());
    }

    if LOCAL_NODE.is_ingester() {
        // ingest directly for ingester node
        let (org_id, stream_name): (String, String) = (
            stream_params.org_id.into(),
            stream_params.stream_name.into(),
        );
        let bytes = bytes::Bytes::from(json::to_string(&reporting_data_json).unwrap());
        let req = ingestion::IngestionRequest::Usage(&bytes);
        match service::logs::ingest::ingest(0, &org_id, &stream_name, req, "", None).await {
            Ok(resp) if resp.code == 200 => {
                log::info!(
                    "[SELF-REPORTING] ReportingData successfully ingested to stream {org_id}/{stream_name}"
                );
                Ok(())
            }
            error => {
                let err =
                    error.map_or_else(|e| e.to_string(), |resp| resp.error.unwrap_or_default());
                log::error!(
                    "[SELF-REPORTING] ReportingData errored while ingesting to stream {org_id}/{stream_name}. Error: {err}"
                );
                Err(anyhow!("{err}"))
            }
        }
    } else {
        // call gRPC ingestion service
        let (org_id, stream_name, stream_type): (String, String, String) = (
            stream_params.org_id.into(),
            stream_params.stream_name.into(),
            stream_params.stream_type.to_string(),
        );

        let req = cluster_rpc::IngestionRequest {
            org_id: org_id.clone(),
            stream_name: stream_name.clone(),
            stream_type,
            data: Some(cluster_rpc::IngestionData::from(reporting_data_json)),
            ingestion_type: Some(cluster_rpc::IngestionType::Usage.into()),
            metadata: None,
        };

        match service::ingestion::ingestion_service::ingest(req).await {
            Ok(resp) if resp.status_code == 200 => {
                log::info!(
                    "[SELF-REPORTING] ReportingData successfully ingested to stream {org_id}/{stream_name}"
                );
                Ok(())
            }
            error => {
                let err = error.map_or_else(|e| e.to_string(), |resp| resp.message);
                log::error!(
                    "[SELF-REPORTING] ReportingData errored while ingesting to stream {org_id}/{stream_name}. Error: {err}"
                );
                Err(anyhow!("{err}"))
            }
        }
    }
}
