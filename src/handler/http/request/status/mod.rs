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

use std::{io::Error, sync::Arc};

use actix_web::{
    cookie,
    cookie::{Cookie, SameSite},
    get, head,
    http::header,
    put, web, HttpRequest, HttpResponse,
};
use arrow_schema::Schema;
use config::{
    cluster::LOCAL_NODE,
    get_config, get_instance_id,
    meta::{cluster::NodeStatus, function::ZoFunction},
    utils::{json, schema_ext::SchemaExt},
    Config, QUICK_MODEL_FIELDS, SQL_FULL_TEXT_SEARCH_FIELDS, TIMESTAMP_COL_NAME,
};
use hashbrown::HashMap;
use infra::{
    cache::{self, file_data::disk::FileType},
    file_list,
    schema::{STREAM_SCHEMAS, STREAM_SCHEMAS_COMPRESSED, STREAM_SCHEMAS_LATEST},
};
use serde::Serialize;
use utoipa::ToSchema;
#[cfg(feature = "enterprise")]
use {
    crate::common::utils::{auth::extract_auth_str, jwt::verify_decode_token},
    crate::handler::http::auth::{
        jwt::process_token,
        validator::{get_user_email_from_auth_str, ID_TOKEN_HEADER, PKCE_STATE_ORG},
    },
    crate::service::self_reporting::audit,
    config::{ider, utils::base64},
    o2_dex::{
        config::{get_config as get_dex_config, refresh_config as refresh_dex_config},
        service::auth::{exchange_code, get_dex_jwks, get_dex_login, refresh_token},
    },
    o2_enterprise::enterprise::common::{
        auditor::{AuditMessage, HttpMeta, Protocol},
        infra::config::{get_config as get_o2_config, refresh_config as refresh_o2_config},
        settings::{get_logo, get_logo_text},
    },
    o2_openfga::config::{
        get_config as get_openfga_config, refresh_config as refresh_openfga_config,
    },
    std::io::ErrorKind,
};

use crate::{
    common::{
        infra::{cluster, config::*},
        meta::{
            http::HttpResponse as MetaHttpResponse,
            user::{AuthTokens, AuthTokensExt},
        },
    },
    service::{
        db,
        search::{
            datafusion::{storage::file_statistics_cache, udf::DEFAULT_FUNCTIONS},
            tantivy::puffin_directory::reader_cache,
        },
    },
};

#[derive(Serialize, ToSchema)]
pub struct HealthzResponse {
    status: String,
}

#[derive(Serialize)]
struct ConfigResponse<'a> {
    version: String,
    instance: String,
    commit_hash: String,
    build_date: String,
    build_type: String,
    default_fts_keys: Vec<String>,
    default_quick_mode_fields: Vec<String>,
    telemetry_enabled: bool,
    default_functions: Vec<ZoFunction<'a>>,
    sql_base64_enabled: bool,
    timestamp_column: String,
    syslog_enabled: bool,
    data_retention_days: i64,
    extended_data_retention_days: i64,
    restricted_routes_on_empty_data: bool,
    sso_enabled: bool,
    native_login_enabled: bool,
    rbac_enabled: bool,
    super_cluster_enabled: bool,
    query_on_stream_selection: bool,
    show_stream_stats_doc_num: bool,
    custom_logo_text: String,
    custom_slack_url: String,
    custom_docs_url: String,
    rum: Rum,
    custom_logo_img: Option<String>,
    custom_hide_menus: String,
    custom_hide_self_logo: bool,
    meta_org: String,
    quick_mode_enabled: bool,
    user_defined_schemas_enabled: bool,
    user_defined_schema_max_fields: usize,
    all_fields_name: String,
    usage_enabled: bool,
    usage_publish_interval: i64,
    websocket_enabled: bool,
    min_auto_refresh_interval: u32,
    query_default_limit: i64,
}

#[derive(Serialize)]
struct Rum {
    pub enabled: bool,
    pub client_token: String,
    pub application_id: String,
    pub site: String,
    pub service: String,
    pub env: String,
    pub version: String,
    pub organization_identifier: String,
    pub api_version: String,
    pub insecure_http: bool,
}

