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

// Warning: The logic in this file should not be modified. Doing so will change
// the behavior of the migration sequence and could result in users who execute
// this migration before the changes and users who execute the migration after
// the changes in executing different migration logic.

use chrono::Utc;
use config::{
    meta::{
        dashboards::reports::Report,
        triggers::{Trigger, TriggerModule, TriggerStatus},
    },
    utils::json,
};
use datafusion::arrow::datatypes::Schema;
use infra::{
    db::{self as infra_db, NO_NEED_WATCH},
    dist_lock,
    errors::{DbError, Error},
    scheduler,
};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::openfga::add_init_ofga_tuples;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::openfga::authorizer::authz::get_ownership_tuple;

use crate::{
    common::{
        infra::config::VERSION,
        utils::auth::{into_ofga_supported_format, is_ofga_unsupported},
    },
    service::db,
};

const SCHEMA_MIGRATION_KEY: &str = "/migration/schema_versions/status";
const META_MIGRATION_VERSION_KEY: &str = "/migration/meta/version";

#[deprecated(since = "0.14.0", note = "will be removed in 0.17.0")]
pub async fn run() -> Result<(), anyhow::Error> {
    match upgrade_schema_row_per_version().await {
        std::result::Result::Ok(true) => {
            log::info!("[Schema:Migration]: Starting schema migration");
        }
        std::result::Result::Ok(false) => {
            log::info!("[Schema:Migration]: Schema migration already done");
            return Ok(());
        }
        Err(err) => {
            log::error!(
                "[Schema:Migration]: Error checking schema migration status: {}",
                err
            );
            return Err(err);
        }
    };

    // get lock
    let locker = infra::dist_lock::lock(SCHEMA_MIGRATION_KEY, 0).await?;

    // after get lock, need check again
    match upgrade_schema_row_per_version().await {
        std::result::Result::Ok(true) => {
            log::info!("[Schema:Migration]: Starting schema migration");
        }
        std::result::Result::Ok(false) => {
            log::info!("[Schema:Migration]: Schema migration already done");
            dist_lock::unlock(&locker).await?;
            return Ok(());
        }
        Err(err) => {
            log::error!(
                "[Schema:Migration]: Error checking schema migration status: {}",
                err
            );
            dist_lock::unlock(&locker).await?;
            return Err(err);
        }
    };

    // upgrade meta
    if let Err(e) = upgrade_meta().await {
        dist_lock::unlock(&locker).await?;
        return Err(e);
    }

    // upgrade triggers
    if let Err(e) = upgrade_trigger().await {
        dist_lock::unlock(&locker).await?;
        return Err(e);
    }

    // set migration status
    let db = infra_db::get_db().await;
    if let Err(e) = db
        .put(
            SCHEMA_MIGRATION_KEY,
            Utc::now().timestamp_micros().to_string().into(),
            NO_NEED_WATCH,
            None,
        )
        .await
    {
        // unlock the lock
        dist_lock::unlock(&locker).await?;
        return Err(e.into());
    }

    // unlock the lock
    dist_lock::unlock(&locker).await?;

    Ok(())
}

async fn upgrade_meta() -> Result<(), anyhow::Error> {
    let default_end_dt = "0".to_string();
    let cc = infra_db::get_coordinator().await;
    if let Err(e) = cc.add_start_dt_column().await {
        return Err(e.into());
    }

    let db = infra_db::get_db().await;
    if let Err(e) = db.add_start_dt_column().await {
        return Err(e.into());
    }

    log::info!("[Schema:Migration]: Migrating schemas");
    let db_key = "/schema/".to_string();
    log::info!("[Schema:Migration]: Listing all schemas");
    let data = db.list(&db_key).await?;
    for (key, val) in data {
        log::info!("[Schema:Migration]: Start migrating schema: {}", key);
        let schemas: Vec<Schema> = json::from_slice(&val).unwrap();
        let versions_count = schemas.len();
        let mut prev_end_dt: i64 = 0;

        for schema in schemas {
            if schema.fields().is_empty() && versions_count > 1 {
                continue; // Skip empty schema when there are multiple versions
            }
            let meta = schema.metadata();
            let start_dt: i64 = match meta.get("start_dt") {
                Some(val) => val.clone().parse().unwrap(),
                None => {
                    if prev_end_dt == 0 {
                        meta.get("created_at").unwrap().clone().parse().unwrap()
                    } else {
                        prev_end_dt
                    }
                }
            };
            prev_end_dt = meta
                .get("end_dt")
                .unwrap_or(&default_end_dt)
                .clone()
                .parse()
                .unwrap();
            if let Err(e) = db
                .put(
                    &key,
                    json::to_vec(&vec![schema]).unwrap().into(),
                    NO_NEED_WATCH,
                    Some(start_dt),
                )
                .await
            {
                return Err(e.into());
            }
        }
        log::info!(
            "[Schema:Migration]: Done creating row per version of schema: {}",
            key
        );
        if let Err(e) = db.delete(&key, false, infra_db::NEED_WATCH, Some(0)).await {
            return Err(e.into());
        }
        log::info!("[Schema:Migration]: Done migrating schema: {}", key);
    }

    Ok(())
}

