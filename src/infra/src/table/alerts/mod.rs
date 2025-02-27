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

use std::str::FromStr;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use config::meta::{
    alerts::{
        alert::{Alert as MetaAlert, ListAlertsParams},
        QueryCondition as MetaQueryCondition, TriggerCondition as MetaTriggerCondition,
    },
    folder::{Folder as MetaFolder, FolderType},
    stream::StreamType as MetaStreamType,
};
use hashbrown::HashMap;
use itertools::Itertools;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, RuntimeErr, Set, TransactionTrait, TryIntoModel,
};
use sqlx::Error as SqlxError;
use svix_ksuid::{Ksuid, KsuidLike};

use super::{
    entity::{alerts, folders},
    folders::folder_type_into_i16,
};
use crate::errors::{self, FromStrError, PutAlertError};

pub mod intermediate;

impl TryFrom<alerts::Model> for MetaAlert {
    type Error = errors::Error;

    fn try_from(value: alerts::Model) -> Result<Self, Self::Error> {
        let id: Ksuid = Ksuid::from_str(&value.id).map_err(|_| FromStrError {
            value: value.id,
            ty: "svix_ksuid::Ksuid".to_owned(),
        })?;

        // Transform database string values into intermediate types which can be
        // directly translated into service layer types.
        let stream_type: intermediate::StreamType = value.stream_type.parse()?;
        let trigger_threshold_operator: intermediate::TriggerThresholdOperator =
            value.trigger_threshold_operator.parse()?;

        // Transform database i16 values into intermediate types which can be
        // directly translated into service layer types.
        let query_type: intermediate::QueryType = value.query_type.try_into()?;
        let query_search_event_type: Option<intermediate::QuerySearchEventType> = value
            .query_search_event_type
            .map(|i| i.try_into())
            .transpose()?;
        let trigger_frequency_type: intermediate::TriggerFrequencyType =
            value.trigger_frequency_type.try_into()?;

        // Transform database JSON values into intermediate types which can be
        // directly translated into service layer types.
        let destinations: Vec<String> = serde_json::from_value(value.destinations)?;
        let context_attributes: Option<HashMap<String, String>> = value
            .context_attributes
            .map(serde_json::from_value)
            .transpose()?;
        let query_conditions: Option<Vec<intermediate::QueryCondition>> = value
            .query_conditions
            .map(serde_json::from_value)
            .transpose()?;
        let query_promql_condition: Option<intermediate::QueryCondition> = value
            .query_promql_condition
            .map(serde_json::from_value)
            .transpose()?;
        let query_aggregation: Option<intermediate::QueryAggregation> = value
            .query_aggregation
            .map(serde_json::from_value)
            .transpose()?;
        let query_multi_time_range: Option<Vec<intermediate::QueryCompareHistoricData>> = value
            .query_multi_time_range
            .map(serde_json::from_value)
            .transpose()?;

        // Transform the Unix timestamp into a date time that will always use
        // the UTC timezone.
        let updated_at_utc: Option<DateTime<FixedOffset>> = value
            .updated_at
            .and_then(|secs| Utc.timestamp_opt(secs, 0).single())
            .map(|dt| dt.into());

        let mut alert: MetaAlert = Default::default();
        alert.id = Some(id);
        alert.name = value.name;
        alert.org_id = value.org;
        alert.stream_type = stream_type.into();
        alert.stream_name = value.stream_name;
        alert.is_real_time = value.is_real_time;
        alert.destinations = destinations;
        alert.context_attributes = context_attributes;
        alert.row_template = value.row_template.unwrap_or_default();
        alert.description = value.description.unwrap_or_default();
        alert.enabled = value.enabled;
        alert.tz_offset = value.tz_offset;
        alert.owner = value.owner;
        alert.last_edited_by = value.last_edited_by;
        alert.updated_at = updated_at_utc;
        alert.query_condition = MetaQueryCondition {
            query_type: query_type.into(),
            conditions: query_conditions.map(|cs| cs.into_iter().map(|c| c.into()).collect()),
            sql: value.query_sql,
            promql: value.query_promql,
            promql_condition: query_promql_condition.map(|c| c.into()),
            aggregation: query_aggregation.map(|a| a.into()),
            vrl_function: value.query_vrl_function,
            search_event_type: query_search_event_type.map(|t| t.into()),
            multi_time_range: query_multi_time_range
                .map(|ds| ds.into_iter().map(|d| d.into()).collect()),
        };
        alert.trigger_condition = MetaTriggerCondition {
            // DB model stores period in seconds, but service layer stores
            // minutes.
            period: value.trigger_period_seconds / 60,
            operator: trigger_threshold_operator.into(),
            threshold: value.trigger_threshold_count,
            frequency: value.trigger_frequency_seconds,
            cron: value.trigger_frequency_cron.unwrap_or_default(),
            frequency_type: trigger_frequency_type.into(),
            silence: value.trigger_silence_seconds / 60,
            timezone: value.trigger_frequency_cron_timezone,
            tolerance_in_secs: value.trigger_tolerance_seconds,
        };
        alert.set_last_satisfied_at(value.last_satisfied_at);
        alert.set_last_triggered_at(value.last_triggered_at);

        Ok(alert)
    }
}