/// Healthz
#[utoipa::path(
    path = "/healthz",
    tag = "Meta",
    responses(
        (status = 200, description="Status OK", content_type = "application/json", body = HealthzResponse, example = json!({"status": "ok"}))
    )
)]
#[get("/healthz")]
pub async fn healthz() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(HealthzResponse {
        status: "ok".to_string(),
    }))
}

/// Healthz HEAD
/// Vector pipeline healthcheck support
#[head("/healthz")]
pub async fn healthz_head() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
}

/// Healthz of the node for scheduled status
#[utoipa::path(
    path = "/schedulez",
    tag = "Meta",
    responses(
        (status = 200, description="Status OK", content_type = "application/json", body = HealthzResponse, example = json!({"status": "ok"})),
        (status = 404, description="Status Not OK", content_type = "application/json", body = HealthzResponse, example = json!({"status": "not ok"})),
    )
)]
#[get("/schedulez")]
pub async fn schedulez() -> Result<HttpResponse, Error> {
    let node_id = LOCAL_NODE.uuid.clone();
    let Some(node) = cluster::get_node_by_uuid(&node_id).await else {
        return Ok(HttpResponse::NotFound().json(HealthzResponse {
            status: "not ok".to_string(),
        }));
    };
    Ok(if node.scheduled && node.status == NodeStatus::Online {
        HttpResponse::Ok().json(HealthzResponse {
            status: "ok".to_string(),
        })
    } else {
        HttpResponse::NotFound().json(HealthzResponse {
            status: "not ok".to_string(),
        })
    })
}

