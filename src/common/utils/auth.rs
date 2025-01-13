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

use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use base64::Engine;
use config::{
    meta::user::UserRole,
    utils::{hash::get_passcode_hash, json},
};
use futures::future::{ready, Ready};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;
use once_cell::sync::Lazy;
use regex::Regex;

#[cfg(feature = "enterprise")]
use crate::common::infra::config::USER_SESSIONS;
#[cfg(feature = "enterprise")]
use crate::common::meta::ingestion::INGESTION_EP;
use crate::common::{
    infra::config::{ORG_USERS, PASSWORD_HASH},
    meta::{
        authz::Authz,
        organization::DEFAULT_ORG,
        user::{AuthTokens, UserOrgRole},
    },
};

pub static RE_OFGA_UNSUPPORTED_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"[:#?\s'"%&]+"#).unwrap());
static RE_SPACE_AROUND: Lazy<Regex> = Lazy::new(|| {
    let char_pattern = r#"[^a-zA-Z0-9:#?'"&%\s]"#;
    let pattern = format!(r"(\s+{char_pattern}\s+)|(\s+{char_pattern})|({char_pattern}\s+)");
    Regex::new(&pattern).unwrap()
});

pub static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^([a-zA-Z0-9_+]([a-zA-Z0-9_+.-]*[a-zA-Z0-9_+])?)@([a-zA-Z0-9]+([\-\.]{1}[a-zA-Z0-9]+)*\.[a-zA-Z]{2,6})",
    )
    .unwrap()
});

pub fn is_valid_email(email: &str) -> bool {
    EMAIL_REGEX.is_match(email)
}

pub fn into_ofga_supported_format(name: &str) -> String {
    // remove spaces around special characters
    let result = RE_SPACE_AROUND.replace_all(name, |caps: &regex::Captures| {
        caps.iter()
            .find_map(|m| m)
            .map(|m| m.as_str().trim())
            .unwrap_or("")
            .to_string()
    });
    RE_OFGA_UNSUPPORTED_NAME
        .replace_all(&result, "_")
        .to_string()
}

pub fn is_ofga_unsupported(name: &str) -> bool {
    RE_OFGA_UNSUPPORTED_NAME.is_match(name)
}

pub(crate) fn get_hash(pass: &str, salt: &str) -> String {
    let key = format!("{pass}{salt}");
    let hash = PASSWORD_HASH.get(&key);
    match hash {
        Some(ret_hash) => ret_hash.value().to_string(),
        None => {
            let password_hash = get_passcode_hash(pass, salt);
            PASSWORD_HASH.insert(key, password_hash.clone());
            password_hash
        }
    }
}

// TODO
pub fn generate_invite_token() -> String {
    "".to_string()
}

pub(crate) fn is_root_user(user_id: &str) -> bool {
    match ORG_USERS.get(&format!("{DEFAULT_ORG}/{user_id}")) {
        Some(user) => user.role.eq(&UserRole::Root),
        None => false,
    }
}

#[cfg(feature = "enterprise")]
pub async fn save_org_tuples(org_id: &str) {
    use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;

    if get_o2_config().openfga.enabled {
        o2_enterprise::enterprise::openfga::authorizer::authz::save_org_tuples(org_id).await
    }
}

#[cfg(not(feature = "enterprise"))]
pub async fn save_org_tuples(_org_id: &str) {}

#[cfg(feature = "enterprise")]
pub async fn delete_org_tuples(org_id: &str) {
    use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;

    if get_o2_config().openfga.enabled {
        o2_enterprise::enterprise::openfga::authorizer::authz::delete_org_tuples(org_id).await
    }
}

#[cfg(not(feature = "enterprise"))]
pub async fn delete_org_tuples(_org_id: &str) {}

#[cfg(feature = "enterprise")]
pub fn get_role(role: &UserOrgRole) -> UserRole {
    use std::str::FromStr;

    let role = o2_enterprise::enterprise::openfga::authorizer::roles::get_role(format!(
        "{}",
        role.base_role
    ));
    UserRole::from_str(&role).unwrap()
}

#[cfg(not(feature = "enterprise"))]
pub fn get_role(_role: &UserOrgRole) -> UserRole {
    UserRole::Admin
}