async fn upgrade_trigger() -> Result<(), anyhow::Error> {
    let db = infra_db::get_db().await;
    log::info!("[Trigger:Migration]: Migrating triggers");
    let db_key_prefix = "/trigger/".to_string();
    log::info!("[Trigger:Migration]: Listing all triggers");
    let data = db.list(&db_key_prefix).await?;
    for (key, val) in data {
        let db_key = key;
        let key = db_key.strip_prefix(&db_key_prefix).unwrap();
        log::info!("[Trigger:Migration]: Start migrating trigger: {}", key);
        let (org_id, module_key) = match key.split_once('/') {
            Some(columns) => columns,
            None => {
                // Corrupted data, delete the trigger
                _ = db.delete(&db_key, false, infra_db::NEED_WATCH, None).await;
                continue;
            }
        };
        let data: json::Value = json::from_slice(&val).unwrap();
        let data = data.as_object().unwrap();
        scheduler::push(Trigger {
            org: org_id.to_string(),
            module: TriggerModule::Alert,
            module_key: module_key.to_string(),
            next_run_at: data.get("next_run_at").unwrap().as_i64().unwrap(),
            is_realtime: data.get("is_realtime").unwrap().as_bool().unwrap(),
            is_silenced: data.get("is_silenced").unwrap().as_bool().unwrap(),
            status: TriggerStatus::Waiting,
            ..Default::default()
        })
        .await?;
        log::info!("[Schema:Migration]: Done migrating trigger: {}", key);
    }

    Ok(())
}

async fn upgrade_schema_row_per_version() -> Result<bool, anyhow::Error> {
    let db = infra_db::get_db().await;
    match db.get(SCHEMA_MIGRATION_KEY).await {
        std::result::Result::Ok(val) => {
            let val_str = std::str::from_utf8(&val).unwrap();
            let val = val_str.parse::<i64>().unwrap_or(0);
            if val > 0 {
                Ok(false)
            } else {
                Ok(true)
            }
        }
        Err(_) => Ok(true),
    }
}

/// Migrate alerts, reports, templates, destination names with ofga compatible format
pub async fn migrate_resource_names() -> Result<(), anyhow::Error> {
    // fast path
    if let Ok(false) = need_meta_resource_name_migration().await {
        return Ok(()); // Resource name migration already done
    }
    // slow path
    let locker = infra::dist_lock::lock(META_MIGRATION_VERSION_KEY, 0).await?;
    match need_meta_resource_name_migration().await {
        Ok(true) => {
            log::info!("Starting migration of unsupported resource names");
        }
        Ok(false) => {
            log::info!("Resource name migration already done");
            dist_lock::unlock(&locker).await?;
            return Ok(());
        }
        Err(e) => {
            log::error!("Error when checking migration version");
            dist_lock::unlock(&locker).await?;
            return Err(e);
        }
    }

    if let Err(e) = migrate_alert_template_names().await {
        dist_lock::unlock(&locker).await?;
        return Err(e);
    }
    if let Err(e) = migrate_alert_destination_names().await {
        dist_lock::unlock(&locker).await?;
        return Err(e);
    }
    if let Err(e) = migrate_alert_names().await {
        dist_lock::unlock(&locker).await?;
        return Err(e);
    }
    if let Err(e) = migrate_report_names().await {
        dist_lock::unlock(&locker).await?;
        return Err(e);
    }

    // set migration status
    let db = infra_db::get_db().await;
    if let Err(e) = db
        .put(
            META_MIGRATION_VERSION_KEY,
            VERSION.to_string().into(),
            NO_NEED_WATCH,
            None,
        )
        .await
    {
        // unlock the lock
        dist_lock::unlock(&locker).await?;
        return Err(e.into());
    }

    // unlock the lock
    dist_lock::unlock(&locker).await?;
    Ok(())
}