#[get("")]
pub async fn zo_config() -> Result<HttpResponse, Error> {
    #[cfg(feature = "enterprise")]
    let o2cfg = get_o2_config();
    #[cfg(feature = "enterprise")]
    let dex_cfg = get_dex_config();
    #[cfg(feature = "enterprise")]
    let openfga_cfg = get_openfga_config();
    #[cfg(feature = "enterprise")]
    let sso_enabled = dex_cfg.dex_enabled;
    #[cfg(not(feature = "enterprise"))]
    let sso_enabled = false;
    #[cfg(feature = "enterprise")]
    let native_login_enabled = dex_cfg.native_login_enabled;
    #[cfg(not(feature = "enterprise"))]
    let native_login_enabled = true;

    #[cfg(feature = "enterprise")]
    let rbac_enabled = openfga_cfg.enabled;
    #[cfg(not(feature = "enterprise"))]
    let rbac_enabled = false;

    #[cfg(feature = "enterprise")]
    let super_cluster_enabled = o2cfg.super_cluster.enabled;
    #[cfg(not(feature = "enterprise"))]
    let super_cluster_enabled = false;

    #[cfg(feature = "enterprise")]
    let custom_logo_text = match get_logo_text().await {
        Some(data) => data,
        None => o2cfg.common.custom_logo_text.clone(),
    };
    #[cfg(not(feature = "enterprise"))]
    let custom_logo_text = "".to_string();
    #[cfg(feature = "enterprise")]
    let custom_slack_url = &o2cfg.common.custom_slack_url;
    #[cfg(not(feature = "enterprise"))]
    let custom_slack_url = "";
    #[cfg(feature = "enterprise")]
    let custom_docs_url = &o2cfg.common.custom_docs_url;
    #[cfg(not(feature = "enterprise"))]
    let custom_docs_url = "";

    #[cfg(feature = "enterprise")]
    let logo = get_logo().await;

    #[cfg(not(feature = "enterprise"))]
    let logo = None;

    #[cfg(feature = "enterprise")]
    let custom_hide_menus = &o2cfg.common.custom_hide_menus;
    #[cfg(not(feature = "enterprise"))]
    let custom_hide_menus = "";

    #[cfg(feature = "enterprise")]
    let custom_hide_self_logo = o2cfg.common.custom_hide_self_logo;
    #[cfg(not(feature = "enterprise"))]
    let custom_hide_self_logo = false;

    #[cfg(feature = "enterprise")]
    let build_type = "enterprise";
    #[cfg(not(feature = "enterprise"))]
    let build_type = "opensource";
    let cfg = get_config();
    Ok(HttpResponse::Ok().json(ConfigResponse {
        version: VERSION.to_string(),
        instance: get_instance_id(),
        commit_hash: COMMIT_HASH.to_string(),
        build_date: BUILD_DATE.to_string(),
        build_type: build_type.to_string(),
        telemetry_enabled: cfg.common.telemetry_enabled,
        default_fts_keys: SQL_FULL_TEXT_SEARCH_FIELDS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        default_quick_mode_fields: QUICK_MODEL_FIELDS.to_vec(),
        default_functions: DEFAULT_FUNCTIONS.to_vec(),
        sql_base64_enabled: cfg.common.ui_sql_base64_enabled,
        timestamp_column: TIMESTAMP_COL_NAME.to_string(),
        syslog_enabled: *SYSLOG_ENABLED.read(),
        data_retention_days: cfg.compact.data_retention_days,
        extended_data_retention_days: cfg.compact.extended_data_retention_days,
        restricted_routes_on_empty_data: cfg.common.restricted_routes_on_empty_data,
        sso_enabled,
        native_login_enabled,
        rbac_enabled,
        super_cluster_enabled,
        query_on_stream_selection: cfg.common.query_on_stream_selection,
        show_stream_stats_doc_num: cfg.common.show_stream_dates_doc_num,
        custom_logo_text,
        custom_slack_url: custom_slack_url.to_string(),
        custom_docs_url: custom_docs_url.to_string(),
        custom_logo_img: logo,
        custom_hide_menus: custom_hide_menus.to_string(),
        custom_hide_self_logo,
        rum: Rum {
            enabled: cfg.rum.enabled,
            client_token: cfg.rum.client_token.to_string(),
            application_id: cfg.rum.application_id.to_string(),
            site: cfg.rum.site.to_string(),
            service: cfg.rum.service.to_string(),
            env: cfg.rum.env.to_string(),
            version: cfg.rum.version.to_string(),
            organization_identifier: cfg.rum.organization_identifier.to_string(),
            api_version: cfg.rum.api_version.to_string(),
            insecure_http: cfg.rum.insecure_http,
        },
        meta_org: cfg.common.usage_org.to_string(),
        quick_mode_enabled: cfg.limit.quick_mode_enabled,
        user_defined_schemas_enabled: cfg.common.allow_user_defined_schemas,
        user_defined_schema_max_fields: cfg.limit.user_defined_schema_max_fields,
        all_fields_name: cfg.common.column_all.to_string(),
        usage_enabled: cfg.common.usage_enabled,
        usage_publish_interval: cfg.common.usage_publish_interval,
        websocket_enabled: cfg.websocket.enabled,
        min_auto_refresh_interval: cfg.common.min_auto_refresh_interval,
        query_default_limit: cfg.limit.query_default_limit,
    }))
}

