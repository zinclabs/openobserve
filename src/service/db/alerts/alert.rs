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

use bytes::Bytes;
use config::{
    meta::{
        alerts::alert::{Alert, ListAlertsParams},
        folder::Folder,
        stream::StreamType,
    },
    utils::json,
};
use infra::{
    cluster_coordinator::alerts as cluster,
    db::{connect_to_orm, ORM_CLIENT},
    table::alerts as table,
};
use sea_orm::{ConnectionTrait, TransactionTrait};
use svix_ksuid::Ksuid;

use crate::{common::infra::config::STREAM_ALERTS, service::db};

/// Gets the alert and its parent folder.
pub async fn get_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<Option<(Folder, Alert)>, infra::errors::Error> {
    // We cannot check the cache because the cache stores alerts by stream type
    // and stream name which are currently unknown.
    let folder_and_alert = table::get_by_id(conn, org_id, alert_id).await?;
    Ok(folder_and_alert)
}

pub async fn get_by_name(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<Option<Alert>, infra::errors::Error> {
    let stream_key = cache_stream_key(org_id, stream_type, stream_name);
    let mut value: Option<Alert> = if let Some(v) = STREAM_ALERTS.read().await.get(&stream_key) {
        v.iter().find(|x| x.name.eq(name)).cloned()
    } else {
        None
    };
    if value.is_none() {
        let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
        let alert =
            table::get_by_name(client, org_id, "default", stream_type, stream_name, name).await?;
        value = alert.map(|(_f, a)| a);
    }
    Ok(value)
}

pub async fn set(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    alert: Alert,
    create: bool,
) -> Result<(), infra::errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match table::put(client, org_id, "default", alert).await {
        Ok(alert) => {
            cluster::emit_put_event(org_id, &alert).await?;
            #[cfg(feature = "enterprise")]
            if create {
                super_cluster::emit_create_event(org_id, "default", alert.clone()).await?;
            } else {
                super_cluster::emit_update_event(org_id, None, alert.clone()).await?;
            }

            let schedule_key = scheduler_key(stream_type, stream_name, &alert.name);
            // Get the trigger from scheduler
            let mut trigger = db::scheduler::Trigger {
                org: org_id.to_string(),
                module_key: schedule_key.clone(),
                next_run_at: chrono::Utc::now().timestamp_micros(),
                is_realtime: alert.is_real_time,
                is_silenced: false,
                ..Default::default()
            };

            if create {
                match db::scheduler::push(trigger).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
                        Ok(())
                    }
                }
            } else {
                match db::scheduler::get(org_id, db::scheduler::TriggerModule::Alert, &schedule_key)
                    .await
                {
                    Ok(job) => {
                        trigger.data = job.data;
                        trigger.start_time = job.start_time;
                        match db::scheduler::update_trigger(trigger).await {
                            Ok(_) => Ok(()),
                            Err(e) => {
                                log::error!(
                                    "Failed to update trigger for alert {schedule_key}: {}",
                                    e
                                );
                                Ok(())
                            }
                        }
                    }
                    Err(_) => match db::scheduler::push(trigger).await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
                            Ok(())
                        }
                    },
                }
            }
        }
        Err(e) => Err(e),
    }
}

pub async fn set_without_updating_trigger(org_id: &str, alert: Alert) -> Result<(), anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let alert = table::put(client, org_id, "default", alert).await?;
    cluster::emit_put_event(org_id, &alert).await?;
    #[cfg(feature = "enterprise")]
    if alert.id.is_some() {
        super_cluster::emit_update_event(org_id, None, alert.clone()).await?;
    } else {
        super_cluster::emit_create_event(org_id, "default", alert.clone()).await?;
    }
    Ok(())
}

pub async fn create<C: TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: &str,
    alert: Alert,
) -> Result<Alert, infra::errors::Error> {
    let alert = table::create(conn, org_id, folder_id, alert, false).await?;

    cluster::emit_put_event(org_id, &alert).await?;
    #[cfg(feature = "enterprise")]
    super_cluster::emit_create_event(org_id, folder_id, alert.clone()).await?;

    let schedule_key = scheduler_key(alert.stream_type, &alert.stream_name, &alert.name);
    let trigger = db::scheduler::Trigger {
        org: org_id.to_string(),
        module_key: schedule_key.clone(),
        next_run_at: chrono::Utc::now().timestamp_micros(),
        is_realtime: alert.is_real_time,
        is_silenced: false,
        ..Default::default()
    };

    let _ = db::scheduler::push(trigger).await.map_err(|e| {
        log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
        e
    });

    Ok(alert)
}

