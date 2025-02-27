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

use actix_web::{get, web, HttpRequest, Responder};
use config::meta::{
    alerts::alert::AlertListFilter,
    dashboards::{reports::ReportListFilters, ListDashboardsParams},
    stream::StreamType,
};

use crate::{
    common::meta::http::HttpResponse as MetaHttpResponse,
    handler::http::models::lookup::{
        LookupRequestQuery, PaginatedResponse, PaginationMetadata, Resource,
    },
    service::{
        alerts::alert,
        dashboards::{list_dashboards, reports},
        functions, pipeline, stream,
    },
};

/// LookupByName
#[utoipa::path(
    context_path = "/api",
    tag = "Lookup",
    operation_id = "LookupByName",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        LookupRequestQuery
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = PaginatedResponse<Stream>),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
    ),
)]
#[get("/{org_id}/lookup")]
pub async fn lookup_by_name(org_id: web::Path<String>, req: HttpRequest) -> impl Responder {
    let org_id = org_id.into_inner();
    let Ok(query) = web::Query::<LookupRequestQuery>::from_query(req.query_string()) else {
        return MetaHttpResponse::bad_request("Error parsing query parameters");
    };
    let lookup_params = query.0;

    let mut _user_id: Option<&str> = None;
    let mut _list_resources_from_rbac: Option<Vec<String>> = None;
    #[cfg(feature = "enterprise")]
    {
        let Ok(_user_id) = req.headers().get("user_id").map(|v| v.to_str()).transpose() else {
            return MetaHttpResponse::forbidden("");
        };

        let object_type = match lookup_params.resource {
            Resource::Pipeline => o2_openfga::meta::mapping::OFGA_MODELS
                .get("pipelines")
                .map_or("pipelines", |model| model.key),
            _ => lookup_params.resource.as_str(),
        };
        match crate::handler::http::auth::validator::list_objects_for_user(
            &org_id,
            _user_id.unwrap(),
            "GET",
            object_type,
        )
        .await
        {
            Ok(list) => {
                _list_resources_from_rbac = list;
            }
            Err(e) => {
                return MetaHttpResponse::forbidden(e.to_string());
            }
        }
    }

    match lookup_params.resource {
        Resource::Stream => {
            let stream_types = lookup_params.stream_type.map_or(
                vec![
                    StreamType::Logs,
                    StreamType::Traces,
                    StreamType::Metrics,
                    StreamType::Metadata,
                    StreamType::Index,
                ],
                |stream_type| vec![stream_type],
            );
            let fetch_schema = lookup_params.fetch_schema.unwrap_or_default();
            let mut all_streams = vec![];
            for stream_type in stream_types {
                let mut _stream_list_from_rbac = None;
                #[cfg(feature = "enterprise")]
                {
                    let stream_type_str = stream_type.to_string();
                    match crate::handler::http::auth::validator::list_objects_for_user(
                        &org_id,
                        _user_id.unwrap(),
                        "GET",
                        o2_openfga::meta::mapping::OFGA_MODELS
                            .get(stream_type_str.as_str())
                            .map_or(stream_type_str.as_str(), |model| model.key),
                    )
                    .await
                    {
                        Ok(stream_list) => {
                            _stream_list_from_rbac = stream_list;
                        }
                        Err(e) => {
                            return MetaHttpResponse::forbidden(e.to_string());
                        }
                    }
                }

                let mut streams = stream::get_streams(
                    org_id.as_str(),
                    Some(stream_type),
                    fetch_schema,
                    _stream_list_from_rbac,
                )
                .await;
                streams.retain(|stream| stream.name.contains(&lookup_params.key));
                all_streams.extend(streams);
            }
            all_streams.sort_by(|a, b| a.name.cmp(&b.name));
            let pagination_metadata = match lookup_params.page_size {
                Some(page_size) if page_size > 0 => {
                    let page_idx = lookup_params.page_idx.unwrap_or_default();
                    all_streams = all_streams
                        .into_iter()
                        .skip(page_idx * page_size)
                        .take(page_size)
                        .collect();
                    Some(PaginationMetadata {
                        total_items: all_streams.len(),
                        total_pages: all_streams.len().div_ceil(page_size),
                        current_page: page_idx.max(1),
                        page_size,
                    })
                }
                _ => None,
            };
            MetaHttpResponse::json(PaginatedResponse {
                data: all_streams,
                pagination_metadata,
            })
        }
        Resource::Alert => {
            match alert::list(
                &org_id,
                None,
                None,
                _list_resources_from_rbac,
                AlertListFilter::default(),
            )
            .await
            {
                Ok(mut alerts) => {
                    alerts.retain(|alert| alert.name.contains(&lookup_params.key));
                    let pagination_metadata = match lookup_params.page_size {
                        Some(page_size) if page_size > 0 => {
                            let page_idx = lookup_params.page_idx.unwrap_or_default();
                            alerts = alerts
                                .into_iter()
                                .skip(page_idx * page_size)
                                .take(page_size)
                                .collect();
                            Some(PaginationMetadata {
                                total_items: alerts.len(),
                                total_pages: alerts.len().div_ceil(page_size),
                                current_page: page_idx,
                                page_size,
                            })
                        }
                        _ => None,
                    };
                    MetaHttpResponse::json(PaginatedResponse {
                        data: alerts,
                        pagination_metadata,
                    })
                }
                Err(e) => e.into(),
            }
        }
        Resource::Dashboard => {
            let page_size_and_idx = lookup_params.page_size.map(|size| {
                (
                    size as u64,
                    lookup_params.page_idx.unwrap_or_default() as u64,
                )
            });
            let list_params = ListDashboardsParams {
                org_id,
                folder_id: lookup_params.folder_id,
                title_pat: Some(lookup_params.key),
                page_size_and_idx,
            };
            let dashboards = match list_dashboards(list_params).await {
                Ok(dashboards) => dashboards
                    .into_iter()
                    .map(|(_, dashboard)| dashboard)
                    .collect::<Vec<_>>(),
                Err(e) => {
                    return e.into();
                }
            };
            let pagination_metadata = match page_size_and_idx {
                Some((page_size, page_idx)) if page_size > 0 => Some(PaginationMetadata {
                    total_items: dashboards.len(),
                    total_pages: dashboards.len().div_ceil(page_size as usize),
                    current_page: page_idx.max(1) as usize,
                    page_size: page_size as usize,
                }),
                _ => None,
            };
            MetaHttpResponse::json(PaginatedResponse {
                data: dashboards,
                pagination_metadata,
            })
        }
        Resource::Report => {
            let filters = ReportListFilters {
                folder: lookup_params.folder_id,
                dashboard: lookup_params.dashboard_id,
                destination_less: None,
            };
            match reports::list(&org_id, filters, _list_resources_from_rbac).await {
                Ok(mut reports) => {
                    reports.retain(|report| report.name.contains(&lookup_params.key));
                    let pagination_metadata = match lookup_params.page_size {
                        Some(page_size) if page_size > 0 => {
                            let page_idx = lookup_params.page_idx.unwrap_or_default();
                            reports = reports
                                .into_iter()
                                .skip(page_idx * page_size)
                                .take(page_size)
                                .collect();
                            Some(PaginationMetadata {
                                total_items: reports.len(),
                                total_pages: reports.len().div_ceil(page_size),
                                current_page: page_idx.max(1),
                                page_size,
                            })
                        }
                        _ => None,
                    };
                    MetaHttpResponse::json(PaginatedResponse {
                        data: reports,
                        pagination_metadata,
                    })
                }
                Err(e) => MetaHttpResponse::bad_request(e.to_string()),
            }
        }
        Resource::Pipeline => {
            match pipeline::list_pipelines(org_id, _list_resources_from_rbac).await {
                Ok(pipeline_list) => {
                    let mut pipelines = pipeline_list.list;
                    pipelines.retain(|pipeline| pipeline.name.contains(&lookup_params.key));
                    let pagination_metadata = match lookup_params.page_size {
                        Some(page_size) if page_size > 0 => {
                            let page_idx = lookup_params.page_idx.unwrap_or_default();
                            pipelines = pipelines
                                .into_iter()
                                .skip(page_idx * page_size)
                                .take(page_size)
                                .collect();
                            Some(PaginationMetadata {
                                total_items: pipelines.len(),
                                total_pages: pipelines.len().div_ceil(page_size),
                                current_page: page_idx.max(1),
                                page_size,
                            })
                        }
                        _ => None,
                    };
                    MetaHttpResponse::json(PaginatedResponse {
                        data: pipelines,
                        pagination_metadata,
                    })
                }
                Err(e) => e.into(),
            }
        }
        Resource::Function => {
            match functions::list_functions(org_id, _list_resources_from_rbac).await {
                Ok(func_list) => {
                    let mut functions = func_list.list;
                    functions.retain(|function| function.name.contains(&lookup_params.key));
                    let pagination_metadata = match lookup_params.page_size {
                        Some(page_size) if page_size > 0 => {
                            let page_idx = lookup_params.page_idx.unwrap_or_default();
                            functions = functions
                                .into_iter()
                                .skip(page_idx * page_size)
                                .take(page_size)
                                .collect();
                            Some(PaginationMetadata {
                                total_items: functions.len(),
                                total_pages: functions.len().div_ceil(page_size),
                                current_page: page_idx.max(1),
                                page_size,
                            })
                        }
                        _ => None,
                    };
                    MetaHttpResponse::json(PaginatedResponse {
                        data: functions,
                        pagination_metadata,
                    })
                }
                Err(e) => MetaHttpResponse::bad_request(e.to_string()),
            }
        }
    }
}