#[get("/status")]
pub async fn cache_status() -> Result<HttpResponse, Error> {
    let cfg = get_config();
    let mut stats: HashMap<&str, json::Value> = HashMap::default();
    stats.insert("LOCAL_NODE_UUID", json::json!(LOCAL_NODE.uuid.clone()));
    stats.insert("LOCAL_NODE_NAME", json::json!(&cfg.common.instance_name));
    stats.insert("LOCAL_NODE_ROLE", json::json!(&cfg.common.node_role));
    let nodes = cluster::get_cached_online_nodes().await;
    stats.insert("NODE_LIST", json::json!(nodes));

    let (stream_num, stream_schema_num, mem_size) = get_stream_schema_status().await;
    stats.insert("STREAM_SCHEMA", json::json!({"stream_num": stream_num,"stream_schema_num": stream_schema_num, "mem_size": mem_size}));

    let stream_num = cache::stats::get_stream_stats_len();
    let mem_size = cache::stats::get_stream_stats_in_memory_size();
    stats.insert(
        "STREAM_STATS",
        json::json!({"stream_num": stream_num, "mem_size": mem_size}),
    );

    let mem_file_num = cache::file_data::memory::len().await;
    let (mem_max_size, mem_cur_size) = cache::file_data::memory::stats().await;
    let disk_file_num = cache::file_data::disk::len(FileType::DATA).await;
    let (disk_max_size, disk_cur_size) = cache::file_data::disk::stats(FileType::DATA).await;
    let disk_result_file_num = cache::file_data::disk::len(FileType::RESULT).await;
    let (disk_result_max_size, disk_result_cur_size) =
        cache::file_data::disk::stats(FileType::RESULT).await;
    stats.insert(
        "FILE_DATA",
        json::json!({
            "memory":{"cache_files":mem_file_num, "cache_limit":mem_max_size,"cache_bytes": mem_cur_size},
            "disk":{"cache_files":disk_file_num, "cache_limit":disk_max_size,"cache_bytes": disk_cur_size},
            "results":{"cache_files":disk_result_file_num, "cache_limit":disk_result_max_size,"cache_bytes": disk_result_cur_size}
        }),
    );

    let file_list_num = file_list::len().await;
    let file_list_max_id = file_list::get_max_pk_value().await.unwrap_or_default();
    stats.insert(
        "FILE_LIST",
        json::json!({"num":file_list_num,"max_id": file_list_max_id}),
    );

    let tmpfs_mem_size = cache::tmpfs::stats().unwrap();
    stats.insert("TMPFS", json::json!({ "mem_size": tmpfs_mem_size }));

    let last_file_list_offset = db::compact::file_list::get_offset().await.unwrap();
    stats.insert(
        "COMPACT",
        json::json!({"file_list_offset": last_file_list_offset}),
    );
    stats.insert(
        "DATAFUSION",
        json::json!({"file_stat_cache": file_statistics_cache::GLOBAL_CACHE.clone().len()}),
    );
    stats.insert(
        "INVERTED_INDEX",
        json::json!({"reader_cache": reader_cache::GLOBAL_CACHE.clone().len()}),
    );

    let consistent_hashing = cluster::print_consistent_hash().await;
    stats.insert("CONSISTENT_HASHING", json::json!(consistent_hashing));

    Ok(HttpResponse::Ok().json(stats))
}

#[get("")]
pub async fn config_reload() -> Result<HttpResponse, Error> {
    if let Err(e) = config::refresh_config() {
        return Ok(
            HttpResponse::InternalServerError().json(serde_json::json!({"status": e.to_string()}))
        );
    }
    #[cfg(feature = "enterprise")]
    if let Err(e) = refresh_o2_config()
        .and_then(|_| refresh_dex_config())
        .and_then(|_| refresh_openfga_config())
    {
        return Ok(
            HttpResponse::InternalServerError().json(serde_json::json!({"status": e.to_string()}))
        );
    }
    let status = "successfully reloaded config";
    // Audit this event
    #[cfg(feature = "enterprise")]
    audit(AuditMessage {
        // Since this is not a protected route, there is no way to get the user email
        user_email: "".to_string(),
        org_id: "".to_string(),
        _timestamp: chrono::Utc::now().timestamp_micros(),
        protocol: Protocol::Http(HttpMeta {
            method: "GET".to_string(),
            path: "/config/reload".to_string(),
            query_params: "".to_string(),
            body: "".to_string(),
            response_code: 200,
        }),
    })
    .await;
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": status})))
}

async fn get_stream_schema_status() -> (usize, usize, usize) {
    let mut stream_num = 0;
    let mut stream_schema_num = 0;
    let mut mem_size = 0;
    let r = STREAM_SCHEMAS.read().await;
    for (key, val) in r.iter() {
        stream_num += 1;
        mem_size += std::mem::size_of::<Vec<Schema>>();
        mem_size += key.len();
        for schema in val.iter() {
            stream_schema_num += 1;
            mem_size += schema.1.size();
        }
    }
    drop(r);
    let r = STREAM_SCHEMAS_COMPRESSED.read().await;
    for (key, val) in r.iter() {
        stream_num += 1;
        mem_size += key.len();
        mem_size += val.len();
    }
    drop(r);
    let r = STREAM_SCHEMAS_LATEST.read().await;
    for (key, schema) in r.iter() {
        stream_num += 1;
        stream_schema_num += 1;
        mem_size += key.len();
        mem_size += schema.schema().size();
    }
    drop(r);
    (stream_num, stream_schema_num, mem_size)
}

