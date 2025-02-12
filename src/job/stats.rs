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

use config::{cluster::LOCAL_NODE, get_config};
use tokio::time;

use crate::service::{compact::stats::update_stats_from_file_list, db};

pub async fn run() -> Result<(), anyhow::Error> {
    // tokio::task::spawn(async move { usage_report_stats().await });
    tokio::task::spawn(async move { file_list_update_stats().await });
    tokio::task::spawn(async move { cache_stream_stats().await });
    Ok(())
}

// get stats from file_list to update stream_stats
async fn file_list_update_stats() -> Result<(), anyhow::Error> {
    let cfg = get_config();
    if !LOCAL_NODE.is_compactor() {
        return Ok(());
    }

    // should run it at least every 10 seconds
    let mut interval = time::interval(time::Duration::from_secs(std::cmp::max(
        10,
        cfg.limit.calculate_stats_interval,
    )));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        match update_stats_from_file_list().await {
            Err(e) => {
                log::error!(
                    "[STATS] run update stream stats from file list error: {}",
                    e
                );
            }
            Ok(Some((offset, max_pk))) => {
                log::debug!(
                    "[STATS] run update stream stats success, offset: {offset}, max_pk: {max_pk}"
                );
            }
            Ok(None) => {}
        }
    }
}

async fn cache_stream_stats() -> Result<(), anyhow::Error> {
    if !LOCAL_NODE.is_querier() {
        return Ok(());
    }

    // should run it at least every 5 minutes
    let mut interval = time::interval(time::Duration::from_secs(std::cmp::max(
        300,
        get_config().limit.calculate_stats_interval,
    )));
    interval.tick().await; // trigger the first run
    loop {
        interval.tick().await;
        if let Err(e) = db::file_list::cache_stats().await {
            log::error!("[STATS] run cached stream stats error: {}", e);
        } else {
            log::debug!("[STATS] run cached stream stats success");
        }
    }
}
