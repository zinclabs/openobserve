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

use chrono::{DateTime, Datelike, Timelike};
use config::{
    get_config,
    meta::{
        self_reporting::{
            error::ErrorData,
            usage::{RequestStats, TriggerData, UsageData, UsageEvent, UsageType},
            ReportingData,
        },
        stream::StreamType,
    },
    metrics, SIZE_IN_MB,
};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::auditor;
#[cfg(feature = "enterprise")]
use proto::cluster_rpc;
use tokio::sync::oneshot;

mod ingestion;
mod queues;

pub async fn run() {
    let cfg = get_config();
    if !cfg.common.usage_enabled {
        return;
    }

    // Force initialization usage queue
    let (usage_start_sender, usage_start_receiver) = oneshot::channel();
    if let Err(e) = queues::USAGE_QUEUE.start(usage_start_sender).await {
        log::error!("[SELF-REPORTING] Failed to initialize usage queue: {e}");
        return;
    }

    if let Err(e) = usage_start_receiver.await {
        log::error!("[SELF-REPORTING] Usage queue initialization failed: {e}");
        return;
    }

    // Force initialization error queue
    let (error_start_sender, error_start_receiver) = oneshot::channel();
    if let Err(e) = queues::ERROR_QUEUE.start(error_start_sender).await {
        log::error!("[SELF-REPORTING] Failed to initialize error queue: {e}");
        return;
    }

    if let Err(e) = error_start_receiver.await {
        log::error!("[SELF-REPORTING] Error queue initialization failed: {e}");
        return;
    }

    log::debug!("[SELF-REPORTING] successfully initialized reporting queues");
}

pub async fn report_request_usage_stats(
    stats: RequestStats,
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    usage_type: UsageType,
    num_functions: u16,
    timestamp: i64,
) {
    metrics::INGEST_RECORDS
        .with_label_values(&[org_id, stream_name, stream_type.to_string().as_str()])
        .inc_by(stats.records as u64);
    metrics::INGEST_BYTES
        .with_label_values(&[org_id, stream_name, stream_type.to_string().as_str()])
        .inc_by((stats.size * SIZE_IN_MB) as u64);
    let event: UsageEvent = usage_type.into();
    let now = DateTime::from_timestamp_micros(timestamp).unwrap();

    if !get_config().common.usage_enabled {
        return;
    }

    let request_body = stats.request_body.unwrap_or(usage_type.to_string());
    let user_email = stats.user_email.unwrap_or("".to_owned());

    let mut usage = vec![];

    if num_functions > 0 {
        usage.push(UsageData {
            _timestamp: timestamp,
            event: UsageEvent::Functions,
            day: now.day(),
            hour: now.hour(),
            month: now.month(),
            year: now.year(),
            event_time_hour: format!(
                "{:04}{:02}{:02}{:02}",
                now.year(),
                now.month(),
                now.day(),
                now.hour()
            ),
            org_id: org_id.to_owned(),
            request_body: request_body.to_owned(),
            function: None,
            size: stats.size,
            unit: "MB".to_owned(),
            user_email: user_email.to_owned(),
            response_time: stats.response_time,
            num_records: stats.records * num_functions as i64,
            dropped_records: stats.dropped_records,
            stream_type,
            stream_name: stream_name.to_owned(),
            min_ts: None,
            max_ts: None,
            cached_ratio: None,
            compressed_size: None,
            search_type: stats.search_type,
            search_event_context: stats.search_event_context.clone(),
            trace_id: None,
            took_wait_in_queue: stats.took_wait_in_queue,
            result_cache_ratio: None,
            is_partial: stats.is_partial,
            work_group: None,
        });
    };

    usage.push(UsageData {
        _timestamp: timestamp,
        event,
        day: now.day(),
        hour: now.hour(),
        month: now.month(),
        year: now.year(),
        event_time_hour: format!(
            "{:04}{:02}{:02}{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour()
        ),
        org_id: org_id.to_owned(),
        request_body: request_body.to_owned(),
        size: stats.size,
        unit: "MB".to_owned(),
        user_email,
        response_time: stats.response_time,
        function: stats.function,
        num_records: stats.records,
        dropped_records: stats.dropped_records,
        stream_type,
        stream_name: stream_name.to_owned(),
        min_ts: stats.min_ts,
        max_ts: stats.max_ts,
        cached_ratio: stats.cached_ratio,
        compressed_size: None,
        search_type: stats.search_type,
        search_event_context: stats.search_event_context.clone(),
        trace_id: stats.trace_id,
        took_wait_in_queue: stats.took_wait_in_queue,
        result_cache_ratio: stats.result_cache_ratio,
        is_partial: stats.is_partial,
        work_group: stats.work_group,
    });
    if !usage.is_empty() {
        publish_usage(usage).await;
    }
}