#[cfg(feature = "enterprise")]
pub async fn set_ownership(org_id: &str, obj_type: &str, obj: Authz) {
    if get_o2_config().openfga.enabled {
        use o2_enterprise::enterprise::openfga::{authorizer, meta::mapping::OFGA_MODELS};

        let obj_str = format!("{}:{}", OFGA_MODELS.get(obj_type).unwrap().key, obj.obj_id);

        let parent_type = if obj.parent_type.is_empty() {
            ""
        } else {
            OFGA_MODELS.get(obj.parent_type.as_str()).unwrap().key
        };

        // Default folder is already created in case of new org, this handles the case for old org
        if obj_type.eq("folders")
            && authorizer::authz::check_folder_exists(org_id, &obj.obj_id).await
        {
            // If the folder tuples are missing, it automatically creates them
            // So we can return here
            log::debug!(
                "folder tuples already exists for org: {org_id}; folder: {}",
                &obj.obj_id
            );
            return;
        } else if obj.parent_type.eq("folders") {
            log::debug!("checking parent folder tuples for folder: {}", &obj.parent);
            // In case of dashboard, we need to check if the tuples for its folder exist
            // If not, the below function creates the proper tuples for the folder
            authorizer::authz::check_folder_exists(org_id, &obj.parent).await;
        }
        authorizer::authz::set_ownership(org_id, &obj_str, &obj.parent, parent_type).await;
    }
}
#[cfg(not(feature = "enterprise"))]
pub async fn set_ownership(_org_id: &str, _obj_type: &str, _obj: Authz) {}

#[cfg(feature = "enterprise")]
pub async fn remove_ownership(org_id: &str, obj_type: &str, obj: Authz) {
    if get_o2_config().openfga.enabled {
        use o2_enterprise::enterprise::openfga::{authorizer, meta::mapping::OFGA_MODELS};
        let obj_str = format!("{}:{}", OFGA_MODELS.get(obj_type).unwrap().key, obj.obj_id);

        let parent_type = if obj.parent_type.is_empty() {
            ""
        } else {
            OFGA_MODELS.get(obj.parent_type.as_str()).unwrap().key
        };

        authorizer::authz::remove_ownership(org_id, &obj_str, &obj.parent, parent_type).await;
    }
}
#[cfg(not(feature = "enterprise"))]
pub async fn remove_ownership(_org_id: &str, _obj_type: &str, _obj: Authz) {}

pub struct UserEmail {
    pub user_id: String,
}

impl FromRequest for UserEmail {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        if let Some(auth_header) = req.headers().get("user_id") {
            if let Ok(user_str) = auth_header.to_str() {
                return ready(Ok(UserEmail {
                    user_id: user_str.to_lowercase(),
                }));
            }
        }
        ready(Err(actix_web::error::ErrorUnauthorized("No user found")))
    }
}

#[derive(Debug)]
pub struct AuthExtractor {
    pub auth: String,
    pub method: String,
    pub o2_type: String,
    pub org_id: String,
    pub bypass_check: bool,
    pub parent_id: String,
}

impl FromRequest for AuthExtractor {
    type Error = Error;
    type Future = Ready<Result<Self, Error>>;

    #[cfg(feature = "enterprise")]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let start = std::time::Instant::now();

        use std::collections::HashMap;

        use actix_web::web;
        use config::meta::stream::StreamType;
        use o2_enterprise::enterprise::openfga::meta::mapping::OFGA_MODELS;

        use crate::common::utils::http::{get_folder, get_stream_type_from_request};

        let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
        let stream_type = match get_stream_type_from_request(&query) {
            Ok(v) => v,
            Err(_) => Some(StreamType::Logs),
        };

        let folder = get_folder(&query);

        let mut method = req.method().to_string();
        let local_path = req.path().to_string();
        let path = match local_path
            .strip_prefix(format!("{}/api/", config::get_config().common.base_uri).as_str())
        {
            Some(path) => path,
            None => &local_path,
        };

        let path_columns = path.split('/').collect::<Vec<&str>>();
        let url_len = path_columns.len();
        let org_id = path_columns[0].to_string();