async fn need_meta_resource_name_migration() -> Result<bool, anyhow::Error> {
    let db = infra_db::get_db().await;
    match db.get(META_MIGRATION_VERSION_KEY).await {
        std::result::Result::Ok(_val) => {
            // Key is present meaning the migration is done
            Ok(false)
        }
        Err(e) => {
            if let Error::DbError(DbError::KeyNotExists(_)) = e {
                Ok(true)
            } else {
                Err(anyhow::anyhow!("Error checking migration version: {e}"))
            }
        }
    }
}

async fn migrate_report_names() -> Result<(), anyhow::Error> {
    let db = infra_db::get_db().await;
    log::info!("[Report:Migration]: Migrating reports");
    let db_key_prefix = "/reports/".to_string();
    log::info!("[Report:Migration]: Listing all reports");
    let data = db.list(&db_key_prefix).await?;
    #[cfg(feature = "enterprise")]
    let mut write_tuples = vec![];
    for (key, val) in data {
        let db_key = key;
        let key = db_key.strip_prefix(&db_key_prefix).unwrap();
        log::info!("[Report:Migration]: Start migrating report: {}", key);
        let keys: Vec<&str> = key.split('/').collect();
        let report_name = keys[keys.len() - 1];
        if is_ofga_unsupported(report_name) && keys.len() == 2 {
            let mut report: Report = json::from_slice(&val).unwrap();
            report.name = into_ofga_supported_format(report_name);
            #[cfg(feature = "enterprise")]
            get_ownership_tuple(keys[0], "reports", &report.name, &mut write_tuples);
            // First create an report copy with formatted report name
            match db::dashboards::reports::set(keys[0], &report, true).await {
                // Delete report with unsupported report name
                Ok(_) => {
                    if let Err(e) = db::dashboards::reports::delete(keys[0], report_name).await {
                        log::error!(
                            "[Report:Migration]: Error deleting report with unsupported report name: {report_name}: {e}"
                        );
                    }
                }
                Err(e) => {
                    log::error!(
                        "[Report:Migration]: error updating unsupported report name {report_name}: {e}"
                    );
                }
            }
        }
        log::info!("[Report:Migration]: Done migrating report: {}", key);
    }
    #[cfg(feature = "enterprise")]
    if !write_tuples.is_empty() && get_o2_config().openfga.enabled {
        add_init_ofga_tuples(write_tuples).await;
    }
    Ok(())
}

async fn migrate_alert_template_names() -> Result<(), anyhow::Error> {
    let db = infra_db::get_db().await;
    log::info!("[Template:Migration]: Migrating templates");
    let db_key_prefix = "/templates/".to_string();
    log::info!("[Template:Migration]: Listing all templates");
    let data = db.list(&db_key_prefix).await?;
    #[cfg(feature = "enterprise")]
    let mut write_tuples = vec![];
    for (key, val) in data {
        let db_key = key;
        let key = db_key.strip_prefix(&db_key_prefix).unwrap();
        log::info!("[Template:Migration]: Start migrating template: {}", key);
        let keys: Vec<&str> = key.split('/').collect();
        let temp_name = keys[keys.len() - 1];
        if is_ofga_unsupported(temp_name) && keys.len() == 2 {
            let mut temp: meta::Template = json::from_slice(&val).unwrap();
            temp.name = into_ofga_supported_format(temp_name);
            #[cfg(feature = "enterprise")]
            get_ownership_tuple(keys[0], "templates", &temp.name, &mut write_tuples);
            // First create an alert copy with formatted template name
            match meta::set_template_in_meta(keys[0], &mut temp).await {
                // Delete template with unsupported template name
                Ok(_) => {
                    if let Err(e) = db::alerts::templates::delete(keys[0], temp_name).await {
                        log::error!(
                            "[Template:Migration]: Error deleting unsupported template name {temp_name}: {e}"
                        );
                    }
                }
                Err(e) => {
                    log::error!(
                        "[Template:Migration]: Error updating unsupported template name {temp_name}: {e}"
                    );
                }
            }
        }
        log::info!("[Template:Migration]: Done migrating template: {}", key);
    }
    #[cfg(feature = "enterprise")]
    if !write_tuples.is_empty() && get_o2_config().openfga.enabled {
        add_init_ofga_tuples(write_tuples).await;
    }
    Ok(())
}