/// Gets an alert by its ID.
pub async fn get_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<Option<(MetaFolder, MetaAlert)>, errors::Error> {
    let _lock = super::get_lock().await;
    let models = get_model_by_id(conn, org_id, alert_id).await?;

    if let Some((folder_model, alert_model)) = models {
        let folder = folder_model.into();
        let alert = alert_model.try_into()?;
        Ok(Some((folder, alert)))
    } else {
        Ok(None)
    }
}

/// Gets an alert by its name.
pub async fn get_by_name<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: &str,
    stream_type: MetaStreamType,
    stream_name: &str,
    alert_name: &str,
) -> Result<Option<(MetaFolder, MetaAlert)>, errors::Error> {
    let _lock = super::get_lock().await;
    let models = get_model_by_name(
        conn,
        org_id,
        folder_id,
        stream_type,
        stream_name,
        alert_name,
    )
    .await?
    .and_then(|(f, maybe_a)| maybe_a.map(|a| (f, a)));

    if let Some((folder_model, alert_model)) = models {
        let folder = folder_model.into();
        let alert = alert_model.try_into()?;
        Ok(Some((folder, alert)))
    } else {
        Ok(None)
    }
}

/// Creates a new alert or updates an existing alert in the database. Returns
/// the new or updated alert.
pub async fn put<C: TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: &str,
    alert: MetaAlert,
) -> Result<MetaAlert, errors::Error> {
    let _lock = super::get_lock().await;
    let txn = conn.begin().await?;
    let rslt: Result<alerts::Model, errors::Error> = match get_model_by_name(
        &txn,
        org_id,
        folder_id,
        alert.stream_type,
        &alert.stream_name,
        &alert.name,
    )
    .await?
    {
        None => {
            // Destination folder does not exist so the alert can neither be
            // created nor updated.
            return Err(
                errors::DbError::PutAlert(errors::PutAlertError::FolderDoesNotExist).into(),
            );
        }
        Some((_folder_m, Some(alert_m))) => {
            // Destination folder exists and alert already exists, so convert
            // the alert model to an active model and update it.
            let mut alert_am: alerts::ActiveModel = alert_m.into();
            update_mutable_fields(&mut alert_am, alert)?;
            let model: alerts::Model = alert_am.update(&txn).await?.try_into_model()?;
            Ok(model)
        }
        Some((folder_m, None)) => {
            // Destination folder exists and alert does not exist, so create an
            // active model for creating a new record.
            let id = svix_ksuid::Ksuid::new(None, None).to_string();
            let stream_type = intermediate::StreamType::from(alert.stream_type).to_string();
            let stream_name = alert.stream_name.clone();
            let alert_name = alert.name.clone();
            let mut alert_am = alerts::ActiveModel {
                // The following fields can only be set on creation.
                id: Set(id),
                org: Set(org_id.to_owned()),
                folder_id: Set(folder_m.id),
                stream_type: Set(stream_type),
                stream_name: Set(stream_name),
                name: Set(alert_name),
                // All remaining fields can be set on creation or updated so
                // they are set below.
                ..Default::default()
            };
            update_mutable_fields(&mut alert_am, alert)?;

            // Triggered and satisfied timestamps should always be initialized
            // to None.
            alert_am.last_triggered_at = Set(None);
            alert_am.last_satisfied_at = Set(None);

            let model: alerts::Model = alert_am.insert(&txn).await?.try_into_model()?;
            Ok(model)
        }
    };
    let alert = rslt?.try_into()?;
    txn.commit().await?;
    Ok(alert)
}

