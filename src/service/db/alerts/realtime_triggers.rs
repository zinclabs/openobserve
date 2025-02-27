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

use config::utils::json;

use crate::{common::infra::config::REALTIME_ALERT_TRIGGERS, service::db};

// Watches only for realtime alert triggers
pub async fn watch() -> Result<(), anyhow::Error> {
    let key = format!(
        "{}{}/",
        db::scheduler::TRIGGERS_KEY,
        db::scheduler::TriggerModule::Alert
    );
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(&key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching alert realtime triggers");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_alert_realtime_triggers: event channel closed");
                break;
            }
        };
        match ev {
            // Cluster coordinator sends put events only for realtime alerts
            db::Event::Put(ev) => {
                // Currently, cluster coordinator sends only the alert triggers
                // ev.key format -> /triggers/<alert|report>/{item_key}
                // item_key format -> {org_id}/{module_key}
                let item_key = ev.key.strip_prefix(&key).unwrap();
                let columns = item_key.split_once('/').unwrap();
                let item_value: db::scheduler::Trigger =
                    if ev.value.is_none() || ev.value.as_ref().unwrap().is_empty() {
                        match db::scheduler::get(
                            columns.0,
                            config::meta::triggers::TriggerModule::Alert,
                            columns.1,
                        )
                        .await
                        {
                            Ok(val) => val,
                            Err(e) => {
                                log::error!("Error getting value: {}", e);
                                continue;
                            }
                        }
                    } else {
                        json::from_slice(&ev.value.unwrap()).unwrap()
                    };
                REALTIME_ALERT_TRIGGERS
                    .write()
                    .await
                    .insert(item_key.to_owned(), item_value);
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(&key).unwrap();
                REALTIME_ALERT_TRIGGERS.write().await.remove(item_key);
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let triggers = db::scheduler::list(Some(db::scheduler::TriggerModule::Alert)).await?;
    let mut cache = REALTIME_ALERT_TRIGGERS.write().await;
    for trigger in triggers {
        if trigger.is_realtime {
            cache.insert(format!("{}/{}", trigger.org, trigger.module_key), trigger);
        }
    }
    log::info!("Alert realtime triggers Cached");
    Ok(())
}
