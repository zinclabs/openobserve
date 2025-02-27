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
use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use tokio::{
    sync::{mpsc, Mutex, Semaphore},
    time,
};

use super::handlers::handle_triggers;
use crate::service::db::scheduler::{pull as scheduler_pull, Trigger, TriggerModule};

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub trace_id: String,
    pub trigger: Trigger,
}

/// Configuration for the scheduler
#[derive(Clone)]
pub struct SchedulerConfig {
    pub alert_schedule_concurrency: usize,
    pub alert_schedule_timeout: i64,
    pub report_schedule_timeout: i64,
    pub poll_interval_secs: u64,
}

/// Scheduler worker for processing triggers
#[derive(Clone)]
pub struct SchedulerWorker {
    pub id: usize,
    pub rx: Arc<Mutex<mpsc::Receiver<ScheduledJob>>>,
    pub available_workers: Arc<Semaphore>,
}

/// Scheduler job puller that fetches jobs from the database
#[derive(Clone)]
pub struct SchedulerJobPuller {
    pub tx: mpsc::Sender<ScheduledJob>,
    pub available_workers: Arc<Semaphore>,
    pub config: SchedulerConfig,
}

/// Main scheduler that coordinates workers and the job puller
pub struct Scheduler {
    pub config: SchedulerConfig,
    pub workers: Vec<SchedulerWorker>,
    pub job_puller: SchedulerJobPuller,
    pub tx: mpsc::Sender<ScheduledJob>,
    pub rx: Arc<Mutex<mpsc::Receiver<ScheduledJob>>>,
    pub available_workers: Arc<Semaphore>,
}

impl SchedulerWorker {
    pub fn new(
        id: usize,
        rx: Arc<Mutex<mpsc::Receiver<ScheduledJob>>>,
        available_workers: Arc<Semaphore>,
    ) -> Self {
        Self {
            id,
            rx,
            available_workers,
        }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            // Try to get next trigger
            let trigger = {
                let mut rx = self.rx.lock().await;
                rx.recv().await
            };

            match trigger {
                Some(trigger) => {
                    // Acquire permit before processing
                    let _permit = self.available_workers.acquire().await?;

                    // Process the trigger
                    if let Err(e) = self
                        .handle_trigger(&trigger.trace_id, trigger.trigger)
                        .await
                    {
                        log::error!(
                            "[SCHEDULER][Worker-{}] trace_id: {} Error handling trigger: {}",
                            self.id,
                            trigger.trace_id,
                            e
                        );
                    }

                    // Permit is automatically released when _permit is dropped
                }
                None => {
                    // Channel closed, no more work
                    log::info!(
                        "[SCHEDULER][Worker-{}] Channel closed, no more work",
                        self.id
                    );
                    break;
                }
            }
        }
        Ok(())
    }

    /// Handles a given trigger
    pub async fn handle_trigger(&self, trace_id: &str, trigger: Trigger) -> Result<()> {
        handle_triggers(trace_id, trigger).await
    }
}

impl SchedulerJobPuller {
    pub fn new(
        tx: mpsc::Sender<ScheduledJob>,
        available_workers: Arc<Semaphore>,
        config: SchedulerConfig,
    ) -> Self {
        Self {
            tx,
            available_workers,
            config,
        }
    }

