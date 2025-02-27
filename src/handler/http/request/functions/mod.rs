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

use std::{collections::HashMap, io::Error};

use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};
use config::meta::function::{TestVRLRequest, Transform};

/// CreateFunction
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "createFunction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = Transform, description = "Function data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/functions")]
pub async fn save_function(
    path: web::Path<String>,
    func: web::Json<Transform>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let mut transform = func.into_inner();
    transform.name = transform.name.trim().to_string();
    transform.function = transform.function.trim().to_string();
    crate::service::functions::save_function(org_id, transform).await
}

/// ListFunctions
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "listFunctions",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = FunctionList),
    )
)]
#[get("/{org_id}/functions")]
async fn list_functions(
    org_id: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let mut _permitted = None;
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();

    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        let user_id = req.headers().get("user_id").unwrap();
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            user_id.to_str().unwrap(),
            "GET",
            "function",
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

    match crate::service::functions::list_functions(org_id.into_inner(), _permitted).await {
        Ok(mut functions) => {
            if let Some(page_size) = query
                .get("pageSize")
                .and_then(|size| size.parse::<usize>().ok())
            {
                let page_idx = query
                    .get("pageIdx")
                    .and_then(|size| size.parse::<usize>().ok())
                    .unwrap_or_default();
                functions.list = functions
                    .list
                    .into_iter()
                    .skip(page_idx * page_size)
                    .take(page_size)
                    .collect();
            }
            Ok(HttpResponse::Ok().json(functions))
        }
        Err(e) => Ok(HttpResponse::BadRequest().body(e.to_string())),
    }
}

/// DeleteFunction
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "deleteFunction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Function name"),
        ("force" = bool, Query, description = "Force delete function regardless pipeline dependencies"),
    ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/functions/{name}")]
async fn delete_function(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    crate::service::functions::delete_function(org_id, name).await
}

/// UpdateFunction
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "updateFunction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Function name"),
    ),
    request_body(content = Transform, description = "Function data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/functions/{name}")]
pub async fn update_function(
    path: web::Path<(String, String)>,
    func: web::Json<Transform>,
) -> Result<HttpResponse, Error> {
    let (org_id, name) = path.into_inner();
    let name = name.trim();
    let mut transform = func.into_inner();
    transform.name = transform.name.trim().to_string();
    transform.function = transform.function.trim().to_string();
    crate::service::functions::update_function(&org_id, name, transform).await
}

/// FunctionPipelineDependency
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "functionPipelineDependency",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("name" = String, Path, description = "Function name"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = FunctionList),
        (status = 404, description = "Function not found", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Internal server error", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/functions/{name}")]
pub async fn list_pipeline_dependencies(
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    let (org_id, fn_name) = path.into_inner();
    crate::service::functions::get_pipeline_dependencies(&org_id, &fn_name).await
}

/// Test a Function
#[utoipa::path(
    context_path = "/api",
    tag = "Functions",
    operation_id = "testFunction",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = TestVRLRequest, description = "Test run function", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/functions/test")]
pub async fn test_function(
    path: web::Path<String>,
    req_body: web::Json<TestVRLRequest>,
) -> Result<HttpResponse, Error> {
    let org_id = path.into_inner();
    let TestVRLRequest { function, events } = req_body.into_inner();

    // Assuming `test_function` applies the VRL function to each event
    match crate::service::functions::test_run_function(&org_id, function, events).await {
        Ok(result) => Ok(result),
        Err(err) => Ok(HttpResponse::BadRequest().body(err.to_string())),
    }
}