/// Creates a new alert in the database. Returns the new alert.
pub async fn create<C: TransactionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: &str,
    alert: MetaAlert,
    use_given_id: bool,
) -> Result<MetaAlert, errors::Error> {
    let _lock = super::get_lock().await;
    let txn = conn.begin().await?;

    // Get the destination folder.
    let Some(folder_m) =
        super::folders::get_model(&txn, org_id, folder_id, FolderType::Alerts).await?
    else {
        return Err(errors::DbError::PutAlert(PutAlertError::FolderDoesNotExist).into());
    };

    let id: String = if use_given_id {
        alert
            .id
            .unwrap_or_else(|| svix_ksuid::Ksuid::new(None, None))
            .to_string()
    } else {
        svix_ksuid::Ksuid::new(None, None).to_string()
    };
    let stream_type = intermediate::StreamType::from(alert.stream_type).to_string();
    let mut alert_am = alerts::ActiveModel {
        id: Set(id.clone()),
        org: Set(org_id.to_owned()),
        folder_id: Set(folder_m.id),
        stream_type: Set(stream_type),
        stream_name: Set(alert.stream_name.clone()),
        name: Set(alert.name.clone()),
        // All remaining fields can be set on creation or updated so
        // they are set below.
        ..Default::default()
    };
    update_mutable_fields(&mut alert_am, alert.clone())?;

    // Triggered and satisfied timestamps should always be initialized
    // to None so overwrite any value that might have been set already.
    alert_am.last_triggered_at = Set(None);
    alert_am.last_satisfied_at = Set(None);

    let mut alert = alert;
    let alert_m: alerts::Model = match alert_am.insert(&txn).await {
        Ok(m) => m.try_into_model()?,
        Err(DbErr::Exec(RuntimeErr::SqlxError(SqlxError::Database(e)))) => {
            // unique violation will occur when we try to re-insert the same combination
            // which is ok, because what we want is already there.
            if e.is_unique_violation() {
                alert.id = Some(Ksuid::from_str(&id).unwrap());
                return Ok(alert);
            } else {
                return Err(errors::DbError::SeaORMError(e.to_string()).into());
            }
        }
        Err(e) => Err(errors::Error::DbError(errors::DbError::SeaORMError(
            e.to_string(),
        )))?,
    };
    let alert = alert_m.try_into()?;
    txn.commit().await?;
    log::debug!("Alert created: {:?}", alert);
    Ok(alert)
}

/// Updates an alert in the database. Returns the new alert.
pub async fn update<C: TransactionTrait + ConnectionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: Option<&str>,
    alert: MetaAlert,
) -> Result<MetaAlert, errors::Error> {
    // Ensure that ID is provided.
    let Some(alert_id) = alert.id else {
        return Err(errors::DbError::PutAlert(PutAlertError::UpdateAlertMissingID).into());
    };

    let _lock = super::get_lock().await;
    let txn = conn.begin().await?;

    // Try to get the new parent folder if a folder ID is provided.
    let maybe_folder_m = match folder_id {
        Some(f_id) => {
            let Some(folder_m) =
                super::folders::get_model(&txn, org_id, f_id, FolderType::Alerts).await?
            else {
                return Err(errors::DbError::PutAlert(PutAlertError::FolderDoesNotExist).into());
            };
            Some(folder_m)
        }
        None => None,
    };

    // Try to get the alert to update.
    let Some((_, alert_m)) = get_model_by_id(conn, org_id, alert_id).await? else {
        return Err(errors::DbError::PutAlert(PutAlertError::UpdateAlertNotFound).into());
    };

    // Update fields using values from the given alert.
    let mut alert_am: alerts::ActiveModel = alert_m.into();
    update_mutable_fields(&mut alert_am, alert)?;

    // Update the folder if a new parent folder was provided.
    if let Some(folder_m) = maybe_folder_m {
        alert_am.folder_id = Set(folder_m.id);
    }

    let alert_m: alerts::Model = alert_am.update(&txn).await?.try_into_model()?;
    let alert = alert_m.try_into()?;
    txn.commit().await?;
    Ok(alert)
}