async fn migrate_alert_destination_names() -> Result<(), anyhow::Error> {
    let db = infra_db::get_db().await;
    log::info!("[Destination:Migration]: Migrating destinations");
    let db_key_prefix = "/destinations/".to_string();
    log::info!("[Destination:Migration]: Listing all destinations");
    let data = db.list(&db_key_prefix).await?;
    #[cfg(feature = "enterprise")]
    let mut write_tuples = vec![];
    for (key, val) in data {
        let db_key = key;
        let key = db_key.strip_prefix(&db_key_prefix).unwrap();
        log::info!(
            "[Destination:Migration]: Start migrating destination: {}",
            key
        );
        let keys: Vec<&str> = key.split('/').collect();
        let dest_name = keys[keys.len() - 1];
        let mut dest: meta::Destination = json::from_slice(&val).unwrap();
        let mut need_update = false;
        let mut create = false;
        if is_ofga_unsupported(dest_name) && keys.len() == 2 {
            dest.name = into_ofga_supported_format(dest_name);
            #[cfg(feature = "enterprise")]
            get_ownership_tuple(keys[0], "destinations", &dest.name, &mut write_tuples);
            need_update = true;
            create = true;
        }
        if is_ofga_unsupported(&dest.template) {
            dest.template = into_ofga_supported_format(&dest.template);
            need_update = true;
        }

        if need_update {
            // Create a new destination copy with formatted destination name
            match meta::set_destination(keys[0], &dest).await {
                // Delete destination with unsupported destination name
                Ok(_) => {
                    // New destination created, delete the old one
                    if create {
                        if let Err(e) = db::alerts::destinations::delete(keys[0], dest_name).await {
                            log::error!(
                                "Destination:Migration]: Error updating unsupported destination name {dest_name}: {e}"
                            );
                        }
                    }
                }
                Err(e) => {
                    log::error!(
                        "[Destination:Migration]: Error updating unsupported destination name {dest_name}: {e}"
                    );
                }
            }
        }
        log::info!(
            "[Destination:Migration]: Done migrating destination: {}",
            key
        );
    }
    #[cfg(feature = "enterprise")]
    if !write_tuples.is_empty() && get_o2_config().openfga.enabled {
        add_init_ofga_tuples(write_tuples).await;
    }
    Ok(())
}

