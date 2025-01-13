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

use config::{cluster::LOCAL_NODE, get_config};
use infra::table::enrichment_table_jobs;
use tokio::time;

use crate::service;

pub async fn run() -> Result<(), anyhow::Error> {
    if !LOCAL_NODE.is_ingester() {
        return Ok(());
    }
    log::info!("Spawning ingester jobs");

    let cfg = get_config();
    for i in 0..cfg.limit.enrichment_table_job_workers {
        tokio::task::spawn(async move { run_enrichment_table_jobs(i).await });
    }
    tokio::task::spawn(async move { run_enrichment_table_delete_jobs().await });

    Ok(())
}

async fn run_enrichment_table_jobs(id: i64) -> Result<(), anyhow::Error> {
    let interval = get_config().limit.enrichment_table_job_scheduler_interval;
    let mut interval = time::interval(time::Duration::from_secs(interval as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = service::enrichment_table_jobs::run(id).await {
            log::error!(
                "[ENRICHMENT_TABLE_JOB: {}] run enrichment table job error: {}",
                id,
                e
            );
        }
    }
}

async fn run_enrichment_table_delete_jobs() -> Result<(), anyhow::Error> {
    let interval = get_config().limit.enrichment_table_job_delete_interval;
    let mut interval = time::interval(time::Duration::from_secs(interval as u64));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = enrichment_table_jobs::delete_jobs().await {
            log::error!("[ENRICHMENT_TABLE_JOB] run delete jobs error: {}", e);
        }
    }
}
