// Copyright 2022 Zinc Labs Inc. and Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use actix_web::{http, post, web, HttpRequest, HttpResponse};
use std::io::Error;

use crate::meta;

#[post("/{org_id}/metadata/{table_name}")]
pub async fn set(
    path: web::Path<(String, String)>,
    body: actix_web::web::Bytes,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, key) = path.into_inner();
    let content_type = req.headers().get("Content-Type").unwrap();
    if content_type == "text/csv" {
        Ok(HttpResponse::Ok().into())
    } else {
        Ok(
            HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Bad Request".to_string(),
            )),
        )
    }
}