        if method.eq("POST") && INGESTION_EP.contains(&path_columns[url_len - 1]) {
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    return ready(Ok(AuthExtractor {
                        auth: auth_str.to_owned(),
                        method,
                        o2_type: format!("stream:{org_id}"),
                        org_id,
                        bypass_check: true,
                        parent_id: folder,
                    }));
                }
            }
            return ready(Err(actix_web::error::ErrorUnauthorized(
                "Unauthorized Access",
            )));
        }
        let object_type = if url_len == 1 {
            if path_columns[0].eq("organizations") {
                if method.eq("GET") {
                    method = "LIST".to_string();
                };

                "org:##user_id##".to_string()
            } else {
                path_columns[0].to_string()
            }
        } else if url_len == 2 || (url_len > 2 && path_columns[1].starts_with("settings")) {
            if path_columns[1].starts_with("settings") {
                if method.eq("POST") || method.eq("DELETE") {
                    method = "PUT".to_string();
                }
            } else if method.eq("GET") {
                method = "LIST".to_string();
            }
            let key = if path_columns[1].eq("invites") {
                "users"
            } else {
                path_columns[1]
            };
            format!(
                "{}:{}",
                OFGA_MODELS.get(key).map_or(key, |model| model.key),
                path_columns[0]
            )
        } else if path_columns[1].starts_with("groups") || path_columns[1].starts_with("roles") {
            format!(
                "{}:{org_id}/{}",
                OFGA_MODELS
                    .get(path_columns[1])
                    .map_or(path_columns[1], |model| model.key),
                path_columns[2]
            )
        } else if url_len == 3 {
            if path_columns[2].starts_with("alerts")
                || path_columns[2].starts_with("templates")
                || path_columns[2].starts_with("destinations")
                || path.ends_with("users/roles")
            {
                if method.eq("GET") {
                    method = "LIST".to_string();
                }
                if method.eq("PUT") || method.eq("DELETE") || path_columns[1].eq("search_jobs") {
                    format!(
                        "{}:{}",
                        OFGA_MODELS
                            .get(path_columns[1])
                            .map_or(path_columns[1], |model| model.key),
                        path_columns[2]
                    )
                } else {
                    format!(
                        "{}:{}",
                        OFGA_MODELS
                            .get(path_columns[2])
                            .map_or(path_columns[2], |model| model.key),
                        path_columns[0]
                    )
                }
            } else if path_columns[2].starts_with("_values")
                || path_columns[2].starts_with("_around")
            {
                format!(
                    "{}:{}",
                    OFGA_MODELS.get("streams").unwrap().key,
                    path_columns[1]
                )
            } else if path_columns[1].starts_with("rename") {
                // Org rename
                format!(
                    "{}:{}",
                    OFGA_MODELS.get("organizations").unwrap().key,
                    org_id
                )
            } else if method.eq("PUT")
                || method.eq("DELETE")
                || path_columns[1].starts_with("reports")
                || path_columns[1].starts_with("savedviews")
                || path_columns[1].starts_with("functions")
                || path_columns[1].starts_with("service_accounts")
            {
                format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(path_columns[1])
                        .map_or(path_columns[1], |model| model.key),
                    path_columns[2]
                )
            } else if method.eq("GET") && path_columns[1].starts_with("dashboards") {
                format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(path_columns[1])
                        .map_or(path_columns[1], |model| model.key),
                    path_columns[2] // dashboard id
                )
            } else {
                format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(path_columns[1])
                        .map_or(path_columns[1], |model| model.key),
                    path_columns[0]
                )
            }
        } else if url_len == 4 {
            if method.eq("PUT") && path_columns[1].eq("reports") {
                format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(path_columns[1])
                        .map_or(path_columns[1], |model| model.key),
                    path_columns[2]
                )
            } else if method.eq("PUT")
                && path_columns[1] != "streams"
                && path_columns[1] != "pipelines"
                || method.eq("DELETE")
            {
                format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(path_columns[2])
                        .map_or(path_columns[2], |model| model.key),
                    path_columns[3]
                )
            } else {
                format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(path_columns[1])
                        .map_or(path_columns[1], |model| model.key),
                    path_columns[2]
                )
            }
        } else if method.eq("PUT") || method.eq("DELETE") {
            if path_columns[url_len - 1].eq("delete_fields") {
                method = "DELETE".to_string();
            }
            if path_columns[url_len - 1].eq("enable") {
                format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(path_columns[2])
                        .map_or(path_columns[2], |model| model.key),
                    path_columns[3]
                )
            } else {
                format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(path_columns[1])
                        .map_or(path_columns[1], |model| model.key),
                    path_columns[2]
                )
            }
        } else {
            format!(
                "{}:{}",
                OFGA_MODELS
                    .get(path_columns[1])
                    .map_or(path_columns[1], |model| model.key),
                path_columns[2]
            )
        };

        let auth_str = extract_auth_str(req);

        // if let Some(auth_header) = req.headers().get("Authorization") {
        if !auth_str.is_empty() {
            if (method.eq("POST")
                && path_columns.len() > 1
                && path_columns[1].starts_with("_search"))
                || path.contains("/prometheus/api/v1/query")
                || path.contains("/resources")
                || path.contains("/format_query")
                || path.contains("/prometheus/api/v1/series")
                || path.contains("/traces/latest")
                || path.contains("clusters")
                || path.contains("query_manager")
                || path.contains("/short")
                || path.contains("/ws")
            {
                return ready(Ok(AuthExtractor {
                    auth: auth_str.to_owned(),
                    method: "".to_string(),
                    o2_type: "".to_string(),
                    org_id: "".to_string(),
                    bypass_check: true, // bypass check permissions
                    parent_id: folder,
                }));
            }
            if object_type.starts_with("stream") {
                let object_type = match stream_type {
                    Some(stream_type) => {
                        if stream_type.eq(&StreamType::EnrichmentTables) {
                            // since enrichment tables have separate permissions
                            let stream_type_str = format!("{stream_type}");

                            object_type.replace(
                                "stream:",
                                format!(
                                    "{}:",
                                    OFGA_MODELS
                                        .get(stream_type_str.as_str())
                                        .map_or(stream_type_str.as_str(), |model| model.key)
                                )
                                .as_str(),
                            )
                        } else {
                            object_type.replace("stream:", format!("{}:", stream_type).as_str())
                        }
                    }
                    None => object_type,
                };
                return ready(Ok(AuthExtractor {
                    auth: auth_str.to_owned(),
                    method,
                    o2_type: object_type,
                    org_id,
                    bypass_check: false,
                    parent_id: folder,
                }));
            }
            if object_type.contains("dashboard") && url_len > 1 {
                let object_type = if method.eq("POST") || method.eq("LIST") {
                    format!(
                        "{}:{}",
                        OFGA_MODELS
                            .get(path_columns[1])
                            .map_or("dfolder", |model| model.parent),
                        folder.as_str(),
                    )
                } else {
                    object_type
                };

                return ready(Ok(AuthExtractor {
                    auth: auth_str.to_owned(),
                    method,
                    o2_type: object_type,
                    org_id,
                    bypass_check: false,
                    parent_id: folder,
                }));
            }

            return ready(Ok(AuthExtractor {
                auth: auth_str.to_owned(),
                method,
                o2_type: object_type,
                org_id,
                bypass_check: false,
                parent_id: folder,
            }));
        }
        //}
        log::info!(
            "AuthExtractor::from_request took {} ms",
            start.elapsed().as_millis()
        );
        ready(Err(actix_web::error::ErrorUnauthorized(
            "Unauthorized Access",
        )))
    }

    #[cfg(not(feature = "enterprise"))]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let auth_str = if let Some(cookie) = req.cookie("auth_tokens") {
            let auth_tokens: AuthTokens = json::from_str(cookie.value()).unwrap_or_default();
            let access_token = auth_tokens.access_token;
            if access_token.starts_with("Basic") || access_token.starts_with("Bearer") {
                access_token
            } else {
                format!("Bearer {}", access_token)
            }
        } else if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                auth_str.to_owned()
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        // if let Some(auth_header) = req.headers().get("Authorization") {
        if !auth_str.is_empty() {
            return ready(Ok(AuthExtractor {
                auth: auth_str.to_owned(),
                method: "".to_string(),
                o2_type: "".to_string(),
                org_id: "".to_string(),
                bypass_check: true, // bypass check permissions
                parent_id: "".to_string(),
            }));
        }

        ready(Err(actix_web::error::ErrorUnauthorized(
            "Unauthorized Access",
        )))
    }
}

