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
use infra::table::cipher::CipherEntry;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::cipher::{Cipher, CipherData};

#[cfg(feature = "enterprise")]
use crate::cipher::{KeyAddRequest, KeyGetResponse, KeyInfo, KeyListResponse};
use crate::common::meta::http::HttpResponse as MetaHttpResponse;

/// Store a key credential in db
#[utoipa::path(
    post,
    context_path = "/api",
    request_body(
        content = KeyAddRequest,
        description = "Key data to add",
        content_type = "application/json",
        // example = json!({            
        // })
    ),
    responses(
        (
            status = 200,
            description = "Empty response",
            body = (),
            content_type = "application/json",
            // example = json!({
            //     "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            // })
        ),
        (status = 400, description = "Invalid request", content_type = "application/json")
    ),
    tag = "Key"
)]
#[post("/{org_id}/cipher_keys")]
pub async fn save(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    {
        let req: KeyAddRequest = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
        };

        let user_id = match in_req.headers().get("user_id").map(|v| v.to_str().unwrap()) {
            None => return Ok(MetaHttpResponse::bad_request("invalid user_id in request")),
            Some(id) => id,
        };

        if req.name.contains(":") {
            return Ok(MetaHttpResponse::bad_request(
                "key name cannot have ':' in it",
            ));
        }

        let cd: CipherData = match req.key.try_into() {
            Ok(v) => v,
            Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
        };

        match cd.get_key().await {
            // here we are just checking that the key can encrypt a string, i.e.
            // it is set up correctly. We don't care what the actual entrypted string is.
            Ok(mut k) => match k.encrypt("hello world") {
                Ok(_) => {}
                Err(e) => {
                    return Ok(MetaHttpResponse::bad_request(format!(
                        "error creating key from request : {e}"
                    )))
                }
            },
            Err(e) => {
                return Ok(MetaHttpResponse::bad_request(format!(
                    "error creating key from request : {e}"
                )))
            }
        }

        match crate::service::db::keys::add(CipherEntry {
            org: org_id.to_string(),
            created_at: chrono::Utc::now().timestamp_micros(),
            created_by: user_id.to_string(),
            name: req.name,
            data: serde_json::to_string(&cd).unwrap(),
            kind: infra::table::cipher::EntryKind::CipherKey,
        })
        .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                "key created successfully".to_string(),
            ))),
            Err(e) => Ok(MetaHttpResponse::bad_request(format!(
                "error in saving : {e}"
            ))),
        }
    }
    #[cfg(not(feature = "enterprise"))]
    {
        Ok(MetaHttpResponse::forbidden("not supported"))
    }
}

/// get credentials for given key name
#[utoipa::path(
    get,
    context_path = "/api",
    params(
        ("key_name" = String, Path, description = "Name of the key to retrieve", example = "test_key")
    ),
    responses(
        (
            status = 200,
            description = "Key info",
            body = KeyGetResponse,
            content_type = "application/json",
            // example = json!({
            //     "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            // })
        ),
        (status = 404, description = "Key not found", content_type = "text/plain")
    ),
    tag = "Key"
)]
#[get("/{org_id}/cipher_keys/{key_name}")]
pub async fn get(
    _req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    {
        let (org_id, key_name) = path.into_inner();

        let kdata = match infra::table::cipher::get_data(
            &org_id,
            infra::table::cipher::EntryKind::CipherKey,
            &key_name,
        )
        .await
        {
            Ok(Some(k)) => k,
            Ok(None) => {
                return Ok(MetaHttpResponse::not_found(format!(
                    "key {key_name} not found"
                )))
            }
            Err(e) => {
                return Ok(MetaHttpResponse::internal_error(format!(
                    "error in getting key {e}"
                )))
            }
        };

        // we can be fairly certain that in db we have proper json
        let cd: CipherData = serde_json::from_str(&kdata).unwrap();

        let res = KeyGetResponse {
            name: key_name,
            key: cd.into(),
        };
        Ok(HttpResponse::Ok().json(res))
    }
    #[cfg(not(feature = "enterprise"))]
    Ok(MetaHttpResponse::forbidden("not supported"))
}

