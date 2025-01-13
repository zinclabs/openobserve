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

use std::{collections::HashMap, io::Error, sync::Arc};

use actix_web::{
    cookie, delete, get,
    http::{self},
    post, put, web, HttpRequest, HttpResponse,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use config::{
    get_config,
    meta::user::UserRole,
    utils::{base64, json},
    Config,
};
use serde::Serialize;
#[cfg(feature = "enterprise")]
use {
    crate::service::self_reporting::audit,
    o2_enterprise::enterprise::common::{
        auditor::{AuditMessage, HttpMeta, Protocol},
        infra::config::get_config as get_o2_config,
    },
};

use crate::{
    common::{
        meta::{
            self,
            user::{
                get_roles, AuthTokens, PostUserRequest, RolesResponse, SignInResponse, SignInUser,
                UpdateUser, UserOrgRole, UserRequest, UserRoleRequest,
            },
        },
        utils::auth::{generate_presigned_url, UserEmail},
    },
    service::users,
};

pub mod service_accounts;

/// ListUsers
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserList",
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
#[get("/{org_id}/users")]
pub async fn list(org_id: web::Path<String>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let list_all = match query.get("list_all") {
        Some(v) => v.parse::<bool>().unwrap_or(false),
        None => false,
    };

    let user_id = req.headers().get("user_id").unwrap().to_str().unwrap();
    let mut _user_list_from_rbac = None;
    // Get List of allowed objects
    #[cfg(feature = "enterprise")]
    {
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id, user_id, "GET", "user",
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

    users::list_users(&org_id, user_id, None, _user_list_from_rbac, list_all).await
}

/// CreateUser
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserSave",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = PostUserRequest, description = "User data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/users")]
pub async fn save(
    org_id: web::Path<String>,
    user: web::Json<PostUserRequest>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let org_id = org_id.into_inner();
    let initiator_id = user_email.user_id;
    let mut user = UserRequest::from(&user.into_inner());
    user.email = user.email.trim().to_lowercase();

    if user.role.base_role.eq(&config::meta::user::UserRole::Root) {
        return Ok(
            HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Not allowed".to_string(),
            )),
        );
    }
    #[cfg(not(feature = "enterprise"))]
    {
        user.role.base_role = config::meta::user::UserRole::Admin;
    }
    users::post_user(&org_id, user, &initiator_id).await
}

/// UpdateUser
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserUpdate",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("email_id" = String, Path, description = "User's email id"),
    ),
    request_body(content = UpdateUser, description = "User data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/users/{email_id}")]
pub async fn update(
    params: web::Path<(String, String)>,
    user: web::Json<UpdateUser>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = params.into_inner();
    let email_id = email_id.trim().to_lowercase();
    #[cfg(not(feature = "enterprise"))]
    let mut user = user.into_inner();
    #[cfg(feature = "enterprise")]
    let user = user.into_inner();
    if user.eq(&UpdateUser::default()) {
        return Ok(
            HttpResponse::BadRequest().json(meta::http::HttpResponse::error(
                http::StatusCode::BAD_REQUEST.into(),
                "Please specify appropriate fields to update user".to_string(),
            )),
        );
    }
    #[cfg(not(feature = "enterprise"))]
    {
        user.role = Some(UserRoleRequest {
            role: config::meta::user::UserRole::Admin.to_string(),
            custom: None,
        });
    }
    let initiator_id = &user_email.user_id;
    let self_update = user_email.user_id.eq(&email_id);
    users::update_user(&org_id, &email_id, self_update, initiator_id, user).await
}

