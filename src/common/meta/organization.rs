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

use config::meta::user::UserRole;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[cfg(feature = "cloud")]
use super::user::InviteStatus;

pub const DEFAULT_ORG: &str = "default";
pub const CUSTOM: &str = "custom";
pub const USER_DEFAULT: &str = "user_default";
pub const THRESHOLD: i64 = 9383939382;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct Organization {
    #[serde(default)]
    pub identifier: String,
    pub name: String,
    #[serde(default)]
    pub org_type: String,
}

#[cfg(feature = "cloud")]
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct OrganizationInvites {
    #[serde(default)]
    pub invites: Vec<String>, // user emails
    pub role: UserRole,
}

#[cfg(feature = "cloud")]
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct OrganizationInviteResponse {
    #[serde(default)]
    pub data: OrganizationInviteResponseData, // user emails
    pub message: String,
}

#[cfg(feature = "cloud")]
#[derive(Serialize, Deserialize, Default, ToSchema, Clone, Debug)]
pub struct OrganizationInviteResponseData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_members: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_members: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_email: Option<String>,
}

#[cfg(feature = "cloud")]
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct OrganizationInviteUserRecord {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub status: InviteStatus,
    pub expires_at: i64,
    pub is_external: bool,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct OrgRoleMapping {
    pub org_id: String,
    pub org_name: String,
    pub role: UserRole,
}

#[derive(Serialize, Clone, ToSchema)]
pub struct OrgUser {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Serialize, ToSchema)]
pub struct OrgDetails {
    pub id: i64,
    pub identifier: String,
    pub name: String,
    pub user_email: String,
    pub ingest_threshold: i64,
    pub search_threshold: i64,
    #[serde(rename = "type")]
    pub org_type: String,
    #[serde(rename = "UserObj")]
    pub user_obj: OrgUser,
    #[serde(default)]
    pub plan: i32,
}

#[derive(Serialize, ToSchema)]
pub struct OrganizationResponse {
    pub data: Vec<OrgDetails>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OrgSummary {
    pub streams: StreamSummary,
    pub pipelines: PipelineSummary,
    pub alerts: AlertSummary,
    pub total_functions: i64,
    pub total_dashboards: i64,
}

#[derive(Default, Serialize, Deserialize, ToSchema)]
pub struct StreamSummary {
    pub num_streams: i64,
    pub total_records: i64,
    pub total_storage_size: f64,
    pub total_compressed_size: f64,
    pub total_index_size: f64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PipelineSummary {
    pub num_realtime: i64,
    pub num_scheduled: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AlertSummary {
    pub num_realtime: i64,
    pub num_scheduled: i64,
}

/// A container for passcodes and rumtokens
#[derive(Serialize, ToSchema)]
pub enum IngestionTokensContainer {
    Passcode(IngestionPasscode),
    RumToken(RumIngestionToken),
}

#[derive(Serialize, ToSchema)]
pub struct IngestionPasscode {
    pub passcode: String,
    pub user: String,
}

#[derive(Serialize, ToSchema)]
pub struct PasscodeResponse {
    pub data: IngestionPasscode,
}

#[derive(Serialize, ToSchema)]
pub struct RumIngestionToken {
    pub user: String,
    pub rum_token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct RumIngestionResponse {
    pub data: RumIngestionToken,
}

fn default_scrape_interval() -> u32 {
    config::get_config().common.default_scrape_interval
}

fn default_auto_refresh_interval() -> u32 {
    config::get_config().common.min_auto_refresh_interval
}

fn default_trace_id_field_name() -> String {
    "trace_id".to_string()
}

fn default_span_id_field_name() -> String {
    "span_id".to_string()
}

fn default_toggle_ingestion_logs() -> bool {
    false
}

fn default_enable_websocket_search() -> bool {
    false
}

#[derive(Serialize, ToSchema, Deserialize, Debug, Clone)]
pub struct OrganizationSettingPayload {
    /// Ideally this should be the same as prometheus-scrape-interval (in
    /// seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scrape_interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id_field_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id_field_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggle_ingestion_logs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_websocket_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_auto_refresh_interval: Option<u32>,
}

#[derive(Serialize, ToSchema, Deserialize, Debug, Clone)]
pub struct OrganizationSetting {
    /// Ideally this should be the same as prometheus-scrape-interval (in
    /// seconds).
    #[serde(default = "default_scrape_interval")]
    pub scrape_interval: u32,
    #[serde(default = "default_trace_id_field_name")]
    pub trace_id_field_name: String,
    #[serde(default = "default_span_id_field_name")]
    pub span_id_field_name: String,
    #[serde(default = "default_toggle_ingestion_logs")]
    pub toggle_ingestion_logs: bool,
    #[serde(default = "default_enable_websocket_search")]
    pub enable_websocket_search: bool,
    #[serde(default = "default_auto_refresh_interval")]
    pub min_auto_refresh_interval: u32,
}

impl Default for OrganizationSetting {
    fn default() -> Self {
        Self {
            scrape_interval: default_scrape_interval(),
            trace_id_field_name: default_trace_id_field_name(),
            span_id_field_name: default_span_id_field_name(),
            toggle_ingestion_logs: default_toggle_ingestion_logs(),
            enable_websocket_search: default_enable_websocket_search(),
            min_auto_refresh_interval: default_auto_refresh_interval(),
        }
    }
}

#[derive(Serialize, ToSchema, Deserialize, Debug, Clone)]
pub struct OrganizationSettingResponse {
    pub data: OrganizationSetting,
}