#[cfg(feature = "enterprise")]
pub fn extract_auth_str(req: &HttpRequest) -> String {
    let auth_ext_cookie = |req: &HttpRequest| -> String {
        req.cookie("auth_ext")
            .map(|cookie| cookie.value().to_string())
            .unwrap_or_default()
    };

    if let Some(cookie) = req.cookie("auth_tokens") {
        let auth_tokens: AuthTokens = json::from_str(cookie.value()).unwrap_or_default();
        let access_token = auth_tokens.access_token;
        if access_token.is_empty() {
            // If cookie was set but access token is still empty
            // we check auth_ext cookie to get the token.
            auth_ext_cookie(req)
        } else if access_token.starts_with("Basic") || access_token.starts_with("Bearer") {
            access_token
        } else if access_token.starts_with("session") {
            let session_key = access_token.strip_prefix("session ").unwrap().to_string();
            match USER_SESSIONS.get(&session_key) {
                Some(token) => {
                    format!("Bearer {}", *token)
                }
                None => access_token,
            }
        } else {
            format!("Bearer {}", access_token)
        }
    } else if let Some(cookie) = req.cookie("auth_ext") {
        cookie.value().to_string()
    } else if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            auth_str.to_owned()
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    }
}

/// Constructs the login URL with the provided parameters.
///
/// # Arguments
///
/// * `base_url` - The base URL of the authentication service.
/// * `time` - The request time.
/// * `exp_in` - The expiration time.
/// * `auth` - The authentication token.
///
/// # Returns
///
/// The constructed login URL.
pub fn generate_presigned_url(
    username: &str,
    password: &str,
    salt: &str,
    base_url: &str,
    exp_in: i64,
    time: i64,
) -> String {
    // let time = chrono::Utc::now().timestamp();
    let stage1 = get_hash(password, salt);
    let stage2 = get_hash(&format!("{}{}", &stage1, time), salt);
    let stage3 = get_hash(&format!("{}{}", &stage2, exp_in), salt);

    let user_pass = format!("{}:{}", username, stage3);
    let auth = base64::engine::general_purpose::STANDARD.encode(user_pass);

    format!(
        "{}/auth/login?request_time={}&exp_in={}&auth={}",
        base_url, time, exp_in, auth
    )
}

