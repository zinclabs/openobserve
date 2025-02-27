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

use chrono::{DateTime, FixedOffset, Utc};
use config::meta::{
    dashboards::{v1, v2, v3, v4, v5, Dashboard as MetaDashboard},
    folder::Folder as MetaFolder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// HTTP request body for the `CreateDashboard` endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDashboardRequestBody(JsonValue);

/// HTTP response body for `CreateDashboard` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateDashboardResponseBody(DashboardDetails);

/// HTTP response body for `GetDashboard` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetDashboardResponseBody(DashboardDetails);

/// HTTP request body for `UpdateDashboard` endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDashboardRequestBody(JsonValue);

/// HTTP response body for `UpdateDashboard` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateDashboardResponseBody(DashboardDetails);

/// HTTP URL query component that contains parameters for listing dashboards.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct ListDashboardsQuery {
    /// Optional folder ID filter parameter
    ///
    /// If neither `folder` nor any other filter parameter are set then this
    /// will search for all dashboards in the "default" folder.
    ///
    /// If `folder` is not set and another filter parameter, such as `title`, is
    /// set then this will search for dashboards in all folders.
    folder: Option<String>,

    /// The optional case-insensitive title substring with which to filter
    /// dashboards.
    title: Option<String>,

    /// The optional number of dashboards to retrieve. If not set then all
    /// dashboards that match the query parameters will be returned.
    ///
    /// Currently this parameter is only untilized by the API when the `title`
    /// parameter is also set.
    page_size: Option<u64>,
}

/// HTTP response body for `ListDashboards` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListDashboardsResponseBody {
    pub dashboards: Vec<ListDashboardsResponseBodyItem>,
}

/// An item in the list returned by the `ListDashboards` endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListDashboardsResponseBodyItem {
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v1: Option<v1::Dashboard>,
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v2: Option<v2::Dashboard>,
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v3: Option<v3::Dashboard>,
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v4: Option<v4::Dashboard>,
    #[deprecated(note = "use GetDashboard endpoint to get dashboard details")]
    pub v5: Option<v5::Dashboard>,

    pub version: i32,
    pub hash: String,

    pub folder_id: String,
    pub folder_name: String,
    pub dashboard_id: String,
    pub title: String,
    pub description: String,
    pub role: String,
    pub owner: String,
    #[schema(value_type = String, format = DateTime)]
    pub created: DateTime<FixedOffset>,
    // TODO: All client APIs should return camelCase
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
}

/// HTTP request body for `MoveDashboard` endpoint.
#[derive(Debug, Clone, PartialEq, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MoveDashboardRequestBody {
    pub from: String,
    pub to: String,
}

/// Version-specific dashboard details and hash.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDetails {
    pub v1: Option<v1::Dashboard>,
    pub v2: Option<v2::Dashboard>,
    pub v3: Option<v3::Dashboard>,
    pub v4: Option<v4::Dashboard>,
    pub v5: Option<v5::Dashboard>,
    pub version: i32,
    pub hash: String,
    pub updated_at: i64,
}

impl TryFrom<CreateDashboardRequestBody> for MetaDashboard {
    type Error = serde_json::Error;

    fn try_from(value: CreateDashboardRequestBody) -> Result<Self, Self::Error> {
        let json = value.0;
        parse_dashboard_request(json)
    }
}

impl From<MetaDashboard> for CreateDashboardResponseBody {
    fn from(value: MetaDashboard) -> Self {
        Self(value.into())
    }
}

impl From<MetaDashboard> for GetDashboardResponseBody {
    fn from(value: MetaDashboard) -> Self {
        Self(value.into())
    }
}

impl TryFrom<UpdateDashboardRequestBody> for MetaDashboard {
    type Error = serde_json::Error;

    fn try_from(value: UpdateDashboardRequestBody) -> Result<Self, Self::Error> {
        let json = value.0;
        parse_dashboard_request(json)
    }
}

impl From<MetaDashboard> for UpdateDashboardResponseBody {
    fn from(value: MetaDashboard) -> Self {
        Self(value.into())
    }
}

