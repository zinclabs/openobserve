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

use std::io::Error;

use actix_web::{delete, http, post, put, web, HttpRequest, HttpResponse};
use config::ider;
use infra::table::ratelimit::RatelimitRule;

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse,
    service::{ratelimit, ratelimit::rule::RatelimitError},
};

impl From<RatelimitError> for HttpResponse {
    fn from(value: RatelimitError) -> Self {
        match value {
            RatelimitError::NotFound(_) => MetaHttpResponse::not_found(value),
            error => MetaHttpResponse::bad_request(error),
        }
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Ratelimit",
    operation_id = "createRatelimitRule",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = Ratelimit, description = "Ratelimit rule data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/ratelimit")]
pub async fn save_ratelimit(
    path: web::Path<String>,
    rule: web::Json<RatelimitRule>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let mut rr = rule.into_inner();
    rr.resource = rr.resource.trim().to_string();
    rr.org = org_id;
    if rr.rule_id.is_none() {
        rr.rule_id = Some(ider::generate());
    }
    match ratelimit::rule::save(rr.clone()).await {
        Ok(()) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            serde_json::to_string(&rr)?,
        ))),
        Err(e) => Ok(e.into()),
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Ratelimit",
    operation_id = "DeleteRatelimitRule",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("rule_id" = String, Path, description = "Rule ID"),
    ),
    request_body(content = Ratelimit, description = "Ratelimit rule data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/ratelimit/{rule_id}")]
pub async fn delete_ratelimit(
    path: web::Path<(String, String)>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, rule_id) = path.into_inner();
    let rr = RatelimitRule {
        org: org_id,
        rule_id: Some(rule_id),
        ..Default::default()
    };

    match ratelimit::rule::delete(rr).await {
        Ok(()) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Ratelimit rule deleted successfully".to_string(),
        ))),
        Err(e) => Ok(e.into()),
    }
}

#[utoipa::path(
    context_path = "/api",
    tag = "Ratelimit",
    operation_id = "UpdateRatelimitRule",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("rule_id" = String, Path, description = "Rule ID"),
    ),
    request_body(content = Ratelimit, description = "Ratelimit rule data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/ratelimit/{rule_id}")]
pub async fn update_ratelimit(
    path: web::Path<(String, String)>,
    rule: web::Json<RatelimitRule>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (_, rule_id) = path.into_inner();
    let mut rr = rule.into_inner();
    if rr.rule_id.is_none() {
        rr.rule_id = Some(rule_id)
    }

    match ratelimit::rule::update(rr).await {
        Ok(()) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Ratelimit rule updated successfully".to_string(),
        ))),
        Err(e) => Ok(e.into()),
    }
}