/// Deletes an alert by its ID.
pub async fn delete_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<(), errors::Error> {
    let _lock = super::get_lock().await;
    alerts::Entity::delete_many()
        .filter(alerts::Column::Org.eq(org_id))
        .filter(alerts::Column::Id.eq(alert_id.to_string()))
        .exec(conn)
        .await?;
    Ok(())
}

/// Deletes an alert by its name.
pub async fn delete_by_name<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: &str,
    stream_type: MetaStreamType,
    stream_name: &str,
    alert_name: &str,
) -> Result<(), errors::Error> {
    let _lock = super::get_lock().await;
    let model = get_model_by_name(
        conn,
        org_id,
        folder_id,
        stream_type,
        stream_name,
        alert_name,
    )
    .await?
    .and_then(|(_folder, maybe_alert)| maybe_alert);

    if let Some(model) = model {
        let _ = model.delete(conn).await?;
    }

    Ok(())
}

/// Lists alerts.
pub async fn list<C: ConnectionTrait>(
    conn: &C,
    params: ListAlertsParams,
) -> Result<Vec<(MetaFolder, MetaAlert)>, errors::Error> {
    let _lock = super::get_lock().await;
    let alerts = list_models(conn, params)
        .await?
        .into_iter()
        .map(|(f, a)| {
            let f = MetaFolder::from(f);
            let a = MetaAlert::try_from(a)?;
            Ok((f, a))
        })
        .collect::<Result<_, errors::Error>>()?;
    Ok(alerts)
}

/// Lists all alerts.
pub async fn list_all<C: ConnectionTrait>(conn: &C) -> Result<Vec<MetaAlert>, errors::Error> {
    let _lock = super::get_lock().await;
    let alerts = list_all_models(conn)
        .await?
        .into_iter()
        .map(MetaAlert::try_from)
        .collect::<Result<_, errors::Error>>()?;
    Ok(alerts)
}

/// Tries to get an alert ORM entity and its parent folder ORM entity.
async fn get_model_by_id<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    alert_id: Ksuid,
) -> Result<Option<(folders::Model, alerts::Model)>, sea_orm::DbErr> {
    let maybe_f_a = alerts::Entity::find_by_id(alert_id.to_string())
        .filter(alerts::Column::Org.eq(org_id))
        .find_also_related(folders::Entity)
        .one(conn)
        .await?
        .and_then(|(a, maybe_f)| maybe_f.map(|f| (f, a)));
    Ok(maybe_f_a)
}

/// Tries to get an alert ORM entity and its parent folder ORM entity.
async fn get_model_by_name<C: ConnectionTrait>(
    conn: &C,
    org_id: &str,
    folder_id: &str,
    stream_type: MetaStreamType,
    stream_name: &str,
    alert_name: &str,
) -> Result<Option<(folders::Model, Option<alerts::Model>)>, sea_orm::DbErr> {
    let select_folders = folders::Entity::find()
        .filter(folders::Column::Org.eq(org_id))
        .filter(folders::Column::Type.eq::<i16>(folder_type_into_i16(FolderType::Alerts)))
        .filter(folders::Column::FolderId.eq(folder_id));

    let Some(folder) = select_folders.one(conn).await? else {
        return Ok(None);
    };

    let stream_type_str = intermediate::StreamType::from(stream_type).to_string();
    let maybe_alert = folder
        .find_related(alerts::Entity)
        .filter(alerts::Column::StreamType.eq(stream_type_str))
        .filter(alerts::Column::StreamName.eq(stream_name))
        .filter(alerts::Column::Name.eq(alert_name))
        .one(conn)
        .await?;

    Ok(Some((folder, maybe_alert)))
}