/// AddUserToOrganization
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "AddUserToOrg",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("email_id" = String, Path, description = "User's email id"),
    ),
    request_body(content = UserRoleRequest, description = "User role", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/users/{email_id}")]
pub async fn add_user_to_org(
    params: web::Path<(String, String)>,
    role: web::Json<UserRoleRequest>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = params.into_inner();
    let initiator_id = user_email.user_id;
    let role = UserOrgRole::from(&role.into_inner());
    users::add_user_to_org(&org_id, &email_id, role, &initiator_id).await
}

fn _prepare_cookie<'a, T: Serialize + ?Sized, E: Into<cookie::Expiration>>(
    conf: &Arc<Config>,
    cookie_name: &'a str,
    token_struct: &T,
    cookie_expiry: E,
) -> cookie::Cookie<'a> {
    let tokens = json::to_string(token_struct).unwrap();
    let mut auth_cookie = cookie::Cookie::new(cookie_name, tokens);
    auth_cookie.set_expires(cookie_expiry.into());
    auth_cookie.set_http_only(true);
    auth_cookie.set_secure(conf.auth.cookie_secure_only);
    auth_cookie.set_path("/");
    if conf.auth.cookie_same_site_lax {
        auth_cookie.set_same_site(cookie::SameSite::Lax);
    } else {
        auth_cookie.set_same_site(cookie::SameSite::None);
    }
    auth_cookie
}
/// RemoveUserFromOrganization
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "RemoveUserFromOrg",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("email_id" = String, Path, description = "User name"),
      ),
    responses(
        (status = 200, description = "Success",  content_type = "application/json", body = HttpResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[delete("/{org_id}/users/{email_id}")]
pub async fn delete(
    path: web::Path<(String, String)>,
    user_email: UserEmail,
) -> Result<HttpResponse, Error> {
    let (org_id, email_id) = path.into_inner();
    let initiator_id = user_email.user_id;
    users::remove_user_from_org(&org_id, &email_id, &initiator_id).await
}

/// AuthenticateUser
#[utoipa::path(
context_path = "/auth",
    tag = "Auth",
    operation_id = "UserLoginCheck",
    request_body(content = SignInUser, description = "User login", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SignInResponse),
    )
)]
#[post("/login")]
pub async fn authentication(
    auth: Option<web::Json<SignInUser>>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    let native_login_enabled = get_o2_config().dex.native_login_enabled;
    #[cfg(not(feature = "enterprise"))]
    let native_login_enabled = true;

    if !native_login_enabled {
        return Ok(HttpResponse::Forbidden().json("Not Supported"));
    }

    // Until decoding the token or body, we can not know the the user_email
    #[cfg(feature = "enterprise")]
    let mut audit_message = AuditMessage {
        user_email: "".to_string(),
        org_id: "".to_string(),
        _timestamp: chrono::Utc::now().timestamp_micros(),
        protocol: Protocol::Http(HttpMeta {
            method: "POST".to_string(),
            path: "/auth/login".to_string(),
            body: "".to_string(),
            query_params: _req.query_string().to_string(),
            response_code: 200,
        }),
    };

    let mut resp = SignInResponse::default();
    let auth = match auth {
        Some(auth) => {
            let mut auth = auth.into_inner();
            auth.name = auth.name.to_lowercase();
            auth
        }
        None => {
            // get Authorization header from request
            #[cfg(feature = "enterprise")]
            {
                let auth_header = _req.headers().get("Authorization");
                if auth_header.is_some() {
                    let auth_header = auth_header.unwrap().to_str().unwrap();
                    if let Some((name, password)) =
                        o2_enterprise::enterprise::dex::service::auth::get_user_from_token(
                            auth_header,
                        )
                    {
                        SignInUser { name, password }
                    } else {
                        audit_unauthorized_error(audit_message).await;
                        return unauthorized_error(resp);
                    }
                } else {
                    audit_unauthorized_error(audit_message).await;
                    return unauthorized_error(resp);
                }
            }
            #[cfg(not(feature = "enterprise"))]
            {
                return unauthorized_error(resp);
            }
        }
    };

    #[cfg(feature = "enterprise")]
    {
        audit_message.user_email = auth.name.clone();
    }

    #[cfg(feature = "enterprise")]
    {
        if get_o2_config().dex.root_only_login
            && !crate::common::utils::auth::is_root_user(&auth.name)
        {
            audit_unauthorized_error(audit_message).await;
            return unauthorized_error(resp);
        }
    }
    match crate::handler::http::auth::validator::validate_user(&auth.name, &auth.password).await {
        Ok(v) => {
            if v.is_valid {
                resp.status = true;
            } else {
                #[cfg(feature = "enterprise")]
                audit_unauthorized_error(audit_message).await;
                return unauthorized_error(resp);
            }
        }
        Err(_e) => {
            #[cfg(feature = "enterprise")]
            audit_unauthorized_error(audit_message).await;
            return unauthorized_error(resp);
        }
    };
    if resp.status {
        let cfg = get_config();

        let access_token = format!(
            "Basic {}",
            base64::encode(&format!("{}:{}", auth.name, auth.password))
        );
        let tokens = json::to_string(&AuthTokens {
            access_token,
            refresh_token: "".to_string(),
        })
        .unwrap();

        let mut auth_cookie = cookie::Cookie::new("auth_tokens", tokens);
        auth_cookie.set_expires(
            cookie::time::OffsetDateTime::now_utc()
                + cookie::time::Duration::seconds(cfg.auth.cookie_max_age),
        );
        auth_cookie.set_http_only(true);
        auth_cookie.set_secure(cfg.auth.cookie_secure_only);
        auth_cookie.set_path("/");
        if cfg.auth.cookie_same_site_lax {
            auth_cookie.set_same_site(cookie::SameSite::Lax);
        } else {
            auth_cookie.set_same_site(cookie::SameSite::None);
        }
        // audit the successful login
        #[cfg(feature = "enterprise")]
        audit(audit_message).await;
        Ok(HttpResponse::Ok().cookie(auth_cookie).json(resp))
    } else {
        #[cfg(feature = "enterprise")]
        audit_unauthorized_error(audit_message).await;
        unauthorized_error(resp)
    }
}

