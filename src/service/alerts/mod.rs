// Copyright 2023 Zinc Labs Inc.
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

use actix_web::http;

use crate::common::meta::{alerts::Alert, StreamType};
use crate::common::utils::json;
use crate::common::utils::schema_ext::SchemaExt;
use crate::service::db;

pub mod alert_manager;
pub mod destinations;
pub mod templates;
pub mod triggers;

#[tracing::instrument(skip_all)]
pub async fn save_alert(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    name: &str,
    mut alert: Alert,
) -> Result<(), anyhow::Error> {
    alert.name = name.to_string();
    alert.stream_type = stream_type;
    alert.stream_name = stream_name.to_string();

    // before saving alert check alert destination
    for dest in alert.destinations.iter() {
        if db::alerts::destinations::get(org_id, dest).await.is_err() {
            return Err(anyhow::anyhow!("Alert destination {dest} not found"));
        };
    }

    // before saving alert check column type to decide numeric condition
    let schema = db::schema::get(org_id, stream_name, stream_type)
        .await
        .unwrap();
    let fields = schema.to_cloned_fields();
    if fields.is_empty() {
        return Err(anyhow::anyhow!("Stream {stream_name} not found"));
    }

    // check the query conditions is valid
    // TODO

    // if alert.query.is_none() {
    //     alert.query = Some(Query {
    //         sql: format!("select * from \"{stream_name}\""),
    //         start_time: 0,
    //         end_time: 0,
    //         sort_by: None,
    //         sql_mode: "full".to_owned(),
    //         query_type: "".to_owned(),
    //         track_total_hits: false,
    //         from: 0,
    //         size: 0,
    //         query_context: None,
    //         uses_zo_fn: false,
    //         query_fn: None,
    //     });
    //     alert.is_real_time = true;

    //     let mut local_fields = fields.clone();
    //     local_fields.retain(|field| field.name().eq(&alert.condition.column));

    //     if local_fields.is_empty() {
    //         return Err(anyhow::anyhow!(
    //             "Column named {} not found on stream {stream_name}",
    //             &alert.condition.column
    //         ));
    //     }
    //     alert.condition.is_numeric = Some(!matches!(
    //         local_fields[0].data_type(),
    //         DataType::Boolean | DataType::Utf8
    //     ));
    // } else {
    //     let meta_req = meta::search::Request {
    //         query: alert.clone().query.unwrap(),
    //         aggs: HashMap::new(),
    //         encoding: meta::search::RequestEncoding::Empty,
    //         timeout: 0,
    //     };
    //     let req: cluster_rpc::SearchRequest = meta_req.into();
    //     let sql = Sql::new(&req).await;
    //     if sql.is_err() {
    //         return Err(anyhow::anyhow!("Invalid query : {:?} ", sql.err()));
    //     }
    // }

    db::alerts::set(org_id, stream_name, stream_type, name, alert).await
}

pub async fn list_alert(
    org_id: &str,
    stream_name: Option<&str>,
    stream_type: Option<StreamType>,
) -> Result<Vec<Alert>, anyhow::Error> {
    db::alerts::list(org_id, stream_name, stream_type).await
}

pub async fn delete_alert(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    name: &str,
) -> Result<(), (http::StatusCode, anyhow::Error)> {
    if db::alerts::get(org_id, stream_name, stream_type, name)
        .await
        .is_err()
    {
        return Err((
            http::StatusCode::NOT_FOUND,
            anyhow::anyhow!("Alert not found"),
        ));
    }
    db::alerts::delete(org_id, stream_name, stream_type, name)
        .await
        .map_err(|e| (http::StatusCode::INTERNAL_SERVER_ERROR, e))
}

#[tracing::instrument]
pub async fn enable_alert(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    name: &str,
    value: bool,
) -> Result<(), (http::StatusCode, anyhow::Error)> {
    let mut alert = match db::alerts::get(org_id, stream_name, stream_type, name).await {
        Ok(Some(alert)) => alert,
        _ => {
            return Err((
                http::StatusCode::NOT_FOUND,
                anyhow::anyhow!("Alert not found"),
            ));
        }
    };
    alert.enabled = value;
    db::alerts::set(org_id, stream_name, stream_type, name, alert)
        .await
        .map_err(|e| (http::StatusCode::INTERNAL_SERVER_ERROR, e))
}

#[tracing::instrument]
pub async fn get_alert(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    name: &str,
) -> Result<Option<Alert>, anyhow::Error> {
    db::alerts::get(org_id, stream_name, stream_type, name).await
}

#[tracing::instrument]
pub async fn trigger_alert(
    org_id: &str,
    stream_name: &str,
    stream_type: StreamType,
    name: &str,
) -> Result<(), anyhow::Error> {
    // let result = db::alerts::get(org_id, stream_name, stream_type, name).await;
    // match result {
    //     Ok(Some(alert)) => {
    //         let trigger = Trigger {
    //             timestamp: Utc::now().timestamp_micros(),
    //             is_valid: false,
    //             alert_name: name.to_string(),
    //             stream: stream_name.to_string(),
    //             org: org_id.to_string(),
    //             last_sent_at: 0,
    //             count: 0,
    //             is_ingest_time: alert.is_real_time,
    //             stream_type,
    //             parent_alert_deleted: false,
    //         };
    //         send_notification(&alert, &trigger)
    //             .await
    //             .map_err(|e| anyhow::anyhow!("Failed to send notification: {}", e))?;
    //         Ok(())
    //     }
    //     _ => Err(anyhow::anyhow!("Alert not found")),
    // }
    Ok(())
}

impl Alert {
    pub async fn check_realtime(
        &self,
        _data: &json::Map<String, json::Value>,
    ) -> Result<Option<Vec<json::Map<String, json::Value>>>, anyhow::Error> {
        Ok(None)
    }

    pub async fn check_schedule(
        &self,
    ) -> Result<Option<Vec<json::Map<String, json::Value>>>, anyhow::Error> {
        Ok(None)
    }
}