async fn publish_usage(usages: Vec<UsageData>) {
    let cfg = get_config();
    if !cfg.common.usage_enabled {
        return;
    }

    for usage in usages {
        if let Err(e) = queues::USAGE_QUEUE
            .enqueue(ReportingData::Usage(Box::new(usage)))
            .await
        {
            log::error!(
                "[SELF-REPORTING] Failed to send usage data to background ingesting job: {e}"
            );
        }
    }
}

pub async fn publish_triggers_usage(trigger: TriggerData) {
    let cfg = get_config();
    if !cfg.common.usage_enabled {
        return;
    }

    match queues::USAGE_QUEUE
        .enqueue(ReportingData::Trigger(Box::new(trigger)))
        .await
    {
        Err(e) => {
            log::error!(
                "[SELF-REPORTING] Failed to send trigger usage data to background ingesting job: {e}"
            )
        }
        Ok(()) => {
            log::debug!("[SELF-REPORTING] Successfully queued trigger usage data to be ingested")
        }
    }
}

pub async fn publish_error(error_data: ErrorData) {
    let cfg = get_config();
    if !cfg.common.usage_enabled {
        return;
    }

    match queues::ERROR_QUEUE
        .enqueue(ReportingData::Error(Box::new(error_data)))
        .await
    {
        Err(e) => {
            log::error!(
                "[SELF-REPORTING] Failed to send error data to background ingesting job: {e}"
            )
        }
        Ok(()) => {
            log::debug!("[SELF-REPORTING] Successfully queued error data to be ingested");
        }
    }
}

pub async fn flush() {
    // flush audit data
    #[cfg(feature = "enterprise")]
    flush_audit().await;

    let cfg = get_config();
    if !cfg.common.usage_enabled {
        return;
    }

    // shutdown usage_queuer
    for _ in 0..get_config().limit.usage_reporting_thread_num {
        let (res_sender, res_receiver) = oneshot::channel();
        if let Err(e) = queues::USAGE_QUEUE.shutdown(res_sender).await {
            log::error!("[SELF-REPORTING] Error shutting down USAGE_QUEUER: {e}");
        }
        // wait for flush ingestion job
        res_receiver.await.ok();

        let (error_sender, error_receiver) = oneshot::channel();
        if let Err(e) = queues::ERROR_QUEUE.shutdown(error_sender).await {
            log::error!("[SELF-REPORTING] Error shutting down ERROR_QUEUE: {e}");
        }
        // wait for flush ingestion job
        error_receiver.await.ok();
    }
}

// Cron job to frequently publish auditted events
#[cfg(feature = "enterprise")]
pub async fn run_audit_publish() {
    let o2cfg = o2_enterprise::enterprise::common::infra::config::get_config();
    if !o2cfg.common.audit_enabled {
        return;
    }

    let mut audit_interval = tokio::time::interval(tokio::time::Duration::from_secs(
        o2cfg.common.audit_publish_interval.try_into().unwrap(),
    ));
    audit_interval.tick().await; // trigger the first run
    loop {
        log::debug!("Audit ingestion loop running");
        audit_interval.tick().await;
        o2_enterprise::enterprise::common::auditor::publish_existing_audits(
            &get_config().common.usage_org,
            publish_audit,
        )
        .await;
    }
}

#[cfg(feature = "enterprise")]
pub async fn audit(msg: auditor::AuditMessage) {
    auditor::audit(&get_config().common.usage_org, msg, publish_audit).await;
}

#[cfg(feature = "enterprise")]
pub async fn flush_audit() {
    auditor::flush_audit(&get_config().common.usage_org, publish_audit).await;
}

#[cfg(feature = "enterprise")]
async fn publish_audit(
    req: cluster_rpc::IngestionRequest,
) -> Result<cluster_rpc::IngestionResponse, anyhow::Error> {
    crate::service::ingestion::ingestion_service::ingest(req).await
}

#[inline]
pub fn http_report_metrics(
    start: std::time::Instant,
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    code: &str,
    uri: &str,
) {
    let time = start.elapsed().as_secs_f64();
    let uri = format!("/api/org/{}", uri);
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            &uri,
            code,
            org_id,
            stream_name,
            stream_type.to_string().as_str(),
        ])
        .observe(time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            &uri,
            code,
            org_id,
            stream_name,
            stream_type.to_string().as_str(),
        ])
        .inc();
}