#[derive(serde::Deserialize)]
struct PresignedURLGenerator {
    #[serde(default = "default_exp_in")]
    exp_in: u32,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct PresignedURLGeneratorResponse {
    url: String,
}

const fn default_exp_in() -> u32 {
    600
}

#[get("/presigned-url")]
pub async fn get_presigned_url(
    _req: HttpRequest,
    basic_auth: BasicAuth,
    query: web::Query<PresignedURLGenerator>,
) -> Result<HttpResponse, Error> {
    let cfg = get_config();
    let time = chrono::Utc::now().timestamp();
    let password_ext_salt = cfg.auth.ext_auth_salt.as_str();

    let base_url = format!("{}{}", cfg.common.web_url, cfg.common.base_uri);
    let url = generate_presigned_url(
        basic_auth.user_id(),
        basic_auth.password().unwrap(),
        password_ext_salt,
        &base_url,
        query.exp_in as i64,
        time,
    );

    let payload = PresignedURLGeneratorResponse { url };
    #[cfg(feature = "enterprise")]
    {
        let audit_message = AuditMessage {
            user_email: basic_auth.user_id().to_string(),
            org_id: "".to_string(),
            _timestamp: chrono::Utc::now().timestamp_micros(),
            protocol: Protocol::Http(HttpMeta {
                method: "GET".to_string(),
                path: "/auth/presigned-url".to_string(),
                body: "".to_string(),
                query_params: _req.query_string().to_string(),
                response_code: 200,
            }),
        };
        audit(audit_message).await;
    }
    Ok(HttpResponse::Ok().json(&payload))
}

#[get("/login")]
pub async fn get_auth(_req: HttpRequest) -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    {
        use actix_web::http::header;
        use chrono::Utc;

        use crate::{
            common::meta::user::AuthTokensExt, handler::http::auth::validator::ID_TOKEN_HEADER,
        };

        let mut resp = SignInResponse::default();

        let query = web::Query::<std::collections::HashMap<String, String>>::from_query(
            _req.query_string(),
        )
        .unwrap();

        let mut request_time = None;
        let mut expires_in = 300;
        let mut req_ts = 0;

        let mut audit_message = AuditMessage {
            user_email: "".to_string(),
            org_id: "".to_string(),
            _timestamp: chrono::Utc::now().timestamp_micros(),
            protocol: Protocol::Http(HttpMeta {
                method: "GET".to_string(),
                path: "/auth/login".to_string(),
                body: "".to_string(),
                // Don't include query string as it may contain the auth token
                query_params: "".to_string(),
                response_code: 302,
            }),
        };

        let (name, password) = {
            let auth_header = if let Some(s) = query.get("auth") {
                match query.get("request_time") {
                    Some(req_time_str) => {
                        if let Ok(ts) = config::utils::time::parse_str_to_time(req_time_str) {
                            req_ts = ts.timestamp();
                        } else {
                            audit_unauthorized_error(audit_message).await;
                            return unauthorized_error(resp);
                        }
                        request_time = Some(req_time_str);
                    }
                    None => {
                        audit_unauthorized_error(audit_message).await;
                        return unauthorized_error(resp);
                    }
                };

                match query.get("exp_in") {
                    Some(exp_in_str) => {
                        expires_in = exp_in_str.parse::<i64>().unwrap();
                    }
                    None => {
                        audit_unauthorized_error(audit_message).await;
                        return unauthorized_error(resp);
                    }
                };
                if Utc::now().timestamp() - req_ts > expires_in {
                    audit_unauthorized_error(audit_message).await;
                    return unauthorized_error(resp);
                }
                format!("q_auth {}", s)
            } else if let Some(auth_header) = _req.headers().get("Authorization") {
                match auth_header.to_str() {
                    Ok(auth_header_str) => auth_header_str.to_string(),
                    Err(_) => {
                        audit_unauthorized_error(audit_message).await;
                        return unauthorized_error(resp);
                    }
                }
            } else {
                audit_unauthorized_error(audit_message).await;
                return unauthorized_error(resp);
            };

            use o2_enterprise::enterprise::dex::service::auth::get_user_from_token;

            use crate::handler::http::auth::validator::{
                validate_user, validate_user_for_query_params,
            };

            let (name, password) = if let Some((name, password)) = get_user_from_token(&auth_header)
            {
                let token_validation_response = match request_time {
                    Some(req_ts) => {
                        log::debug!("Validating user for query params");
                        validate_user_for_query_params(&name, &password, Some(req_ts), expires_in)
                            .await
                    }
                    None => {
                        log::debug!("Validating user for basic auth header");
                        validate_user(&name, &password).await
                    }
                };

                audit_message.user_email = name.clone();
                match token_validation_response {
                    Ok(v) => {
                        if v.is_valid {
                            resp.status = true;
                            (name, password)
                        } else {
                            audit_unauthorized_error(audit_message).await;
                            return unauthorized_error(resp);
                        }
                    }
                    Err(_) => {
                        audit_unauthorized_error(audit_message).await;
                        return unauthorized_error(resp);
                    }
                }
            } else {
                audit_unauthorized_error(audit_message).await;
                return unauthorized_error(resp);
            };
            (name, password)
        };

        if resp.status {
            let cfg = get_config();
            let id_token = config::utils::json::json!({
                "email": name,
                "name": name,
            });
            let cookie_name = "auth_tokens";
            let auth_cookie = if req_ts == 0 {
                let access_token = format!(
                    "Basic {}",
                    base64::encode(&format!("{}:{}", &name, &password))
                );
                let tokens = AuthTokens {
                    access_token,
                    refresh_token: "".to_string(),
                };
                let expiry = cookie::time::OffsetDateTime::now_utc()
                    + cookie::time::Duration::seconds(cfg.auth.cookie_max_age);

                log::debug!("Setting cookie for user: {} - {}", name, cookie_name);
                _prepare_cookie(&cfg, cookie_name, &tokens, expiry)
            } else {
                let cookie_name = "auth_ext";
                let auth_ext = format!(
                    "{} {}",
                    cookie_name,
                    base64::encode(&format!("{}:{}", &name, &password))
                );

                let tokens = AuthTokensExt {
                    auth_ext,
                    refresh_token: "".to_string(),
                    request_time: req_ts,
                    expires_in,
                };
                let expiry = cookie::time::OffsetDateTime::now_utc()
                    + cookie::time::Duration::seconds(req_ts);

                log::debug!("Setting cookie for user: {} - {}", name, cookie_name);
                _prepare_cookie(&cfg, cookie_name, &tokens, expiry)
            };

            let url = format!(
                "{}{}/web/cb#id_token={}.{}",
                cfg.common.web_url,
                cfg.common.base_uri,
                ID_TOKEN_HEADER,
                base64::encode(&id_token.to_string())
            );
            audit_message._timestamp = Utc::now().timestamp_micros();
            audit(audit_message).await;
            Ok(HttpResponse::Found()
                .append_header((header::LOCATION, url))
                .cookie(auth_cookie)
                .json(resp))
        } else {
            audit_unauthorized_error(audit_message).await;
            unauthorized_error(resp)
        }
    }