#[cfg(feature = "enterprise")]
#[get("/redirect")]
pub async fn redirect(req: HttpRequest) -> Result<HttpResponse, Error> {
    use crate::common::meta::user::{AuthTokens, UserRole};

    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let code = match query.get("code") {
        Some(code) => code,
        None => {
            return Err(Error::new(ErrorKind::Other, "no code in request"));
        }
    };
    let mut audit_message = AuditMessage {
        user_email: "".to_string(),
        org_id: "".to_string(),
        _timestamp: chrono::Utc::now().timestamp_micros(),
        protocol: Protocol::Http(HttpMeta {
            method: "GET".to_string(),
            path: "/config/redirect".to_string(),
            body: "".to_string(),
            query_params: req.query_string().to_string(),
            response_code: 302,
        }),
    };

    match query.get("state") {
        Some(code) => match crate::service::kv::get(PKCE_STATE_ORG, code).await {
            Ok(_) => {
                let _ = crate::service::kv::delete(PKCE_STATE_ORG, code).await;
            }
            Err(_) => {
                // Bad Request
                if let Protocol::Http(ref mut http_meta) = audit_message.protocol {
                    http_meta.response_code = 400;
                }
                audit(audit_message).await;
                return Err(Error::new(ErrorKind::Other, "invalid state in request"));
            }
        },

        None => {
            // Bad Request
            if let Protocol::Http(ref mut http_meta) = audit_message.protocol {
                http_meta.response_code = 400;
            }
            audit(audit_message).await;
            return Err(Error::new(ErrorKind::Other, "no state in request"));
        }
    };

    log::info!("entering exchange_code: {}", code);

    match exchange_code(code).await {
        Ok(login_data) => {
            let login_url;
            let access_token = login_data.access_token;
            let keys = get_dex_jwks().await;
            let token_ver = verify_decode_token(
                &access_token,
                &keys,
                &get_dex_config().client_id,
                true,
                true,
            )
            .await;
            let id_token;
            match token_ver {
                Ok(res) => {
                    // check for service accounts , do not to allow login
                    if let Some(db_user) = db::user::get_user_by_email(&res.0.user_email).await {
                        if db_user
                            .organizations
                            .iter()
                            .any(|org| org.role.eq(&UserRole::ServiceAccount))
                        {
                            return Ok(HttpResponse::Unauthorized()
                                .json("Service accounts are not allowed to login".to_string()));
                        }
                    }

                    audit_message.user_email = res.0.user_email.clone();
                    id_token = json::to_string(&json::json!({
                        "email": res.0.user_email,
                        "name": res.0.user_name,
                        "family_name": res.0.family_name,
                        "given_name": res.0.given_name,
                        "is_valid": res.0.is_valid,
                    }))
                    .unwrap();
                    login_url = format!(
                        "{}#id_token={}.{}",
                        login_data.url,
                        ID_TOKEN_HEADER,
                        base64::encode(&id_token)
                    );
                    process_token(res).await
                }
                Err(e) => {
                    if let Protocol::Http(ref mut http_meta) = audit_message.protocol {
                        http_meta.response_code = 400;
                    }
                    audit_message._timestamp = chrono::Utc::now().timestamp_micros();
                    audit(audit_message).await;
                    return Ok(HttpResponse::Unauthorized().json(e.to_string()));
                }
            }

            // generate new UUID for access token & store token in DB
            let session_id = ider::uuid();

            // store session_id in cluster co-ordinator
            let _ = crate::service::session::set_session(&session_id, &access_token).await;

            let access_token = format!("session {}", session_id);

            let tokens = json::to_string(&AuthTokens {
                access_token,
                refresh_token: login_data.refresh_token,
            })
            .unwrap();
            let cfg = get_config();
            let mut auth_cookie = Cookie::new("auth_tokens", tokens);
            auth_cookie.set_expires(
                cookie::time::OffsetDateTime::now_utc()
                    + cookie::time::Duration::seconds(cfg.auth.cookie_max_age),
            );
            auth_cookie.set_http_only(true);
            auth_cookie.set_secure(cfg.auth.cookie_secure_only);
            auth_cookie.set_path("/");
            if cfg.auth.cookie_same_site_lax {
                auth_cookie.set_same_site(SameSite::Lax);
            } else {
                auth_cookie.set_same_site(SameSite::None);
            }
            log::info!("Redirecting user after processing token");

            audit_message._timestamp = chrono::Utc::now().timestamp_micros();
            audit(audit_message).await;
            Ok(HttpResponse::Found()
                .append_header((header::LOCATION, login_url))
                .cookie(auth_cookie)
                .finish())
        }
        Err(e) => {
            if let Protocol::Http(ref mut http_meta) = audit_message.protocol {
                http_meta.response_code = 400;
            }
            audit_message._timestamp = chrono::Utc::now().timestamp_micros();
            audit(audit_message).await;
            Ok(HttpResponse::Unauthorized().json(e.to_string()))
        }
    }
}

