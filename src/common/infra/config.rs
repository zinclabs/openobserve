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

use std::sync::Arc;

use config::{
    meta::{
        alerts::{alert::Alert, destinations::Destination, templates::Template},
        dashboards::reports,
        function::Transform,
        promql::ClusterLeader,
        stream::StreamParams,
        user::User,
    },
    RwAHashMap, RwHashMap,
};
use dashmap::DashMap;
use hashbrown::HashMap;
use infra::table::short_urls::ShortUrlRecord;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use vector_enrichment::TableRegistry;

use crate::{
    common::meta::{
        maxmind::MaxmindClient,
        organization::{Organization, OrganizationSetting},
        syslog::SyslogRoute,
    },
    handler::http::request::websocket::session::WsSession,
    service::{
        db::scheduler as db_scheduler, enrichment::StreamTable, enrichment_table::geoip::Geoip,
        pipeline::batch_execution::ExecutablePipeline,
    },
};

// global version variables
pub static VERSION: &str = env!("GIT_VERSION");
pub static COMMIT_HASH: &str = env!("GIT_COMMIT_HASH");
pub static BUILD_DATE: &str = env!("GIT_BUILD_DATE");

// global cache variables
pub static KVS: Lazy<RwHashMap<String, bytes::Bytes>> = Lazy::new(Default::default);
pub static QUERY_FUNCTIONS: Lazy<RwHashMap<String, Transform>> = Lazy::new(DashMap::default);
pub static USERS: Lazy<RwHashMap<String, infra::table::users::UserRecord>> =
    Lazy::new(DashMap::default);
pub static ORG_USERS: Lazy<RwHashMap<String, infra::table::org_users::OrgUserRecord>> =
    Lazy::new(DashMap::default);
pub static USERS_RUM_TOKEN: Lazy<Arc<RwHashMap<String, infra::table::org_users::OrgUserRecord>>> =
    Lazy::new(|| Arc::new(DashMap::default()));
pub static ROOT_USER: Lazy<RwHashMap<String, User>> = Lazy::new(DashMap::default);
pub static ORGANIZATION_SETTING: Lazy<Arc<RwAHashMap<String, OrganizationSetting>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())));
pub static ORGANIZATIONS: Lazy<Arc<RwAHashMap<String, Organization>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())));
pub static PASSWORD_HASH: Lazy<RwHashMap<String, String>> = Lazy::new(DashMap::default);
pub static METRIC_CLUSTER_MAP: Lazy<Arc<RwAHashMap<String, Vec<String>>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())));
pub static METRIC_CLUSTER_LEADER: Lazy<Arc<RwAHashMap<String, ClusterLeader>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())));
pub static STREAM_ALERTS: Lazy<RwAHashMap<String, Vec<Alert>>> = Lazy::new(Default::default);
pub static REALTIME_ALERT_TRIGGERS: Lazy<RwAHashMap<String, db_scheduler::Trigger>> =
    Lazy::new(Default::default);
pub static ALERTS_TEMPLATES: Lazy<RwHashMap<String, Template>> = Lazy::new(Default::default);
pub static ALERTS_DESTINATIONS: Lazy<RwHashMap<String, Destination>> = Lazy::new(Default::default);
pub static DASHBOARD_REPORTS: Lazy<RwHashMap<String, reports::Report>> =
    Lazy::new(Default::default);
pub static SYSLOG_ROUTES: Lazy<RwHashMap<String, SyslogRoute>> = Lazy::new(Default::default);
pub static SYSLOG_ENABLED: Lazy<Arc<RwLock<bool>>> = Lazy::new(|| Arc::new(RwLock::new(false)));
pub static ENRICHMENT_TABLES: Lazy<RwHashMap<String, StreamTable>> = Lazy::new(Default::default);
pub static ENRICHMENT_REGISTRY: Lazy<Arc<TableRegistry>> =
    Lazy::new(|| Arc::new(TableRegistry::default()));

pub static MAXMIND_DB_CLIENT: Lazy<Arc<tokio::sync::RwLock<Option<MaxmindClient>>>> =
    Lazy::new(|| Arc::new(tokio::sync::RwLock::new(None)));

pub static GEOIP_CITY_TABLE: Lazy<Arc<RwLock<Option<Geoip>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

pub static GEOIP_ASN_TABLE: Lazy<Arc<RwLock<Option<Geoip>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

#[cfg(feature = "enterprise")]
pub static GEOIP_ENT_TABLE: Lazy<Arc<RwLock<Option<Geoip>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

pub static STREAM_EXECUTABLE_PIPELINES: Lazy<RwAHashMap<StreamParams, ExecutablePipeline>> =
    Lazy::new(Default::default);
pub static USER_SESSIONS: Lazy<RwHashMap<String, String>> = Lazy::new(Default::default);
pub static SHORT_URLS: Lazy<RwHashMap<String, ShortUrlRecord>> = Lazy::new(DashMap::default);
// TODO: Implement rate limiting for maximum number of sessions
pub static WS_SESSIONS: Lazy<RwHashMap<String, WsSession>> = Lazy::new(DashMap::default);