    #[cfg(not(feature = "enterprise"))]
    {
        Ok(HttpResponse::Forbidden().json("Not Supported"))
    }
}

/// ListUsers
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserRoles",
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
#[get("/{org_id}/users/roles")]
pub async fn list_roles(_org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let roles = get_roles()
        .iter()
        .filter_map(check_role_available)
        .collect::<Vec<RolesResponse>>();

    Ok(HttpResponse::Ok().json(roles))
}

fn check_role_available(role: &UserRole) -> Option<RolesResponse> {
    if role.eq(&UserRole::Root) || role.eq(&UserRole::ServiceAccount) {
        None
    } else {
        #[cfg(feature = "enterprise")]
        if !get_o2_config().openfga.enabled && role.ne(&UserRole::Admin) {
            return None;
        }
        Some(RolesResponse {
            label: role.get_label(),
            value: role.to_string(),
        })
    }
}

fn unauthorized_error(mut resp: SignInResponse) -> Result<HttpResponse, Error> {
    resp.status = false;
    resp.message = "Invalid credentials".to_string();
    Ok(HttpResponse::Unauthorized().json(resp))
}

#[cfg(feature = "enterprise")]
async fn audit_unauthorized_error(mut audit_message: AuditMessage) {
    use chrono::Utc;

    audit_message._timestamp = Utc::now().timestamp_micros();
    if let Protocol::Http(http_meta) = &mut audit_message.protocol {
        http_meta.response_code = 401;
    }
    // Even if the user_email of audit_message is not set, still the event should be audited
    audit(audit_message).await;
}

