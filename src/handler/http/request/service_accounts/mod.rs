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

use std::io::{Error, ErrorKind};

use actix_web::{
    delete, get,
    http::{self},
    post, put, web, HttpRequest, HttpResponse,
};
use config::{meta::user::UserRole, utils::rand::generate_random_string};
use hashbrown::HashMap;

use crate::{
    common::{
        meta::{
            self,
            http::HttpResponse as MetaHttpResponse,
            service_account::{APIToken, ServiceAccountRequest, UpdateServiceAccountRequest},
            user::{UpdateUser, UserRequest},
        },
        utils::auth::UserEmail,
    },
    service::users,
};

/// ListServiceAccounts
#[utoipa::path(
    context_path = "/api",
    tag = "ServiceAccounts",
    operation_id = "ServiceAccountsList",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = UserList),
    )
)]
#[get("/{org_id}/service_accounts")]
pub async fn list(org_id: web::Path<String>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let mut _user_list_from_rbac = None;
    let user_id = req.headers().get("user_id").unwrap().to_str().unwrap();
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            user_id,
            "GET",
            "service_accounts",
        )
        .await
        {
            Ok(user_list) => {
                _user_list_from_rbac = user_list;
            }
            Err(e) => {
                return Ok(crate::common::meta::http::HttpResponse::forbidden(
                    e.to_string(),
                ));
            }
        }
        // Get List of allowed objects ends
    }
    users::list_users(
        &org_id,
        user_id,
        Some(UserRole::ServiceAccount),
        _user_list_from_rbac,
        false,
    )
    .await
}

/// CreateServiceAccount
#[utoipa::path(
    context_path = "/api",
    tag = "ServiceAccounts",
    operation_id = "ServiceAccountSave",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = ServiceAccountRequest, description = "ServiceAccount data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/service_accounts")]
pub async fn save(
    org_id: web::Path<String>,
    service_account: web::Json<ServiceAccountRequest>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let initiator_id = user_email.user_id;
    let service_account = service_account.into_inner();
    let user = UserRequest {
        email: service_account.email.trim().to_string(),
        first_name: service_account.first_name.trim().to_string(),
        last_name: service_account.last_name.trim().to_string(),
        password: generate_random_string(16),
        role: meta::user::UserOrgRole {
            base_role: UserRole::ServiceAccount,
            custom_role: None,
        },
        is_external: false,
    };

    users::post_user(&org_id, user, &initiator_id).await
}

/// UpdateServiceAccount
#[utoipa::path(
    context_path = "/api",
    tag = "ServiceAccounts",
    operation_id = "ServiceAccountUpdate",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("email_id" = String, Path, description = "Service Account email id"),
    ),
    request_body(content = UpdateServiceAccountRequest, description = "Service Account data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/service_accounts/{email_id}")]
pub async fn update(
    params: web::Path<(String, String)>,
    service_account: web::Json<UpdateServiceAccountRequest>,
    user_email: UserEmail,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = params.into_inner();
    let email_id = email_id.trim().to_lowercase();
    let query = match web::Query::<HashMap<String, String>>::from_query(req.query_string()) {
        Ok(query) => query,
        Err(e) => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Invalid query string: {}", e),
            ));
        }
    };

    let rotate_token = match query.get("rotateToken") {
        Some(s) => match s.to_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => {
                return Err(Error::new(
                    ErrorKind::Other,
                    " 'rotateToken' query param with value 'true' or 'false' allowed",
                ));
            }
        },
        None => false,
    };

    if rotate_token {
        return match crate::service::organization::update_passcode(Some(&org_id), &email_id).await {
            Ok(passcode) => Ok(HttpResponse::Ok().json(APIToken {
                token: passcode.passcode,
                user: passcode.user,
            })),
            Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                http::StatusCode::NOT_FOUND.into(),
                e.to_string(),
            ))),
        };
    };
    let service_account = service_account.into_inner();

    let user = UpdateUser {
        change_password: false,
        first_name: Some(service_account.first_name.trim().to_string()),
        last_name: Some(service_account.last_name.trim().to_string()),
        old_password: None,
        new_password: None,
        role: None,
        token: None,
    };
    let initiator_id = &user_email.user_id;

    users::update_user(&org_id, &email_id, false, initiator_id, user).await
}

/// RemoveServiceAccount
#[utoipa::path(
    context_path = "/api",
    tag = "ServiceAccounts",
    operation_id = "RemoveServiceAccount",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("email_id" = String, Path, description = "Service Account email id"),
      ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/service_accounts/{email_id}")]
pub async fn delete(
    path: web::Path<(String, String)>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = path.into_inner();
    let initiator_id = user_email.user_id;
    users::remove_user_from_org(&org_id, &email_id, &initiator_id).await
}

/// GetAPIToken
#[utoipa::path(
    context_path = "/api",
     tag = "ServiceAccounts",
    operation_id = "GetServiceAccountToken",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = APIToken),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/service_accounts/{email_id}")]
pub async fn get_api_token(path: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let (org, user_id) = path.into_inner();
    let org_id = Some(org.as_str());
    match crate::service::organization::get_passcode(org_id, &user_id).await {
        Ok(passcode) => Ok(HttpResponse::Ok().json(APIToken {
            token: passcode.passcode,
            user: passcode.user,
        })),
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}
