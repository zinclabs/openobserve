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

use core::clone::Clone;

use actix_web::{
    dev::ServiceRequest,
    error::{ErrorForbidden, ErrorNotFound, ErrorUnauthorized},
    http::{header, Method},
    web, Error,
};
use config::{
    get_config,
    meta::user::{DBUser, UserRole},
    utils::base64,
};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;

use crate::{
    common::{
        meta::{
            ingestion::INGESTION_EP,
            organization::DEFAULT_ORG,
            user::{
                get_default_user_role, AuthTokensExt, TokenValidationResponse,
                TokenValidationResponseBuilder,
            },
        },
        utils::{
            auth::{get_hash, is_root_user, AuthExtractor},
            redirect_response::RedirectResponseBuilder,
        },
    },
    service::{db, users},
};

pub const PKCE_STATE_ORG: &str = "o2_pkce_state";
pub const ACCESS_TOKEN: &str = "access_token";
pub const REFRESH_TOKEN: &str = "refresh_token";
pub const ID_TOKEN_HEADER: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";

pub async fn validator(
    req: ServiceRequest,
    user_id: &str,
    password: &str,
    auth_info: AuthExtractor,
    path_prefix: &str,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let cfg = get_config();
    let path = match req
        .request()
        .path()
        .strip_prefix(format!("{}{}", cfg.common.base_uri, path_prefix).as_str())
    {
        Some(path) => path,
        None => req.request().path(),
    };
    match if auth_info.auth.starts_with("{\"auth_ext\":") {
        let auth_token: AuthTokensExt =
            config::utils::json::from_str(&auth_info.auth).unwrap_or_default();
        validate_credentials_ext(user_id, password, path, auth_token).await
    } else {
        validate_credentials(user_id, password.trim(), path).await
    } {
        Ok(res) => {
            if res.is_valid {
                // Check and create organization if needed
                if let Err(e) = check_and_create_org(user_id, req.method(), path).await {
                    return Err((e, req));
                }

                #[cfg(feature = "enterprise")]
                let path = path.to_owned();

                // / Hack for prometheus, need support POST and check the header
                let mut req = req;
                if req.method().eq(&Method::POST) && !req.headers().contains_key("content-type") {
                    req.headers_mut().insert(
                        header::CONTENT_TYPE,
                        header::HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                }
                req.headers_mut().insert(
                    header::HeaderName::from_static("user_id"),
                    header::HeaderValue::from_str(&res.user_email).unwrap(),
                );

                #[cfg(feature = "enterprise")]
                if let Some(role) = &res.user_role {
                    if role.eq(&UserRole::Viewer)
                        && req.method().eq(&Method::PUT)
                        && path.ends_with(&format!("users/{}", res.user_email))
                    {
                        // Viewer should be able to update its own details
                        return Ok(req);
                    }
                }

                if auth_info.bypass_check
                    || check_permissions(
                        user_id,
                        auth_info,
                        res.user_role.unwrap_or(get_default_user_role()),
                        !res.is_internal_user,
                    )
                    .await
                {
                    Ok(req)
                } else {
                    Err((ErrorForbidden("Unauthorized Access"), req))
                }
            } else {
                Err((ErrorUnauthorized("Unauthorized Access"), req))
            }
        }
        Err(err) => {
            log::debug!("Token Validation Error: {:#?}", err);
            Err((err, req))
        }
    }
}

/// `validate_token` validates the endpoints which are token only.
/// This includes endpoints like `rum` etc.
///
/// ### Args:
/// - token: The token to validate
///  
pub async fn validate_token(token: &str, org_id: &str) -> Result<(), Error> {
    match users::get_user_by_token(org_id, token).await {
        Some(_user) => Ok(()),
        None => Err(ErrorForbidden("User associated with this token not found")),
    }
}

pub async fn validate_credentials(
    user_id: &str,
    user_password: &str,
    path: &str,
) -> Result<TokenValidationResponse, Error> {
    let mut path_columns = path.split('/').collect::<Vec<&str>>();
    if let Some(v) = path_columns.last() {
        if v.is_empty() {
            path_columns.pop();
        }
    }

    let user = if path_columns.last().unwrap_or(&"").eq(&"organizations") {
        let db_user = db::user::get_db_user(user_id).await;
        match db_user {
            Ok(user) => {
                let all_users = user.get_all_users();
                if all_users.is_empty() {
                    None
                } else {
                    all_users.first().cloned()
                }
            }
            Err(e) => {
                log::debug!("Error getting user in validate_credentials: {}", e);
                None
            }
        }
    } else {
        match path.find('/') {
            Some(index) => {
                // hack for v2 api for alerts and folders
                let org_id = if path_columns.len() > 2
                    && path_columns[0].eq("v2")
                    && (path_columns[2].eq("alerts") || path_columns[2].eq("folders"))
                {
                    path_columns[1]
                } else {
                    &path[0..index]
                };

                if is_root_user(user_id) {
                    users::get_user(Some(DEFAULT_ORG), user_id).await
                } else {
                    users::get_user(Some(org_id), user_id).await
                }
            }
            None => users::get_user(None, user_id).await,
        }
    };

    if user.is_none() {
        return Ok(TokenValidationResponse {
            is_valid: false,
            user_email: "".to_string(),
            is_internal_user: false,
            user_role: None,
            user_name: "".to_string(),
            family_name: "".to_string(),
            given_name: "".to_string(),
        });
    }
    let user = user.unwrap();

    #[cfg(feature = "enterprise")]
    {
        if !o2_enterprise::enterprise::common::infra::config::get_config()
            .dex
            .native_login_enabled
            && !user.is_external
        {
            return Ok(TokenValidationResponse {
                is_valid: false,
                user_email: "".to_string(),
                is_internal_user: false,
                user_role: None,
                user_name: "".to_string(),
                family_name: "".to_string(),
                given_name: "".to_string(),
            });
        }

        if o2_enterprise::enterprise::common::infra::config::get_config()
            .dex
            .root_only_login
            && !is_root_user(user_id)
        {
            return Ok(TokenValidationResponse {
                is_valid: false,
                user_email: "".to_string(),
                is_internal_user: false,
                user_role: None,
                user_name: "".to_string(),
                family_name: "".to_string(),
                given_name: "".to_string(),
            });
        }
    }

    if user.role.eq(&UserRole::ServiceAccount) && user.token.eq(&user_password) {
        return Ok(TokenValidationResponse {
            is_valid: true,
            user_email: user.email,
            is_internal_user: !user.is_external,
            user_role: Some(user.role),
            user_name: user.first_name.to_owned(),
            family_name: user.last_name,
            given_name: user.first_name,
        });
    }

    if (path_columns.len() == 1 || INGESTION_EP.iter().any(|s| path_columns.contains(s)))
        && user.token.eq(&user_password)
    {
        return Ok(TokenValidationResponse {
            is_valid: true,
            user_email: user.email,
            is_internal_user: !user.is_external,
            user_role: Some(user.role),
            user_name: user.first_name.to_owned(),
            family_name: user.last_name,
            given_name: user.first_name,
        });
    }

    let in_pass = get_hash(user_password, &user.salt);
    if !user.password.eq(&in_pass)
        && !user
            .password_ext
            .unwrap_or("".to_string())
            .eq(&user_password)
    {
        return Ok(TokenValidationResponse {
            is_valid: false,
            user_email: "".to_string(),
            is_internal_user: false,
            user_role: None,
            user_name: "".to_string(),
            family_name: "".to_string(),
            given_name: "".to_string(),
        });
    }
    if !path.contains("/user")
        || (path.contains("/user")
            && (user.role.eq(&UserRole::Admin)
                || user.role.eq(&UserRole::Root)
                || user.email.eq(user_id)))
    {
        Ok(TokenValidationResponse {
            is_valid: true,
            user_email: user.email,
            is_internal_user: !user.is_external,
            user_role: Some(user.role),
            user_name: user.first_name.to_owned(),
            family_name: user.last_name,
            given_name: user.first_name,
        })
    } else {
        Err(ErrorForbidden("Not allowed"))
    }
}

#[cfg(feature = "enterprise")]
pub async fn validate_credentials_ext(
    user_id: &str,
    in_password: &str,
    path: &str,
    auth_token: AuthTokensExt,
) -> Result<TokenValidationResponse, Error> {
    let config = get_config();
    let password_ext_salt = config.auth.ext_auth_salt.as_str();
    let mut path_columns = path.split('/').collect::<Vec<&str>>();
    if let Some(v) = path_columns.last() {
        if v.is_empty() {
            path_columns.pop();
        }
    }

    let user = if path_columns.last().unwrap_or(&"").eq(&"organizations") {
        let db_user = db::user::get_db_user(user_id).await;
        match db_user {
            Ok(user) => {
                let all_users = user.get_all_users();
                if all_users.is_empty() {
                    None
                } else {
                    all_users.first().cloned()
                }
            }
            Err(_) => None,
        }
    } else {
        match path.find('/') {
            Some(index) => {
                // hack for v2 api for alerts and folders
                let org_id = if path_columns.len() > 2
                    && path_columns[0].eq("v2")
                    && (path_columns[2].eq("alerts") || path_columns[2].eq("folders"))
                {
                    path_columns[1]
                } else {
                    &path[0..index]
                };
                if is_root_user(user_id) {
                    users::get_user(Some(DEFAULT_ORG), user_id).await
                } else {
                    users::get_user(Some(org_id), user_id).await
                }
            }
            None => users::get_user(None, user_id).await,
        }
    };

    if user.is_none() {
        return Ok(TokenValidationResponse::default());
    }
    let user = user.unwrap();

    let hashed_pass = get_hash(
        &format!(
            "{}{}",
            get_hash(
                &format!("{}{}", user.password_ext.unwrap(), auth_token.request_time),
                password_ext_salt
            ),
            auth_token.expires_in
        ),
        password_ext_salt,
    );
    if !hashed_pass.eq(&in_password) {
        return Ok(TokenValidationResponse::default());
    }
    if !path.contains("/user")
        || (path.contains("/user")
            && (user.role.eq(&UserRole::Admin)
                || user.role.eq(&UserRole::Root)
                || user.email.eq(user_id)))
    {
        Ok(TokenValidationResponse {
            is_valid: true,
            user_email: user.email,
            is_internal_user: !user.is_external,
            user_role: Some(user.role),
            user_name: user.first_name.to_owned(),
            family_name: user.last_name,
            given_name: user.first_name,
        })
    } else {
        Err(ErrorForbidden("Not allowed"))
    }
}

/// Creates the org if all the below conditions satisfied
/// - The org does not exist in the meta table
/// - The user is a root user
/// - This is a ingestion POST endpoint
async fn check_and_create_org(user_id: &str, method: &Method, path: &str) -> Result<(), Error> {
    let config = get_config();
    let path_columns = path.split('/').collect::<Vec<&str>>();
    let url_len = path_columns.len();
    if path_columns.len() < 2 {
        return Ok(());
    }
    let org_id = path_columns[0];
    if crate::service::organization::get_org(org_id)
        .await
        .is_none()
    {
        if !config.common.create_org_through_ingestion {
            Err(ErrorNotFound("Organization not found"))
        } else if is_root_user(user_id)
            && method.eq(&Method::POST)
            && INGESTION_EP.contains(&path_columns[url_len - 1])
            && crate::service::organization::check_and_create_org(org_id)
                .await
                .is_ok()
        {
            Ok(())
        } else {
            Err(ErrorNotFound("Organization not found"))
        }
    } else {
        Ok(())
    }
}

#[cfg(not(feature = "enterprise"))]
pub async fn validate_credentials_ext(
    _user_id: &str,
    _in_password: &str,
    _path: &str,
    _auth_token: AuthTokensExt,
) -> Result<TokenValidationResponse, Error> {
    Err(ErrorForbidden("Not allowed"))
}

async fn validate_user_from_db(
    db_user: Result<DBUser, anyhow::Error>,
    user_password: &str,
    req_time: Option<&String>,
    exp_in: i64,
    password_ext_salt: &str,
) -> Result<TokenValidationResponse, Error> {
    // let db_user = db::user::get_db_user(user_id).await;
    match db_user {
        Ok(mut user) => {
            let in_pass = get_hash(user_password, &user.salt);
            if req_time.is_none() && user.password.eq(&in_pass) {
                if user.password_ext.is_none() {
                    let password_ext = get_hash(user_password, password_ext_salt);
                    user.password_ext = Some(password_ext);
                    let _ = db::user::update(
                        &user.email,
                        &user.first_name,
                        &user.last_name,
                        &user.password,
                        user.password_ext.clone(),
                    )
                    .await;
                }
                let resp = TokenValidationResponseBuilder::from_db_user(&user).build();
                Ok(resp)
            } else if user.password_ext.is_some() && req_time.is_some() {
                log::debug!("Validating user for query params");
                let hashed_pass = get_hash(
                    &format!(
                        "{}{}",
                        get_hash(
                            &format!(
                                "{}{}",
                                user.password_ext.as_ref().unwrap(),
                                req_time.unwrap()
                            ),
                            password_ext_salt
                        ),
                        exp_in
                    ),
                    password_ext_salt,
                );
                if hashed_pass.eq(&user_password) {
                    let resp = TokenValidationResponseBuilder::from_db_user(&user).build();
                    return Ok(resp);
                } else {
                    Err(ErrorForbidden("Not allowed"))
                }
            } else {
                Err(ErrorForbidden("Not allowed"))
            }
        }
        Err(_) => Err(ErrorForbidden("Not allowed")),
    }
}

pub async fn validate_user(
    user_id: &str,
    user_password: &str,
) -> Result<TokenValidationResponse, Error> {
    let db_user = db::user::get_user_record(user_id)
        .await
        .map(|user| DBUser::from(&user));
    let config = get_config();
    validate_user_from_db(db_user, user_password, None, 0, &config.auth.ext_auth_salt).await
}

pub async fn validate_user_for_query_params(
    user_id: &str,
    user_password: &str,
    req_time: Option<&String>,
    exp_in: i64,
) -> Result<TokenValidationResponse, Error> {
    let db_user = db::user::get_db_user(user_id).await;
    let config = get_config();
    validate_user_from_db(
        db_user,
        user_password,
        req_time,
        exp_in,
        &config.auth.ext_auth_salt,
    )
    .await
}

pub async fn validator_aws(
    req: ServiceRequest,
    _thread_id: web::Data<usize>,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let cfg = get_config();
    let path = req
        .request()
        .path()
        .strip_prefix(format!("{}/aws/", cfg.common.base_uri).as_str())
        .unwrap_or(req.request().path());

    match req.headers().get("X-Amz-Firehose-Access-Key") {
        Some(val) => match val.to_str() {
            Ok(val) => {
                let amz_creds = match base64::decode(val) {
                    Ok(val) => val,
                    Err(_) => return Err((ErrorUnauthorized("Unauthorized Access"), req)),
                };
                let creds = amz_creds
                    .split(':')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();

                match validate_credentials(&creds[0], &creds[1], path).await {
                    Ok(res) => {
                        if res.is_valid {
                            let mut req = req;
                            req.headers_mut().insert(
                                header::HeaderName::from_static("user_id"),
                                header::HeaderValue::from_str(&res.user_email).unwrap(),
                            );
                            Ok(req)
                        } else {
                            Err((ErrorUnauthorized("Unauthorized Access"), req))
                        }
                    }
                    Err(err) => Err((err, req)),
                }
            }
            Err(_) => Err((ErrorUnauthorized("Unauthorized Access"), req)),
        },
        None => Err((ErrorUnauthorized("Unauthorized Access"), req)),
    }
}

pub async fn validator_gcp(
    req: ServiceRequest,
    _thread_id: web::Data<usize>,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let cfg = get_config();
    let path = req
        .request()
        .path()
        .strip_prefix(format!("{}/gcp/", cfg.common.base_uri).as_str())
        .unwrap_or(req.request().path());

    let query =
        web::Query::<std::collections::HashMap<String, String>>::from_query(req.query_string())
            .unwrap();
    match query.get("API-Key") {
        Some(val) => {
            let gcp_creds = base64::decode(val).unwrap();
            let creds = gcp_creds
                .split(':')
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            match validate_credentials(&creds[0], &creds[1], path).await {
                Ok(res) => {
                    if res.is_valid {
                        let mut req = req;
                        req.headers_mut().insert(
                            header::HeaderName::from_static("user_id"),
                            header::HeaderValue::from_str(&res.user_email).unwrap(),
                        );
                        Ok(req)
                    } else {
                        Err((ErrorUnauthorized("Unauthorized Access"), req))
                    }
                }
                Err(err) => Err((err, req)),
            }
        }
        None => Err((ErrorUnauthorized("Unauthorized Access"), req)),
    }
}

pub async fn validator_rum(
    req: ServiceRequest,
    _thread_id: web::Data<usize>,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let path = req
        .request()
        .path()
        .strip_prefix(format!("{}/rum/v1/", get_config().common.base_uri).as_str())
        .unwrap_or(req.request().path());

    // After this previous path clean we should get only the
    // remaining `org_id/rum` or `org_id/replay` or `org_id/logs`
    let org_id_end_point: Vec<&str> = path.split('/').collect();
    if org_id_end_point.len() != 2 {
        return Err((
            ErrorUnauthorized("Unauthorized Access. Please pass a valid org_id."),
            req,
        ));
    }

    let query =
        web::Query::<std::collections::HashMap<String, String>>::from_query(req.query_string())
            .unwrap();
    let token = query.get("oo-api-key").or_else(|| query.get("o2-api-key"));
    match token {
        Some(token) => match validate_token(token, org_id_end_point[0]).await {
            Ok(_res) => Ok(req),
            Err(err) => {
                log::error!(
                    "validate_token: Token not found for org_id: {}",
                    org_id_end_point[0]
                );
                Err((err, req))
            }
        },
        None => {
            log::error!(
                "validate_token: Missing api key for rum endpoint org_id: {}",
                org_id_end_point[0]
            );
            Err((ErrorUnauthorized("Unauthorized Access"), req))
        }
    }
}

async fn oo_validator_internal(
    req: ServiceRequest,
    auth_info: AuthExtractor,
    path_prefix: &str,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    if auth_info.auth.starts_with("Basic") {
        let decoded = match base64::decode(auth_info.auth.strip_prefix("Basic").unwrap().trim()) {
            Ok(val) => val,
            Err(_) => return Err((ErrorUnauthorized("Unauthorized Access"), req)),
        };

        let (username, password) = match get_user_details(decoded) {
            Some(value) => value,
            None => return Err((ErrorUnauthorized("Unauthorized Access"), req)),
        };
        validator(req, &username, &password, auth_info, path_prefix).await
    } else if auth_info.auth.starts_with("Bearer") {
        log::debug!("Bearer token found");
        super::token::token_validator(req, auth_info).await
    } else if auth_info.auth.starts_with("{\"auth_ext\":") {
        log::debug!("Auth ext token found");
        let auth_tokens: AuthTokensExt =
            config::utils::json::from_str(&auth_info.auth).unwrap_or_default();
        if chrono::Utc::now().timestamp() - auth_tokens.request_time > auth_tokens.expires_in {
            Err((ErrorUnauthorized("Unauthorized Access"), req))
        } else {
            log::debug!("Auth ext token found: decoding");
            let decoded = match base64::decode(
                auth_tokens
                    .auth_ext
                    .strip_prefix("auth_ext")
                    .unwrap()
                    .trim(),
            ) {
                Ok(val) => val,
                Err(_) => return Err((ErrorUnauthorized("Unauthorized Access"), req)),
            };
            let (username, password) = match get_user_details(decoded) {
                Some(value) => value,
                None => return Err((ErrorUnauthorized("Unauthorized Access"), req)),
            };
            log::info!("Auth ext token found: validating: {username} {password}");
            validator(req, &username, &password, auth_info, path_prefix).await
        }
    } else {
        Err((ErrorUnauthorized("Unauthorized Access"), req))
    }
}

#[cfg(feature = "enterprise")]
pub async fn get_user_email_from_auth_str(auth_str: &str) -> Option<String> {
    if auth_str.starts_with("Basic") {
        let decoded = match base64::decode(auth_str.strip_prefix("Basic").unwrap().trim()) {
            Ok(val) => val,
            Err(_) => return None,
        };

        match get_user_details(decoded) {
            Some(value) => Some(value.0),
            None => None,
        }
    } else if auth_str.starts_with("Bearer") {
        super::token::get_user_name_from_token(auth_str).await
    } else if auth_str.starts_with("{\"auth_ext\":") {
        let auth_tokens: AuthTokensExt =
            config::utils::json::from_str(auth_str).unwrap_or_default();
        if chrono::Utc::now().timestamp() - auth_tokens.request_time > auth_tokens.expires_in {
            None
        } else {
            let decoded = match base64::decode(
                auth_tokens
                    .auth_ext
                    .strip_prefix("auth_ext")
                    .unwrap()
                    .trim(),
            ) {
                Ok(val) => val,
                Err(_) => return None,
            };
            match get_user_details(decoded) {
                Some(value) => Some(value.0),
                None => None,
            }
        }
    } else {
        None
    }
}

fn get_user_details(decoded: String) -> Option<(String, String)> {
    let credentials = match String::from_utf8(decoded.into()).map_err(|_| ()) {
        Ok(val) => val,
        Err(_) => return None,
    };
    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    let (username, password) = (parts[0], parts[1]);
    let username = username.to_owned();
    let password = password.to_owned();
    Some((username, password))
}

/// Validates the authentication information in the incoming request and returns the request if
/// valid, or an error if invalid.
///
/// This function is responsible for validating the authentication information in the incoming
/// request. It supports both Basic and Bearer authentication (in enterprise).
/// Works exclusively on `/api` prefix
///
/// For Basic authentication, it decodes the base64-encoded credentials, splits them into username
/// and password, and calls the `validator` function to validate the credentials.
///
/// For Bearer authentication, it calls the `token_validator` function to validate the token.
///
/// If the authentication is invalid, it returns an `ErrorUnauthorized` error.
pub async fn oo_validator(
    req: ServiceRequest,
    auth_result: Result<AuthExtractor, Error>,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let path_prefix = "/api/";
    let path = extract_relative_path(req.request().path(), path_prefix);
    let path_columns = path.split('/').collect::<Vec<&str>>();
    let is_short_url = is_short_url_path(&path_columns);

    let auth_info = match auth_result {
        Ok(info) => info,
        Err(e) => {
            return if is_short_url {
                Err(handle_auth_failure_for_redirect(req, &e))
            } else {
                Err((e, req))
            };
        }
    };

    match oo_validator_internal(req, auth_info, path_prefix).await {
        Ok(service_req) => Ok(service_req),
        Err((err, err_req)) => {
            if is_short_url {
                Err(handle_auth_failure_for_redirect(err_req, &err))
            } else {
                Err((err, err_req))
            }
        }
    }
}

/// Validates the authentication information in the request and returns the request if valid, or an
/// error if invalid.
///
/// This function is a proxy for the `oo_validator_internal` function, setting the `path_prefix` to
/// "/proxy/".
///
/// # Arguments
/// * `req` - The `ServiceRequest` to validate.
/// * `auth_info` - The authentication information extracted from the request.
///
/// # Returns
/// * `Result<ServiceRequest, (Error, ServiceRequest)>` - The validated request, or an error if the
///   authentication is invalid.
pub async fn validator_proxy_url(
    req: ServiceRequest,
    auth_info: AuthExtractor,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let path_prefix = "/proxy/";
    oo_validator_internal(req, auth_info, path_prefix).await
}

#[cfg(feature = "enterprise")]
pub(crate) async fn check_permissions(
    user_id: &str,
    auth_info: AuthExtractor,
    role: UserRole,
    _is_external: bool,
) -> bool {
    use crate::common::infra::config::ORG_USERS;

    let cfg = get_config();
    if !get_o2_config().openfga.enabled {
        return true;
    }

    let object_str = auth_info.o2_type;
    log::debug!("Role of user {user_id} is {:#?}", role);
    let obj_str = if object_str.contains("##user_id##") {
        object_str.replace("##user_id##", user_id)
    } else {
        object_str
    };
    let role = if role.eq(&UserRole::Root) {
        // root user should have access to everything , bypass check in openfga
        return true;
    } else if auth_info.org_id.eq("organizations") && auth_info.method.eq("POST") {
        match ORG_USERS.get(&format!("{}/{user_id}", cfg.common.usage_org)) {
            Some(user) => format!("{}", user.role),
            None => "".to_string(),
        }
    } else {
        format!("{role}")
    };
    let org_id = if auth_info.org_id.eq("organizations") {
        if auth_info.method.eq("POST") {
            // The user is trying to create a new organization
            // Use the usage org to check for permission
            &cfg.common.usage_org
        } else {
            user_id
        }
    } else {
        &auth_info.org_id
    };

    o2_enterprise::enterprise::openfga::authorizer::authz::is_allowed(
        org_id,
        user_id,
        &auth_info.method,
        &obj_str,
        &auth_info.parent_id,
        &role.to_string(),
    )
    .await
}

#[cfg(not(feature = "enterprise"))]
pub(crate) async fn check_permissions(
    _user_id: &str,
    _auth_info: AuthExtractor,
    _role: UserRole,
    _is_external: bool,
) -> bool {
    true
}

#[cfg(feature = "enterprise")]
async fn list_objects(
    user_id: &str,
    permission: &str,
    object_type: &str,
    org_id: &str,
) -> Result<Vec<String>, anyhow::Error> {
    o2_enterprise::enterprise::openfga::authorizer::authz::list_objects(
        user_id,
        permission,
        object_type,
        org_id,
    )
    .await
}

#[cfg(feature = "enterprise")]
pub(crate) async fn list_objects_for_user(
    org_id: &str,
    user_id: &str,
    permission: &str,
    object_type: &str,
) -> Result<Option<Vec<String>>, Error> {
    let o2cfg = get_o2_config();
    if !is_root_user(user_id) && o2cfg.openfga.enabled && o2cfg.openfga.list_only_permitted {
        match crate::handler::http::auth::validator::list_objects(
            user_id,
            permission,
            object_type,
            org_id,
        )
        .await
        {
            Ok(resp) => {
                log::debug!(
                    "list_objects_for_user for user {user_id} from {org_id} org returns: {:#?}",
                    resp
                );
                Ok(Some(resp))
            }
            Err(_) => Err(ErrorForbidden("Unauthorized Access")),
        }
    } else {
        Ok(None)
    }
}

/// Helper function to extract the relative path after the base URI and path prefix
fn extract_relative_path(full_path: &str, path_prefix: &str) -> String {
    let base_uri = config::get_config().common.base_uri.clone();
    let full_prefix = format!("{}{}", base_uri, path_prefix);
    full_path
        .strip_prefix(&full_prefix)
        .unwrap_or(full_path)
        .to_string()
}

/// Helper function to check if the path corresponds to a short URL
fn is_short_url_path(path_columns: &[&str]) -> bool {
    path_columns
        .get(1)
        .map_or(false, |&segment| segment.to_lowercase() == "short")
}

/// Handles authentication failure by logging the error and returning a redirect response.
///
/// This function is responsible for logging the authentication failure and returning a redirect
/// response. It takes in the request and the error message, and returns a tuple containing the
/// redirect response and the service request.
fn handle_auth_failure_for_redirect(req: ServiceRequest, error: &Error) -> (Error, ServiceRequest) {
    let full_url = extract_full_url(&req);
    let redirect_http = RedirectResponseBuilder::default()
        .with_query_param("short_url", &full_url)
        .build();
    log::warn!(
        "Authentication failed for path: {}, err: {}, {}",
        req.path(),
        error,
        &redirect_http,
    );
    (redirect_http.into(), req)
}

/// Extracts the full URL from the request.
fn extract_full_url(req: &ServiceRequest) -> String {
    let connection_info = req.connection_info();
    let scheme = connection_info.scheme();
    let host = connection_info.host();
    let path = req
        .request()
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("");

    format!("{}://{}{}", scheme, host, path)
}

#[cfg(test)]
mod tests {
    use infra::{db as infra_db, table as infra_table};

    use super::*;
    use crate::{common::meta::user::UserRequest, service::organization};

    #[tokio::test]
    async fn test_validation_response_builder_from_db_user() {
        let user = DBUser {
            email: "test@email.com".into(),
            first_name: "first_name".into(),
            last_name: "last_name".into(),
            password: "some_pass".into(),
            salt: "some_salt".into(),
            organizations: vec![],
            is_external: false,
            password_ext: Some("some_pass_ext".into()),
        };

        let resp_from_builder = TokenValidationResponseBuilder::from_db_user(&user).build();

        let resp = TokenValidationResponse {
            is_valid: true,
            user_email: user.email,
            is_internal_user: !user.is_external,
            user_role: None,
            user_name: user.first_name.to_owned(),
            family_name: user.last_name,
            given_name: user.first_name,
        };

        assert_eq!(resp_from_builder.is_valid, resp.is_valid);
        assert!(resp_from_builder.user_email.eq(&resp.user_email));
        assert_eq!(resp_from_builder.is_internal_user, resp.is_internal_user);
        assert_eq!(resp_from_builder.user_role, resp.user_role);
        assert!(resp_from_builder.user_name.eq(&resp.user_name));
        assert!(resp_from_builder.family_name.eq(&resp.family_name));
        assert!(resp_from_builder.given_name.eq(&resp.given_name));
    }

    #[tokio::test]
    async fn test_validation_response_default() {
        let actual = TokenValidationResponse {
            is_valid: false,
            user_email: "".to_string(),
            is_internal_user: false,
            user_role: None,
            user_name: "".to_string(),
            family_name: "".to_string(),
            given_name: "".to_string(),
        };
        let expected1 = TokenValidationResponseBuilder::new().build();
        let expected2 = TokenValidationResponse::default();

        assert!(actual == expected1);
        assert!(actual == expected2);
        assert!(expected1 == expected2);
    }

    #[tokio::test]
    async fn test_validate() {
        let org_id = "default";
        let user_id = "user1@example.com";
        let init_user = "root@example.com";
        let pwd = "Complexpass#123";

        infra_db::create_table().await.unwrap();
        infra_table::create_user_tables().await.unwrap();
        organization::check_and_create_org_without_ofga(org_id)
            .await
            .unwrap();
        users::create_root_user_if_not_exists(
            org_id,
            UserRequest {
                email: init_user.to_string(),
                password: pwd.to_string(),
                role: crate::common::meta::user::UserOrgRole {
                    base_role: config::meta::user::UserRole::Root,
                    custom_role: None,
                },
                first_name: "root".to_owned(),
                last_name: "".to_owned(),
                is_external: false,
            },
        )
        .await
        .unwrap();
        users::post_user(
            org_id,
            UserRequest {
                email: user_id.to_string(),
                password: pwd.to_string(),
                role: crate::common::meta::user::UserOrgRole {
                    base_role: config::meta::user::UserRole::Admin,
                    custom_role: None,
                },
                first_name: "root".to_owned(),
                last_name: "".to_owned(),
                is_external: true,
            },
            init_user,
        )
        .await
        .unwrap();

        assert!(
            validate_credentials(init_user, pwd, "default/_bulk")
                .await
                .unwrap()
                .is_valid
        );
        assert!(
            !validate_credentials("", pwd, "default/_bulk")
                .await
                .unwrap()
                .is_valid
        );
        assert!(!validate_credentials("", pwd, "/").await.unwrap().is_valid);
        assert!(
            !validate_credentials(user_id, pwd, "/")
                .await
                .unwrap()
                .is_valid
        );
        // TODO: In these unit tests, is_root_user function does not work,
        // So, the below test case will not work, move these tests to integration tests
        // assert!(
        //     validate_credentials(user_id, pwd, "default/user")
        //         .await
        //         .unwrap()
        //         .is_valid
        // );
        assert!(
            !validate_credentials(user_id, "x", "default/user")
                .await
                .unwrap()
                .is_valid
        );
        assert!(validate_user(init_user, pwd).await.unwrap().is_valid);
    }
}
