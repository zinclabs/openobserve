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

use std::{rc::Rc, str::FromStr};

use actix_cors::Cors;
use actix_web::{
    body::MessageBody,
    dev::{Service, ServiceRequest, ServiceResponse},
    http::header,
    middleware, web, HttpRequest, HttpResponse,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web_lab::middleware::{from_fn, Next};
use config::get_config;
use futures::FutureExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
#[cfg(feature = "enterprise")]
use {
    crate::{common::meta::ingestion::INGESTION_EP, service::self_reporting::audit},
    actix_http::h1::Payload,
    actix_web::{web::BytesMut, HttpMessage},
    base64::{engine::general_purpose, Engine as _},
    futures::StreamExt,
    o2_enterprise::enterprise::common::{
        auditor::{AuditMessage, HttpMeta, Protocol},
        infra::config::get_config as get_o2_config,
    },
};

use super::request::*;
use crate::common::meta::{middleware_data::RumExtraData, proxy::PathParamProxyURL};

pub mod middlewares;
pub mod openapi;
pub mod ui;

pub fn get_cors() -> Rc<Cors> {
    let cors = Cors::default()
        .allowed_methods(vec!["HEAD", "GET", "POST", "PUT", "OPTIONS", "DELETE"])
        .allowed_headers(vec![
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::HeaderName::from_lowercase(b"traceparent").unwrap(),
        ])
        .allow_any_origin()
        .supports_credentials()
        .max_age(3600);
    Rc::new(cors)
}

#[cfg(feature = "enterprise")]
async fn audit_middleware(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    let method = req.method().to_string();
    let prefix = format!("{}/api/", get_config().common.base_uri);
    let path = req.path().strip_prefix(&prefix).unwrap().to_string();
    let path_columns = path.split('/').collect::<Vec<&str>>();
    let path_len = path_columns.len();
    if get_o2_config().common.audit_enabled
        && !path_columns.get(1).unwrap_or(&"").to_string().eq("ws")
        && !(method.eq("POST") && INGESTION_EP.contains(&path_columns[path_len - 1]))
    {
        let query_params = req.query_string().to_string();
        let org_id = {
            let org = path_columns[0];
            if org.eq("organizations") {
                "".to_string()
            } else {
                org.to_string()
            }
        };

        let mut request_body = BytesMut::new();
        let mut payload_stream = req.take_payload();
        while let Some(chunk) = payload_stream.next().await {
            request_body.extend_from_slice(&chunk.unwrap());
        }
        let user_email = req
            .headers()
            .get("user_id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // Put the payload back into the req
        let (_, mut payload) = Payload::create(true);
        payload.unread_data(request_body.clone().into());
        req.set_payload(payload.into());

        // Call the next service in the chain
        let res = next.call(req).await?;

        if res.response().error().is_none() {
            let body = if path.ends_with("/settings/logo") {
                // Binary data, encode it with base64
                general_purpose::STANDARD.encode(&request_body)
            } else {
                String::from_utf8(request_body.to_vec()).unwrap_or_default()
            };
            audit(AuditMessage {
                user_email,
                org_id,
                _timestamp: chrono::Utc::now().timestamp_micros(),
                protocol: Protocol::Http(HttpMeta {
                    method,
                    path,
                    body,
                    query_params,
                    response_code: res.response().status().as_u16(),
                }),
            })
            .await;
        }
        Ok(res)
    } else {
        next.call(req).await
    }
}

#[cfg(not(feature = "enterprise"))]
async fn audit_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    next.call(req).await
}

/// This is a very trivial proxy to overcome the cors errors while
/// session-replay in rrweb.
pub fn get_proxy_routes(svc: &mut web::ServiceConfig) {
    let enable_authentication_on_route = true;
    get_proxy_routes_inner(svc, enable_authentication_on_route)
}

pub fn get_proxy_routes_inner(svc: &mut web::ServiceConfig, enable_validator: bool) {
    let cors = Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header();

    if enable_validator {
        svc.service(
            web::resource("/proxy/{org_id}/{target_url:.*}")
                .wrap(cors)
                .wrap(HttpAuthentication::with_fn(
                    super::auth::validator::validator_proxy_url,
                ))
                .route(web::get().to(proxy)),
        );
    } else {
        svc.service(
            web::resource("/proxy/{org_id}/{target_url:.*}")
                .wrap(cors)
                .route(web::get().to(proxy)),
        );
    };
}
async fn proxy(
    path: web::Path<PathParamProxyURL>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let client = reqwest::Client::new();
    let method = reqwest::Method::from_str(req.method().as_str()).unwrap();
    let forwarded_resp = client
        .request(method, &path.target_url)
        .send()
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Request failed: {}", e))
        })?;

    let status = forwarded_resp.status().as_u16();
    let body = forwarded_resp.bytes().await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to read the response: {}", e))
    })?;

    Ok(HttpResponse::build(actix_web::http::StatusCode::from_u16(status).unwrap()).body(body))
}

