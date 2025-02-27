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

use actix_web::{delete, get, http, post, put, web, HttpRequest, HttpResponse};
use ahash::HashMap;
use config::{ider, meta::pipeline::Pipeline};

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse,
    service::{db::pipeline::PipelineError, pipeline},
};

impl From<PipelineError> for HttpResponse {
    fn from(value: PipelineError) -> Self {
        match value {
            PipelineError::InfraError(err) => MetaHttpResponse::internal_error(err),
            PipelineError::NotFound(_) => MetaHttpResponse::not_found(value),
            PipelineError::Modified(_) => MetaHttpResponse::conflict(value),
            error => MetaHttpResponse::bad_request(error),
        }
    }
}

/// CreatePipeline
#[utoipa::path(
    context_path = "/api",
    tag = "Pipeline",
    operation_id = "createPipeline",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = Pipeline, description = "Pipeline data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/pipelines")]
pub async fn save_pipeline(
    path: web::Path<String>,
    pipeline: web::Json<Pipeline>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let mut pipeline = pipeline.into_inner();
    pipeline.name = pipeline.name.trim().to_string();
    pipeline.org = org_id;
    pipeline.id = ider::generate();
    match pipeline::save_pipeline(pipeline).await {
        Ok(()) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Pipeline created successfully".to_string(),
        ))),
        Err(e) => Ok(e.into()),
    }
}

/// ListPipelines
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "listPipelines",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = PipelineList),
    )
)]
#[get("/{org_id}/pipelines")]
async fn list_pipelines(
    org_id: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let mut _permitted = None;
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        use o2_openfga::meta::mapping::OFGA_MODELS;

        let user_id = req.headers().get("user_id").unwrap();
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            user_id.to_str().unwrap(),
            "GET",
            OFGA_MODELS
                .get("pipelines")
                .map_or("pipelines", |model| model.key),
        )
        .await
        {
            Ok(list) => {
                _permitted = list;
            }
            Err(e) => {
                return Ok(crate::common::meta::http::HttpResponse::forbidden(
                    e.to_string(),
                ));
            }
        }
        // Get List of allowed objects ends
    }

    match pipeline::list_pipelines(org_id.into_inner(), _permitted).await {
        Ok(mut pipeline_list) => {
            if let Some(page_size) = query
                .get("pageSize")
                .and_then(|size| size.parse::<usize>().ok())
            {
                let page_idx = query
                    .get("pageIdx")
                    .and_then(|size| size.parse::<usize>().ok())
                    .unwrap_or_default();
                pipeline_list.list = pipeline_list
                    .list
                    .into_iter()
                    .skip(page_idx * page_size)
                    .take(page_size)
                    .collect();
            }
            Ok(HttpResponse::Ok().json(pipeline_list))
        }
        Err(e) => Ok(e.into()),
    }
}

/// GetStreamsWithPipeline
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "getStreamsWithPipeline",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = PipelineList),
    )
)]
#[get("/{org_id}/pipelines/streams")]
async fn list_streams_with_pipeline(
    path: web::Path<String>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    match pipeline::list_streams_with_pipeline(&org_id).await {
        Ok(stream_params) => Ok(HttpResponse::Ok().json(stream_params)),
        Err(e) => Ok(e.into()),
    }
}

/// DeletePipeline
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "deletePipeline",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("pipeline_id" = String, Path, description = "Pipeline ID"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/pipelines/{pipeline_id}")]
async fn delete_pipeline(
    path: web::Path<(String, String)>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (_org_id, pipeline_id) = path.into_inner();
    match pipeline::delete_pipeline(&pipeline_id).await {
        Ok(()) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Pipeline deleted successfully".to_string(),
        ))),
        Err(e) => Ok(e.into()),
    }
}

/// UpdatePipeline
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "updatePipeline",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = Pipeline, description = "Pipeline data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/pipelines")]
pub async fn update_pipeline(
    pipeline: web::Json<Pipeline>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let pipeline = pipeline.into_inner();
    match pipeline::update_pipeline(pipeline).await {
        Ok(()) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            "Pipeline updated successfully".to_string(),
        ))),
        Err(e) => Ok(e.into()),
    }
}

/// EnablePipeline
#[utoipa::path(
    context_path = "/api",
    tag = "Pipelines",
    operation_id = "enablePipeline",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("pipeline_id" = String, Path, description = "Pipeline ID"),
        ("value" = bool, Query, description = "Enable or disable pipeline"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure",  content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/pipelines/{pipeline_id}/enable")]
pub async fn enable_pipeline(
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, pipeline_id) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let enable = match query.get("value") {
        Some(v) => v.parse::<bool>().unwrap_or_default(),
        None => false,
    };
    let resp_msg =
        "Pipeline successfully ".to_string() + if enable { "enabled" } else { "disabled" };
    match pipeline::enable_pipeline(&org_id, &pipeline_id, enable).await {
        Ok(()) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
            http::StatusCode::OK.into(),
            resp_msg,
        ))),
        Err(e) => Ok(e.into()),
    }
}
