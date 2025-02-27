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

//! These models define the schemas of HTTP request and response JSON bodies in
//! lookup API endpoints.

use config::meta::stream::StreamType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// HTTP URL query component that contains parameters for lookup.
#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams, ToSchema)]
#[into_params(style = Form, parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct LookupRequestQuery {
    pub resource: Resource,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_type: Option<StreamType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_schema: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashboard_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_idx: Option<usize>,
}

/// The resource type to lookup.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Resource {
    Alert,
    Dashboard,
    Report,
    Pipeline,
    Function,
    Stream,
}

impl Resource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Resource::Alert => "alert",
            Resource::Dashboard => "dashboard",
            Resource::Report => "report",
            Resource::Pipeline => "pipeline",
            Resource::Function => "function",
            Resource::Stream => "stream",
        }
    }
}

/// A paginated response containing a list of items and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination_metadata: Option<PaginationMetadata>,
}

/// Metadata about the pagination of a paginated response.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct PaginationMetadata {
    pub total_items: usize,
    pub total_pages: usize,
    pub current_page: usize,
    pub page_size: usize,
}
