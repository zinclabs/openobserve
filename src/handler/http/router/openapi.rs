// Copyright 2023 Zinc Labs Inc.
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

use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::{
    common::{infra::config::CONFIG, meta},
    handler::http::request,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        request::status::healthz,
        request::users::list,
        request::users::save,
        request::users::update,
        request::users::delete,
        request::users::authentication,
        request::users::add_user_to_org,
        request::organization::organizations,
        request::organization::org_summary,
        request::organization::get_user_passcode,
        request::organization::update_user_passcode,
        request::organization::get_user_rumtoken,
        request::organization::update_user_rumtoken,
        request::organization::create_user_rumtoken,
        request::organization::settings::get,
        request::organization::settings::create,
        request::stream::list,
        request::stream::schema,
        request::stream::settings,
        request::stream::delete_fields,
        request::stream::delete,
        request::logs::ingest::bulk,
        request::logs::ingest::multi,
        request::logs::ingest::json,
        request::traces::traces_write,
        request::traces::get_latest_traces,
        request::metrics::ingest::json,
        request::prom::remote_write,
        request::prom::query_get,
        request::prom::query_range_get,
        request::prom::metadata,
        request::prom::series_get,
        request::prom::labels_get,
        request::prom::label_values,
        request::prom::format_query_get,
        request::enrichment_table::save_enrichment_table,
        request::rum::ingest::log,
        request::rum::ingest::data,
        request::rum::ingest::sessionreplay,
        request::search::search,
        request::search::around,
        request::search::values,
        request::search::saved_view::create_view,
        request::search::saved_view::delete_view,
        request::search::saved_view::get_view,
        request::search::saved_view::get_views,
        request::search::saved_view::update_view,
        request::functions::list_functions,
        request::functions::update_function,
        request::functions::save_function,
        request::functions::delete_function,
        request::functions::list_stream_functions,
        request::functions::add_function_to_stream,
        request::functions::delete_stream_function,
        request::dashboards::create_dashboard,
        request::dashboards::update_dashboard,
        request::dashboards::list_dashboards,
        request::dashboards::get_dashboard,
        request::dashboards::delete_dashboard,
        request::dashboards::folders::delete_folder,
        request::dashboards::folders::create_folder,
        request::dashboards::folders::list_folders,
        request::dashboards::folders::get_folder,
        request::dashboards::folders::update_folder,
        request::dashboards::move_dashboard,
        request::alerts::save_alert,
        request::alerts::list_stream_alerts,
        request::alerts::list_alerts,
        request::alerts::get_alert,
        request::alerts::delete_alert,
        request::alerts::enable_alert,
        request::alerts::trigger_alert,
        request::alerts::templates::list_templates,
        request::alerts::templates::get_template,
        request::alerts::templates::save_template,
        request::alerts::templates::delete_template,
        request::alerts::destinations::list_destinations,
        request::alerts::destinations::get_destination,
        request::alerts::destinations::save_destination,
        request::alerts::destinations::delete_destination,
        request::kv::get,
        request::kv::set,
        request::kv::delete,
        request::kv::list,
        request::syslog::create_route,
        request::syslog::update_route,
        request::syslog::list_routes,
        request::syslog::delete_route,
    ),
    components(
        schemas(
            meta::http::HttpResponse,
            meta::StreamType,
            meta::stream::Stream,
            meta::stream::StreamStats,
            meta::stream::StreamProperty,
            meta::stream::StreamSettings,
            meta::stream::StreamDeleteFields,
            meta::stream::ListStream,
            meta::stream::PartitionTimeLevel,
            meta::ingestion::RecordStatus,
            meta::ingestion::StreamStatus,
            meta::ingestion::IngestionResponse,
            meta::dashboards::Dashboard,
            meta::dashboards::Dashboards,
            meta::dashboards::v1::AxisItem,
            meta::dashboards::v1::Dashboard,
            meta::dashboards::v1::AggregationFunc,
            meta::dashboards::v1::Layout,
            meta::dashboards::v1::Panel,
            meta::dashboards::v1::PanelConfig,
            meta::dashboards::v1::PanelFields,
            meta::dashboards::v1::PanelFilter,
            meta::dashboards::v1::Variables,
            meta::dashboards::v1::QueryData,
            meta::dashboards::v1::CustomFieldsOption,
            meta::dashboards::v1::VariableList,
            meta::dashboards::Folder,
            meta::dashboards::MoveDashboard,
            meta::dashboards::FolderList,
            meta::search::Query,
            meta::search::Request,
            meta::search::RequestEncoding,
            meta::search::Response,
            meta::search::ResponseTook,
            meta::saved_view::View,
            meta::saved_view::ViewWithoutData,
            meta::saved_view::ViewsWithoutData,
            meta::saved_view::CreateViewRequest,
            meta::saved_view::DeleteViewResponse,
            meta::saved_view::CreateViewResponse,
            meta::saved_view::UpdateViewRequest,
            meta::alerts::Alert,
            meta::alerts::Condition,
            meta::alerts::Operator,
            meta::alerts::Aggregation,
            meta::alerts::TriggerCondition,
            meta::alerts::QueryCondition,
            meta::alerts::destinations::Destination,
            meta::alerts::destinations::DestinationWithTemplate,
            meta::alerts::destinations::HTTPType,
            meta::alerts::templates::Template,
            meta::functions::Transform,
            meta::functions::FunctionList,
            meta::functions::StreamFunctionsList,
            meta::functions::StreamTransform,
            meta::functions::StreamOrder,
            meta::user::UserRequest,
            meta::user::UpdateUser,
            meta::user::UserRole,
            meta::user::UserOrgRole,
            meta::user::UserList,
            meta::user::UserResponse,
            meta::user::UpdateUser,
            meta::user::SignInUser,
            meta::user::SignInResponse,
            meta::organization::OrgSummary,
            meta::organization::OrganizationResponse,
            meta::organization::OrgDetails,
            meta::organization::OrgUser,
            meta::organization::IngestionPasscode,
            meta::organization::PasscodeResponse,
            meta::organization::OrganizationSetting,
            meta::organization::OrganizationSettingResponse,
            meta::organization::RumIngestionResponse,
            meta::organization::RumIngestionToken,
            request::status::HealthzResponse,
            meta::ingestion::BulkResponse,
            meta::ingestion::BulkResponseItem,
            meta::ingestion::ShardResponse,
            meta::ingestion::BulkResponseError,
            meta::syslog::SyslogRoute,
            meta::syslog::SyslogRoutes,
            meta::prom::Metadata,
            meta::prom::MetricType,
         ),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Meta", description = "Meta details about the OpenObserve state itself. e.g. healthz"),
        (name = "Auth", description = "User login authentication"),
        (name = "Logs", description = "Logs data ingestion operations"),
        (name = "Dashboards", description = "Dashboard operations"),
        (name = "Search", description = "Search/Query operations"),
        (name = "Saved Views", description = "Collection of saved search views for easy retrieval"),
        (name = "Alerts", description = "Alerts retrieval & management operations"),
        (name = "Functions", description = "Functions retrieval & management operations"),
        (name = "Organizations", description = "Organizations retrieval & management operations"),
        (name = "Streams", description = "Stream retrieval & management operations"),
        (name = "Users", description = "Users retrieval & management operations"),
        (name = "KV", description = "Key Value retrieval & management operations"),
        (name = "Metrics", description = "Metrics data ingestion operations"),
        (name = "Traces", description = "Traces data ingestion operations"),
        (name = "Syslog Routes", description = "Syslog Routes retrieval & management operations"),
    ),
    info(
        description = "OpenObserve API documents [https://openobserve.ai/docs/](https://openobserve.ai/docs/)",
        contact(name = "OpenObserve", email = "hello@zinclabs.io", url = "https://openobserve.ai/"),
    ),
)]
pub struct ApiDoc;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if !CONFIG.common.base_uri.is_empty() {
            openapi.servers = Some(vec![utoipa::openapi::Server::new(&CONFIG.common.base_uri)]);
        }
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "Authorization",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Basic).build()),
        );
    }
}