/// Lists alert ORM models using the given parameters. Returns each alert and
/// its parent folder.
async fn list_models<C: ConnectionTrait>(
    conn: &C,
    params: ListAlertsParams,
) -> Result<Vec<(folders::Model, alerts::Model)>, sea_orm::DbErr> {
    let query = alerts::Entity::find()
        .find_also_related(folders::Entity)
        .filter(folders::Column::Type.eq::<i16>(folder_type_into_i16(FolderType::Alerts)))
        .filter(folders::Column::Org.eq(params.org_id));

    // Apply the optional folder_id filter.
    let query = if let Some(folder_id) = &params.folder_id {
        query.filter(folders::Column::FolderId.eq(folder_id))
    } else {
        query
    };

    // Apply the optional stream filter.
    let query = if let Some((stream_type, maybe_stream_name)) = &params.stream_type_and_name {
        let stream_type_str = intermediate::StreamType::from(*stream_type).to_string();

        if let Some(stream_name) = maybe_stream_name {
            query
                .filter(alerts::Column::StreamType.eq(stream_type_str))
                .filter(alerts::Column::StreamName.eq(stream_name))
        } else {
            query.filter(alerts::Column::StreamType.eq(stream_type_str))
        }
    } else {
        query
    };

    // Apply the optional enabled filter.
    let query = if let Some(enabled) = &params.enabled {
        query.filter(alerts::Column::Enabled.eq(*enabled))
    } else {
        query
    };

    // Apply ordering.
    let query = query
        .order_by_asc(alerts::Column::Name)
        .order_by_asc(folders::Column::Name);

    // Execute the query, either getting all results or a specific page of results.
    let results = if let Some((page_size, page_idx)) = params.page_size_and_idx {
        query.paginate(conn, page_size).fetch_page(page_idx).await?
    } else {
        query.all(conn).await?
    };

    // Flatten the results so that each dashboard is returned alongside its
    // parent folder.
    let folders_and_dashboards = results
        .into_iter()
        .filter_map(|(d, maybe_f)| maybe_f.map(|f| (f, d)))
        .collect();
    Ok(folders_and_dashboards)
}

/// Lists all alert ORM models.
async fn list_all_models<C: ConnectionTrait>(
    conn: &C,
) -> Result<Vec<alerts::Model>, sea_orm::DbErr> {
    let alerts = alerts::Entity::find()
        .all(conn)
        .await?
        .into_iter()
        .collect();
    Ok(alerts)
}