async fn migrate_alert_names() -> Result<(), anyhow::Error> {
    let db = infra_db::get_db().await;
    log::info!("[Alert:Migration]: Migrating alerts");
    let db_key_prefix = "/alerts/".to_string();
    log::info!("[Alert:Migration]: Listing all alerts");
    let data = db.list(&db_key_prefix).await?;
    #[cfg(feature = "enterprise")]
    let mut write_tuples = vec![];
    for (key, val) in data {
        let db_key = key;
        let key = db_key.strip_prefix(&db_key_prefix).unwrap();
        log::info!("[Alert:Migration]: Start migrating alert: {}", key);
        let keys: Vec<&str> = key.split('/').collect();
        let alert_name = keys[keys.len() - 1];
        let mut need_update = false;
        let mut create = false;
        let mut alert: meta::Alert = match json::from_slice(&val) {
            Ok(alert) => alert,
            Err(_) => {
                log::error!("[Alert:Migration]: Failed to deserialize alert {alert_name}");
                continue;
            }
        };
        if is_ofga_unsupported(alert_name) && keys.len() == 4 {
            alert.name = into_ofga_supported_format(alert_name);
            #[cfg(feature = "enterprise")]
            {
                get_ownership_tuple(keys[0], "alerts", &alert.name, &mut write_tuples);
            }
            need_update = true;
            create = true;
        }

        let mut destinations = vec![];
        for dest in alert.destinations.iter() {
            if is_ofga_unsupported(dest) {
                need_update = true;
            }
            destinations.push(into_ofga_supported_format(dest));
        }

        if need_update {
            // Format the associated destinations as well. In case there were some errors while
            // formatting destination names in the last migration, this destinations list contain
            // wrong destination names which don't exist. This needs to be manually resolved by
            // updating the destinations of the alert.
            alert.destinations = destinations;
            // First create an alert copy with formatted alert name
            match meta::set_alert(
                keys[0],
                meta::StreamType::from(keys[1]),
                keys[2],
                &alert,
                create,
            )
            .await
            {
                // Delete alert with unsupported alert name
                Ok(_) => {
                    if create
                        && meta::delete_alert(
                            keys[0],
                            meta::StreamType::from(keys[1]),
                            keys[2],
                            alert_name,
                        )
                        .await
                        .is_err()
                    {
                        log::error!(
                            "[Alert:Migration]: Error deleting alerts with unsupported alert name: {alert_name}"
                        );
                    }
                }
                Err(e) => {
                    log::error!(
                        "[Alert:Migration]: Error updating unsupported alert name {alert_name}: {e}"
                    );
                }
            }
        }
        log::info!("[Alert:Migration]: Done migrating alert: {}", key);
    }

    #[cfg(feature = "enterprise")]
    if !write_tuples.is_empty() && get_o2_config().openfga.enabled {
        add_init_ofga_tuples(write_tuples).await;
    }
    Ok(())
}

/// This types and logic in this embedded module originally existed in other
/// parts of the codebase. However the types and logic in those module need to
/// be able to change and evolve over time, whereas the logic inside this
/// migration needs be immutable to maintain consistent behavior. Therefore
/// logic from other parts of the codebase has been copied into this migration
/// script to preserve its original behvaior. This logic should not be changed.
mod meta {
    use std::collections::HashMap;

    use chrono::{DateTime, FixedOffset};
    use serde::{Deserialize, Serialize};
    use serde_json::Value as JsonValue;

    use super::*;

    const DEFAULT_ORG: &str = "default";