pub async fn update<C: ConnectionTrait + TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: Option<&str>,
    alert: Alert,
) -> Result<Alert, infra::errors::Error> {
    let alert = table::update(conn, org_id, folder_id, alert).await?;

    cluster::emit_put_event(org_id, &alert).await?;
    #[cfg(feature = "enterprise")]
    super_cluster::emit_update_event(org_id, folder_id, alert.clone()).await?;

    let schedule_key = scheduler_key(alert.stream_type, &alert.stream_name, &alert.name);
    let mut trigger = db::scheduler::Trigger {
        org: org_id.to_string(),
        module_key: schedule_key.clone(),
        next_run_at: chrono::Utc::now().timestamp_micros(),
        is_realtime: alert.is_real_time,
        is_silenced: false,
        ..Default::default()
    };

    if let Ok(job) =
        db::scheduler::get(org_id, db::scheduler::TriggerModule::Alert, &schedule_key).await
    {
        trigger.data = job.data;
        trigger.start_time = job.start_time;
        let _ = db::scheduler::update_trigger(trigger).await.map_err(|e| {
            log::error!("Failed to update trigger for alert {schedule_key}: {}", e);
        });
    } else {
        let _ = db::scheduler::push(trigger).await.map_err(|e| {
            log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
            e
        });
    }

    Ok(alert)
}

pub async fn delete_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<(), infra::errors::Error> {
    let Some((_folder, alert)) = table::get_by_id(conn, org_id, alert_id).await? else {
        return Ok(());
    };

    table::delete_by_id(conn, org_id, alert_id).await?;
    cluster::emit_delete_event(org_id, alert.stream_type, &alert.stream_name, &alert.name).await?;
    #[cfg(feature = "enterprise")]
    super_cluster::emit_delete_event(
        org_id,
        alert.stream_type,
        &alert.stream_name,
        &alert.name,
        alert_id,
    )
    .await?;

    let schedule_key = scheduler_key(alert.stream_type, &alert.stream_name, &alert.name);
    if let Err(e) =
        db::scheduler::delete(org_id, db::scheduler::TriggerModule::Alert, &schedule_key).await
    {
        log::error!("Failed to delete trigger: {}", e);
    };
    Ok(())
}

pub async fn delete_by_name(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    name: &str,
) -> Result<(), infra::errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    #[cfg(feature = "enterprise")]
    let Some(alert_id) =
        table::get_by_name(client, org_id, "default", stream_type, stream_name, name)
            .await?
            .and_then(|(_, a)| a.id)
    else {
        return Ok(());
    };

    table::delete_by_name(client, org_id, "default", stream_type, stream_name, name).await?;
    cluster::emit_delete_event(org_id, stream_type, stream_name, name).await?;

    #[cfg(feature = "enterprise")]
    super_cluster::emit_delete_event(org_id, stream_type, stream_name, name, alert_id).await?;

    let schedule_key = scheduler_key(stream_type, stream_name, name);
    if let Err(e) =
        db::scheduler::delete(org_id, db::scheduler::TriggerModule::Alert, &schedule_key).await
    {
        log::error!("Failed to delete trigger: {}", e);
    };
    Ok(())
}

pub async fn list(
    org_id: &str,
    stream_type: Option<StreamType>,
    stream_name: Option<&str>,
) -> Result<Vec<Alert>, infra::errors::Error> {
    let params = ListAlertsParams::new(org_id).in_folder("default");
    let params = if let Some(stream_name) = stream_name {
        params.for_stream(stream_type.unwrap_or_default(), Some(stream_name))
    } else {
        params
    };

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let alerts = table::list(client, params)
        .await?
        .into_iter()
        .map(|(_f, a)| a)
        .collect();
    Ok(alerts)
}

pub async fn list_with_folders<C: ConnectionTrait>(
    conn: &C,
    params: ListAlertsParams,
) -> Result<Vec<(Folder, Alert)>, infra::errors::Error> {
    table::list(conn, params).await
}

pub async fn watch() -> Result<(), anyhow::Error> {
    cluster::watch_events(put_into_cache, delete_from_cache).await
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let alerts = table::list_all(client).await?;

    for alert in alerts {
        let mut cacher = STREAM_ALERTS.write().await;
        let stream_key = cache_stream_key(&alert.org_id, alert.stream_type, &alert.stream_name);
        let group = cacher.entry(stream_key.to_string()).or_default();
        group.push(alert);
    }
    log::info!("Alerts Cached");
    Ok(())
}

pub async fn reset() -> Result<(), anyhow::Error> {
    let key = "/alerts/";
    Ok(db::delete(key, true, db::NO_NEED_WATCH, None).await?)
}