#[cfg(feature = "enterprise")]
#[get("/dex_login")]
pub async fn dex_login() -> Result<HttpResponse, Error> {
    use o2_dex::meta::auth::PreLoginData;

    let login_data: PreLoginData = get_dex_login();
    let state = login_data.state;
    let _ = crate::service::kv::set(PKCE_STATE_ORG, &state, state.to_owned().into()).await;

    Ok(HttpResponse::Ok().json(login_data.url))
}

#[cfg(feature = "enterprise")]
#[get("/dex_refresh")]
async fn refresh_token_with_dex(req: actix_web::HttpRequest) -> HttpResponse {
    let token = if let Some(cookie) = req.cookie("auth_tokens") {
        let auth_tokens: AuthTokens = json::from_str(cookie.value()).unwrap_or_default();

        // remove old session id from cluster co-ordinator

        let access_token = auth_tokens.access_token;
        if access_token.starts_with("session") {
            crate::service::session::remove_session(access_token.strip_prefix("session ").unwrap())
                .await;
        }

        auth_tokens.refresh_token
    } else {
        return HttpResponse::Unauthorized().finish();
    };

    // Exchange the refresh token for a new access token
    match refresh_token(&token).await {
        Ok((access_token, refresh_token)) => {
            // generate new UUID for access token & store token in DB
            let session_id = ider::uuid();

            // store session_id in cluster co-ordinator
            let _ = crate::service::session::set_session(&session_id, &access_token).await;

            let access_token = format!("session {}", session_id);

            let tokens = json::to_string(&AuthTokens {
                access_token,
                refresh_token,
            })
            .unwrap();
            let conf = get_config();
            let mut auth_cookie = Cookie::new("auth_tokens", tokens);
            auth_cookie.set_expires(
                cookie::time::OffsetDateTime::now_utc()
                    + cookie::time::Duration::seconds(conf.auth.cookie_max_age),
            );
            auth_cookie.set_http_only(true);
            auth_cookie.set_secure(conf.auth.cookie_secure_only);
            auth_cookie.set_path("/");
            if conf.auth.cookie_same_site_lax {
                auth_cookie.set_same_site(SameSite::Lax);
            } else {
                auth_cookie.set_same_site(SameSite::None);
            }

            HttpResponse::Ok().cookie(auth_cookie).finish()
        }
        Err(_) => {
            let conf = get_config();
            let tokens = json::to_string(&AuthTokens::default()).unwrap();
            let mut auth_cookie = Cookie::new("auth_tokens", tokens);
            auth_cookie.set_expires(
                cookie::time::OffsetDateTime::now_utc()
                    + cookie::time::Duration::seconds(conf.auth.cookie_max_age),
            );
            auth_cookie.set_http_only(true);
            auth_cookie.set_secure(conf.auth.cookie_secure_only);
            auth_cookie.set_path("/");
            if conf.auth.cookie_same_site_lax {
                auth_cookie.set_same_site(SameSite::Lax);
            } else {
                auth_cookie.set_same_site(SameSite::None);
            }

            HttpResponse::Unauthorized()
                .append_header((header::LOCATION, "/"))
                .cookie(auth_cookie)
                .finish()
        }
    }
}

