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

use std::collections::HashMap;

use config::{get_config, ider};
use futures::future::try_join_all;

use crate::service::db;

pub mod handlers;
pub mod worker;

pub async fn run_old() -> Result<(), anyhow::Error> {
    let trace_id = ider::generate();
    log::debug!("[SCHEDULER trace_id {trace_id}] Pulling jobs from scheduler");
    let cfg = get_config();

    // Scheduler pulls only those triggers that match the conditions-
    // - trigger.next_run_at <= now
    // - !(trigger.is_realtime && !trigger.is_silenced)
    // - trigger.status == "Waiting"
    let triggers = db::scheduler::pull(
        cfg.limit.alert_schedule_concurrency,
        cfg.limit.alert_schedule_timeout,
        cfg.limit.report_schedule_timeout,
    )
    .await?;

    log::info!(
        "[SCHEDULER trace_id {trace_id}] Pulled {} jobs from scheduler",
        triggers.len()
    );

    if !triggers.is_empty() {
        let mut grouped_triggers: std::collections::HashMap<
            db::scheduler::TriggerModule,
            Vec<&db::scheduler::Trigger>,
        > = HashMap::new();

        // Group triggers by module
        for trigger in &triggers {
            grouped_triggers
                .entry(trigger.module.clone())
                .or_default()
                .push(trigger);
        }

        // Print counts for each module
        for (module, triggers) in grouped_triggers {
            log::info!(
                "[SCHEDULER trace_id {trace_id}] Pulled {:?}: {} jobs",
                module,
                triggers.len()
            );
        }
    }

    let mut tasks = Vec::new();
    for (i, trigger) in triggers.into_iter().enumerate() {
        let trace_id = format!("{}-{}", trace_id, i);
        let task = tokio::task::spawn(async move {
            let key = format!("{}-{}/{}", trigger.module, trigger.org, trigger.module_key);
            log::debug!(
                "[SCHEDULER trace_id {trace_id}] start processing trigger: {}",
                key
            );
            // if let Err(e) = handle_triggers(&trace_id, trigger).await {
            //     log::error!(
            //         "[SCHEDULER trace_id {trace_id}] Error handling trigger: {}",
            //         e
            //     );
            // }
            log::debug!(
                "[SCHEDULER trace_id {trace_id}] finished processing trigger: {}",
                key
            );
        });
        tasks.push(task);
    }
    if let Err(e) = try_join_all(tasks).await {
        log::error!(
            "[SCHEDULER trace_id {trace_id}] Error handling triggers: {}",
            e
        );
    }
    Ok(())
}

// The main function that creates and runs the scheduler
pub async fn run() -> Result<(), anyhow::Error> {
    // Get configuration
    let cfg = get_config();

    // Create scheduler config
    let scheduler_config = worker::SchedulerConfig {
        alert_schedule_concurrency: cfg.limit.alert_schedule_concurrency as usize,
        alert_schedule_timeout: cfg.limit.alert_schedule_timeout,
        report_schedule_timeout: cfg.limit.report_schedule_timeout,
        poll_interval_secs: cfg.limit.alert_schedule_interval as u64, // Can be configurable
    };

    // Create and run scheduler
    let scheduler = worker::Scheduler::new(scheduler_config);
    scheduler.run().await
}