async fn put_into_cache(
    org: String,
    stream_type: StreamType,
    stream_name: String,
    alert_name: String,
    value: Option<Bytes>,
) -> Result<(), anyhow::Error> {
    let item_value: Alert = if config::get_config().common.meta_store_external {
        let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
        match table::get_by_name(
            client,
            &org,
            "default",
            stream_type,
            &stream_name,
            &alert_name,
        )
        .await
        {
            Ok(Some(val)) => val.1,
            Ok(None) => {
                log::error!("Tried to get alert that does not exist in DB");
                return Ok(());
            }
            Err(e) => {
                log::error!("Error getting value: {}", e);
                return Ok(());
            }
        }
    } else {
        json::from_slice(&value.unwrap()).unwrap()
    };
    let mut cacher = STREAM_ALERTS.write().await;
    let stream_key = cache_stream_key(&org, stream_type, &stream_name);
    let group = cacher.entry(stream_key.to_string()).or_default();
    if group.contains(&item_value) {
        let idx = group.iter().position(|x| x.eq(&item_value)).unwrap();
        let _ = std::mem::replace(&mut group[idx], item_value);
    } else {
        group.push(item_value);
    }
    Ok(())
}

async fn delete_from_cache(
    org: String,
    stream_type: StreamType,
    stream_name: String,
    alert_name: String,
) -> Result<(), anyhow::Error> {
    let mut cacher = STREAM_ALERTS.write().await;
    let stream_key = cache_stream_key(&org, stream_type, &stream_name);
    let group = match cacher.get_mut(&stream_key) {
        Some(v) => v,
        None => return Ok(()),
    };
    group.retain(|v| !v.name.eq(&alert_name));
    Ok(())
}

/// Returns the key used to store alerts in the in-memory cache, grouped by
/// stream.
fn cache_stream_key(org: &str, stream_type: StreamType, stream_name: &str) -> String {
    format!("{org}/{stream_type}/{stream_name}")
}

/// Returns the key used to schedule a trigger for the alert.
fn scheduler_key(stream_type: StreamType, stream_name: &str, alert_name: &str) -> String {
    format!("{stream_type}/{stream_name}/{alert_name}")
}

/// Helper functions for sending events to the super cluster queue.
#[cfg(feature = "enterprise")]
mod super_cluster {
    use config::meta::{alerts::alert::Alert, stream::StreamType};
    use infra::errors::Error;
    use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;
    use svix_ksuid::Ksuid;

    /// Sends event to the super cluster queue indicating that an alert has been
    /// created in the database.
    pub async fn emit_create_event(
        org: &str,
        folder_id: &str,
        alert: Alert,
    ) -> Result<(), infra::errors::Error> {
        if get_o2_config().super_cluster.enabled {
            // let key = alert_key(org, alert.stream_type, &alert.stream_name, &alert.name);
            // let value = json::to_vec(&alert)?.into();
            log::debug!("Sending super cluster alert creation event: {:?}", alert);
            // o2_enterprise::enterprise::super_cluster::queue::put(&key, value, true, None)
            //     .await
            //     .map_err(|e| Error::Message(e.to_string()))?;
            o2_enterprise::enterprise::super_cluster::queue::alerts_create(org, folder_id, alert)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    /// Sends event to the super cluster queue indicating that an alert has been
    /// updated in the database.
    pub async fn emit_update_event(
        org: &str,
        folder_id: Option<&str>,
        alert: Alert,
    ) -> Result<(), infra::errors::Error> {
        if get_o2_config().super_cluster.enabled {
            // let key = alert_key(org, alert.stream_type, &alert.stream_name, &alert.name);
            // let value = json::to_vec(&alert)?.into();
            log::debug!("Sending super cluster alert update event: {:?}", alert);
            // o2_enterprise::enterprise::super_cluster::queue::put(&key, value, true, None)
            //     .await
            //     .map_err(|e| Error::Message(e.to_string()))?;
            o2_enterprise::enterprise::super_cluster::queue::alerts_update(org, folder_id, alert)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    /// Sends event to the super cluster queue indicating that an alert has been
    /// deleted from the database.
    pub async fn emit_delete_event(
        org: &str,
        stream_type: StreamType,
        stream_name: &str,
        alert_name: &str,
        alert_id: Ksuid,
    ) -> Result<(), infra::errors::Error> {
        if get_o2_config().super_cluster.enabled {
            let key = alert_key(org, stream_type, stream_name, alert_name);
            log::debug!("Sending super cluster alert delete event: {:?}", key);
            // o2_enterprise::enterprise::super_cluster::queue::delete(&key, false, true, None)
            //     .await
            //     .map_err(|e| Error::Message(e.to_string()))?;
            o2_enterprise::enterprise::super_cluster::queue::alerts_delete(org, alert_id)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    /// Sends event to the super cluster queue indicating that all alert have
    /// been deleted from the database.
    pub async fn _emit_delete_all_event() -> Result<(), infra::errors::Error> {
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::delete("/alerts/", true, false, None)
                .await
                .map_err(|e| Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    /// Returns the key used to identify an individual alert in events sent to
    /// the super cluster queue.
    fn alert_key(
        org: &str,
        stream_type: StreamType,
        stream_name: &str,
        alert_name: &str,
    ) -> String {
        format!("/alerts/{org}/{stream_type}/{stream_name}/{alert_name}")
    }
}