/// get credentials for given key name
#[utoipa::path(
    get,
    context_path = "/api",
    responses(
        (
            status = 200,
            description = "list all keys in the org",
            body = KeyListResponse,
            content_type = "application/json",
            // example = json!({
            //     "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            // })
        ),
    ),
    tag = "Key"
)]
#[get("/{org_id}/cipher_keys")]
pub async fn list(_req: HttpRequest, path: web::Path<String>) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    {
        let org_id = path.into_inner();

        let filter = infra::table::cipher::ListFilter {
            org: Some(org_id),
            kind: Some(infra::table::cipher::EntryKind::CipherKey),
        };

        let kdata = match infra::table::cipher::list_filtered(filter, None).await {
            Ok(list) => list,
            Err(e) => {
                return Ok(MetaHttpResponse::internal_error(format!(
                    "error in listing keys: {e}"
                )))
            }
        };

        let kdata = kdata
            .into_iter()
            .map(|d| {
                let cd = serde_json::from_str::<CipherData>(&d.data).unwrap();
                KeyInfo {
                    name: d.name,
                    key: cd.into(),
                }
            })
            .collect::<Vec<_>>();

        let res = KeyListResponse { keys: kdata };
        Ok(HttpResponse::Ok().json(res))
    }
    #[cfg(not(feature = "enterprise"))]
    Ok(MetaHttpResponse::forbidden("not supported"))
}

/// delete key credentials for given key name
#[utoipa::path(
    delete,
    context_path = "/api",
    params(
        ("key_name" = String, Path, description = "name of the key to delete", example = "test_key")
    ),
    responses(
        (
            status = 200,
            description = "Empty response",
            body = (),
            content_type = "application/json",
            // example = json!({
            //     "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            // })
        ),
    ),
    tag = "Keys"
)]
#[delete("/{org_id}/cipher_keys/{key_name}")]
pub async fn delete(
    _req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    {
        let (org_id, key_name) = path.into_inner();
        match crate::service::db::keys::remove(
            &org_id,
            infra::table::cipher::EntryKind::CipherKey,
            &key_name,
        )
        .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                "cipher key removed successfully".to_string(),
            ))),
            Err(e) => Ok(MetaHttpResponse::internal_error(format!(
                "error in removing key {e}"
            ))),
        }
    }
    #[cfg(not(feature = "enterprise"))]
    Ok(MetaHttpResponse::forbidden("not supported"))
}

/// update the credentials for given key
#[utoipa::path(
    post,
    context_path = "/api",
    request_body(
        content = KeyAddRequest,
        description = "Key data to update",
        content_type = "application/json",
        // example = json!({            
        // })
    ),
    responses(
        (
            status = 200,
            description = "Empty response",
            body = (),
            content_type = "application/json",
            // example = json!({
            //     "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            // })
        ),
        (status = 400, description = "Invalid request", content_type = "application/json")
    ),
    tag = "Key"
)]
#[put("/{org_id}/cipher_keys")]
pub async fn update(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    {
        let req: KeyAddRequest = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
        };

        let user_id = match in_req.headers().get("user_id").map(|v| v.to_str().unwrap()) {
            None => return Ok(MetaHttpResponse::bad_request("invalid user_id in request")),
            Some(id) => id,
        };

        if req.name.contains(":") {
            return Ok(MetaHttpResponse::bad_request(
                "key name cannot have ':' in it",
            ));
        }

        let cd: CipherData = match req.key.try_into() {
            Ok(v) => v,
            Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
        };

        match cd.get_key().await {
            // here we are just checking that the key can encrypt a string, i.e.
            // it is set up correctly. We don't care what the actual entrypted string is.
            Ok(mut k) => match k.encrypt("hello world") {
                Ok(_) => {}
                Err(e) => {
                    return Ok(MetaHttpResponse::bad_request(format!(
                        "error creating key from request : {e}"
                    )))
                }
            },
            Err(e) => {
                return Ok(MetaHttpResponse::bad_request(format!(
                    "error creating key from request : {e}"
                )))
            }
        }

        match crate::service::db::keys::update(CipherEntry {
            org: org_id.to_string(),
            created_at: chrono::Utc::now().timestamp_micros(),
            created_by: user_id.to_string(),
            name: req.name,
            data: serde_json::to_string(&cd).unwrap(),
            kind: infra::table::cipher::EntryKind::CipherKey,
        })
        .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(MetaHttpResponse::message(
                http::StatusCode::OK.into(),
                "key updated successfully".to_string(),
            ))),
            Err(e) => Ok(MetaHttpResponse::bad_request(format!(
                "error in saving : {e}"
            ))),
        }
    }
    #[cfg(not(feature = "enterprise"))]
    {
        Ok(MetaHttpResponse::forbidden("not supported"))
    }
}