/// Updates all mutable fields on the [alerts::ActiveModel].
///
/// For some fields the values will be extracted from and transformed from the
/// given [MetaAlert] or [folders::Model]. Other fields such as updated
/// timestamps may be generated from the current timestamp.
///
/// Fields which should only be set on initial creation of an alert and which
/// should be treated as immutable will not be updated.
fn update_mutable_fields(
    alert_am: &mut alerts::ActiveModel,
    alert: MetaAlert,
) -> Result<(), errors::Error> {
    let last_triggered_at = alert.get_last_triggered_at_from_table();
    let last_satisfied_at = alert.get_last_satisfied_at_from_table();
    let is_real_time = alert.is_real_time;
    let destinations = serde_json::to_value(alert.destinations)?;
    let context_attributes = alert
        .context_attributes
        .map(serde_json::to_value)
        .transpose()?;
    let row_template = Some(alert.row_template).filter(|s| !s.is_empty());
    let description = Some(alert.description).filter(|s| !s.is_empty());
    let enabled = alert.enabled;
    let tz_offset = alert.tz_offset;
    let query_type: i16 = intermediate::QueryType::from(alert.query_condition.query_type).into();
    let query_conditions = alert
        .query_condition
        .conditions
        .map(|cs| {
            cs.into_iter()
                .map(intermediate::QueryCondition::from)
                .collect_vec()
        })
        .map(serde_json::to_value)
        .transpose()?;
    let query_sql = alert.query_condition.sql.filter(|s| !s.is_empty());
    let query_promql = alert.query_condition.promql.filter(|s| !s.is_empty());
    let query_promql_condition = alert
        .query_condition
        .promql_condition
        .map(intermediate::QueryCondition::from)
        .map(serde_json::to_value)
        .transpose()?;
    let query_aggregation = alert
        .query_condition
        .aggregation
        .map(intermediate::QueryAggregation::from)
        .map(serde_json::to_value)
        .transpose()?;
    let query_vrl_function = alert.query_condition.vrl_function.filter(|s| !s.is_empty());
    let query_search_event_type: Option<i16> = alert
        .query_condition
        .search_event_type
        .map(intermediate::QuerySearchEventType::from)
        .map(|t| t.into());
    let query_multi_time_range = alert
        .query_condition
        .multi_time_range
        .map(|ds| {
            ds.into_iter()
                .map(intermediate::QueryCompareHistoricData::from)
                .collect_vec()
        })
        .map(serde_json::to_value)
        .transpose()?;
    let trigger_threshold_operator: String =
        intermediate::TriggerThresholdOperator::try_from(alert.trigger_condition.operator)
            .map_err(|_| {
                errors::DbError::PutAlert(errors::PutAlertError::IntoTriggerThresholdOperator(
                    alert.trigger_condition.operator,
                ))
            })?
            .to_string();
    let trigger_period_seconds = alert.trigger_condition.period * 60;
    let trigger_threshold_count = alert.trigger_condition.threshold;
    let trigger_frequency_type: i16 =
        intermediate::TriggerFrequencyType::from(alert.trigger_condition.frequency_type).into();
    let trigger_frequency_seconds = alert.trigger_condition.frequency;
    let trigger_frequency_cron = Some(alert.trigger_condition.cron).filter(|s| !s.is_empty());
    let trigger_frequency_cron_timezone =
        alert.trigger_condition.timezone.filter(|s| !s.is_empty());
    let trigger_silence_seconds = alert.trigger_condition.silence * 60;
    let trigger_tolerance_seconds = alert.trigger_condition.tolerance_in_secs;
    let owner = alert.owner.filter(|s| !s.is_empty());
    let last_edited_by = alert.last_edited_by.filter(|s| !s.is_empty());
    let updated_at: i64 = chrono::Utc::now().timestamp();

    alert_am.is_real_time = Set(is_real_time);
    alert_am.destinations = Set(destinations);
    alert_am.context_attributes = Set(context_attributes);
    alert_am.row_template = Set(row_template);
    alert_am.description = Set(description);
    alert_am.enabled = Set(enabled);
    alert_am.tz_offset = Set(tz_offset);
    alert_am.last_triggered_at = Set(last_triggered_at);
    alert_am.last_satisfied_at = Set(last_satisfied_at);
    alert_am.query_type = Set(query_type);
    alert_am.query_conditions = Set(query_conditions);
    alert_am.query_sql = Set(query_sql);
    alert_am.query_promql = Set(query_promql);
    alert_am.query_promql_condition = Set(query_promql_condition);
    alert_am.query_aggregation = Set(query_aggregation);
    alert_am.query_vrl_function = Set(query_vrl_function);
    alert_am.query_search_event_type = Set(query_search_event_type);
    alert_am.query_multi_time_range = Set(query_multi_time_range);
    alert_am.trigger_threshold_operator = Set(trigger_threshold_operator);
    alert_am.trigger_period_seconds = Set(trigger_period_seconds);
    alert_am.trigger_threshold_count = Set(trigger_threshold_count);
    alert_am.trigger_frequency_type = Set(trigger_frequency_type);
    alert_am.trigger_frequency_seconds = Set(trigger_frequency_seconds);
    alert_am.trigger_frequency_cron = Set(trigger_frequency_cron);
    alert_am.trigger_frequency_cron_timezone = Set(trigger_frequency_cron_timezone);
    alert_am.trigger_silence_seconds = Set(trigger_silence_seconds);
    alert_am.trigger_tolerance_seconds = Set(trigger_tolerance_seconds);
    alert_am.owner = Set(owner);
    alert_am.last_edited_by = Set(last_edited_by);
    alert_am.updated_at = Set(Some(updated_at));

    Ok(())
}