fn prepare_empty_cookie<'a, T: Serialize + ?Sized>(
    cookie_name: &'a str,
    token_struct: &T,
    conf: &Arc<Config>,
) -> Cookie<'a> {
    let tokens = json::to_string(token_struct).unwrap();
    let mut auth_cookie = Cookie::new(cookie_name, tokens);
    auth_cookie.set_expires(
        cookie::time::OffsetDateTime::now_utc()
            + cookie::time::Duration::seconds(conf.auth.cookie_max_age),
    );
    auth_cookie.set_http_only(true);
    auth_cookie.set_secure(conf.auth.cookie_secure_only);
    auth_cookie.set_path("/");
    if conf.auth.cookie_same_site_lax {
        auth_cookie.set_same_site(SameSite::Lax);
    } else {
        auth_cookie.set_same_site(SameSite::None);
    }
    auth_cookie
}

#[get("/logout")]
async fn logout(req: actix_web::HttpRequest) -> HttpResponse {
    // remove the session
    let conf = get_config();

    #[cfg(feature = "enterprise")]
    let auth_str = extract_auth_str(&req);
    // Only get the user email from the auth_str, no need to check for permissions and others
    #[cfg(feature = "enterprise")]
    let user_email = get_user_email_from_auth_str(&auth_str).await;

    if let Some(cookie) = req.cookie("auth_tokens") {
        let auth_tokens: AuthTokens = json::from_str(cookie.value()).unwrap_or_default();
        let access_token = auth_tokens.access_token;

        if access_token.starts_with("session") {
            crate::service::session::remove_session(access_token.strip_prefix("session ").unwrap())
                .await;
        }
    };
    let auth_cookie = prepare_empty_cookie("auth_tokens", &AuthTokens::default(), &conf);
    let auth_ext_cookie = prepare_empty_cookie("auth_ext", &AuthTokensExt::default(), &conf);

    #[cfg(feature = "enterprise")]
    if let Some(user_email) = user_email {
        audit(AuditMessage {
            user_email,
            org_id: "".to_string(),
            _timestamp: chrono::Utc::now().timestamp_micros(),
            protocol: Protocol::Http(HttpMeta {
                method: "GET".to_string(),
                path: "/config/logout".to_string(),
                query_params: req.query_string().to_string(),
                body: "".to_string(),
                response_code: 200,
            }),
        })
        .await;
    }

    HttpResponse::Ok()
        .append_header((header::LOCATION, "/"))
        .cookie(auth_cookie)
        .cookie(auth_ext_cookie)
        .finish()
}

#[put("/enable")]
async fn enable_node(req: HttpRequest) -> Result<HttpResponse, Error> {
    let node_id = LOCAL_NODE.uuid.clone();
    let Some(mut node) = cluster::get_node_by_uuid(&node_id).await else {
        return Ok(MetaHttpResponse::not_found("node not found"));
    };

    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let enable = match query.get("value") {
        Some(v) => v.parse::<bool>().unwrap_or_default(),
        None => false,
    };
    node.scheduled = enable;
    if !node.scheduled {
        // release all the searching files
        crate::common::infra::wal::clean_lock_files();
    }
    match cluster::update_local_node(&node).await {
        Ok(_) => Ok(MetaHttpResponse::json(true)),
        Err(e) => Ok(MetaHttpResponse::internal_error(e)),
    }
}

#[put("/flush")]
async fn flush_node() -> Result<HttpResponse, Error> {
    if !LOCAL_NODE.is_ingester() {
        return Ok(MetaHttpResponse::not_found("local node is not an ingester"));
    };

    // release all the searching files
    crate::common::infra::wal::clean_lock_files();

    match ingester::flush_all().await {
        Ok(_) => Ok(MetaHttpResponse::json(true)),
        Err(e) => Ok(MetaHttpResponse::internal_error(e)),
    }
}

#[get("/list")]
async fn list_node() -> Result<HttpResponse, Error> {
    let nodes = cluster::get_cached_nodes(|_| true).await;
    Ok(MetaHttpResponse::json(nodes))
}
