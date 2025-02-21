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

mod action_scripts;
mod alerts;
mod cipher_keys;
mod dashboards;
mod destinations;
mod distinct_values;
mod folders;
mod meta;
mod pipelines;
mod ratelimit;
mod scheduler;
mod schemas;
mod search_job;
mod short_urls;
mod templates;

use config::cluster::{is_offline, LOCAL_NODE};
use o2_enterprise::enterprise::super_cluster::{
    queue::{
        ActionScriptsQueue, AlertsQueue, DashboardsQueue, DestinationsQueue, FoldersQueue,
        MetaQueue, PipelinesQueue, SchemasQueue, SearchJobsQueue, SuperClusterQueueTrait,
        TemplatesQueue,
    },
    queue_item::ratelimit::RatelimitSuperClusterQueue,
};

/// Creates a super cluster queue for each super cluster topic and begins
/// polling messages from each queue in a separate thread.
pub async fn init() -> Result<(), anyhow::Error> {
    let meta_queue = MetaQueue {
        on_meta_msg: meta::process,
        on_distinct_values_msg: distinct_values::process,
        on_short_url_msg: short_urls::process,
        on_schema_msg: schemas::process,
        on_alert_msg: alerts::process,
        on_scheduler_msg: scheduler::process,
        on_search_job_msg: search_job::process,
        on_dashboard_msg: dashboards::process,
        on_pipeline_msg: pipelines::process,
        on_cipher_key_msg: cipher_keys::process,
    };
    let schema_queue = SchemasQueue {
        on_schema_msg: schemas::process,
    };
    let alerts_queue = AlertsQueue {
        on_alert_msg: alerts::process,
        on_scheduler_msg: scheduler::process,
    };
    let search_jobs_queue = SearchJobsQueue {
        on_search_job_msg: search_job::process,
    };
    let dashboards_queue = DashboardsQueue {
        on_dashboard_msg: dashboards::process,
    };
    let pipelines_queue = PipelinesQueue {
        on_pipeline_msg: pipelines::process,
    };
    let folders_queue = FoldersQueue {
        on_folder_msg: folders::process,
    };
    let templates_queue = TemplatesQueue {
        on_template_msg: templates::process,
    };
    let destinations_queue = DestinationsQueue {
        on_destination_msg: destinations::process,
    };
    let action_scripts_queue = ActionScriptsQueue {
        on_action_script_msg: action_scripts::process,
    };

    let ratelimit_queue = RatelimitSuperClusterQueue {
        on_rate_limit_msg: ratelimit::process,
    };

    let queues: Vec<Box<dyn SuperClusterQueueTrait + Sync + Send>> = vec![
        Box::new(meta_queue),
        Box::new(schema_queue),
        Box::new(alerts_queue),
        Box::new(search_jobs_queue),
        Box::new(dashboards_queue),
        Box::new(pipelines_queue),
        Box::new(folders_queue),
        Box::new(templates_queue),
        Box::new(destinations_queue),
        Box::new(action_scripts_queue),
        Box::new(ratelimit_queue),
    ];

    for queue in queues {
        queue.create_queue().await?;

        if LOCAL_NODE.is_compactor() {
            tokio::task::spawn(async move {
                loop {
                    if is_offline() {
                        break;
                    }
                    if let Err(e) = queue.subscribe().await {
                        log::error!("[SUPER_CLUSTER:sync] failed to subscribe: {}", e);
                    }
                }
            });
        }
    }

    Ok(())
}