impl ListDashboardsQuery {
    pub fn into(self, org_id: &str) -> config::meta::dashboards::ListDashboardsParams {
        let mut query = match &self {
            Self {
                folder: Some(f),
                title: Some(t),
                ..
            } => config::meta::dashboards::ListDashboardsParams::new(org_id)
                .with_folder_id(f)
                .where_title_contains(t),
            Self {
                folder: None,
                title: Some(t),
                ..
            } => {
                config::meta::dashboards::ListDashboardsParams::new(org_id).where_title_contains(t)
            }
            Self {
                folder: Some(f),
                title: None,
                ..
            } => config::meta::dashboards::ListDashboardsParams::new(org_id).with_folder_id(f),
            Self {
                folder: None,
                title: None,
                ..
            } => {
                // To preserve backwards-compatability when no filter parameters
                // are given we will list the contents of the default folder.
                config::meta::dashboards::ListDashboardsParams::new(org_id)
                    .with_folder_id(config::meta::folder::DEFAULT_FOLDER)
            }
        };

        // The API currently only supports using page_size to limit the output
        // to the top results. And the page_size parameter is only used when the
        // title parameter is provided to search dashboards by title pattern.
        // When the title parameter is not set we simply want to return all
        // dashboards that match the selected folder so we ignore the page_size
        // parameter.
        if self.title.is_some_and(|t| !t.is_empty()) {
            if let Some(page_size) = self.page_size {
                query = query.paginate(page_size, 0)
            }
        }

        query
    }
}

impl From<Vec<(MetaFolder, MetaDashboard)>> for ListDashboardsResponseBody {
    fn from(value: Vec<(MetaFolder, MetaDashboard)>) -> Self {
        let dashboards = value.into_iter().map(|fd| fd.into()).collect();
        Self { dashboards }
    }
}

impl From<(MetaFolder, MetaDashboard)> for ListDashboardsResponseBodyItem {
    #[allow(deprecated)]
    fn from(value: (MetaFolder, MetaDashboard)) -> Self {
        let folder = value.0;
        let dashboard = value.1;
        Self {
            folder_id: folder.folder_id,
            folder_name: folder.name,
            dashboard_id: dashboard.dashboard_id().unwrap_or_default().to_owned(),
            title: dashboard.title().unwrap_or_default().to_owned(),
            description: dashboard.description().unwrap_or_default().to_owned(),
            role: dashboard.role().unwrap_or_default().to_owned(),
            owner: dashboard.owner().unwrap_or_default().to_owned(),
            // TODO: Return timestamp in microseconds just like all other apis
            created: dashboard.created_at_deprecated().unwrap_or_else(|| {
                Utc::now().with_timezone(
                    &FixedOffset::east_opt(0).expect("Out of bounds timezone difference"),
                )
            }),
            hash: dashboard.hash,
            version: dashboard.version,
            updated_at: dashboard.updated_at,
            // Populate deprecated fields until they are removed from the API.
            v1: dashboard.v1,
            v2: dashboard.v2,
            v3: dashboard.v3,
            v4: dashboard.v4,
            v5: dashboard.v5,
        }
    }
}

impl From<MetaDashboard> for DashboardDetails {
    fn from(value: MetaDashboard) -> Self {
        Self {
            version: value.version,
            v1: value.v1,
            v2: value.v2,
            v3: value.v3,
            v4: value.v4,
            v5: value.v5,
            hash: value.hash,
            updated_at: value.updated_at,
        }
    }
}

/// Parses the JSON value from an HTTP request body into a dashboard.
fn parse_dashboard_request(value: JsonValue) -> Result<MetaDashboard, serde_json::Error> {
    // Pull the version field out of the JSON. If it doesn't exist, assume the
    // default version is 1.
    let version = value
        .as_object()
        .and_then(|o| o.get("version"))
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    let dash = match version {
        1 => {
            let inner: v1::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
        2 => {
            let inner: v2::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
        3 => {
            let inner: v3::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
        4 => {
            let inner: v4::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
        _ => {
            let inner: v5::Dashboard = serde_json::from_value(value)?;
            inner.into()
        }
    };
    Ok(dash)
}