/// ListUserInvitations
#[cfg(feature = "cloud")]
#[utoipa::path(
    context_path = "/api",
    tag = "Users",
    operation_id = "UserInvitations",
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
#[get("/invites")]
pub async fn list_invitations(user_email: UserEmail) -> Result<HttpResponse, Error> {
    let user_id = user_email.user_id.as_str();
    users::list_user_invites(user_id).await
}

#[cfg(not(feature = "cloud"))]
#[get("/invites")]
pub async fn list_invitations(_user_email: UserEmail) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Forbidden().json("Not Supported"))
}

#[cfg(test)]
mod tests {
    use actix_web::{test, App};
    use actix_web_httpauth::headers::authorization::Basic;

    use super::*;

    #[tokio::test]
    async fn test_get_presigned_url() {
        let mut app = test::init_service(App::new().service(get_presigned_url)).await;

        let auth = Basic::new("username", Some("password"));
        let req = test::TestRequest::get()
            .uri("/presigned-url")
            .append_header((actix_web::http::header::AUTHORIZATION, auth))
            .to_request();
        let resp = test::call_service(&mut app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);

        let body = test::read_body(resp).await;
        let response_body: PresignedURLGeneratorResponse = serde_json::from_slice(&body).unwrap();

        assert!(!response_body.url.is_empty());
    }
}
