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

use std::sync::Arc;

use anyhow::{anyhow, Context};
use bytes::Bytes;
use chrono::Utc;
use config::get_config;
use infra::{
    db::{Event, NEED_WATCH},
    table::short_urls,
};

use crate::{common::infra::config::SHORT_URLS, service::db};

// DBKey to set short URL's
pub const SHORT_URL_KEY: &str = "/short_urls/";
// GC interval for `SHORT_URLS` cache in days
const SHORT_URL_GC_INTERVAL: i64 = 1; // days
const SHORT_URL_CACHE_LIMIT: i64 = 10_000; // records

pub async fn get(short_id: &str) -> Result<String, anyhow::Error> {
    if let Some(v) = SHORT_URLS.get(short_id) {
        return Ok(v.original_url.to_string());
    }

    let val = short_urls::get(short_id)
        .await
        .map_err(|_| anyhow!("Short URL not found in db"))?;
    let original_url = val.original_url.clone();
    SHORT_URLS.insert(short_id.to_string(), val);
    Ok(original_url)
}

pub async fn set(short_id: &str, entry: short_urls::ShortUrlRecord) -> Result<(), anyhow::Error> {
    if let Err(e) = short_urls::add(short_id, &entry.original_url).await {
        return Err(e).context("Failed to add short URL to DB");
    }

    // trigger watch event by putting empty value to cluster coordinator
    let cache_key = format!("{SHORT_URL_KEY}{short_id}");
    let cluster_coordinator = super::get_coordinator().await;
    cluster_coordinator
        .put(&cache_key, Bytes::new(), NEED_WATCH, None)
        .await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if o2_enterprise::enterprise::common::infra::config::get_config()
        .super_cluster
        .enabled
    {
        o2_enterprise::enterprise::super_cluster::queue::put(&cache_key, Bytes::new(), true, None)
            .await?;
    }

    Ok(())
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = SHORT_URL_KEY;
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching short URLs");

    // Spawn a background task for garbage collection
    let config = get_config();
    tokio::spawn(run_gc_task(
        days_to_minutes(SHORT_URL_GC_INTERVAL),
        days_to_minutes(config.limit.short_url_retention_days),
    ));

    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_short_url: event channel closed");
                return Ok(());
            }
        };

        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                let item_value = match short_urls::get(item_key).await {
                    Ok(val) => val,
                    Err(e) => {
                        log::error!("Error getting value: {}", e);
                        continue;
                    }
                };
                SHORT_URLS.insert(item_key.to_string(), item_value);
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                SHORT_URLS.remove(item_key);
            }
            Event::Empty => {}
        }
    }
}

/// Preload all short URLs from the database into the cache at startup.
pub async fn cache() -> Result<(), anyhow::Error> {
    let ret = short_urls::list(Some(SHORT_URL_CACHE_LIMIT)).await?;
    for row in ret.into_iter() {
        SHORT_URLS.insert(row.short_id.to_owned(), row);
    }
    log::info!("[SHORT_URLS] Cached with len: {}", SHORT_URLS.len());
    Ok(())
}

// Background task to run GC at a regular interval
async fn run_gc_task(gc_interval_minutes: i64, retention_period_minutes: i64) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
        gc_interval_minutes as u64 * 60,
    ));

    loop {
        interval.tick().await;

        if let Err(e) = gc_cache(retention_period_minutes).await {
            log::error!("Error during garbage collection: {}", e);
        }
    }
}

// Garbage Collection to clean up expired entries from cache and db
pub async fn gc_cache(retention_period_minutes: i64) -> Result<(), anyhow::Error> {
    log::info!(
        "[SHORT_URLS] Garbage collection start with cache size: {}",
        SHORT_URLS.len(),
    );

    let retention_period = chrono::Duration::minutes(retention_period_minutes);
    let expired_before = Utc::now() - retention_period;

    // get expired ids
    if let Ok(expired_short_ids) = short_urls::get_expired(
        expired_before.timestamp_micros(),
        Some(SHORT_URL_CACHE_LIMIT),
    )
    .await
    {
        if !expired_short_ids.is_empty() {
            // delete from db
            short_urls::batch_remove(expired_short_ids.clone()).await?;

            // delete from cache
            let cluster_coordinator = super::get_coordinator().await;
            for short_id in expired_short_ids {
                let cache_key = format!("{SHORT_URL_KEY}{short_id}");

                // super cluster
                #[cfg(feature = "enterprise")]
                if o2_enterprise::enterprise::common::infra::config::get_config()
                    .super_cluster
                    .enabled
                {
                    o2_enterprise::enterprise::super_cluster::queue::delete(
                        &cache_key, false, true, None,
                    )
                    .await?;
                }

                cluster_coordinator
                    .delete(&cache_key, false, NEED_WATCH, None)
                    .await?;
            }
        }
    }

    log::info!(
        "[SHORT_URLS] Garbage collection end with cache size: {}",
        SHORT_URLS.len(),
    );
    Ok(())
}

fn days_to_minutes(days: i64) -> i64 {
    days * 24 * 60
}