    /// Inserts the alert into the meta table if it doesn't already exist or updates
    /// it if it does already exist.
    pub async fn set_alert(
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        alert: &Alert,
        create: bool,
    ) -> Result<(), anyhow::Error> {
        let schedule_key = format!("{stream_type}/{stream_name}/{}", alert.name);
        let key = format!("/alerts/{org_id}/{}", &schedule_key);
        match db::put(
            &key,
            json::to_vec(alert).unwrap().into(),
            db::NEED_WATCH,
            None,
        )
        .await
        {
            Ok(_) => {
                let trigger = db::scheduler::Trigger {
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
                } else if db::scheduler::exists(
                    org_id,
                    db::scheduler::TriggerModule::Alert,
                    &schedule_key,
                )
                .await
                {
                    match db::scheduler::update_trigger(trigger).await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            log::error!("Failed to update trigger for alert {schedule_key}: {}", e);
                            Ok(())
                        }
                    }
                } else {
                    match db::scheduler::push(trigger).await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            log::error!("Failed to save trigger for alert {schedule_key}: {}", e);
                            Ok(())
                        }
                    }
                }
            }
            Err(e) => Err(anyhow::anyhow!("Error save alert {schedule_key}: {}", e)),
        }
    }

    /// Deletes an alert from the meta table.
    pub async fn delete_alert(
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        name: &str,
    ) -> Result<(), anyhow::Error> {
        let schedule_key = format!("{stream_type}/{stream_name}/{name}");
        let key = format!("/alerts/{org_id}/{}", &schedule_key);
        match db::delete(&key, false, db::NEED_WATCH, None).await {
            Ok(_) => {
                match db::scheduler::delete(
                    org_id,
                    db::scheduler::TriggerModule::Alert,
                    &schedule_key,
                )
                .await
                {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        log::error!("Failed to delete trigger: {}", e);
                        Ok(())
                    }
                }
            }
            Err(e) => Err(anyhow::anyhow!("Error deleting alert: {e}")),
        }
    }

    /// Inserts the destination into the meta table if it doesn't already exist or
    /// updates it if it does already exist.
    pub async fn set_destination(
        org_id: &str,
        destination: &Destination,
    ) -> Result<(), anyhow::Error> {
        let key = format!("/destinations/{org_id}/{}", destination.name);
        Ok(db::put(
            &key,
            json::to_vec(destination).unwrap().into(),
            db::NEED_WATCH,
            None,
        )
        .await?)
    }

    /// Inserts the template into the meta table if it doesn't already exist or
    /// updates it if it does already exist.
    pub async fn set_template_in_meta(
        org_id: &str,
        template: &mut Template,
    ) -> Result<(), anyhow::Error> {
        template.is_default = Some(org_id == DEFAULT_ORG);
        let key = format!("/templates/{org_id}/{}", template.name);
        Ok(db::put(
            &key,
            json::to_vec(template).unwrap().into(),
            db::NEED_WATCH,
            None,
        )
        .await?)
    }

    /// Defines the JSON schema for alerts stored in the meta table at the time
    /// this migration was originally written.
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Alert {
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub org_id: String,
        #[serde(default)]
        pub stream_type: StreamType,
        #[serde(default)]
        pub stream_name: String,
        #[serde(default)]
        pub is_real_time: bool,
        #[serde(default)]
        pub query_condition: QueryCondition,
        #[serde(default)]
        pub trigger_condition: TriggerCondition,
        pub destinations: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub context_attributes: Option<HashMap<String, String>>,
        #[serde(default)]
        pub row_template: String,
        #[serde(default)]
        pub description: String,
        #[serde(default)]
        pub enabled: bool,
        #[serde(default)]
        /// Timezone offset in minutes.
        /// The negative secs means the Western Hemisphere
        pub tz_offset: i32,
        #[serde(default)]
        pub last_triggered_at: Option<i64>,
        #[serde(default)]
        pub last_satisfied_at: Option<i64>,
        #[serde(default)]
        pub owner: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub updated_at: Option<DateTime<FixedOffset>>,
        #[serde(default)]
        pub last_edited_by: Option<String>,
    }

    #[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
    pub struct TriggerCondition {
        pub period: i64, // 10 minutes
        #[serde(default)]
        pub operator: Operator, // >=
        #[serde(default)]
        pub threshold: i64, // 3 times
        #[serde(default)]
        pub frequency: i64, // 1 minute
        #[serde(default)]
        pub cron: String, // Cron Expression
        #[serde(default)]
        pub frequency_type: FrequencyType,
        #[serde(default)]
        pub silence: i64, // silence for 10 minutes after fire an alert
        #[serde(skip_serializing_if = "Option::is_none")]
        pub timezone: Option<String>,
        #[serde(default)]
        pub tolerance_in_secs: Option<i64>,
    }

    #[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
    pub struct CompareHistoricData {
        #[serde(rename = "offSet")]
        pub offset: String,
    }

    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    pub enum FrequencyType {
        #[serde(rename = "cron")]
        Cron,
        #[serde(rename = "minutes")]
        #[default]
        Minutes,
    }

    #[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
    pub struct QueryCondition {
        #[serde(default)]
        #[serde(rename = "type")]
        pub query_type: QueryType,
        pub conditions: Option<Vec<Condition>>,
        pub sql: Option<String>,
        pub promql: Option<String>,              // (cpu usage / cpu total)
        pub promql_condition: Option<Condition>, // value >= 80
        pub aggregation: Option<Aggregation>,
        #[serde(default)]
        pub vrl_function: Option<String>,
        #[serde(default)]
        pub search_event_type: Option<SearchEventType>,
        #[serde(default)]
        pub multi_time_range: Option<Vec<CompareHistoricData>>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub struct Aggregation {
        pub group_by: Option<Vec<String>>,
        pub function: AggFunction,
        pub having: Condition,
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub enum AggFunction {
        #[serde(rename = "avg")]
        Avg,
        #[serde(rename = "min")]
        Min,
        #[serde(rename = "max")]
        Max,
        #[serde(rename = "sum")]
        Sum,
        #[serde(rename = "count")]
        Count,
        #[serde(rename = "median")]
        Median,
        #[serde(rename = "p50")]
        P50,
        #[serde(rename = "p75")]
        P75,
        #[serde(rename = "p90")]
        P90,
        #[serde(rename = "p95")]
        P95,
        #[serde(rename = "p99")]
        P99,
    }

    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    #[allow(clippy::upper_case_acronyms)] // Original code uses uppercase acronyms.
    pub enum QueryType {
        #[default]
        #[serde(rename = "custom")]
        Custom,
        #[serde(rename = "sql")]
        SQL,
        #[serde(rename = "promql")]
        PromQL,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Condition {
        pub column: String,
        pub operator: Operator,
        pub value: JsonValue,
        #[serde(default)]
        pub ignore_case: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Operator {
        #[serde(rename = "=")]
        EqualTo,
        #[serde(rename = "!=")]
        NotEqualTo,
        #[serde(rename = ">")]
        GreaterThan,
        #[serde(rename = ">=")]
        GreaterThanEquals,
        #[serde(rename = "<")]
        LessThan,
        #[serde(rename = "<=")]
        LessThanEquals,
        Contains,
        NotContains,
    }

    impl Default for Operator {
        fn default() -> Self {
            Self::EqualTo
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Destination {
        #[serde(default)]
        pub name: String,
        /// Required for `Http` destination_type
        #[serde(default)]
        pub url: String,
        /// Required for `Http` destination_type
        #[serde(default)]
        pub method: HTTPType,
        #[serde(default)]
        pub skip_tls_verify: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub headers: Option<HashMap<String, String>>,
        pub template: String,
        /// Required when `destination_type` is `Email`
        #[serde(default)]
        pub emails: Vec<String>,
        // New SNS-specific fields
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sns_topic_arn: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub aws_region: Option<String>,
        #[serde(rename = "type")]
        #[serde(default)]
        pub destination_type: DestinationType,
    }

    #[derive(Serialize, Debug, Default, PartialEq, Eq, Deserialize, Clone)]
    pub enum DestinationType {
        #[default]
        #[serde(rename = "http")]
        Http,
        #[serde(rename = "email")]
        Email,
        #[serde(rename = "sns")]
        Sns,
    }

    #[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
    #[allow(clippy::upper_case_acronyms)] // Original code uses uppercase acronyms.
    pub enum HTTPType {
        #[default]
        #[serde(rename = "post")]
        POST,
        #[serde(rename = "put")]
        PUT,
        #[serde(rename = "get")]
        GET,
    }

    #[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
    pub struct Template {
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub body: String,
        #[serde(rename = "isDefault")]
        #[serde(default)]
        pub is_default: Option<bool>,
        /// Indicates whether the body is an http, email, or sns body.
        #[serde(rename = "type")]
        #[serde(default)]
        pub template_type: DestinationType,
        #[serde(default)]
        pub title: String,
    }

    #[derive(Hash, Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    #[allow(clippy::upper_case_acronyms)] // Original code uses uppercase acronyms.
    pub enum SearchEventType {
        UI,
        Dashboards,
        Reports,
        Alerts,
        Values,
        Other,
        RUM,
        DerivedStream,
    }

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Hash)]
    #[serde(rename_all = "lowercase")]
    pub enum StreamType {
        #[default]
        Logs,
        Metrics,
        Traces,
        #[serde(rename = "enrichment_tables")]
        EnrichmentTables,
        #[serde(rename = "file_list")]
        Filelist,
        Metadata,
        Index,
    }

    impl From<&str> for StreamType {
        fn from(s: &str) -> Self {
            match s.to_lowercase().as_str() {
                "logs" => StreamType::Logs,
                "metrics" => StreamType::Metrics,
                "traces" => StreamType::Traces,
                "enrichment_tables" => StreamType::EnrichmentTables,
                "file_list" => StreamType::Filelist,
                "metadata" => StreamType::Metadata,
                "index" => StreamType::Index,
                _ => StreamType::Logs,
            }
        }
    }

    impl std::fmt::Display for StreamType {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                StreamType::Logs => write!(f, "logs"),
                StreamType::Metrics => write!(f, "metrics"),
                StreamType::Traces => write!(f, "traces"),
                StreamType::EnrichmentTables => write!(f, "enrichment_tables"),
                StreamType::Filelist => write!(f, "file_list"),
                StreamType::Metadata => write!(f, "metadata"),
                StreamType::Index => write!(f, "index"),
            }
        }
    }
}
