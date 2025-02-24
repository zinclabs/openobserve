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

use std::io::Error as StdErr;

use actix_web::{delete, get, post, web, HttpResponse};
use config::get_config;
use infra::errors::{DbError, Error};
#[cfg(feature = "enterprise")]
use {
    actix_multipart::Multipart,
    futures::{StreamExt, TryStreamExt},
    o2_enterprise::enterprise::common::settings,
};

use crate::{
    common::meta::{
        http::HttpResponse as MetaHttpResponse,
        organization::{
            OrganizationSetting, OrganizationSettingPayload, OrganizationSettingResponse,
        },
    },
    service::db::organization::{get_org_setting, set_org_setting},
};

/// Organization specific settings
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "OrganizationSettingCreate",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = OrganizationSettingPayload, description = "Organization settings", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/settings")]
async fn create(
    path: web::Path<String>,
    settings: web::Json<OrganizationSettingPayload>,
) -> Result<HttpResponse, StdErr> {
    let org_id = path.into_inner();
    let settings = settings.into_inner();
    let mut data = match get_org_setting(&org_id).await {
        Ok(data) => data,
        Err(err) => {
            if let Error::DbError(DbError::KeyNotExists(_e)) = &err {
                OrganizationSetting::default()
            } else {
                return Ok(MetaHttpResponse::bad_request(&err));
            }
        }
    };

    let mut field_found = false;
    if let Some(scrape_interval) = settings.scrape_interval {
        if scrape_interval == 0 {
            return Ok(MetaHttpResponse::bad_request(
                "scrape_interval should be a positive value",
            ));
        }
        field_found = true;
        data.scrape_interval = scrape_interval;
    }
    if let Some(trace_id_field_name) = settings.trace_id_field_name {
        field_found = true;
        data.trace_id_field_name = trace_id_field_name;
    }
    if let Some(span_id_field_name) = settings.span_id_field_name {
        field_found = true;
        data.span_id_field_name = span_id_field_name;
    }
    if let Some(toggle_ingestion_logs) = settings.toggle_ingestion_logs {
        field_found = true;
        data.toggle_ingestion_logs = toggle_ingestion_logs;
    }

    if let Some(enable_websocket_search) = settings.enable_websocket_search {
        // allow only if websocket is enabled
        if get_config().websocket.enabled {
            field_found = true;
            data.enable_websocket_search = enable_websocket_search;
        }
    }

    if !field_found {
        return Ok(MetaHttpResponse::bad_request("No valid field found"));
    }

    match set_org_setting(&org_id, &data).await {
        Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e.to_string().as_str())),
    }
}

/// Retrieve organization specific settings
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "OrganizationSettingGet",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/settings")]
async fn get(path: web::Path<String>) -> Result<HttpResponse, StdErr> {
    let org_id = path.into_inner();
    match get_org_setting(&org_id).await {
        Ok(data) => Ok(HttpResponse::Ok().json(OrganizationSettingResponse { data })),
        Err(err) => {
            if let Error::DbError(DbError::KeyNotExists(_e)) = &err {
                let setting = OrganizationSetting::default();
                if let Ok(()) = set_org_setting(&org_id, &setting).await {
                    return Ok(
                        HttpResponse::Ok().json(OrganizationSettingResponse { data: setting })
                    );
                }
            };
            Ok(MetaHttpResponse::bad_request(&err))
        }
    }
}

#[cfg(feature = "enterprise")]
#[post("/{org_id}/settings/logo")]
async fn upload_logo(mut payload: Multipart) -> Result<HttpResponse, StdErr> {
    match payload.try_next().await {
        Ok(field) => {
            let mut data: Vec<u8> = Vec::<u8>::new();
            if let Some(mut field) = field {
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.unwrap();
                    data.extend(chunk);
                }
                if data.is_empty() {
                    return Ok(MetaHttpResponse::bad_request("Image data not present"));
                }

                match settings::upload_logo(data).await {
                    Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
                    Err(e) => Ok(MetaHttpResponse::bad_request(e)),
                }
            } else {
                Ok(MetaHttpResponse::bad_request("Payload file not present"))
            }
        }
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[post("/{org_id}/settings/logo")]
async fn upload_logo() -> Result<HttpResponse, StdErr> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[delete("/{org_id}/settings/logo")]
async fn delete_logo() -> Result<HttpResponse, StdErr> {
    match settings::delete_logo().await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::internal_error(e)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[delete("/{org_id}/settings/logo")]
async fn delete_logo() -> Result<HttpResponse, StdErr> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[post("/{org_id}/settings/logo/text")]
async fn set_logo_text(body: web::Bytes) -> Result<HttpResponse, StdErr> {
    match settings::set_logo_text(body).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::bad_request(e)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[post("/{org_id}/settings/logo/text")]
async fn set_logo_text() -> Result<HttpResponse, StdErr> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(feature = "enterprise")]
#[delete("/{org_id}/settings/logo/text")]
async fn delete_logo_text() -> Result<HttpResponse, StdErr> {
    match settings::delete_logo_text().await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"successful": "true"}))),
        Err(e) => Ok(MetaHttpResponse::internal_error(e)),
    }
}

#[cfg(not(feature = "enterprise"))]
#[delete("/{org_id}/settings/logo/text")]
async fn delete_logo_text() -> Result<HttpResponse, StdErr> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}