pub fn get_basic_routes(svc: &mut web::ServiceConfig) {
    let cors = get_cors();
    svc.service(status::healthz)
        .service(status::healthz_head)
        .service(status::schedulez);
    svc.service(
        web::scope("/auth")
            .wrap(cors.clone())
            .service(users::authentication)
            .service(users::get_presigned_url)
            .service(users::get_auth),
    );

    svc.service(
        web::scope("/node")
            .wrap(HttpAuthentication::with_fn(
                super::auth::validator::oo_validator,
            ))
            .wrap(cors.clone())
            .service(status::cache_status)
            .service(status::enable_node)
            .service(status::flush_node)
            .service(status::list_node)
            .service(status::node_metrics),
    );

    if get_config().common.swagger_enabled {
        svc.service(
            SwaggerUi::new("/swagger/{_:.*}")
                .url(
                    format!("{}/api-doc/openapi.json", get_config().common.base_uri),
                    openapi::ApiDoc::openapi(),
                )
                .url("/api-doc/openapi.json", openapi::ApiDoc::openapi()),
        );
        svc.service(web::redirect("/swagger", "/swagger/"));
        svc.service(web::redirect("/docs", "/swagger/"));
    }

    if get_config().common.ui_enabled {
        svc.service(web::redirect("/", "./web/"));
        svc.service(web::redirect("/web", "./web/"));
        svc.service(
            web::scope("/web")
                .wrap_fn(|req, srv| {
                    let cfg = get_config();
                    let prefix = format!("{}/web/", cfg.common.base_uri);
                    let path = req.path().strip_prefix(&prefix).unwrap().to_string();

                    srv.call(req).map(move |res| {
                        if path.starts_with("src/")
                            || path.starts_with("assets/")
                            || path.starts_with("monacoeditorwork/")
                            || path.eq("favicon.ico")
                        {
                            res
                        } else {
                            let res = res.unwrap();
                            let req = res.request().clone();
                            let body = res.into_body();
                            let body = body.try_into_bytes().unwrap();
                            let body = String::from_utf8(body.to_vec()).unwrap();
                            let body = body.replace(
                                r#"<base href="/" />"#,
                                &format!(r#"<base href="{prefix}" />"#),
                            );
                            Ok(ServiceResponse::new(
                                req,
                                HttpResponse::Ok()
                                    .content_type(header::ContentType::html())
                                    .body(body),
                            ))
                        }
                    })
                })
                .service(ui::serve),
        );
    }
}

#[cfg(not(feature = "enterprise"))]
pub fn get_config_routes(svc: &mut web::ServiceConfig) {
    let cors = get_cors();
    svc.service(
        web::scope("/config")
            .wrap(cors.clone())
            .service(status::zo_config)
            .service(status::logout)
            .service(web::scope("/reload").service(status::config_reload)),
    );
}

#[cfg(feature = "enterprise")]
pub fn get_config_routes(svc: &mut web::ServiceConfig) {
    let cors = get_cors();
    svc.service(
        web::scope("/config")
            .wrap(cors)
            .service(status::zo_config)
            .service(status::redirect)
            .service(status::dex_login)
            .service(status::refresh_token_with_dex)
            .service(status::logout)
            .service(users::service_accounts::exchange_token)
            .service(web::scope("/reload").service(status::config_reload)),
    );
}

pub fn get_service_routes(svc: &mut web::ServiceConfig) {
    let cfg = get_config();
    let cors = get_cors();
    // set server header
    #[cfg(feature = "enterprise")]
    let server = format!(
        "{}-{}",
        get_o2_config().super_cluster.region,
        cfg.common.instance_name_short
    );
    #[cfg(not(feature = "enterprise"))]
    let server = cfg.common.instance_name_short.to_string();

    let service = web::scope("/api")
        .wrap(from_fn(audit_middleware))
        .wrap(HttpAuthentication::with_fn(
            super::auth::validator::oo_validator,
        ))
        .wrap(cors.clone())
        .wrap(middleware::DefaultHeaders::new().add(("X-Api-Node", server)))
        .service(lookup::lookup_by_name)
        .service(users::list)
        .service(users::save)
        .service(users::delete)
        .service(users::update)
        .service(users::add_user_to_org)
        .service(organization::org::organizations)
        .service(organization::settings::get)
        .service(organization::settings::create)
        .service(organization::settings::upload_logo)
        .service(organization::settings::delete_logo)
        .service(organization::settings::set_logo_text)
        .service(organization::settings::delete_logo_text)
        .service(organization::org::org_summary)
        .service(organization::org::get_user_passcode)
        .service(organization::org::update_user_passcode)
        .service(organization::org::create_user_rumtoken)
        .service(organization::org::get_user_rumtoken)
        .service(organization::org::update_user_rumtoken)
        .service(organization::es::org_index)
        .service(organization::es::org_license)
        .service(organization::es::org_xpack)
        .service(organization::es::org_ilm_policy)
        .service(organization::es::org_index_template)
        .service(organization::es::org_index_template_create)
        .service(organization::es::org_data_stream)
        .service(organization::es::org_data_stream_create)
        .service(organization::es::org_pipeline)
        .service(organization::es::org_pipeline_create)
        .service(stream::schema)
        .service(stream::settings)
        .service(stream::update_settings)
        .service(stream::delete_fields)
        .service(stream::delete)
        .service(stream::list)
        .service(logs::ingest::bulk)
        .service(logs::ingest::multi)
        .service(logs::ingest::json)
        .service(logs::ingest::otlp_logs_write)
        .service(traces::traces_write)
        .service(traces::otlp_traces_write)
        .service(traces::get_latest_traces)
        .service(metrics::ingest::json)
        .service(metrics::ingest::otlp_metrics_write)
        .service(promql::remote_write)
        .service(promql::query_get)
        .service(promql::query_post)
        .service(promql::query_range_get)
        .service(promql::query_range_post)
        .service(promql::query_exemplars_get)
        .service(promql::query_exemplars_post)
        .service(promql::metadata)
        .service(promql::series_get)
        .service(promql::series_post)
        .service(promql::labels_get)
        .service(promql::labels_post)
        .service(promql::label_values)
        .service(promql::format_query_get)
        .service(promql::format_query_post)
        .service(enrichment_table::save_enrichment_table)
        .service(search::search)
        .service(search::search_partition)
        .service(search::around)
        .service(search::values)
        .service(search::search_history)
        .service(search::saved_view::create_view)
        .service(search::saved_view::update_view)
        .service(search::saved_view::get_view)
        .service(search::saved_view::get_views)
        .service(search::saved_view::delete_view)
        .service(functions::save_function)
        .service(functions::list_functions)
        .service(functions::test_function)
        .service(functions::delete_function)
        .service(functions::update_function)
        .service(functions::list_pipeline_dependencies)
        .service(dashboards::create_dashboard)
        .service(dashboards::update_dashboard)
        .service(dashboards::list_dashboards)
        .service(dashboards::get_dashboard)
        .service(dashboards::delete_dashboard)
        .service(dashboards::move_dashboard)
        .service(dashboards::reports::create_report)
        .service(dashboards::reports::update_report)
        .service(dashboards::reports::get_report)
        .service(dashboards::reports::list_reports)
        .service(dashboards::reports::delete_report)
        .service(dashboards::reports::enable_report)
        .service(dashboards::reports::trigger_report)
        .service(dashboards::timed_annotations::create_annotations)
        .service(dashboards::timed_annotations::get_annotations)
        .service(dashboards::timed_annotations::delete_annotations)
        .service(dashboards::timed_annotations::update_annotations)
        .service(dashboards::timed_annotations::delete_annotation_panels)
        .service(folders::create_folder)
        .service(folders::list_folders)
        .service(folders::update_folder)
        .service(folders::get_folder)
        .service(folders::get_folder_by_name)
        .service(folders::delete_folder)
        .service(folders::deprecated::create_folder)
        .service(folders::deprecated::list_folders)
        .service(folders::deprecated::update_folder)
        .service(folders::deprecated::get_folder)
        .service(folders::deprecated::get_folder_by_name)
        .service(folders::deprecated::delete_folder)
        .service(alerts::create_alert)
        .service(alerts::get_alert)
        .service(alerts::update_alert)
        .service(alerts::delete_alert)
        .service(alerts::list_alerts)
        .service(alerts::enable_alert)
        .service(alerts::trigger_alert)
        .service(alerts::move_alerts)
        .service(alerts::deprecated::save_alert)
        .service(alerts::deprecated::update_alert)
        .service(alerts::deprecated::get_alert)
        .service(alerts::deprecated::list_alerts)
        .service(alerts::deprecated::list_stream_alerts)
        .service(alerts::deprecated::delete_alert)
        .service(alerts::deprecated::enable_alert)
        .service(alerts::deprecated::trigger_alert)
        .service(alerts::templates::save_template)
        .service(alerts::templates::update_template)
        .service(alerts::templates::get_template)
        .service(alerts::templates::delete_template)
        .service(alerts::templates::list_templates)
        .service(alerts::destinations::save_destination)
        .service(alerts::destinations::update_destination)
        .service(alerts::destinations::get_destination)
        .service(alerts::destinations::list_destinations)
        .service(alerts::destinations::delete_destination)
        .service(kv::get)
        .service(kv::set)
        .service(kv::delete)
        .service(kv::list)
        .service(syslog::list_routes)
        .service(syslog::create_route)
        .service(syslog::delete_route)
        .service(syslog::update_route)
        .service(syslog::toggle_state)
        .service(enrichment_table::save_enrichment_table)
        .service(metrics::ingest::otlp_metrics_write)
        .service(logs::ingest::otlp_logs_write)
        .service(traces::otlp_traces_write)
        .service(dashboards::move_dashboard)
        .service(traces::get_latest_traces)
        .service(logs::ingest::multi)
        .service(logs::ingest::json)
        .service(logs::ingest::handle_kinesis_request)
        .service(logs::ingest::handle_gcp_request)
        .service(organization::org::create_org)
        .service(authz::fga::create_role)
        .service(authz::fga::get_roles)
        .service(authz::fga::update_role)
        .service(authz::fga::get_role_permissions)
        .service(authz::fga::create_group)
        .service(authz::fga::update_group)
        .service(authz::fga::get_groups)
        .service(authz::fga::get_group_details)
        .service(authz::fga::get_resources)
        .service(authz::fga::get_users_with_role)
        .service(authz::fga::delete_role)
        .service(authz::fga::delete_group)
        .service(users::list_roles)
        .service(clusters::list_clusters)
        .service(pipeline::save_pipeline)
        .service(pipeline::update_pipeline)
        .service(pipeline::list_pipelines)
        .service(pipeline::list_streams_with_pipeline)
        .service(pipeline::delete_pipeline)
        .service(pipeline::enable_pipeline)
        .service(search::multi_streams::search_multi)
        .service(search::multi_streams::_search_partition_multi)
        .service(search::multi_streams::around_multi)
        .service(stream::delete_stream_cache)
        .service(short_url::shorten)
        .service(short_url::retrieve)
        .service(service_accounts::list)
        .service(service_accounts::save)
        .service(service_accounts::delete)
        .service(service_accounts::update)
        .service(service_accounts::get_api_token)
        .service(websocket::websocket);

    #[cfg(feature = "enterprise")]
    let service = service
        .service(search::search_job::submit_job)
        .service(search::search_job::list_status)
        .service(search::search_job::get_status)
        .service(search::search_job::get_job_result)
        .service(search::search_job::cancel_job)
        .service(search::search_job::delete_job)
        .service(search::search_job::retry_job)
        .service(search::query_manager::query_status)
        .service(search::query_manager::cancel_multiple_query)
        .service(search::query_manager::cancel_query)
        .service(keys::get)
        .service(keys::delete)
        .service(keys::save)
        .service(keys::list)
        .service(keys::update)
        .service(actions::action::get_action_from_id)
        .service(actions::action::list_actions)
        .service(actions::action::upload_zipped_action)
        .service(actions::action::update_action_details)
        .service(actions::action::serve_action_zip)
        .service(actions::action::delete_action)
        .service(actions::operations::test_action);

    svc.service(service);
}

pub fn get_other_service_routes(svc: &mut web::ServiceConfig) {
    let cors = get_cors();
    svc.service(
        web::scope("/aws")
            .wrap(cors.clone())
            .wrap(HttpAuthentication::with_fn(
                super::auth::validator::validator_aws,
            ))
            .service(logs::ingest::handle_kinesis_request),
    );

    svc.service(
        web::scope("/gcp")
            .wrap(cors.clone())
            .wrap(HttpAuthentication::with_fn(
                super::auth::validator::validator_gcp,
            ))
            .service(logs::ingest::handle_gcp_request),
    );

    // NOTE: Here the order of middlewares matter. Once we consume the api-token in
    // `rum_auth`, we drop it in the RumExtraData data.
    // https://docs.rs/actix-web/latest/actix_web/middleware/index.html#ordering
    svc.service(
        web::scope("/rum")
            .wrap(cors)
            .wrap(from_fn(RumExtraData::extractor))
            .wrap(HttpAuthentication::with_fn(
                super::auth::validator::validator_rum,
            ))
            .service(rum::ingest::log)
            .service(rum::ingest::sessionreplay)
            .service(rum::ingest::data),
    );
}

#[cfg(test)]
mod tests {
    use actix_web::{
        test::{call_service, init_service, TestRequest},
        App,
    };

    use super::*;

    #[tokio::test]
    async fn test_get_proxy_routes() {
        let mut app =
            init_service(App::new().configure(|cfg| get_proxy_routes_inner(cfg, false))).await;

        // Test GET request to /proxy/{org_id}/{target_url}
        let req = TestRequest::get()
            .uri("/proxy/org1/https://cloud.openobserve.ai/assets/flUhRq6tzZclQEJ-Vdg-IuiaDsNa.fd84f88b.woff")
            .to_request();
        let resp = call_service(&mut app, req).await;
        assert_eq!(resp.status().as_u16(), 404);
    }
}