#[cfg(test)]
mod tests {
    use infra::{db as infra_db, table as infra_table};

    use super::*;
    use crate::{
        common::meta::user::UserRequest,
        service::{self, organization, users},
    };

    #[test]
    fn test_generate_presigned_url() {
        let password = "password";
        let salt = "saltsalt";
        let username = "user";
        let base_url = "https://example.com";
        let exp_in = 3600;
        let time = 1634567890;

        let expected_url = format!(
            "{}/auth/login?request_time={}&exp_in={}&auth={}",
            base_url,
            time,
            exp_in,
            "dXNlcjokYXJnb24yZCR2PTE2JG09MjA0OCx0PTQscD0yJGMyRnNkSE5oYkhRJGNwTElHZzdEaFl1Vi9nSWxMaCtRZksrS29Vd2ZFaGVpdHkwc3Z0c243Y1E="
        );

        let generated_url =
            generate_presigned_url(username, password, salt, base_url, exp_in, time);

        assert_eq!(generated_url, expected_url);
    }

    #[tokio::test]
    async fn test_is_root_user() {
        assert!(!is_root_user("dummy"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_is_root_user2() {
        infra_db::create_table().await.unwrap();
        infra_table::create_user_tables().await.unwrap();
        organization::check_and_create_org_without_ofga(DEFAULT_ORG)
            .await
            .unwrap();
        let _ = users::create_root_user_if_not_exists(
            DEFAULT_ORG,
            UserRequest {
                email: "root@example.com".to_string(),
                password: "Complexpass#123".to_string(),
                role: UserOrgRole {
                    base_role: config::meta::user::UserRole::Root,
                    custom_role: None,
                },
                first_name: "root".to_owned(),
                last_name: "".to_owned(),
                is_external: false,
            },
        )
        .await;
        service::db::user::cache().await.unwrap();
        service::db::organization::cache().await.unwrap();
        service::db::org_users::cache().await.unwrap();
        assert!(is_root_user("root@example.com"));
        assert!(!is_root_user("root2@example.com"));
    }

    #[tokio::test]
    async fn test_get_hash() {
        let hash =
            "$argon2d$v=16$m=2048,t=4,p=2$VGVzdFNhbHQ$CZzrFPtqjY4mIPYwoDztCJ3OGD5M0P37GH4QddwrbZk";
        assert_eq!(get_hash("Pass#123", "TestSalt"), hash);
    }

    #[tokio::test]
    async fn test_get_hash_for_pass() {
        let pass1 = get_hash("Pass#123", "openobserve");
        let time = chrono::Utc::now().timestamp();
        let pass2 = get_hash(&format!("{}{}", &pass1, time), "openobserve");
        let exp_in = 600;
        let pass3 = get_hash(&format!("{}{}", &pass2, exp_in), "openobserve");
        println!("time: {}", time);
        println!("pass3: {}", pass3);

        let user_pass = format!("{}:{}", "b@b.com", pass3);
        let auth = base64::engine::general_purpose::STANDARD.encode(user_pass);
        println!(
            "http://localhost:5080/auth/login?request_time={}&exp_in={}&auth={}",
            time, exp_in, auth
        );
    }
}
