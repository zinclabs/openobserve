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

use infra::table::enrichment_table_jobs;

use crate::service::enrichment_table;

// 1. Lookup `enrichment_table_jobs` table and fetch the `task_id` and `task_status` of the rows
//    where `task_status` is `pending` in ascending order of `created_ts`.
// 2. For each task, spawn a task to process the enrichment table. Limit the number of tasks spawned
//    concurrently to the value of `ENRICHMENT_TABLE_JOBS_CONCURRENCY`
// 3. Update the `task_status` to `processing` for the tasks spawned.
// 4. Once done processing, update the `task_status` to `completed` if successful or `failed`.
// 5. If the task is cancelled, update the `task_status` to `cancelled`.
// 6. Remove the file from disk if the task is completed or failed or cancelled.
pub async fn run(id: i64) -> Result<(), anyhow::Error> {
    // Get the oldest pending task
    let job = match enrichment_table_jobs::get_pending_task().await {
        Some(task) => task,
        None => return Ok(()),
    };

    let start = std::time::Instant::now();
    log::info!(
        "[ENRICHMENT_TABLE_JOB: {}] task_id {} Starting enrichment table job",
        id,
        job.task_id
    );

    // update the task status to processing
    update_running_job(&job.task_id).await?;

    // similar to the compactor, we need to update the job status every 15 seconds
    let job_id = job.task_id.clone();
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let mut cancel_rx_clone = cancel_rx.clone();
    tokio::task::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(15)) => {}
                _ = cancel_rx_clone.changed() => {
                    log::debug!(
                        "[ENRICHMENT_TABLE_JOBS: {}] task_id {} enrichment table received cancellation signal.",
                        id,
                        job_id
                    );
                    return;
                }
            }

            if let Err(e) = update_running_job(&job_id).await {
                log::error!(
                    "[ENRICHMENT_TABLE_JOBS : {}] task_id:{} update job status failed: {}",
                    id,
                    job_id,
                    e
                );
                let _ = cancel_tx.send(true);
                return;
            }
        }
    });

    if let Err(e) = enrichment_table::download_and_save_data(&job, cancel_rx.clone()).await {
        log::error!(
            "[ENRICHMENT_TABLE_JOBS: {}] task_id {} enrichment table job failed: {}",
            id,
            &job.task_id,
            e
        );
        let task_status = enrichment_table_jobs::get(&job.task_id).await?.task_status;
        let (org_id, table_name, append_data) = enrichment_table::parse_key(&job.file_key)?;
        // If task is cancelled, remove the temp file and return
        if task_status == enrichment_table_jobs::TaskStatus::Cancelled {
            enrichment_table::remove_temp_file(&org_id, &table_name, append_data).await?;
            return Ok(());
        }

        // update the task status to failed
        enrichment_table_jobs::set_job_status(
            &job.task_id,
            enrichment_table_jobs::TaskStatus::Failed,
        )
        .await?;
        // remove the temp file
        enrichment_table::remove_temp_file(&org_id, &table_name, append_data).await?;
        // remove the table
        enrichment_table::delete_table(&org_id, &table_name).await?;
        return Ok(());
    };

    log::info!(
        "[ENRICHMENT_TABLE_JOBS: {}] task_id {} enrichment table job completed in {}ms",
        id,
        &job.task_id,
        start.elapsed().as_millis()
    );

    Ok(())
}

pub async fn update_running_job(task_id: &str) -> Result<(), anyhow::Error> {
    let job = enrichment_table_jobs::get(task_id).await?;

    // ensure the status is not cancelled
    if job.task_status == enrichment_table_jobs::TaskStatus::Cancelled
        || job.task_status == enrichment_table_jobs::TaskStatus::Failed
    {
        Err(anyhow::anyhow!(
            "Task {} is cancelled. Stopping the job",
            task_id
        ))?;
    }

    // update the task status to processing
    enrichment_table_jobs::set_job_status(task_id, enrichment_table_jobs::TaskStatus::InProgress)
        .await?;

    Ok(())
}
