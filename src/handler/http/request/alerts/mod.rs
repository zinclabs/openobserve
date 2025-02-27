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

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};
use config::meta::{
    alerts::alert::Alert as MetaAlert,
    folder::DEFAULT_FOLDER,
    triggers::{Trigger, TriggerModule},
};
use hashbrown::HashMap;
use infra::db::{connect_to_orm, ORM_CLIENT};
use svix_ksuid::Ksuid;

use crate::{
    common::{meta::http::HttpResponse as MetaHttpResponse, utils::auth::UserEmail},
    handler::http::models::alerts::{
        requests::{
            CreateAlertRequestBody, EnableAlertQuery, ListAlertsQuery, MoveAlertsRequestBody,
            UpdateAlertRequestBody,
        },
        responses::{EnableAlertResponseBody, GetAlertResponseBody, ListAlertsResponseBody},
    },
    service::{
        alerts::alert::{self, AlertError},
        db::scheduler,
    },
};

#[allow(deprecated)]
pub mod deprecated;
pub mod destinations;
pub mod templates;

impl From<AlertError> for HttpResponse {
    fn from(value: AlertError) -> Self {
        match &value {
            AlertError::InfraError(err) => MetaHttpResponse::internal_error(err),
            AlertError::CreateDefaultFolderError => MetaHttpResponse::internal_error(value),
            AlertError::AlertNameMissing => MetaHttpResponse::bad_request(value),
            AlertError::AlertNameOfgaUnsupported => MetaHttpResponse::bad_request(value),
            AlertError::AlertNameContainsForwardSlash => MetaHttpResponse::bad_request(value),
            AlertError::AlertDestinationMissing => MetaHttpResponse::bad_request(value),
            AlertError::CreateAlreadyExists => MetaHttpResponse::conflict(value),
            AlertError::CreateFolderNotFound => MetaHttpResponse::not_found(value),
            AlertError::MoveDestinationFolderNotFound => MetaHttpResponse::not_found(value),
            AlertError::AlertNotFound => MetaHttpResponse::not_found(value),
            AlertError::AlertDestinationNotFound { .. } => MetaHttpResponse::not_found(value),
            AlertError::StreamNotFound { .. } => MetaHttpResponse::not_found(value),
            AlertError::DecodeVrl(err) => MetaHttpResponse::bad_request(err),
            AlertError::ParseCron(err) => MetaHttpResponse::bad_request(err),
            AlertError::RealtimeMissingCustomQuery => MetaHttpResponse::bad_request(value),
            AlertError::SqlMissingQuery => MetaHttpResponse::bad_request(value),
            AlertError::SqlContainsSelectStar => MetaHttpResponse::bad_request(value),
            AlertError::PromqlMissingQuery => MetaHttpResponse::bad_request(value),
            AlertError::SendNotificationError { .. } => MetaHttpResponse::internal_error(value),
            AlertError::GetDestinationWithTemplateError(err) => {
                MetaHttpResponse::internal_error(err)
            }
            AlertError::PeriodExceedsMaxQueryRange { .. } => MetaHttpResponse::bad_request(value),
            AlertError::ResolveStreamNameError(_) => MetaHttpResponse::internal_error(value),
            AlertError::PermittedAlertsMissingUser => MetaHttpResponse::forbidden(""),
            AlertError::PermittedAlertsValidator(err) => MetaHttpResponse::forbidden(err),
        }
    }
}

/// CreateAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "CreateAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    request_body(content = CreateAlertRequestBody, description = "Alert data", content_type = "application/json"),    
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/v2/{org_id}/alerts")]
pub async fn create_alert(
    path: web::Path<String>,
    req_body: web::Json<CreateAlertRequestBody>,
    user_email: UserEmail,
) -> HttpResponse {
    let org_id = path.into_inner();
    let req_body = req_body.into_inner();

    let folder_id = req_body
        .folder_id
        .clone()
        .unwrap_or(DEFAULT_FOLDER.to_string());
    let mut alert: MetaAlert = req_body.into();
    alert.owner = Some(user_email.user_id.clone());
    alert.last_edited_by = Some(user_email.user_id);

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match alert::create(client, &org_id, &folder_id, alert).await {
        Ok(_) => MetaHttpResponse::ok("Alert saved"),
        Err(e) => e.into(),
    }
}

/// GetAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "GetAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("alert_id" = Ksuid, Path, description = "Alert ID"),
      ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = GetAlertResponseBody),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/v2/{org_id}/alerts/{alert_id}")]
async fn get_alert(path: web::Path<(String, Ksuid)>) -> HttpResponse {
    let (org_id, alert_id) = path.into_inner();

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match alert::get_by_id(client, &org_id, alert_id).await {
        Ok(alert) => {
            let key = alert.get_unique_key();
            let scheduled_job = scheduler::get(&org_id, TriggerModule::Alert, &key)
                .await
                .ok();
            let resp_body: GetAlertResponseBody = (alert, scheduled_job).into();
            MetaHttpResponse::json(resp_body)
        }
        Err(e) => e.into(),
    }
}

/// UpdateAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "UpdateAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("alert_id" = Ksuid, Path, description = "Alert ID"),
      ),
    request_body(content = UpdateAlertRequestBody, description = "Alert data", content_type = "application/json"),    
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Error",   content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/v2/{org_id}/alerts/{alert_id}")]
pub async fn update_alert(
    path: web::Path<(String, Ksuid)>,
    req_body: web::Json<UpdateAlertRequestBody>,
    user_email: UserEmail,
) -> HttpResponse {
    let (org_id, _alert_id) = path.into_inner();
    let req_body = req_body.into_inner();

    let mut alert: MetaAlert = req_body.into();
    alert.last_edited_by = Some(user_email.user_id);

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match alert::update(client, &org_id, None, alert).await {
        Ok(_) => MetaHttpResponse::ok("Alert Updated"),
        Err(e) => e.into(),
    }
}

/// DeleteAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "DeleteAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("alert_id" = Ksuid, Path, description = "Alert ID"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/v2/{org_id}/alerts/{alert_id}")]
async fn delete_alert(path: web::Path<(String, Ksuid)>) -> HttpResponse {
    let (org_id, alert_id) = path.into_inner();

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match alert::delete_by_id(client, &org_id, alert_id).await {
        Ok(_) => MetaHttpResponse::ok("Alert deleted"),
        Err(e) => e.into(),
    }
}

/// ListAlerts
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "ListAlerts",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ListAlertsQuery,
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = ListAlertsResponseBody),
    )
)]
#[get("/v2/{org_id}/alerts")]
async fn list_alerts(path: web::Path<String>, req: HttpRequest) -> HttpResponse {
    let org_id = path.into_inner();
    let Ok(query) = web::Query::<ListAlertsQuery>::from_query(req.query_string()) else {
        return MetaHttpResponse::bad_request("Error parsing query parameters");
    };
    let query = query.0;

    #[cfg(not(feature = "enterprise"))]
    let user_id = None;
    #[cfg(feature = "enterprise")]
    let Ok(user_id) = req.headers().get("user_id").map(|v| v.to_str()).transpose() else {
        return MetaHttpResponse::forbidden("");
    };

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let scheduled_jobs = scheduler::list_by_org(&org_id, Some(TriggerModule::Alert))
        .await
        .unwrap_or_default();
    let mut scheduled_jobs: HashMap<String, Trigger> = scheduled_jobs
        .into_iter()
        .map(|t| (t.module_key.clone(), t))
        .collect();
    let folders_and_alerts_scheduled_job =
        match alert::list_v2(client, user_id, query.into(&org_id)).await {
            Ok(f_a) => {
                let f_a: Vec<_> = f_a
                    .into_iter()
                    .map(|(folder, alert)| {
                        let key = alert.get_unique_key();
                        (folder, alert, scheduled_jobs.remove(&key))
                    })
                    .collect();
                f_a
            }
            Err(e) => return e.into(),
        };
    let Ok(resp_body) = ListAlertsResponseBody::try_from(folders_and_alerts_scheduled_job) else {
        return MetaHttpResponse::internal_error("");
    };
    MetaHttpResponse::json(resp_body)
}

/// EnableAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "EnableAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("alert_id" = Ksuid, Path, description = "Alert ID"),
        EnableAlertQuery,
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/v2/{org_id}/alerts/{alert_id}/enable")]
async fn enable_alert(path: web::Path<(String, Ksuid)>, req: HttpRequest) -> HttpResponse {
    let (org_id, alert_id) = path.into_inner();
    let Ok(query) = web::Query::<EnableAlertQuery>::from_query(req.query_string()) else {
        return MetaHttpResponse::bad_request("Error parsing query parameters");
    };
    let should_enable = query.0.value;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match alert::enable_by_id(client, &org_id, alert_id, should_enable).await {
        Ok(_) => {
            let resp_body = EnableAlertResponseBody {
                enabled: should_enable,
            };
            MetaHttpResponse::json(resp_body)
        }
        Err(e) => e.into(),
    }
}

/// TriggerAlert
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "TriggerAlert",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("alert_id" = Ksuid, Path, description = "Alert ID"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/v2/{org_id}/alerts/{alert_id}/trigger")]
async fn trigger_alert(path: web::Path<(String, Ksuid)>) -> HttpResponse {
    let (org_id, alert_id) = path.into_inner();

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match alert::trigger_by_id(client, &org_id, alert_id).await {
        Ok(_) => MetaHttpResponse::ok("Alert triggered"),
        Err(e) => e.into(),
    }
}

/// MoveAlerts
#[utoipa::path(
    context_path = "/api",
    tag = "Alerts",
    operation_id = "MoveAlerts",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = MoveAlertsRequestBody, description = "Identifies alerts and the destination folder", content_type = "application/json"),    
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/v2/{org_id}/alerts/move")]
async fn move_alerts(
    path: web::Path<String>,
    req_body: web::Json<MoveAlertsRequestBody>,
) -> HttpResponse {
    let org_id = path.into_inner();
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match alert::move_to_folder(
        client,
        &org_id,
        &req_body.alert_ids,
        &req_body.dst_folder_id,
    )
    .await
    {
        Ok(_) => {
            let message = if req_body.alert_ids.len() == 1 {
                "Alert moved"
            } else {
                "Alerts moved"
            };
            MetaHttpResponse::ok(message)
        }
        Err(e) => e.into(),
    }
}
