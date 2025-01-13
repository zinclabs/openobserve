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

use std::io::Error;

use actix_multipart::Multipart;
use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};
use config::SIZE_IN_MB;
use hashbrown::HashMap;
use url::Url;

use crate::{
    common::meta::{enrichment_table::EnrichmentTableReq, http::HttpResponse as MetaHttpResponse},
    service::enrichment_table::{
        add_task, cancel_jobs, create_empty_stream, delete_job, extract_multipart, get_job_status,
        list_jobs, save_enrichment_data,
    },
};

/// CreateEnrichmentTable
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "CreateUpdateEnrichmentTable",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("table_name" = String, Path, description = "Table name"),
    ),
    responses(
        (status = StatusCode::CREATED, description = "Saved enrichment table", body = HttpResponse),
        (status = StatusCode::BAD_REQUEST, description = "Bad Request", body = HttpResponse),
    ),
)]
#[post("/{org_id}/enrichment_tables/{table_name}")]
pub async fn save_enrichment_table(
    path: web::Path<(String, String)>,
    payload: Multipart,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let (org_id, table_name) = path.into_inner();
    let content_type = req.headers().get("content-type");
    let content_length = match req.headers().get("content-length") {
        None => 0.0,
        Some(content_length) => {
            content_length
                .to_str()
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0)
                / SIZE_IN_MB
        }
    };
    let cfg = config::get_config();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let append_data = match query.get("append") {
        Some(append_data) => append_data.parse::<bool>().unwrap_or(false),
        None => false,
    };
    match content_type {
        Some(content_type) => {
            if content_type
                .to_str()
                .unwrap_or("")
                .starts_with("multipart/form-data")
            {
                // Only multipart upload to honor the limit
                if content_length > cfg.limit.enrichment_table_limit as f64 {
                    return Ok(MetaHttpResponse::bad_request(format!(
                        "exceeds allowed limit of {} mb",
                        cfg.limit.enrichment_table_limit
                    )));
                }
                let json_record = extract_multipart(payload).await?;
                save_enrichment_data(&org_id, &table_name, json_record, append_data).await
            } else if content_type
                .to_str()
                .unwrap_or("")
                .starts_with("application/json")
            {
                let req: EnrichmentTableReq = match serde_json::from_slice(&body) {
                    Ok(req) => req,
                    Err(_) => {
                        return Ok(MetaHttpResponse::bad_request(
                            "Bad Request, req body must be a valid JSON",
                        ))
                    }
                };
                if req.file_link.is_empty() {
                    return Ok(MetaHttpResponse::bad_request(
                        "Bad Request, file_link empty",
                    ));
                };

                // Validate the file link
                if Url::parse(&req.file_link).is_err() {
                    return Ok(MetaHttpResponse::bad_request("Invalid file link provided"));
                }

                let (task_id, _) =
                    add_task(&org_id, &table_name, append_data, &req.file_link).await?;
                create_empty_stream(&org_id, &table_name).await?;

                return Ok(HttpResponse::Created().json(serde_json::json!({ "task_id": task_id })));
            } else {
                Ok(MetaHttpResponse::bad_request(
                    "Bad Request, content-type must be multipart/form-data or application/json",
                ))
            }
        }
        None => Ok(MetaHttpResponse::bad_request(
            "Bad Request, content-type must be multipart/form-data or application/json",
        )),
    }
}

#[get("/{org_id}/enrichment_tables/jobs")]
pub async fn list_enrichment_table_jobs(path: web::Path<String>) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    list_jobs(&org_id).await
}

#[get("/{org_id}/enrichment_tables/jobs/{task_id}")]
pub async fn status(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (_, task_id) = path.into_inner();
    get_job_status(&task_id).await
}

#[put("/{org_id}/enrichment_tables/jobs/cancel")]
pub async fn cancel(path: web::Path<String>, body: web::Bytes) -> Result<HttpResponse, Error> {
    let _org_id = path.into_inner();
    let job_ids: Vec<String> = match config::utils::json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    cancel_jobs(job_ids).await
}

#[delete("/{org_id}/enrichment_tables/jobs/{task_id}")]
pub async fn delete(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (_, task_id) = path.into_inner();
    delete_job(&task_id).await
}