    /// Runs the job puller. Pulls jobs from the database and sends them to the workers.
    /// When pulling jobs, it will only pull as many jobs as the number of available workers
    /// which is determined by the semaphore and can have the maximum value of
    /// `alert_schedule_concurrency` The advantage of this approach is that the scheduler does
    /// not need to wait for all the workers to complete the job before pulling more jobs. It
    /// will keep pulling jobs at the configured poll interval. The number of jobs pulled will
    /// be the number of available workers.
    pub async fn run(&self) -> Result<()> {
        let interval = self.config.poll_interval_secs;
        let mut interval = time::interval(time::Duration::from_secs(interval));
        interval.tick().await; // The first tick
        loop {
            // Check how many workers are available
            let available_count = self.available_workers.available_permits();
            let trace_id = config::ider::uuid();
            log::info!(
                "[SCHEDULER][JobPuller-{}] Available workers: {}",
                trace_id,
                available_count
            );

            if available_count > 0 {
                // Pull only as many jobs as we have available workers
                let triggers = self.pull(available_count).await?;

                log::info!(
                    "[SCHEDULER][JobPuller-{}] Pulled {} jobs from scheduler",
                    trace_id,
                    triggers.len()
                );

                // Print counts for each module
                if !triggers.is_empty() {
                    let mut grouped_triggers: std::collections::HashMap<
                        TriggerModule,
                        Vec<&Trigger>,
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
                            "[SCHEDULER JobPuller trace_id {trace_id}] Pulled {:?}: {} jobs",
                            module,
                            triggers.len()
                        );
                    }
                }

                // Send all triggers to be processed
                for trigger in triggers {
                    let scheduled_job = ScheduledJob {
                        trace_id: trace_id.clone(),
                        trigger,
                    };
                    if self.tx.send(scheduled_job).await.is_err() {
                        // Channel closed, exit loop
                        log::error!(
                            "[SCHEDULER][JobPuller-{}] Channel closed, exiting job puller",
                            trace_id
                        );
                        return Ok(());
                    }
                }
            }

            // Wait for the next tick
            interval.tick().await;

            // Check if channel is closed
            if self.tx.is_closed() {
                break;
            }
        }
        log::info!("[SCHEDULER] Job puller exiting");
        Ok(())
    }

    pub async fn pull(&self, limit: usize) -> Result<Vec<Trigger>> {
        match scheduler_pull(
            limit as i64,
            self.config.alert_schedule_timeout,
            self.config.report_schedule_timeout,
        )
        .await
        {
            Ok(triggers) => Ok(triggers),
            Err(e) => {
                log::error!("[SCHEDULER] Error pulling triggers: {}", e);
                Ok(vec![])
            }
        }
    }
}

impl Scheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        let max_workers = config.alert_schedule_concurrency;

        // Create a channel for work distribution with capacity for max workers
        let (tx, rx) = mpsc::channel(max_workers);

        // Share receiver between workers
        let rx = Arc::new(Mutex::new(rx));

        // Track available workers with a semaphore
        let available_workers = Arc::new(Semaphore::new(max_workers));

        // Create workers
        let workers = (0..max_workers)
            .map(|id| SchedulerWorker::new(id, rx.clone(), available_workers.clone()))
            .collect();

        // Create job puller
        let job_puller =
            SchedulerJobPuller::new(tx.clone(), available_workers.clone(), config.clone());

        Self {
            config,
            workers,
            job_puller,
            tx,
            rx,
            available_workers,
        }
    }

    pub async fn run(&self) -> Result<()> {
        log::debug!("Starting scheduler");

        // Spawn worker tasks
        let worker_handles: Vec<_> = self
            .workers
            .iter()
            .map(|worker| {
                let worker = worker.clone();
                tokio::spawn(async move {
                    if let Err(e) = worker.run().await {
                        log::error!("[SCHEDULER][Worker] Error in worker: {}", e);
                    }
                })
            })
            .collect();

        // Spawn job puller task
        let puller_handle = {
            let puller = self.job_puller.clone();
            tokio::spawn(async move {
                if let Err(e) = puller.run().await {
                    log::error!("[SCHEDULER] Error in job puller: {}", e);
                }
            })
        };

        // Wait for shutdown signal
        // ...

        // Wait for puller to complete
        if let Err(e) = puller_handle.await {
            log::error!("[SCHEDULER] Error waiting for puller to complete: {}", e);
        }

        // When shutting down:
        drop(self.tx.clone()); // Signal no more work

        // Wait for workers to complete
        if let Err(e) = futures::future::try_join_all(worker_handles).await {
            log::error!("[SCHEDULER] Error waiting for workers to complete: {}", e);
        }

        // Ideally should never reach here
        log::info!("[SCHEDULER] Shutdown complete");
        Ok(())
    }
}
