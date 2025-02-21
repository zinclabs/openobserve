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

use std::{cmp::max, collections::BTreeMap, path::Path, sync::Arc, time::Duration};

use aes_siv::{siv::Aes256Siv, KeyInit};
use arc_swap::ArcSwap;
use base64::{prelude::BASE64_STANDARD, Engine};
use chromiumoxide::{browser::BrowserConfig, handler::viewport::Viewport};
use dotenv_config::EnvConfig;
use dotenvy::dotenv_override;
use hashbrown::{HashMap, HashSet};
use itertools::chain;
use lettre::{
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, Tokio1Executor,
};
use once_cell::sync::Lazy;
use sysinfo::{DiskExt, SystemExt};

use crate::{
    meta::cluster,
    utils::{cgroup, file::get_file_meta},
};

pub type FxIndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;
pub type FxIndexSet<K> = indexmap::IndexSet<K, ahash::RandomState>;
pub type RwHashMap<K, V> = dashmap::DashMap<K, V, ahash::RandomState>;
pub type RwHashSet<K> = dashmap::DashSet<K, ahash::RandomState>;
pub type RwAHashMap<K, V> = tokio::sync::RwLock<HashMap<K, V>>;
pub type RwAHashSet<K> = tokio::sync::RwLock<HashSet<K>>;
pub type RwBTreeMap<K, V> = tokio::sync::RwLock<BTreeMap<K, V>>;

pub const MMDB_CITY_FILE_NAME: &str = "GeoLite2-City.mmdb";
pub const MMDB_ASN_FILE_NAME: &str = "GeoLite2-ASN.mmdb";
pub const GEO_IP_CITY_ENRICHMENT_TABLE: &str = "maxmind_city";
pub const GEO_IP_ASN_ENRICHMENT_TABLE: &str = "maxmind_asn";

pub const SIZE_IN_MB: f64 = 1024.0 * 1024.0;
pub const SIZE_IN_GB: f64 = 1024.0 * 1024.0 * 1024.0;
pub const PARQUET_BATCH_SIZE: usize = 8 * 1024;
pub const PARQUET_PAGE_SIZE: usize = 1024 * 1024;
pub const PARQUET_MAX_ROW_GROUP_SIZE: usize = 1024 * 1024; // this can't be change, it will cause segment matching error
pub const INDEX_SEGMENT_LENGTH: usize = 1024; // this can't be change, it will cause segment matching error
pub const DEFAULT_BLOOM_FILTER_FPP: f64 = 0.01;

pub const FILE_EXT_JSON: &str = ".json";
pub const FILE_EXT_ARROW: &str = ".arrow";
pub const FILE_EXT_PARQUET: &str = ".parquet";
pub const FILE_EXT_PUFFIN: &str = ".puffin";
pub const FILE_EXT_TANTIVY: &str = ".ttv";
pub const FILE_EXT_TANTIVY_FOLDER: &str = ".mmap";

pub const INDEX_FIELD_NAME_FOR_ALL: &str = "_all";

pub const INDEX_MIN_CHAR_LEN: usize = 3;
pub const QUERY_WITH_NO_LIMIT: i32 = -999;

pub const REQUIRED_DB_CONNECTIONS: u32 = 4;

// Columns added to ingested records for _INTERNAL_ use only.
// Used for storing and querying unflattened original data
pub const ORIGINAL_DATA_COL_NAME: &str = "_original";
pub const ID_COL_NAME: &str = "_o2_id";
pub const TIMESTAMP_COL_NAME: &str = "_timestamp";

const _DEFAULT_SQL_FULL_TEXT_SEARCH_FIELDS: [&str; 7] =
    ["log", "message", "msg", "content", "data", "body", "json"];
pub static SQL_FULL_TEXT_SEARCH_FIELDS: Lazy<Vec<String>> = Lazy::new(|| {
    let mut fields = chain(
        _DEFAULT_SQL_FULL_TEXT_SEARCH_FIELDS
            .iter()
            .map(|s| s.to_string()),
        get_config()
            .common
            .feature_fulltext_extra_fields
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }),
    )
    .collect::<Vec<_>>();
    fields.sort();
    fields.dedup();
    fields
});

pub static SQL_SECONDARY_INDEX_SEARCH_FIELDS: Lazy<Vec<String>> = Lazy::new(|| {
    let mut fields = get_config()
        .common
        .feature_secondary_index_extra_fields
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect::<Vec<_>>();
    fields.sort();
    fields.dedup();
    fields
});

pub static QUICK_MODEL_FIELDS: Lazy<Vec<String>> = Lazy::new(|| {
    let mut fields = get_config()
        .common
        .feature_quick_mode_fields
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect::<Vec<_>>();
    fields.sort();
    fields.dedup();
    fields
});

const _DEFAULT_DISTINCT_FIELDS: [&str; 2] = ["service_name", "operation_name"];
pub static DISTINCT_FIELDS: Lazy<Vec<String>> = Lazy::new(|| {
    let mut fields = chain(
        _DEFAULT_DISTINCT_FIELDS.iter().map(|s| s.to_string()),
        get_config()
            .common
            .feature_distinct_extra_fields
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }),
    )
    .collect::<Vec<_>>();
    fields.sort();
    fields.dedup();
    fields
});

const _DEFAULT_BLOOM_FILTER_FIELDS: [&str; 1] = ["trace_id"];
pub static BLOOM_FILTER_DEFAULT_FIELDS: Lazy<Vec<String>> = Lazy::new(|| {
    let mut fields = chain(
        _DEFAULT_BLOOM_FILTER_FIELDS.iter().map(|s| s.to_string()),
        get_config()
            .common
            .bloom_filter_default_fields
            .split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }),
    )
    .collect::<Vec<_>>();
    fields.sort();
    fields.dedup();
    fields
});

pub static MEM_TABLE_INDIVIDUAL_STREAMS: Lazy<HashMap<String, usize>> = Lazy::new(|| {
    let mut map = HashMap::default();
    let streams: Vec<String> = get_config()
        .common
        .mem_table_individual_streams
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect();
    let num_mem_tables = get_config().limit.mem_table_bucket_num;
    for stream in streams.into_iter() {
        if map.contains_key(&stream) {
            continue;
        }
        map.insert(stream, num_mem_tables + map.len());
    }
    map
});

pub static CONFIG: Lazy<ArcSwap<Config>> = Lazy::new(|| ArcSwap::from(Arc::new(init())));
static INSTANCE_ID: Lazy<RwHashMap<String, String>> = Lazy::new(Default::default);

pub static TELEMETRY_CLIENT: Lazy<segment::HttpClient> = Lazy::new(|| {
    segment::HttpClient::new(
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
        CONFIG.load().common.telemetry_url.clone(),
    )
});

pub fn get_config() -> Arc<Config> {
    CONFIG.load().clone()
}

pub fn refresh_config() -> Result<(), anyhow::Error> {
    CONFIG.store(Arc::new(init()));
    Ok(())
}

pub fn cache_instance_id(instance_id: &str) {
    INSTANCE_ID.insert("instance_id".to_owned(), instance_id.to_owned());
}

pub fn get_instance_id() -> String {
    match INSTANCE_ID.get("instance_id") {
        Some(id) => id.clone(),
        None => "".to_string(),
    }
}

static CHROME_LAUNCHER_OPTIONS: tokio::sync::OnceCell<Option<BrowserConfig>> =
    tokio::sync::OnceCell::const_new();

pub async fn get_chrome_launch_options() -> &'static Option<BrowserConfig> {
    CHROME_LAUNCHER_OPTIONS
        .get_or_init(init_chrome_launch_options)
        .await
}

async fn init_chrome_launch_options() -> Option<BrowserConfig> {
    let cfg = get_config();
    if !cfg.chrome.chrome_enabled || !cfg.common.report_server_url.is_empty() {
        None
    } else {
        let mut browser_config = BrowserConfig::builder()
            .window_size(
                cfg.chrome.chrome_window_width,
                cfg.chrome.chrome_window_height,
            )
            .viewport(Viewport {
                width: cfg.chrome.chrome_window_width,
                height: cfg.chrome.chrome_window_height,
                device_scale_factor: Some(1.0),
                ..Viewport::default()
            });

        if cfg.chrome.chrome_with_head {
            browser_config = browser_config.with_head();
        }

        if cfg.chrome.chrome_no_sandbox {
            browser_config = browser_config.no_sandbox();
        }

        if !cfg.chrome.chrome_path.is_empty() {
            browser_config = browser_config.chrome_executable(cfg.chrome.chrome_path.as_str());
        } else {
            panic!("Chrome path must be specified");
        }
        Some(browser_config.build().unwrap())
    }
}

pub static SMTP_CLIENT: Lazy<Option<AsyncSmtpTransport<Tokio1Executor>>> = Lazy::new(|| {
    let cfg = get_config();
    if !cfg.smtp.smtp_enabled {
        None
    } else {
        let tls_parameters = TlsParameters::new(cfg.smtp.smtp_host.clone()).unwrap();
        let mut transport_builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.smtp.smtp_host)
                .port(cfg.smtp.smtp_port);

        let option = &cfg.smtp.smtp_encryption;
        transport_builder = if option == "starttls" {
            transport_builder.tls(Tls::Required(tls_parameters))
        } else if option == "ssltls" {
            transport_builder.tls(Tls::Wrapper(tls_parameters))
        } else {
            transport_builder
        };

        if !cfg.smtp.smtp_username.is_empty() && !cfg.smtp.smtp_password.is_empty() {
            transport_builder = transport_builder.credentials(Credentials::new(
                cfg.smtp.smtp_username.clone(),
                cfg.smtp.smtp_password.clone(),
            ));
        }
        Some(transport_builder.build())
    }
});

static SNS_CLIENT: tokio::sync::OnceCell<aws_sdk_sns::Client> = tokio::sync::OnceCell::const_new();

async fn init_sns_client() -> aws_sdk_sns::Client {
    let cfg = get_config();
    let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;

    let sns_config = aws_sdk_sns::config::Builder::from(&shared_config)
        .endpoint_url(cfg.sns.endpoint.clone())
        .timeout_config(
            aws_config::timeout::TimeoutConfig::builder()
                .connect_timeout(std::time::Duration::from_secs(cfg.sns.connect_timeout))
                .operation_timeout(std::time::Duration::from_secs(cfg.sns.operation_timeout))
                .build(),
        )
        .build();

    aws_sdk_sns::Client::from_conf(sns_config)
}

pub async fn get_sns_client() -> &'static aws_sdk_sns::Client {
    SNS_CLIENT.get_or_init(init_sns_client).await
}

pub static BLOCKED_STREAMS: Lazy<Vec<String>> = Lazy::new(|| {
    let blocked_streams = get_config()
        .common
        .blocked_streams
        .split(',')
        .map(|x| x.to_string())
        .collect();
    blocked_streams
});

#[derive(EnvConfig)]
pub struct Config {
    pub auth: Auth,
    pub report_server: ReportServer,
    pub http: Http,
    pub grpc: Grpc,
    pub route: Route,
    pub common: Common,
    pub limit: Limit,
    pub compact: Compact,
    pub memory_cache: MemoryCache,
    pub disk_cache: DiskCache,
    pub log: Log,
    pub etcd: Etcd,
    pub nats: Nats,
    pub s3: S3,
    pub sns: Sns,
    pub tcp: TCP,
    pub prom: Prometheus,
    pub profiling: Profiling,
    pub smtp: Smtp,
    pub rum: RUM,
    pub chrome: Chrome,
    pub tokio_console: TokioConsole,
    pub pipeline: Pipeline,
    pub health_check: HealthCheck,
    pub encryption: Encryption,
}

#[derive(EnvConfig)]
pub struct ReportServer {
    #[env_config(name = "ZO_ENABLE_EMBEDDED_REPORT_SERVER", default = false)]
    pub enable_report_server: bool,
    #[env_config(name = "ZO_REPORT_USER_EMAIL", default = "")]
    pub user_email: String,
    #[env_config(name = "ZO_REPORT_USER_PASSWORD", default = "")]
    pub user_password: String,
    #[env_config(name = "ZO_REPORT_SERVER_HTTP_PORT", default = 5082)]
    pub port: u16,
    #[env_config(name = "ZO_REPORT_SERVER_HTTP_ADDR", default = "127.0.0.1")]
    pub addr: String,
    #[env_config(name = "ZO_HTTP_IPV6_ENABLED", default = false)]
    pub ipv6_enabled: bool,
}

#[derive(EnvConfig)]
pub struct TokioConsole {
    #[env_config(name = "ZO_TOKIO_CONSOLE_SERVER_ADDR", default = "0.0.0.0")]
    pub tokio_console_server_addr: String,
    #[env_config(name = "ZO_TOKIO_CONSOLE_SERVER_PORT", default = 6699)]
    pub tokio_console_server_port: u16,
    #[env_config(name = "ZO_TOKIO_CONSOLE_RETENTION", default = 60)]
    pub tokio_console_retention: u64,
}

#[derive(EnvConfig)]
pub struct Chrome {
    #[env_config(name = "ZO_CHROME_ENABLED", default = false)]
    pub chrome_enabled: bool,
    #[env_config(name = "ZO_CHROME_PATH", default = "")]
    pub chrome_path: String,
    #[env_config(name = "ZO_CHROME_CHECK_DEFAULT_PATH", default = true)]
    pub chrome_check_default: bool,
    #[env_config(name = "ZO_CHROME_AUTO_DOWNLOAD", default = false)]
    pub chrome_auto_download: bool,
    #[env_config(name = "ZO_CHROME_DOWNLOAD_PATH", default = "./download")]
    pub chrome_download_path: String,
    #[env_config(name = "ZO_CHROME_NO_SANDBOX", default = false)]
    pub chrome_no_sandbox: bool,
    #[env_config(name = "ZO_CHROME_WITH_HEAD", default = false)]
    pub chrome_with_head: bool,
    #[env_config(name = "ZO_CHROME_SLEEP_SECS", default = 20)]
    pub chrome_sleep_secs: u16,
    #[env_config(name = "ZO_CHROME_WINDOW_WIDTH", default = 1370)]
    pub chrome_window_width: u32,
    #[env_config(name = "ZO_CHROME_WINDOW_HEIGHT", default = 730)]
    pub chrome_window_height: u32,
}

#[derive(EnvConfig)]
pub struct Smtp {
    #[env_config(name = "ZO_SMTP_ENABLED", default = false)]
    pub smtp_enabled: bool,
    #[env_config(name = "ZO_SMTP_HOST", default = "localhost")]
    pub smtp_host: String,
    #[env_config(name = "ZO_SMTP_PORT", default = 25)]
    pub smtp_port: u16,
    #[env_config(name = "ZO_SMTP_USER_NAME", default = "")]
    pub smtp_username: String,
    #[env_config(name = "ZO_SMTP_PASSWORD", default = "")]
    pub smtp_password: String,
    #[env_config(name = "ZO_SMTP_REPLY_TO", default = "")]
    pub smtp_reply_to: String,
    #[env_config(name = "ZO_SMTP_FROM_EMAIL", default = "")]
    pub smtp_from_email: String,
    #[env_config(name = "ZO_SMTP_ENCRYPTION", default = "")]
    pub smtp_encryption: String,
}

#[derive(EnvConfig)]
pub struct Profiling {
    #[env_config(
        name = "ZO_PROF_PPROF_ENABLED",
        default = false,
        help = "Enable pprof profiling with pprof-rs"
    )]
    pub pprof_enabled: bool,
    #[env_config(
        name = "ZO_PROF_PPROF_PROTOBUF_ENABLED",
        default = false,
        help = "Enable pprof profiling with pprof-rs encode to protobuf format"
    )]
    pub pprof_protobuf_enabled: bool,
    #[env_config(
        name = "ZO_PROF_PPROF_FLAMEGRAPH_PATH",
        default = "",
        help = "Path to save flamegraph"
    )]
    pub pprof_flamegraph_path: String,
    #[env_config(
        name = "ZO_PROF_PYROSCOPE_ENABLED",
        default = false,
        help = "Enable pyroscope profiling with pyroscope-rs"
    )]
    pub pyroscope_enabled: bool,
    #[env_config(
        name = "ZO_PROF_PYROSCOPE_SERVER_URL",
        default = "http://localhost:4040",
        help = "Pyroscope server URL"
    )]
    pub pyroscope_server_url: String,
    #[env_config(
        name = "ZO_PROF_PYROSCOPE_PROJECT_NAME",
        default = "openobserve",
        help = "Pyroscope project name"
    )]
    pub pyroscope_project_name: String,
}

#[derive(EnvConfig)]
pub struct Auth {
    #[env_config(name = "ZO_ROOT_USER_EMAIL")]
    pub root_user_email: String,
    #[env_config(name = "ZO_ROOT_USER_PASSWORD")]
    pub root_user_password: String,
    #[env_config(name = "ZO_COOKIE_MAX_AGE", default = 2592000)] // seconds, 30 days
    pub cookie_max_age: i64,
    #[env_config(name = "ZO_COOKIE_SAME_SITE_LAX", default = true)]
    pub cookie_same_site_lax: bool,
    #[env_config(name = "ZO_COOKIE_SECURE_ONLY", default = false)]
    pub cookie_secure_only: bool,
    #[env_config(name = "ZO_EXT_AUTH_SALT", default = "openobserve")]
    pub ext_auth_salt: String,
    #[env_config(name = "O2_SCRIPT_SERVER_TOKEN")]
    pub script_server_token: String,
}

#[derive(EnvConfig)]
pub struct Http {
    #[env_config(name = "ZO_HTTP_PORT", default = 5080)]
    pub port: u16,
    #[env_config(name = "ZO_HTTP_ADDR", default = "")]
    pub addr: String,
    #[env_config(name = "ZO_HTTP_IPV6_ENABLED", default = false)]
    pub ipv6_enabled: bool,
    #[env_config(name = "ZO_HTTP_TLS_ENABLED", default = false)]
    pub tls_enabled: bool,
    #[env_config(name = "ZO_HTTP_TLS_CERT_PATH", default = "")]
    pub tls_cert_path: String,
    #[env_config(name = "ZO_HTTP_TLS_KEY_PATH", default = "")]
    pub tls_key_path: String,
    #[env_config(name = "ZO_HTTP_TLS_MIN_VERSION", default = "", help = "Supported values: "1.2" or "1.3", default is all_version")]
    pub tls_min_version: String,
    #[env_config(
        name = "ZO_HTTP_TLS_ROOT_CERTIFICATES",
        default = "webpki",
        help = "this value must use webpki or native. it means use standard root certificates from webpki-roots or native-roots as a rustls certificate store"
    )]
    pub tls_root_certificates: String,
}

#[derive(EnvConfig)]
pub struct Grpc {
    #[env_config(name = "ZO_GRPC_PORT", default = 5081)]
    pub port: u16,
    #[env_config(name = "ZO_GRPC_ADDR", default = "")]
    pub addr: String,
    #[env_config(name = "ZO_GRPC_ORG_HEADER_KEY", default = "organization")]
    pub org_header_key: String,
    #[env_config(name = "ZO_GRPC_STREAM_HEADER_KEY", default = "stream-name")]
    pub stream_header_key: String,
    #[env_config(name = "ZO_INTERNAL_GRPC_TOKEN", default = "")]
    pub internal_grpc_token: String,
    #[env_config(
        name = "ZO_GRPC_MAX_MESSAGE_SIZE",
        default = 16,
        help = "Max grpc message size in MB, default is 16 MB"
    )]
    pub max_message_size: usize,
    #[env_config(name = "ZO_GRPC_CONNECT_TIMEOUT", default = 5)] // in seconds
    pub connect_timeout: u64,
    #[env_config(name = "ZO_GRPC_CHANNEL_CACHE_DISABLED", default = false)]
    pub channel_cache_disabled: bool,
    #[env_config(name = "ZO_GRPC_TLS_ENABLED", default = false)]
    pub tls_enabled: bool,
    #[env_config(name = "ZO_GRPC_TLS_CERT_DOMAIN", default = "")]
    pub tls_cert_domain: String,
    #[env_config(name = "ZO_GRPC_TLS_CERT_PATH", default = "")]
    pub tls_cert_path: String,
    #[env_config(name = "ZO_GRPC_TLS_KEY_PATH", default = "")]
    pub tls_key_path: String,
}

#[derive(EnvConfig)]
pub struct TCP {
    #[env_config(name = "ZO_TCP_PORT", default = 5514)]
    pub tcp_port: u16,
    #[env_config(name = "ZO_UDP_PORT", default = 5514)]
    pub udp_port: u16,
}

#[derive(EnvConfig)]
pub struct Route {
    #[env_config(name = "ZO_ROUTE_TIMEOUT", default = 600)]
    pub timeout: u64,
    #[env_config(name = "ZO_ROUTE_MAX_CONNECTIONS", default = 1024)]
    pub max_connections: usize,
}

#[derive(EnvConfig)]
pub struct Common {
    #[env_config(name = "ZO_APP_NAME", default = "openobserve")]
    pub app_name: String,
    #[env_config(name = "ZO_LOCAL_MODE", default = true)]
    pub local_mode: bool,
    // ZO_LOCAL_MODE_STORAGE is ignored when ZO_LOCAL_MODE is set to false
    #[env_config(name = "ZO_LOCAL_MODE_STORAGE", default = "disk")]
    pub local_mode_storage: String,
    #[env_config(name = "ZO_CLUSTER_COORDINATOR", default = "etcd")]
    pub cluster_coordinator: String,
    #[env_config(name = "ZO_QUEUE_STORE", default = "")]
    pub queue_store: String,
    #[env_config(name = "ZO_META_STORE", default = "")]
    pub meta_store: String,
    #[env_config(name = "ZO_META_POSTGRES_DSN", default = "")]
    pub meta_postgres_dsn: String, // postgres://postgres:12345678@localhost:5432/openobserve
    #[env_config(name = "ZO_META_MYSQL_DSN", default = "")]
    pub meta_mysql_dsn: String, // mysql://root:12345678@localhost:3306/openobserve
    #[env_config(name = "ZO_NODE_ROLE", default = "all")]
    pub node_role: String,
    #[env_config(
        name = "ZO_NODE_ROLE_GROUP",
        default = "",
        help = "Role group can be empty (default), interactive, or background"
    )]
    pub node_role_group: String,
    #[env_config(name = "ZO_CLUSTER_NAME", default = "zo1")]
    pub cluster_name: String,
    #[env_config(name = "ZO_INSTANCE_NAME", default = "")]
    pub instance_name: String,
    pub instance_name_short: String,
    #[env_config(name = "ZO_WEB_URL", default = "http://localhost:5080")]
    pub web_url: String,
    #[env_config(name = "ZO_BASE_URI", default = "")] // /abc
    pub base_uri: String,
    #[env_config(name = "ZO_DATA_DIR", default = "./data/openobserve/")]
    pub data_dir: String,
    #[env_config(name = "ZO_DATA_WAL_DIR", default = "")] // ./data/openobserve/wal/
    pub data_wal_dir: String,
    #[env_config(name = "ZO_DATA_STREAM_DIR", default = "")] // ./data/openobserve/stream/
    pub data_stream_dir: String,
    #[env_config(name = "ZO_DATA_DB_DIR", default = "")] // ./data/openobserve/db/
    pub data_db_dir: String,
    #[env_config(name = "ZO_DATA_CACHE_DIR", default = "")] // ./data/openobserve/cache/
    pub data_cache_dir: String,
    // TODO: should rename to column_all
    #[env_config(name = "ZO_CONCATENATED_SCHEMA_FIELD_NAME", default = "_all")]
    pub column_all: String,
    #[env_config(name = "ZO_FEATURE_PER_THREAD_LOCK", default = false)]
    pub feature_per_thread_lock: bool,
    #[env_config(name = "ZO_FEATURE_FULLTEXT_EXTRA_FIELDS", default = "")]
    pub feature_fulltext_extra_fields: String,
    #[env_config(name = "ZO_FEATURE_INDEX_EXTRA_FIELDS", default = "")]
    pub feature_secondary_index_extra_fields: String,
    #[env_config(name = "ZO_FEATURE_DISTINCT_EXTRA_FIELDS", default = "")]
    pub feature_distinct_extra_fields: String,
    #[env_config(name = "ZO_FEATURE_QUICK_MODE_FIELDS", default = "")]
    pub feature_quick_mode_fields: String,
    #[env_config(name = "ZO_FEATURE_FILELIST_DEDUP_ENABLED", default = false)]
    pub feature_filelist_dedup_enabled: bool,
    #[env_config(name = "ZO_FEATURE_QUERY_QUEUE_ENABLED", default = true)]
    pub feature_query_queue_enabled: bool,
    #[env_config(name = "ZO_FEATURE_QUERY_PARTITION_STRATEGY", default = "file_num")]
    pub feature_query_partition_strategy: String,
    #[env_config(name = "ZO_FEATURE_QUERY_INFER_SCHEMA", default = false)]
    pub feature_query_infer_schema: bool,
    #[env_config(name = "ZO_FEATURE_QUERY_EXCLUDE_ALL", default = true)]
    pub feature_query_exclude_all: bool,
    #[env_config(name = "ZO_FEATURE_QUERY_WITHOUT_INDEX", default = false)]
    pub feature_query_without_index: bool,
    #[env_config(name = "ZO_FEATURE_QUERY_REMOVE_FILTER_WITH_INDEX", default = true)]
    pub feature_query_remove_filter_with_index: bool,
    #[env_config(name = "ZO_FEATURE_QUERY_STREAMING_AGGS", default = false)]
    pub feature_query_streaming_aggs: bool,
    #[env_config(name = "ZO_FEATURE_JOIN_MATCH_ONE_ENABLED", default = false)]
    pub feature_join_match_one_enabled: bool,
    #[env_config(
        name = "ZO_FEATURE_JOIN_RIGHT_SIDE_MAX_ROWS",
        default = 0,
        help = "Default to 50_000 when ZO_FEATURE_JOIN_MATCH_ONE_ENABLED is true"
    )]
    pub feature_join_right_side_max_rows: usize,
    #[env_config(
        name = "ZO_FEATURE_QUERY_SKIP_WAL",
        default = false,
        help = "Skip WAL for query"
    )]
    pub feature_query_skip_wal: bool,
    #[env_config(name = "ZO_UI_ENABLED", default = true)]
    pub ui_enabled: bool,
    #[env_config(name = "ZO_UI_SQL_BASE64_ENABLED", default = false)]
    pub ui_sql_base64_enabled: bool,
    #[env_config(name = "ZO_METRICS_DEDUP_ENABLED", default = true)]
    pub metrics_dedup_enabled: bool,
    #[env_config(name = "ZO_BLOOM_FILTER_ENABLED", default = true)]
    pub bloom_filter_enabled: bool,
    #[env_config(name = "ZO_BLOOM_FILTER_DISABLED_ON_SEARCH", default = false)]
    pub bloom_filter_disabled_on_search: bool,
    #[env_config(name = "ZO_BLOOM_FILTER_DEFAULT_FIELDS", default = "")]
    pub bloom_filter_default_fields: String,
    #[env_config(
        name = "ZO_BLOOM_FILTER_NDV_RATIO",
        default = 100,
        help = "Bloom filter ndv ratio, set to 100 means NDV = row_count / 100, if set to 1 means will use NDV = row_count"
    )]
    pub bloom_filter_ndv_ratio: u64,
    #[env_config(name = "ZO_WAL_FSYNC_DISABLED", default = false)]
    pub wal_fsync_disabled: bool,
    #[env_config(
        name = "ZO_WAL_WRITE_QUEUE_ENABLED",
        default = false,
        help = "Enable write queue for WAL"
    )]
    pub wal_write_queue_enabled: bool,
    #[env_config(
        name = "ZO_WAL_WRITE_QUEUE_FULL_REJECT",
        default = false,
        help = "Reject write when write queue is full"
    )]
    pub wal_write_queue_full_reject: bool,
    #[env_config(name = "ZO_TRACING_ENABLED", default = false)]
    pub tracing_enabled: bool,
    #[env_config(name = "ZO_TRACING_SEARCH_ENABLED", default = false)]
    pub tracing_search_enabled: bool,
    #[env_config(name = "OTEL_OTLP_HTTP_ENDPOINT", default = "")]
    pub otel_otlp_url: String,
    #[env_config(name = "OTEL_OTLP_GRPC_ENDPOINT", default = "")]
    pub otel_otlp_grpc_url: String,
    #[env_config(
        name = "ZO_TRACING_GRPC_ORGANIZATION",
        default = "",
        help = "Used in metadata when exporting traces to grpc endpoint."
    )]
    pub tracing_grpc_header_org: String,
    #[env_config(
        name = "ZO_TRACING_GRPC_STREAM_NAME",
        default = "",
        help = "Used in metadata when exporting traces to grpc endpoint."
    )]
    pub tracing_grpc_header_stream_name: String,
    #[env_config(name = "ZO_TRACING_HEADER_KEY", default = "Authorization")]
    pub tracing_header_key: String,
    #[env_config(
        name = "ZO_TRACING_HEADER_VALUE",
        default = "Basic cm9vdEBleGFtcGxlLmNvbTpDb21wbGV4cGFzcyMxMjM="
    )]
    pub tracing_header_value: String,
    #[env_config(name = "ZO_TELEMETRY", default = true)]
    pub telemetry_enabled: bool,
    #[env_config(name = "ZO_TELEMETRY_URL", default = "https://e1.zinclabs.dev")]
    pub telemetry_url: String,
    #[env_config(name = "ZO_TELEMETRY_HEARTBEAT", default = 1800)] // seconds
    pub telemetry_heartbeat: i64,
    #[env_config(name = "ZO_PROMETHEUS_ENABLED", default = true)]
    pub prometheus_enabled: bool,
    #[env_config(name = "ZO_PRINT_KEY_CONFIG", default = false)]
    pub print_key_config: bool,
    #[env_config(name = "ZO_PRINT_KEY_EVENT", default = false)]
    pub print_key_event: bool,
    #[env_config(name = "ZO_PRINT_KEY_SQL", default = false)]
    pub print_key_sql: bool,
    #[env_config(name = "ZO_USAGE_REPORTING_ENABLED", default = false)]
    pub usage_enabled: bool,
    #[env_config(name = "ZO_USAGE_ORG", default = "_meta")]
    pub usage_org: String,
    #[env_config(
        name = "ZO_USAGE_REPORTING_MODE",
        default = "local",
        help = "possible values - 'local', 'remote', 'both'"
    )] // local, remote , both
    pub usage_reporting_mode: String,
    #[env_config(
        name = "ZO_USAGE_REPORTING_URL",
        default = "http://localhost:5080/api/_meta/usage/_json"
    )]
    pub usage_reporting_url: String,
    #[env_config(name = "ZO_USAGE_REPORTING_CREDS", default = "")]
    pub usage_reporting_creds: String,
    #[env_config(name = "ZO_USAGE_BATCH_SIZE", default = 2000)]
    pub usage_batch_size: usize,
    #[env_config(
        name = "ZO_USAGE_PUBLISH_INTERVAL",
        default = 60,
        help = "duration in seconds after last reporting usage will be published"
    )]
    // in seconds
    pub usage_publish_interval: i64,
    #[env_config(name = "ZO_MMDB_DATA_DIR")] // ./data/openobserve/mmdb/
    pub mmdb_data_dir: String,
    #[env_config(name = "ZO_MMDB_DISABLE_DOWNLOAD", default = false)]
    pub mmdb_disable_download: bool,
    #[env_config(name = "ZO_MMDB_UPDATE_DURATION", default = "86400")] // Everyday to test
    pub mmdb_update_duration: u64,
    #[env_config(
        name = "ZO_MMDB_GEOLITE_CITYDB_URL",
        default = "https://geoip.zinclabs.dev/GeoLite2-City.mmdb"
    )]
    pub mmdb_geolite_citydb_url: String,
    #[env_config(
        name = "ZO_MMDB_GEOLITE_ASNDB_URL",
        default = "https://geoip.zinclabs.dev/GeoLite2-ASN.mmdb"
    )]
    pub mmdb_geolite_asndb_url: String,
    #[env_config(
        name = "ZO_MMDB_GEOLITE_CITYDB_SHA256_URL",
        default = "https://geoip.zinclabs.dev/GeoLite2-City.sha256"
    )]
    pub mmdb_geolite_citydb_sha256_url: String,
    #[env_config(
        name = "ZO_MMDB_GEOLITE_ASNDB_SHA256_URL",
        default = "https://geoip.zinclabs.dev/GeoLite2-ASN.sha256"
    )]
    pub mmdb_geolite_asndb_sha256_url: String,
    #[env_config(name = "ZO_DEFAULT_SCRAPE_INTERVAL", default = 15)]
    // Default scrape_interval value 15s
    pub default_scrape_interval: u32,
    #[env_config(name = "ZO_CIRCUIT_BREAKER_ENABLE", default = false)]
    pub memory_circuit_breaker_enable: bool,
    #[env_config(name = "ZO_CIRCUIT_BREAKER_RATIO", default = 100)]
    pub memory_circuit_breaker_ratio: usize,
    #[env_config(
        name = "ZO_RESTRICTED_ROUTES_ON_EMPTY_DATA",
        default = false,
        help = "Control the redirection of a user to ingestion page in case there is no stream found."
    )]
    pub restricted_routes_on_empty_data: bool,
    #[env_config(
        name = "ZO_ENABLE_INVERTED_INDEX",
        default = true,
        help = "Toggle inverted index generation."
    )]
    pub inverted_index_enabled: bool,
    #[env_config(
        name = "ZO_INVERTED_INDEX_CACHE_ENABLED",
        default = false,
        help = "Toggle inverted index cache."
    )]
    pub inverted_index_cache_enabled: bool,
    #[deprecated(since = "0.14.3", note = "will be removed in 0.15.0")]
    #[env_config(
        name = "ZO_INVERTED_INDEX_SPLIT_CHARS",
        default = "",
        help = "Characters which should be used as a delimiter to split the string, default using all ascii punctuations."
    )]
    pub inverted_index_split_chars: String,
    #[deprecated(since = "0.14.3", note = "will be removed in 0.15.0")]
    #[env_config(
        name = "ZO_INVERTED_INDEX_OLD_FORMAT",
        default = false,
        help = "Use old format for inverted index, it will generate same stream name for index."
    )]
    pub inverted_index_old_format: bool,
    #[deprecated(since = "0.14.3", note = "will be removed in 0.15.0")]
    #[env_config(
        name = "ZO_INVERTED_INDEX_STORE_FORMAT",
        default = "tantivy",
        help = "InvertedIndex store format, parquet(default), tantivy, both"
    )]
    pub inverted_index_store_format: String,
    #[deprecated(since = "0.14.3", note = "will be removed in 0.15.0")]
    #[env_config(
        name = "ZO_INVERTED_INDEX_SEARCH_FORMAT",
        default = "tantivy",
        help = "InvertedIndex search format, parquet(default), tantivy."
    )]
    pub inverted_index_search_format: String,
    #[env_config(
        name = "ZO_INVERTED_INDEX_TANTIVY_MODE",
        default = "",
        help = "Tantivy search mode, puffin or mmap, default is puffin."
    )]
    pub inverted_index_tantivy_mode: String,
    #[env_config(
        name = "ZO_INVERTED_INDEX_CAMEL_CASE_TOKENIZER_DISABLED",
        default = false,
        help = "Disable camel case tokenizer for inverted index."
    )]
    pub inverted_index_camel_case_tokenizer_disabled: bool,
    #[env_config(
        name = "ZO_INVERTED_INDEX_COUNT_OPTIMIZER_ENABLED",
        default = true,
        help = "Toggle inverted index count optimizer."
    )]
    pub inverted_index_count_optimizer_enabled: bool,
    #[deprecated(since = "0.14.3", note = "will be removed in 0.15.0")]
    #[env_config(
        name = "ZO_FULL_TEXT_SEARCH_TYPE",
        default = "eq",
        help = "Search through full text fields with either 'contains' , 'eq' or 'prefix' match."
    )]
    pub full_text_search_type: String,
    #[env_config(
        name = "ZO_QUERY_ON_STREAM_SELECTION",
        default = true,
        help = "Toggle search to be trigger based on button click event."
    )]
    pub query_on_stream_selection: bool,
    #[env_config(
        name = "ZO_SHOW_STREAM_DATES_DOCS_NUM",
        default = true,
        help = "Show docs count and stream dates"
    )]
    pub show_stream_dates_doc_num: bool,
    #[env_config(name = "ZO_INGEST_BLOCKED_STREAMS", default = "")] // use comma to split
    pub blocked_streams: String,
    #[env_config(name = "ZO_REPORT_USER_NAME", default = "")]
    pub report_user_name: String,
    #[env_config(name = "ZO_REPORT_USER_PASSWORD", default = "")]
    pub report_user_password: String,
    #[env_config(name = "ZO_REPORT_SERVER_URL", default = "http://localhost:5082")]
    pub report_server_url: String,
    #[env_config(name = "ZO_REPORT_SERVER_SKIP_TLS_VERIFY", default = false)]
    pub report_server_skip_tls_verify: bool,
    #[env_config(name = "ZO_SCHEMA_CACHE_COMPRESS_ENABLED", default = false)]
    pub schema_cache_compress_enabled: bool,
    #[env_config(name = "ZO_SKIP_FORMAT_STREAM_NAME", default = false)]
    pub skip_formatting_stream_name: bool,
    #[env_config(name = "ZO_BULK_RESPONSE_INCLUDE_ERRORS_ONLY", default = false)]
    pub bulk_api_response_errors_only: bool,
    #[env_config(name = "ZO_ALLOW_USER_DEFINED_SCHEMAS", default = false)]
    pub allow_user_defined_schemas: bool,
    #[env_config(
        name = "ZO_MEM_TABLE_STREAMS",
        default = "",
        help = "Streams for which dedicated MemTable will be used as comma separated values"
    )]
    pub mem_table_individual_streams: String,
    #[env_config(
        name = "ZO_TRACES_SPAN_METRICS_ENABLED",
        default = false,
        help = "enable span metrics for traces"
    )]
    pub traces_span_metrics_enabled: bool,
    #[env_config(
        name = "ZO_TRACES_SPAN_METRICS_EXPORT_INTERVAL",
        default = 60,
        help = "traces span metrics export interval, unit seconds"
    )]
    pub traces_span_metrics_export_interval: u64,
    #[env_config(
        name = "ZO_TRACES_SPAN_METRICS_CHANNEL_BUFFER",
        default = 100000,
        help = "traces span metrics channel send buffer"
    )]
    pub traces_span_metrics_channel_buffer: usize,
    #[env_config(
        name = "ZO_SELF_METRIC_CONSUMPTION_ENABLED",
        default = false,
        help = "self-consume metrics generated by openobserve"
    )]
    pub self_metrics_consumption_enabled: bool,
    #[env_config(
        name = "ZO_SELF_METRIC_CONSUMPTION_INTERVAL",
        default = 60,
        help = "metrics self-consumption interval, unit seconds"
    )]
    pub self_metrics_consumption_interval: u64,
    #[env_config(
        name = "ZO_SELF_METRIC_CONSUMPTION_ACCEPTLIST",
        default = "",
        help = "only these metrics will be self-consumed, comma separated"
    )]
    pub self_metrics_consumption_whitelist: String,
    #[env_config(
        name = "ZO_RESULT_CACHE_ENABLED",
        default = false,
        help = "Enable result cache for query results"
    )]
    pub result_cache_enabled: bool,
    #[env_config(
        name = "ZO_USE_MULTIPLE_RESULT_CACHE",
        default = false,
        help = "Enable to use mulple result caches for query results"
    )]
    pub use_multi_result_cache: bool,
    #[env_config(
        name = "ZO_RESULT_CACHE_SELECTION_STRATEGY",
        default = "overlap",
        help = "Strategy to use for result cache, default is both , possible value - both,overlap , duration"
    )]
    pub result_cache_selection_strategy: String,
    #[env_config(
        name = "ZO_RESULT_CACHE_DISCARD_DURATION",
        default = 60,
        help = "Discard data of last n seconds from cached results"
    )]
    pub result_cache_discard_duration: i64,
    #[env_config(
        name = "ZO_METRICS_CACHE_ENABLED",
        default = true,
        help = "Enable result cache for PromQL metrics queries"
    )]
    pub metrics_cache_enabled: bool,
    #[env_config(name = "ZO_SWAGGER_ENABLED", default = true)]
    pub swagger_enabled: bool,
    #[env_config(name = "ZO_FAKE_ES_VERSION", default = "")]
    pub fake_es_version: String,
    #[env_config(name = "ZO_WEBSOCKET_ENABLED", default = false)]
    pub websocket_enabled: bool,
    #[env_config(name = "ZO_WEBSOCKET_CLOSE_FRAME_DELAY", default = 0)]
    pub websocket_close_frame_delay: u64, // in milliseconds
    #[env_config(
        name = "ZO_MIN_AUTO_REFRESH_INTERVAL",
        default = 5,
        help = "allow minimum auto refresh interval in seconds"
    )] // in seconds
    pub min_auto_refresh_interval: u32,
}

#[derive(EnvConfig)]
pub struct Limit {
    // no need set by environment
    pub cpu_num: usize,
    pub real_cpu_num: usize,
    pub mem_total: usize,
    pub disk_total: usize,
    pub disk_free: usize,
    #[env_config(name = "ZO_JSON_LIMIT", default = 209715200)]
    pub req_json_limit: usize,
    #[env_config(name = "ZO_PAYLOAD_LIMIT", default = 209715200)]
    pub req_payload_limit: usize,
    #[env_config(name = "ZO_MAX_FILE_RETENTION_TIME", default = 600)] // seconds
    pub max_file_retention_time: u64,
    // MB, per log file size limit on disk
    #[env_config(name = "ZO_MAX_FILE_SIZE_ON_DISK", default = 128)]
    pub max_file_size_on_disk: usize,
    // MB, per data file size limit in memory
    #[env_config(name = "ZO_MAX_FILE_SIZE_IN_MEMORY", default = 128)]
    pub max_file_size_in_memory: usize,
    #[deprecated(
        since = "0.14.1",
        note = "Please use `ZO_SCHEMA_MAX_FIELDS_TO_ENABLE_UDS` instead. This ENV is subject to be removed soon"
    )]
    #[env_config(
        name = "ZO_UDSCHEMA_MAX_FIELDS",
        default = 0,
        help = "Exceeding this limit will auto enable user-defined schema"
    )]
    pub udschema_max_fields: usize,
    #[env_config(
        name = "ZO_SCHEMA_MAX_FIELDS_TO_ENABLE_UDS",
        default = 1000,
        help = "Exceeding this limit will auto enable user-defined schema"
    )]
    pub schema_max_fields_to_enable_uds: usize,
    #[env_config(
        name = "ZO_USER_DEFINED_SCHEMA_MAX_FIELDS",
        default = 1000,
        help = "Maximum number of fields allowed in user-defined schema"
    )]
    pub user_defined_schema_max_fields: usize,
    // MB, total data size in memory, default is 50% of system memory
    #[env_config(name = "ZO_MEM_TABLE_MAX_SIZE", default = 0)]
    pub mem_table_max_size: usize,
    #[env_config(
        name = "ZO_MEM_TABLE_BUCKET_NUM",
        default = 0,
        help = "MemTable bucket num, default is 1"
    )] // default is 1
    pub mem_table_bucket_num: usize,
    #[env_config(name = "ZO_MEM_PERSIST_INTERVAL", default = 5)] // seconds
    pub mem_persist_interval: u64,
    #[env_config(name = "ZO_WAL_WRITE_BUFFER_SIZE", default = 16384)] // 16 KB
    pub wal_write_buffer_size: usize,
    #[env_config(name = "ZO_WAL_WRITE_QUEUE_SIZE", default = 10000)] // 10k messages
    pub wal_write_queue_size: usize,
    #[env_config(name = "ZO_FILE_PUSH_INTERVAL", default = 10)] // seconds
    pub file_push_interval: u64,
    #[env_config(name = "ZO_FILE_PUSH_LIMIT", default = 0)] // files
    pub file_push_limit: usize,
    // over this limit will skip merging on ingester
    #[env_config(name = "ZO_FILE_MOVE_FIELDS_LIMIT", default = 2000)]
    pub file_move_fields_limit: usize,
    #[env_config(name = "ZO_FILE_MOVE_THREAD_NUM", default = 0)]
    pub file_move_thread_num: usize,
    #[env_config(name = "ZO_FILE_MERGE_THREAD_NUM", default = 0)]
    pub file_merge_thread_num: usize,
    #[env_config(name = "ZO_MEM_DUMP_THREAD_NUM", default = 0)]
    pub mem_dump_thread_num: usize,
    #[env_config(name = "ZO_USAGE_REPORTING_THREAD_NUM", default = 0)]
    pub usage_reporting_thread_num: usize,
    #[env_config(name = "ZO_QUERY_THREAD_NUM", default = 0)]
    pub query_thread_num: usize,
    #[env_config(name = "ZO_QUERY_TIMEOUT", default = 600)]
    pub query_timeout: u64,
    #[env_config(name = "ZO_QUERY_INGESTER_TIMEOUT", default = 0)]
    // default equal to query_timeout
    pub query_ingester_timeout: u64,
    #[env_config(name = "ZO_QUERY_DEFAULT_LIMIT", default = 1000)]
    pub query_default_limit: i64,
    #[env_config(name = "ZO_QUERY_PARTITION_BY_SECS", default = 1)] // seconds
    pub query_partition_by_secs: usize,
    #[env_config(name = "ZO_QUERY_GROUP_BASE_SPEED", default = 768)] // MB/s/core
    pub query_group_base_speed: usize,
    #[env_config(name = "ZO_INGEST_ALLOWED_UPTO", default = 5)] // in hours - in past
    pub ingest_allowed_upto: i64,
    #[env_config(name = "ZO_INGEST_FLATTEN_LEVEL", default = 3)] // default flatten level
    pub ingest_flatten_level: u32,
    #[env_config(name = "ZO_IGNORE_FILE_RETENTION_BY_STREAM", default = false)]
    pub ignore_file_retention_by_stream: bool,
    #[env_config(name = "ZO_LOGS_FILE_RETENTION", default = "hourly")]
    pub logs_file_retention: String,
    #[env_config(name = "ZO_TRACES_FILE_RETENTION", default = "hourly")]
    pub traces_file_retention: String,
    #[env_config(name = "ZO_METRICS_FILE_RETENTION", default = "daily")]
    pub metrics_file_retention: String,
    #[env_config(name = "ZO_METRICS_LEADER_PUSH_INTERVAL", default = 15)]
    pub metrics_leader_push_interval: u64,
    #[env_config(name = "ZO_METRICS_LEADER_ELECTION_INTERVAL", default = 30)]
    pub metrics_leader_election_interval: i64,
    #[env_config(name = "ZO_METRICS_MAX_SERIES_PER_QUERY", default = 30000)]
    pub metrics_max_series_per_query: usize,
    #[env_config(name = "ZO_METRICS_MAX_POINTS_PER_SERIES", default = 30000)]
    pub metrics_max_points_per_series: usize,
    #[env_config(name = "ZO_METRICS_CACHE_MAX_ENTRIES", default = 100000)]
    pub metrics_cache_max_entries: usize,
    #[env_config(name = "ZO_COLS_PER_RECORD_LIMIT", default = 1000)]
    pub req_cols_per_record_limit: usize,
    #[env_config(name = "ZO_NODE_HEARTBEAT_TTL", default = 30)] // seconds
    pub node_heartbeat_ttl: i64,
    #[env_config(name = "ZO_HTTP_WORKER_NUM", default = 0)]
    pub http_worker_num: usize, // equals to cpu_num if 0
    #[env_config(name = "ZO_HTTP_WORKER_MAX_BLOCKING", default = 0)]
    pub http_worker_max_blocking: usize, // equals to 1024 if 0
    #[env_config(name = "ZO_GRPC_RUNTIME_WORKER_NUM", default = 0)]
    pub grpc_runtime_worker_num: usize, // equals to cpu_num if 0
    #[env_config(name = "ZO_GRPC_RUNTIME_BLOCKING_WORKER_NUM", default = 0)]
    pub grpc_runtime_blocking_worker_num: usize, // equals to 512 if 0
    #[env_config(name = "ZO_GRPC_RUNTIME_SHUTDOWN_TIMEOUT", default = 10)] // seconds
    pub grpc_runtime_shutdown_timeout: u64,
    #[env_config(name = "ZO_JOB_RUNTIME_WORKER_NUM", default = 0)]
    pub job_runtime_worker_num: usize, // equals to cpu_num if 0
    #[env_config(name = "ZO_JOB_RUNTIME_BLOCKING_WORKER_NUM", default = 0)]
    pub job_runtime_blocking_worker_num: usize, // equals to 512 if 0
    #[env_config(name = "ZO_JOB_RUNTIME_SHUTDOWN_TIMEOUT", default = 10)] // seconds
    pub job_runtime_shutdown_timeout: u64,
    #[env_config(name = "ZO_CALCULATE_STATS_INTERVAL", default = 60)] // seconds
    pub calculate_stats_interval: u64,
    #[env_config(name = "ZO_ENRICHMENT_TABLE_LIMIT", default = 10)] // size in mb
    pub enrichment_table_limit: usize,
    #[env_config(name = "ZO_ACTIX_REQ_TIMEOUT", default = 5)] // seconds
    pub http_request_timeout: u64,
    #[env_config(name = "ZO_ACTIX_KEEP_ALIVE", default = 5)] // seconds
    pub http_keep_alive: u64,
    #[env_config(name = "ZO_ACTIX_SHUTDOWN_TIMEOUT", default = 5)] // seconds
    pub http_shutdown_timeout: u64,
    #[env_config(name = "ZO_ACTIX_SLOW_LOG_THRESHOLD", default = 5)] // seconds
    pub http_slow_log_threshold: u64,
    #[env_config(name = "ZO_CIRCUIT_BREAKER_ENABLED", default = false)]
    pub circuit_breaker_enabled: bool,
    #[env_config(name = "ZO_CIRCUIT_BREAKER_WATCHING_WINDOW", default = 60)] // seconds
    pub circuit_breaker_watching_window: i64,
    #[env_config(name = "ZO_CIRCUIT_BREAKER_RESET_WINDOW_NUM", default = 3)] // 3 * watching window
    pub circuit_breaker_reset_window_num: i64,
    #[env_config(
        name = "ZO_CIRCUIT_BREAKER_SLOW_REQUEST_THRESHOLD",
        default = 100,
        help = "Trigger circuit break if over this threshold in watching window, and will be reset after 2 * watching window"
    )] // slow requests
    pub circuit_breaker_slow_request_threshold: u64,
    #[env_config(name = "ZO_ALERT_SCHEDULE_INTERVAL", default = 10)] // seconds
    pub alert_schedule_interval: i64,
    #[env_config(name = "ZO_ALERT_SCHEDULE_CONCURRENCY", default = 5)]
    pub alert_schedule_concurrency: i64,
    #[env_config(name = "ZO_ALERT_SCHEDULE_TIMEOUT", default = 90)] // seconds
    pub alert_schedule_timeout: i64,
    #[env_config(name = "ZO_REPORT_SCHEDULE_TIMEOUT", default = 300)] // seconds
    pub report_schedule_timeout: i64,
    #[env_config(name = "ZO_DERIVED_STREAM_SCHEDULE_INTERVAL", default = 300)] // seconds
    pub derived_stream_schedule_interval: i64,
    #[env_config(name = "ZO_SCHEDULER_MAX_RETRIES", default = 3)]
    pub scheduler_max_retries: i32,
    #[env_config(name = "ZO_SCHEDULER_PAUSE_ALERT_AFTER_RETRIES", default = false)]
    pub pause_alerts_on_retries: bool,
    #[env_config(
        name = "ZO_ALERT_CONSIDERABLE_DELAY",
        default = 20,
        help = "Integer value representing the delay in percentage of the alert frequency that will be included in alert evaluation timerange. Default is 20. This can be changed in runtime."
    )]
    pub alert_considerable_delay: i32,
    #[env_config(name = "ZO_SCHEDULER_CLEAN_INTERVAL", default = 30)] // seconds
    pub scheduler_clean_interval: i64,
    #[env_config(name = "ZO_SCHEDULER_WATCH_INTERVAL", default = 30)] // seconds
    pub scheduler_watch_interval: i64,
    #[env_config(name = "ZO_SEARCH_JOB_WORKS", default = 1)]
    pub search_job_workers: i64,
    #[env_config(name = "ZO_SEARCH_JOB_SCHEDULE_INTERVAL", default = 10)] // seconds
    pub search_job_scheduler_interval: i64,
    #[env_config(
        name = "ZO_SEARCH_JOB_RUM_TIMEOUT",
        default = 600, // seconds
        help = "Timeout for update check"
    )]
    pub search_job_run_timeout: i64,
    #[env_config(name = "ZO_SEARCH_JOB_DELETE_INTERVAL", default = 600)] // seconds
    pub search_job_delete_interval: i64,
    #[env_config(
        name = "ZO_SEARCH_JOB_TIMEOUT",
        default = 36000, // seconds
        help = "Timeout for query"
    )]
    pub search_job_timeout: i64,
    #[env_config(
        name = "ZO_SEARCH_JOB_RETENTION",
        default = 30, // days
        help = "Retention for search job"
    )]
    pub search_job_retention: i64,
    #[env_config(name = "ZO_STARTING_EXPECT_QUERIER_NUM", default = 0)]
    pub starting_expect_querier_num: usize,
    #[env_config(name = "ZO_QUERY_OPTIMIZATION_NUM_FIELDS", default = 1000)]
    pub query_optimization_num_fields: usize,
    #[env_config(name = "ZO_QUICK_MODE_ENABLED", default = false)]
    pub quick_mode_enabled: bool,
    #[env_config(name = "ZO_QUICK_MODE_FORCE_ENABLED", default = true)]
    pub quick_mode_force_enabled: bool,
    #[env_config(name = "ZO_QUICK_MODE_NUM_FIELDS", default = 500)]
    pub quick_mode_num_fields: usize,
    #[env_config(name = "ZO_QUICK_MODE_STRATEGY", default = "")]
    pub quick_mode_strategy: String, // first, last, both
    #[env_config(name = "ZO_META_CONNECTION_POOL_MIN_SIZE", default = 0)] // number of connections
    pub sql_db_connections_min: u32,
    #[env_config(name = "ZO_META_CONNECTION_POOL_MAX_SIZE", default = 0)] // number of connections
    pub sql_db_connections_max: u32,
    #[env_config(
        name = "ZO_META_CONNECTION_POOL_ACQUIRE_TIMEOUT",
        default = 0,
        help = "Seconds, Maximum acquire timeout of individual connections."
    )]
    pub sql_db_connections_acquire_timeout: u64,
    #[env_config(
        name = "ZO_META_CONNECTION_POOL_IDLE_TIMEOUT",
        default = 0,
        help = "Seconds, Maximum idle timeout of individual connections."
    )]
    pub sql_db_connections_idle_timeout: u64,
    #[env_config(
        name = "ZO_META_CONNECTION_POOL_MAX_LIFETIME",
        default = 0,
        help = "Seconds, Maximum lifetime of individual connections."
    )]
    pub sql_db_connections_max_lifetime: u64,
    #[env_config(
        name = "ZO_META_TRANSACTION_RETRIES",
        default = 3,
        help = "max time of transaction will retry"
    )]
    pub meta_transaction_retries: usize,
    #[env_config(
        name = "ZO_META_TRANSACTION_LOCK_TIMEOUT",
        default = 600,
        help = "timeout of transaction lock"
    )] // seconds
    pub meta_transaction_lock_timeout: usize,
    #[env_config(
        name = "ZO_FILE_LIST_ID_BATCH_SIZE",
        default = 5000,
        help = "batch size of file list query"
    )]
    pub file_list_id_batch_size: usize,
    #[env_config(
        name = "ZO_FILE_LIST_MULTI_THREAD",
        default = false,
        help = "use multi thread for file list query"
    )]
    pub file_list_multi_thread: bool,
    #[env_config(name = "ZO_DISTINCT_VALUES_INTERVAL", default = 10)] // seconds
    pub distinct_values_interval: u64,
    #[env_config(name = "ZO_DISTINCT_VALUES_HOURLY", default = false)]
    pub distinct_values_hourly: bool,
    #[env_config(name = "ZO_CONSISTENT_HASH_VNODES", default = 1000)]
    pub consistent_hash_vnodes: usize,
    #[env_config(
        name = "ZO_DATAFUSION_FILE_STAT_CACHE_MAX_ENTRIES",
        default = 100000,
        help = "Maximum number of entries in the file stat cache. Higher values increase memory usage but may improve query performance."
    )]
    pub datafusion_file_stat_cache_max_entries: usize,
    #[env_config(
        name = "ZO_DATAFUSION_STREAMING_AGGS_CACHE_MAX_ENTRIES",
        default = 100000,
        help = "Maximum number of entries in the streaming aggs cache. Higher values increase memory usage but may improve query performance."
    )]
    pub datafusion_streaming_aggs_cache_max_entries: usize,
    #[env_config(name = "ZO_DATAFUSION_MIN_PARTITION_NUM", default = 2)]
    pub datafusion_min_partition_num: usize,
    #[env_config(
        name = "ZO_ENRICHMENT_TABLE_LIMIT",
        default = 256,
        help = "Maximum size of a single enrichment table in mb"
    )]
    pub max_enrichment_table_size: usize,
    #[env_config(name = "ZO_SHORT_URL_RETENTION_DAYS", default = 30)] // days
    pub short_url_retention_days: i64,
    #[env_config(
        name = "ZO_INVERTED_INDEX_CACHE_MAX_ENTRIES",
        default = 100000,
        help = "Maximum number of entries in the inverted index cache. Higher values increase memory usage but may improve query performance."
    )]
    pub inverted_index_cache_max_entries: usize,
    #[env_config(
        name = "ZO_INVERTED_INDEX_SKIP_THRESHOLD",
        default = 0,
        help = "If the inverted index returns row_id more than this threshold(%), it will skip the inverted index."
    )]
    pub inverted_index_skip_threshold: usize,
    #[env_config(
        name = "ZO_MAX_QUERY_RANGE_FOR_SA",
        default = 0,
        help = "unit: Hour. Optional env variable to add restriction for SA, if not set SA will use max_query_range stream setting. When set which ever is smaller value will apply to api calls"
    )]
    pub max_query_range_for_sa: i64,
    #[env_config(
        name = "ZO_TEXT_DATA_TYPE",
        default = "longtext",
        help = "Default data type for LongText compliant DB's"
    )]
    pub db_text_data_type: String,
}

#[derive(EnvConfig)]
pub struct Compact {
    #[env_config(name = "ZO_COMPACT_ENABLED", default = true)]
    pub enabled: bool,
    #[env_config(name = "ZO_COMPACT_INTERVAL", default = 60)] // seconds
    pub interval: u64,
    #[env_config(name = "ZO_COMPACT_OLD_DATA_INTERVAL", default = 3600)] // seconds
    pub old_data_interval: u64,
    #[env_config(name = "ZO_COMPACT_STRATEGY", default = "file_time")] // file_size, file_time
    pub strategy: String,
    #[env_config(name = "ZO_COMPACT_SYNC_TO_DB_INTERVAL", default = 600)] // seconds
    pub sync_to_db_interval: u64,
    #[env_config(name = "ZO_COMPACT_MAX_FILE_SIZE", default = 512)] // MB
    pub max_file_size: usize,
    #[env_config(name = "ZO_COMPACT_EXTENDED_DATA_RETENTION_DAYS", default = 3650)] // days
    pub extended_data_retention_days: i64,
    #[env_config(name = "ZO_COMPACT_DATA_RETENTION_DAYS", default = 3650)] // days
    pub data_retention_days: i64,
    #[env_config(name = "ZO_COMPACT_OLD_DATA_MAX_DAYS", default = 7)] // days
    pub old_data_max_days: i64,
    #[env_config(name = "ZO_COMPACT_OLD_DATA_MIN_HOURS", default = 2)] // hours
    pub old_data_min_hours: i64,
    #[env_config(name = "ZO_COMPACT_OLD_DATA_MIN_RECORDS", default = 100)] // records
    pub old_data_min_records: i64,
    #[env_config(name = "ZO_COMPACT_OLD_DATA_MIN_FILES", default = 10)] // files
    pub old_data_min_files: i64,
    #[env_config(name = "ZO_COMPACT_DELETE_FILES_DELAY_HOURS", default = 2)] // hours
    pub delete_files_delay_hours: i64,
    #[env_config(name = "ZO_COMPACT_BLOCKED_ORGS", default = "")] // use comma to split
    pub blocked_orgs: String,
    #[env_config(name = "ZO_COMPACT_DATA_RETENTION_HISTORY", default = false)]
    pub data_retention_history: bool,
    #[env_config(
        name = "ZO_COMPACT_BATCH_SIZE",
        default = 500,
        help = "Batch size for compact get pending jobs"
    )]
    pub batch_size: i64,
    #[env_config(
        name = "ZO_COMPACT_JOB_RUN_TIMEOUT",
        default = 600, // 10 minutes
        help = "If a compact job is not finished in this time, it will be marked as failed"
    )]
    pub job_run_timeout: i64,
    #[env_config(
        name = "ZO_COMPACT_JOB_CLEAN_WAIT_TIME",
        default = 7200, // 2 hours
        help = "Clean the jobs which are finished more than this time"
    )]
    pub job_clean_wait_time: i64,
    #[env_config(name = "ZO_COMPACT_PENDING_JOBS_METRIC_INTERVAL", default = 300)] // seconds
    pub pending_jobs_metric_interval: u64,
}

#[derive(EnvConfig)]
pub struct MemoryCache {
    #[env_config(name = "ZO_MEMORY_CACHE_ENABLED", default = true)]
    pub enabled: bool,
    // Memory data cache strategy, default is lru, other value is fifo
    #[env_config(name = "ZO_MEMORY_CACHE_STRATEGY", default = "lru")]
    pub cache_strategy: String,
    // Memory data cache bucket num, multiple bucket means multiple locker, default is 0
    #[env_config(name = "ZO_MEMORY_CACHE_BUCKET_NUM", default = 0)]
    pub bucket_num: usize,
    #[env_config(name = "ZO_MEMORY_CACHE_CACHE_LATEST_FILES", default = false)]
    pub cache_latest_files: bool,
    // MB, default is 50% of system memory
    #[env_config(name = "ZO_MEMORY_CACHE_MAX_SIZE", default = 0)]
    pub max_size: usize,
    // MB, will skip the cache when a query need cache great than this value, default is 50% of
    // max_size
    #[env_config(name = "ZO_MEMORY_CACHE_SKIP_SIZE", default = 0)]
    pub skip_size: usize,
    // MB, when cache is full will release how many data once time, default is 1% of max_size
    #[env_config(name = "ZO_MEMORY_CACHE_RELEASE_SIZE", default = 0)]
    pub release_size: usize,
    #[env_config(name = "ZO_MEMORY_CACHE_GC_SIZE", default = 50)] // MB
    pub gc_size: usize,
    #[env_config(name = "ZO_MEMORY_CACHE_GC_INTERVAL", default = 0)] // seconds
    pub gc_interval: u64,
    #[env_config(name = "ZO_MEMORY_CACHE_SKIP_DISK_CHECK", default = false)]
    pub skip_disk_check: bool,
    // MB, default is 50% of system memory
    #[env_config(name = "ZO_MEMORY_CACHE_DATAFUSION_MAX_SIZE", default = 0)]
    pub datafusion_max_size: usize,
    #[env_config(name = "ZO_MEMORY_CACHE_DATAFUSION_MEMORY_POOL", default = "")]
    pub datafusion_memory_pool: String,
}

#[derive(EnvConfig)]
pub struct DiskCache {
    #[env_config(name = "ZO_DISK_CACHE_ENABLED", default = true)]
    pub enabled: bool,
    // Disk data cache strategy, default is lru, other value is fifo
    #[env_config(name = "ZO_DISK_CACHE_STRATEGY", default = "lru")]
    pub cache_strategy: String,
    // Disk data cache bucket num, multiple bucket means multiple locker, default is 0
    #[env_config(name = "ZO_DISK_CACHE_BUCKET_NUM", default = 0)]
    pub bucket_num: usize,
    // MB, default is 50% of local volume available space and maximum 100GB
    #[env_config(name = "ZO_DISK_CACHE_MAX_SIZE", default = 0)]
    pub max_size: usize,
    // MB, default is 10% of local volume available space and maximum 20GB
    #[env_config(name = "ZO_DISK_RESULT_CACHE_MAX_SIZE", default = 0)]
    pub result_max_size: usize,
    // MB, will skip the cache when a query need cache great than this value, default is 50% of
    // max_size
    #[env_config(name = "ZO_DISK_CACHE_SKIP_SIZE", default = 0)]
    pub skip_size: usize,
    // MB, when cache is full will release how many data once time, default is 1% of max_size
    #[env_config(name = "ZO_DISK_CACHE_RELEASE_SIZE", default = 0)]
    pub release_size: usize,
    #[env_config(name = "ZO_DISK_CACHE_GC_SIZE", default = 100)] // MB
    pub gc_size: usize,
    #[env_config(name = "ZO_DISK_CACHE_GC_INTERVAL", default = 0)] // seconds
    pub gc_interval: u64,
    #[env_config(name = "ZO_DISK_CACHE_MULTI_DIR", default = "")] // dir1,dir2,dir3...
    pub multi_dir: String,
}

#[derive(EnvConfig)]
pub struct Log {
    #[env_config(name = "RUST_LOG", default = "info")]
    pub level: String,
    #[env_config(name = "ZO_LOG_JSON_FORMAT", default = false)]
    pub json_format: bool,
    #[env_config(name = "ZO_LOG_FILE_DIR", default = "")]
    pub file_dir: String,
    // default is: o2.{hostname}.log
    #[env_config(name = "ZO_LOG_FILE_NAME_PREFIX", default = "")]
    pub file_name_prefix: String,
    // logger timestamp local setup, eg: %Y-%m-%dT%H:%M:%SZ
    #[env_config(name = "ZO_LOG_LOCAL_TIME_FORMAT", default = "")]
    pub local_time_format: String,
    #[env_config(name = "ZO_EVENTS_ENABLED", default = false)]
    pub events_enabled: bool,
    #[env_config(
        name = "ZO_EVENTS_AUTH",
        default = "cm9vdEBleGFtcGxlLmNvbTpUZ0ZzZFpzTUZQdzg2SzRK"
    )]
    pub events_auth: String,
    #[env_config(
        name = "ZO_EVENTS_EP",
        default = "https://api.openobserve.ai/api/debug/events/_json"
    )]
    pub events_url: String,
    #[env_config(name = "ZO_EVENTS_BATCH_SIZE", default = 10)]
    pub events_batch_size: usize,
}

#[derive(Debug, EnvConfig)]
pub struct Etcd {
    #[env_config(name = "ZO_ETCD_ADDR", default = "localhost:2379")]
    pub addr: String,
    #[env_config(name = "ZO_ETCD_PREFIX", default = "/zinc/observe/")]
    pub prefix: String,
    #[env_config(name = "ZO_ETCD_CONNECT_TIMEOUT", default = 5)]
    pub connect_timeout: u64,
    #[env_config(name = "ZO_ETCD_COMMAND_TIMEOUT", default = 10)]
    pub command_timeout: u64,
    #[env_config(name = "ZO_ETCD_LOCK_WAIT_TIMEOUT", default = 3600)]
    pub lock_wait_timeout: u64,
    #[env_config(name = "ZO_ETCD_USER", default = "")]
    pub user: String,
    #[env_config(name = "ZO_ETCD_PASSWORD", default = "")]
    pub password: String,
    #[env_config(name = "ZO_ETCD_CLIENT_CERT_AUTH", default = false)]
    pub cert_auth: bool,
    #[env_config(name = "ZO_ETCD_TRUSTED_CA_FILE", default = "")]
    pub ca_file: String,
    #[env_config(name = "ZO_ETCD_CERT_FILE", default = "")]
    pub cert_file: String,
    #[env_config(name = "ZO_ETCD_KEY_FILE", default = "")]
    pub key_file: String,
    #[env_config(name = "ZO_ETCD_DOMAIN_NAME", default = "")]
    pub domain_name: String,
    #[env_config(name = "ZO_ETCD_LOAD_PAGE_SIZE", default = 1000)]
    pub load_page_size: i64,
}

#[derive(Debug, EnvConfig)]
pub struct Nats {
    #[env_config(name = "ZO_NATS_ADDR", default = "localhost:4222")]
    pub addr: String,
    #[env_config(name = "ZO_NATS_PREFIX", default = "o2_")]
    pub prefix: String,
    #[env_config(name = "ZO_NATS_USER", default = "")]
    pub user: String,
    #[env_config(name = "ZO_NATS_PASSWORD", default = "")]
    pub password: String,
    #[env_config(
        name = "ZO_NATS_REPLICAS",
        default = 3,
        help = "the copies of a given message to store in the NATS cluster.
        Can not be modified after bucket is initialized.
        To update this, delete and recreate the bucket."
    )]
    pub replicas: usize,
    #[env_config(
        name = "ZO_NATS_HISTORY",
        default = 3,
        help = "in the context of KV to configure how many historical entries to keep for a given bucket.
        Can not be modified after bucket is initialized.
        To update this, delete and recreate the bucket."
    )]
    pub history: i64,
    #[env_config(
        name = "ZO_NATS_DELIVER_POLICY",
        default = "all",
        help = "The point in the stream from which to receive messages, default is: all, valid option is: all, last, new."
    )]
    pub deliver_policy: String,
    #[env_config(name = "ZO_NATS_CONNECT_TIMEOUT", default = 5)]
    pub connect_timeout: u64,
    #[env_config(name = "ZO_NATS_COMMAND_TIMEOUT", default = 10)]
    pub command_timeout: u64,
    #[env_config(name = "ZO_NATS_LOCK_WAIT_TIMEOUT", default = 3600)]
    pub lock_wait_timeout: u64,
    #[env_config(name = "ZO_NATS_SUB_CAPACITY", default = 65535)]
    pub subscription_capacity: usize,
    #[env_config(name = "ZO_NATS_QUEUE_MAX_AGE", default = 60)] // days
    pub queue_max_age: u64,
}

#[derive(Debug, EnvConfig)]
pub struct S3 {
    #[env_config(name = "ZO_S3_PROVIDER", default = "")]
    pub provider: String,
    #[env_config(name = "ZO_S3_SERVER_URL", default = "")]
    pub server_url: String,
    #[env_config(name = "ZO_S3_REGION_NAME", default = "")]
    pub region_name: String,
    #[env_config(name = "ZO_S3_ACCESS_KEY", default = "")]
    pub access_key: String,
    #[env_config(name = "ZO_S3_SECRET_KEY", default = "")]
    pub secret_key: String,
    #[env_config(name = "ZO_S3_BUCKET_NAME", default = "")]
    pub bucket_name: String,
    #[env_config(name = "ZO_S3_BUCKET_PREFIX", default = "")]
    pub bucket_prefix: String,
    #[env_config(name = "ZO_S3_CONNECT_TIMEOUT", default = 10)] // seconds
    pub connect_timeout: u64,
    #[env_config(name = "ZO_S3_REQUEST_TIMEOUT", default = 3600)] // seconds
    pub request_timeout: u64,
    #[env_config(name = "ZO_S3_FEATURE_FORCE_HOSTED_STYLE", default = false)]
    pub feature_force_hosted_style: bool,
    #[env_config(name = "ZO_S3_FEATURE_HTTP1_ONLY", default = false)]
    pub feature_http1_only: bool,
    #[env_config(name = "ZO_S3_FEATURE_HTTP2_ONLY", default = false)]
    pub feature_http2_only: bool,
    #[env_config(name = "ZO_S3_ALLOW_INVALID_CERTIFICATES", default = false)]
    pub allow_invalid_certificates: bool,
    #[env_config(name = "ZO_S3_SYNC_TO_CACHE_INTERVAL", default = 600)] // seconds
    pub sync_to_cache_interval: u64,
    #[env_config(name = "ZO_S3_MAX_RETRIES", default = 10)]
    pub max_retries: usize,
    #[env_config(name = "ZO_S3_MAX_IDLE_PER_HOST", default = 0)]
    pub max_idle_per_host: usize,
    // https://github.com/hyperium/hyper/issues/2136#issuecomment-589488526
    #[env_config(name = "ZO_S3_CONNECTION_KEEPALIVE_TIMEOUT", default = 20)] // seconds
    pub keepalive_timeout: u64, // aws s3 by has timeout of 20 sec
    #[env_config(
        name = "ZO_S3_MULTI_PART_UPLOAD_SIZE",
        default = 100,
        help = "The size of the file will switch to multi-part upload in MB"
    )]
    pub multi_part_upload_size: usize,
}

#[derive(Debug, EnvConfig)]
pub struct Sns {
    #[env_config(name = "ZO_SNS_ENDPOINT", default = "")]
    pub endpoint: String,
    #[env_config(name = "ZO_SNS_CONNECT_TIMEOUT", default = 10)] // seconds
    pub connect_timeout: u64,
    #[env_config(name = "ZO_SNS_OPERATION_TIMEOUT", default = 30)] // seconds
    pub operation_timeout: u64,
}

#[derive(Debug, EnvConfig)]
pub struct Prometheus {
    #[env_config(name = "ZO_PROMETHEUS_HA_CLUSTER", default = "cluster")]
    pub ha_cluster_label: String,
    #[env_config(name = "ZO_PROMETHEUS_HA_REPLICA", default = "__replica__")]
    pub ha_replica_label: String,
}

#[derive(Debug, EnvConfig)]
pub struct RUM {
    #[env_config(name = "ZO_RUM_ENABLED", default = false)]
    pub enabled: bool,
    #[env_config(name = "ZO_RUM_CLIENT_TOKEN", default = "")]
    pub client_token: String,
    #[env_config(name = "ZO_RUM_APPLICATION_ID", default = "")]
    pub application_id: String,
    #[env_config(name = "ZO_RUM_SITE", default = "")]
    pub site: String,
    #[env_config(name = "ZO_RUM_SERVICE", default = "")]
    pub service: String,
    #[env_config(name = "ZO_RUM_ENV", default = "")]
    pub env: String,
    #[env_config(name = "ZO_RUM_VERSION", default = "")]
    pub version: String,
    #[env_config(name = "ZO_RUM_ORGANIZATION_IDENTIFIER", default = "")]
    pub organization_identifier: String,
    #[env_config(name = "ZO_RUM_API_VERSION", default = "")]
    pub api_version: String,
    #[env_config(name = "ZO_RUM_INSECURE_HTTP", default = false)]
    pub insecure_http: bool,
}

#[derive(Debug, EnvConfig)]
pub struct Pipeline {
    #[env_config(
        name = "ZO_PIPELINE_REMOTE_STREAM_WAL_DIR",
        default = "",
        help = "For the remote stream WAL directory, if the pipeline destination is a remote stream, we use a separate path to distinguish between local WAL and remote WAL"
    )]
    pub remote_stream_wal_dir: String,
    #[env_config(
        name = "ZO_PIPELINE_REMOTE_STREAM_CONCURRENT_COUNT",
        default = 30,
        help = "control the remote stream wal send concurrent count"
    )]
    pub remote_stream_wal_concurrent_count: usize,
    #[env_config(
        name = "ZO_PIPELINE_OFFSET_FLUSH_INTERVAL",
        default = 10,
        help = "flush remote stream wal sended-ok-offset interval"
    )]
    pub offset_flush_interval: u64,
    #[env_config(
        name = "ZO_PIPELINE_REMOTE_REQUEST_TIMEOUT",
        default = 600,
        help = "pipeline exporter client request timeout"
    )]
    pub remote_request_timeout: u64,
    #[env_config(
        name = "ZO_PIPELINE_REMOTE_REQUEST_MAX_RETRY_TIME",
        default = 86400,
        help = "pipeline exporter client request max retry times, default 1440 minutes(24 hours)， unit is seconds"
    )]
    pub remote_request_max_retry_time: u64,
    #[env_config(
        name = "ZO_PIPELINE_WAL_SIZE_LIMIT",
        default = 0,
        help = "pipeline wal dir data size limit, default is 50% of local volume available space, unit is MB"
    )]
    pub wal_size_limit: u64,
    #[env_config(
        name = "ZO_PIPELINE_MAX_CONNECTIONS",
        default = 1024,
        help = "pipeline exporter client max connections"
    )]
    pub max_connections: usize,
}

#[derive(EnvConfig)]
pub struct Encryption {
    #[env_config(name = "ZO_MASTER_ENCRYPTION_ALGORITHM", default = "")]
    pub algorithm: String,
    #[env_config(name = "ZO_MASTER_ENCRYPTION_KEY", default = "")]
    pub master_key: String,
}
#[derive(EnvConfig)]
pub struct HealthCheck {
    #[env_config(name = "ZO_HEALTH_CHECK_ENABLED", default = true)]
    pub enabled: bool,
    #[env_config(
        name = "ZO_HEALTH_CHECK_TIMEOUT",
        default = 5,
        help = "Health check timeout in seconds"
    )]
    pub timeout: u64,
    #[env_config(
        name = "ZO_HEALTH_CHECK_FAILED_TIMES",
        default = 3,
        help = "The node will be removed from consistent hash if health check failed exceed this times"
    )]
    pub failed_times: usize,
}

pub fn init() -> Config {
    dotenv_override().ok();
    let mut cfg = Config::init().expect("config init error");

    // set local mode
    if cfg.common.local_mode {
        cfg.common.node_role = "all".to_string();
        cfg.common.node_role_group = "".to_string();
    }

    // check limit config
    if let Err(e) = check_limit_config(&mut cfg) {
        panic!("limit config error: {e}");
    }

    // check common config
    if let Err(e) = check_common_config(&mut cfg) {
        panic!("common config error: {e}");
    }

    // check grpc config
    if let Err(e) = check_grpc_config(&mut cfg) {
        panic!("common config error: {e}");
    }

    // check http config
    if let Err(e) = check_http_config(&mut cfg) {
        panic!("common config error: {e}")
    }

    // check data path config
    if let Err(e) = check_path_config(&mut cfg) {
        panic!("data path config error: {e}");
    }

    // check memory cache
    if let Err(e) = check_memory_config(&mut cfg) {
        panic!("memory cache config error: {e}");
    }

    // check disk cache
    if let Err(e) = check_disk_cache_config(&mut cfg) {
        panic!("disk cache config error: {e}");
    }

    // check compact config
    if let Err(e) = check_compact_config(&mut cfg) {
        panic!("compact config error: {e}");
    }

    // check etcd config
    if let Err(e) = check_etcd_config(&mut cfg) {
        panic!("etcd config error: {e}");
    }

    // check s3 config
    if let Err(e) = check_s3_config(&mut cfg) {
        panic!("s3 config error: {e}");
    }

    // check sns config
    if let Err(e) = check_sns_config(&mut cfg) {
        panic!("sns config error: {e}");
    }

    if let Err(e) = check_encryption_config(&mut cfg) {
        panic!("encryption config error: {e}");
    }
    // check health check config
    if let Err(e) = check_health_check_config(&mut cfg) {
        panic!("health check config error: {e}");
    }

    // check pipeline config
    if let Err(e) = check_pipeline_config(&mut cfg) {
        panic!("pipeline config error: {e}");
    }

    cfg
}

fn check_limit_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    // set real cpu num
    cfg.limit.real_cpu_num = max(1, cgroup::get_cpu_limit());
    // set at least 2 threads
    let cpu_num = max(2, cfg.limit.real_cpu_num);
    cfg.limit.cpu_num = cpu_num;
    if cfg.limit.http_worker_num == 0 {
        cfg.limit.http_worker_num = cpu_num;
    }
    if cfg.limit.http_worker_max_blocking == 0 {
        cfg.limit.http_worker_max_blocking = 1024;
    }
    if cfg.limit.grpc_runtime_worker_num == 0 {
        cfg.limit.grpc_runtime_worker_num = cpu_num;
    }
    if cfg.limit.grpc_runtime_blocking_worker_num == 0 {
        cfg.limit.grpc_runtime_blocking_worker_num = 512;
    }
    if cfg.limit.job_runtime_worker_num == 0 {
        cfg.limit.job_runtime_worker_num = cpu_num;
    }
    if cfg.limit.job_runtime_blocking_worker_num == 0 {
        cfg.limit.job_runtime_blocking_worker_num = 512;
    }
    // HACK for thread_num equal to CPU core * 4
    if cfg.limit.query_thread_num == 0 {
        if cfg.common.local_mode {
            cfg.limit.query_thread_num = cpu_num * 2;
        } else {
            cfg.limit.query_thread_num = cpu_num * 4;
        }
    }
    // HACK for move_file_thread_num equal to CPU core
    if cfg.limit.file_move_thread_num == 0 {
        if cfg.common.local_mode {
            cfg.limit.file_move_thread_num = std::cmp::max(1, cpu_num / 2);
        } else {
            cfg.limit.file_move_thread_num = cpu_num;
        }
    }
    // HACK for file_merge_thread_num equal to CPU core
    if cfg.limit.file_merge_thread_num == 0 {
        if cfg.common.local_mode {
            cfg.limit.file_merge_thread_num = std::cmp::max(1, cpu_num / 2);
        } else {
            cfg.limit.file_merge_thread_num = cpu_num;
        }
    }
    // HACK for mem_dump_thread_num equal to CPU core
    if cfg.limit.mem_dump_thread_num == 0 {
        cfg.limit.mem_dump_thread_num = cpu_num;
    }
    // HACK for usage_reporting_thread_num equal to half of CPU core
    if cfg.limit.usage_reporting_thread_num == 0 {
        if cfg.common.local_mode {
            cfg.limit.usage_reporting_thread_num = std::cmp::max(1, cpu_num / 2);
        } else {
            cfg.limit.usage_reporting_thread_num = cpu_num;
        }
    }
    if cfg.limit.file_push_interval == 0 {
        cfg.limit.file_push_interval = 10;
    }
    if cfg.limit.file_push_limit == 0 {
        cfg.limit.file_push_limit = 10000;
    }

    if cfg.limit.sql_db_connections_min == 0 {
        cfg.limit.sql_db_connections_min = cpu_num as u32
    }

    if cfg.limit.sql_db_connections_max == 0 {
        cfg.limit.sql_db_connections_max = cfg.limit.sql_db_connections_min * 2
    }
    cfg.limit.sql_db_connections_max =
        max(REQUIRED_DB_CONNECTIONS, cfg.limit.sql_db_connections_max);

    if cfg.limit.file_list_id_batch_size == 0 {
        cfg.limit.file_list_id_batch_size = 5000;
    }

    if cfg.limit.consistent_hash_vnodes == 0 {
        cfg.limit.consistent_hash_vnodes = 100;
    }

    // check for uds
    #[allow(deprecated)]
    if cfg.limit.udschema_max_fields > 0 {
        cfg.limit.schema_max_fields_to_enable_uds = cfg.limit.udschema_max_fields;
    }

    Ok(())
}

fn check_common_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.limit.file_push_interval == 0 {
        cfg.limit.file_push_interval = 60;
    }
    if cfg.limit.req_cols_per_record_limit == 0 {
        cfg.limit.req_cols_per_record_limit = 1000;
    }

    // check max_file_size_on_disk to MB
    if cfg.limit.max_file_size_on_disk == 0 {
        cfg.limit.max_file_size_on_disk = 128 * 1024 * 1024; // 128MB
    } else {
        cfg.limit.max_file_size_on_disk *= 1024 * 1024;
    }
    // check max_file_size_in_memory to MB
    if cfg.limit.max_file_size_in_memory == 0 {
        cfg.limit.max_file_size_in_memory = 128 * 1024 * 1024; // 128MB
    } else {
        cfg.limit.max_file_size_in_memory *= 1024 * 1024;
    }

    // check for metrics limit
    if cfg.limit.metrics_max_series_per_query == 0 {
        cfg.limit.metrics_max_series_per_query = 30_000;
    }
    if cfg.limit.metrics_max_points_per_series == 0 {
        cfg.limit.metrics_max_points_per_series = 30_000;
    }
    if cfg.limit.metrics_cache_max_entries == 0 {
        cfg.limit.metrics_cache_max_entries = 100_000;
    }

    // check search job retention
    if cfg.limit.search_job_retention == 0 {
        return Err(anyhow::anyhow!("search job retention is set to zero"));
    }

    // HACK instance_name
    if cfg.common.instance_name.is_empty() {
        cfg.common.instance_name = sysinfo::System::new().host_name().unwrap();
    }
    cfg.common.instance_name_short = cfg
        .common
        .instance_name
        .split('.')
        .next()
        .unwrap()
        .to_string();

    // HACK for tracing, always disable tracing except ingester and querier
    let local_node_role: Vec<cluster::Role> = cfg
        .common
        .node_role
        .clone()
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    if !local_node_role.contains(&cluster::Role::All)
        && !local_node_role.contains(&cluster::Role::Ingester)
        && !local_node_role.contains(&cluster::Role::Querier)
    {
        cfg.common.tracing_enabled = false;
    }

    if local_node_role.contains(&cluster::Role::ScriptServer) {
        // script server does not have external dep, so can ignore their config check
        return Ok(());
    }

    // format local_mode_storage
    cfg.common.local_mode_storage = cfg.common.local_mode_storage.to_lowercase();

    // format metadata storage
    if cfg.common.meta_store.is_empty() {
        if cfg.common.local_mode {
            cfg.common.meta_store = "sqlite".to_string();
        } else {
            cfg.common.meta_store = "etcd".to_string();
        }
    }
    cfg.common.meta_store = cfg.common.meta_store.to_lowercase();
    if !cfg.common.local_mode
        && !cfg.common.meta_store.starts_with("postgres")
        && !cfg.common.meta_store.starts_with("mysql")
    {
        return Err(anyhow::anyhow!(
            "Meta store only support mysql or postgres in cluster mode."
        ));
    }
    if cfg.common.meta_store.starts_with("postgres") && cfg.common.meta_postgres_dsn.is_empty() {
        return Err(anyhow::anyhow!(
            "Meta store is PostgreSQL, you must set ZO_META_POSTGRES_DSN"
        ));
    }
    if cfg.common.meta_store.starts_with("mysql") && cfg.common.meta_mysql_dsn.is_empty() {
        return Err(anyhow::anyhow!(
            "Meta store is MySQL, you must set ZO_META_MYSQL_DSN"
        ));
    }

    // If the default scrape interval is less than 5s, raise an error
    if cfg.common.default_scrape_interval < 5 {
        return Err(anyhow::anyhow!(
            "Default scrape interval can not be set to lesser than 5s ."
        ));
    }

    // check bloom filter ndv ratio
    if cfg.common.bloom_filter_ndv_ratio == 0 {
        cfg.common.bloom_filter_ndv_ratio = 100;
    }

    // check default inverted index search format
    #[allow(deprecated)]
    {
        cfg.common.inverted_index_store_format =
            cfg.common.inverted_index_store_format.to_lowercase();
        if cfg.common.inverted_index_store_format.is_empty() {
            cfg.common.inverted_index_store_format = "parquet".to_string();
        }
        if !["both", "parquet", "tantivy"]
            .contains(&cfg.common.inverted_index_store_format.as_str())
        {
            return Err(anyhow::anyhow!(
                "ZO_INVERTED_INDEX_STORE_FORMAT must be one of parquet, tantivy, both."
            ));
        }
        cfg.common.inverted_index_search_format =
            cfg.common.inverted_index_search_format.to_lowercase();
        if cfg.common.inverted_index_search_format.is_empty() {
            cfg.common.inverted_index_search_format =
                cfg.common.inverted_index_store_format.clone();
        }
        if cfg.common.inverted_index_search_format == "both" {
            cfg.common.inverted_index_search_format = "parquet".to_string();
        }
        if !["parquet", "tantivy"].contains(&cfg.common.inverted_index_search_format.as_str()) {
            return Err(anyhow::anyhow!(
                "ZO_INVERTED_INDEX_SEARCH_FORMAT must be one of parquet, tantivy."
            ));
        }
    }

    // check for join match one
    if cfg.common.feature_join_match_one_enabled && cfg.common.feature_join_right_side_max_rows == 0
    {
        cfg.common.feature_join_right_side_max_rows = 50_000;
    }

    Ok(())
}

fn check_grpc_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.grpc.tls_enabled
        && (cfg.grpc.tls_cert_domain.is_empty()
            || cfg.grpc.tls_cert_path.is_empty()
            || cfg.grpc.tls_key_path.is_empty())
    {
        return Err(anyhow::anyhow!("ZO_GRPC_TLS_CERT_DOMAIN, ZO_GRPC_TLS_CERT_PATH and ZO_GRPC_TLS_KEY_PATH must be set when ZO_GRPC_TLS_ENABLED is true"));
    }
    Ok(())
}

fn check_http_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.http.tls_enabled
        && (cfg.http.tls_cert_path.is_empty() || cfg.http.tls_key_path.is_empty())
    {
        return Err(anyhow::anyhow!(
            "When ZO_HTTP_TLS_ENABLED=true, both ZO_HTTP_TLS_CERT_PATH \
             and ZO_HTTP_TLS_KEY_PATH must be set."
        ));
    }
    Ok(())
}

fn check_path_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    // for web
    if cfg.common.web_url.ends_with('/') {
        cfg.common.web_url = cfg.common.web_url.trim_end_matches('/').to_string();
    }
    if cfg.common.base_uri.ends_with('/') {
        cfg.common.base_uri = cfg.common.base_uri.trim_end_matches('/').to_string();
    }
    // for data
    if cfg.common.data_dir.is_empty() {
        cfg.common.data_dir = "./data/openobserve/".to_string();
    }
    if !cfg.common.data_dir.ends_with('/') {
        cfg.common.data_dir = format!("{}/", cfg.common.data_dir);
    }
    if cfg.common.data_wal_dir.is_empty() {
        cfg.common.data_wal_dir = format!("{}wal/", cfg.common.data_dir);
    }
    if !cfg.common.data_wal_dir.ends_with('/') {
        cfg.common.data_wal_dir = format!("{}/", cfg.common.data_wal_dir);
    }
    if cfg.common.data_stream_dir.is_empty() {
        cfg.common.data_stream_dir = format!("{}stream/", cfg.common.data_dir);
    }
    if !cfg.common.data_stream_dir.ends_with('/') {
        cfg.common.data_stream_dir = format!("{}/", cfg.common.data_stream_dir);
    }
    if cfg.common.data_db_dir.is_empty() {
        cfg.common.data_db_dir = format!("{}db/", cfg.common.data_dir);
    }
    if !cfg.common.data_db_dir.ends_with('/') {
        cfg.common.data_db_dir = format!("{}/", cfg.common.data_db_dir);
    }
    if cfg.common.data_cache_dir.is_empty() {
        cfg.common.data_cache_dir = format!("{}cache/", cfg.common.data_dir);
    }
    if !cfg.common.data_cache_dir.ends_with('/') {
        cfg.common.data_cache_dir = format!("{}/", cfg.common.data_cache_dir);
    }
    if cfg.common.mmdb_data_dir.is_empty() {
        cfg.common.mmdb_data_dir = format!("{}mmdb/", cfg.common.data_dir);
    }
    if !cfg.common.mmdb_data_dir.ends_with('/') {
        cfg.common.mmdb_data_dir = format!("{}/", cfg.common.mmdb_data_dir);
    }

    // check for pprof flamegraph
    if cfg.profiling.pprof_flamegraph_path.is_empty() {
        cfg.profiling.pprof_flamegraph_path = format!("{}flamegraph.svg", cfg.common.data_dir);
    }

    Ok(())
}

fn check_etcd_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if !cfg.etcd.prefix.is_empty() && !cfg.etcd.prefix.ends_with('/') {
        cfg.etcd.prefix = format!("{}/", cfg.etcd.prefix);
    }

    if !cfg.etcd.cert_auth {
        return Ok(());
    }
    if let Err(e) = get_file_meta(&cfg.etcd.ca_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }
    if let Err(e) = get_file_meta(&cfg.etcd.cert_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }
    if let Err(e) = get_file_meta(&cfg.etcd.key_file) {
        return Err(anyhow::anyhow!("ZO_ETCD_TRUSTED_CA_FILE check err: {}", e));
    }

    // check domain name
    if cfg.etcd.domain_name.is_empty() {
        let mut name = cfg.etcd.addr.clone();
        if name.contains("//") {
            name = name.split("//").collect::<Vec<&str>>()[1].to_string();
        }
        if name.contains(':') {
            name = name.split(':').collect::<Vec<&str>>()[0].to_string();
        }
        cfg.etcd.domain_name = name;
    }

    Ok(())
}

fn check_memory_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    let mem_total = cgroup::get_memory_limit();
    cfg.limit.mem_total = mem_total;
    if cfg.memory_cache.max_size == 0 {
        if cfg.common.local_mode {
            cfg.memory_cache.max_size = mem_total / 4; // 25%
        } else {
            cfg.memory_cache.max_size = mem_total / 2; // 50%
        }
    } else {
        cfg.memory_cache.max_size *= 1024 * 1024;
    }
    if cfg.memory_cache.skip_size == 0 {
        // will skip the cache when a query need cache great than this value, default is
        // 50% of max_size
        cfg.memory_cache.skip_size = cfg.memory_cache.max_size / 2;
    } else {
        cfg.memory_cache.skip_size *= 1024 * 1024;
    }
    if cfg.memory_cache.release_size == 0 {
        // when cache is full will release how many data once time, default is 1% of
        // max_size
        cfg.memory_cache.release_size = cfg.memory_cache.max_size / 100;
    } else {
        cfg.memory_cache.release_size *= 1024 * 1024;
    }
    if cfg.memory_cache.gc_size == 0 {
        cfg.memory_cache.gc_size = 10 * 1024 * 1024; // 10 MB
    } else {
        cfg.memory_cache.gc_size *= 1024 * 1024;
    }
    if cfg.memory_cache.datafusion_max_size == 0 {
        if cfg.common.local_mode {
            cfg.memory_cache.datafusion_max_size = (mem_total - cfg.memory_cache.max_size) / 2; // 25%
        } else {
            cfg.memory_cache.datafusion_max_size = mem_total - cfg.memory_cache.max_size; // 50%
        }
    } else {
        cfg.memory_cache.datafusion_max_size *= 1024 * 1024;
    }

    if cfg.memory_cache.bucket_num == 0 {
        cfg.memory_cache.bucket_num = cfg.limit.real_cpu_num;
    }
    cfg.memory_cache.max_size /= cfg.memory_cache.bucket_num;
    cfg.memory_cache.release_size /= cfg.memory_cache.bucket_num;
    cfg.memory_cache.gc_size /= cfg.memory_cache.bucket_num;

    // for memtable limit check
    if cfg.limit.mem_table_max_size == 0 {
        if cfg.common.local_mode {
            cfg.limit.mem_table_max_size = mem_total / 4; // 25%
        } else {
            cfg.limit.mem_table_max_size = mem_total / 2; // 50%
        }
    } else {
        cfg.limit.mem_table_max_size *= 1024 * 1024;
    }
    if cfg.limit.mem_table_bucket_num == 0 {
        cfg.limit.mem_table_bucket_num = 1;
    }

    // wal
    if cfg.limit.wal_write_buffer_size < 4096 {
        cfg.limit.wal_write_buffer_size = 4096;
    }
    if cfg.limit.wal_write_queue_size == 0 {
        cfg.limit.wal_write_queue_size = 10000;
    }

    // check query settings
    if cfg.limit.query_group_base_speed == 0 {
        cfg.limit.query_group_base_speed = SIZE_IN_GB as usize;
    } else {
        cfg.limit.query_group_base_speed *= 1024 * 1024;
    }
    if cfg.limit.query_partition_by_secs == 0 {
        cfg.limit.query_partition_by_secs = 30;
    }
    if cfg.limit.query_default_limit == 0 {
        cfg.limit.query_default_limit = 1000;
    }
    Ok(())
}

fn check_disk_cache_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    let mut system = sysinfo::System::new();
    system.refresh_disks_list();
    let mut disks: Vec<(&str, u64, u64)> = system
        .disks()
        .iter()
        .map(|d| {
            (
                d.mount_point().to_str().unwrap(),
                d.total_space(),
                d.available_space(),
            )
        })
        .collect();
    disks.sort_by(|a, b| b.0.cmp(a.0));

    std::fs::create_dir_all(&cfg.common.data_cache_dir).expect("create cache dir success");
    let cache_dir = Path::new(&cfg.common.data_cache_dir)
        .canonicalize()
        .unwrap();
    let cache_dir = cache_dir.to_str().unwrap();
    let disk = disks.iter().find(|d| cache_dir.starts_with(d.0));
    let (disk_total, disk_free) = match disk {
        Some(d) => (d.1, d.2),
        None => (0, 0),
    };
    cfg.limit.disk_total = disk_total as usize;
    cfg.limit.disk_free = disk_free as usize;
    if cfg.disk_cache.max_size == 0 {
        cfg.disk_cache.max_size = cfg.limit.disk_free / 2; // 50%
        if cfg.disk_cache.max_size > 1024 * 1024 * 1024 * 100 {
            cfg.disk_cache.max_size = 1024 * 1024 * 1024 * 100; // 100GB
        }
    } else {
        cfg.disk_cache.max_size *= 1024 * 1024;
    }

    if cfg.disk_cache.result_max_size == 0 {
        cfg.disk_cache.result_max_size = cfg.limit.disk_free / 10; // 10%
        if cfg.disk_cache.result_max_size > 1024 * 1024 * 1024 * 20 {
            cfg.disk_cache.result_max_size = 1024 * 1024 * 1024 * 20; // 20GB
        }
    } else {
        cfg.disk_cache.result_max_size *= 1024 * 1024;
    }
    if cfg.disk_cache.skip_size == 0 {
        // will skip the cache when a query need cache great than this value, default is
        // 50% of max_size
        cfg.disk_cache.skip_size = cfg.disk_cache.max_size / 2;
    } else {
        cfg.disk_cache.skip_size *= 1024 * 1024;
    }
    if cfg.disk_cache.release_size == 0 {
        // when cache is full will release how many data once time, default is 1% of
        // max_size
        cfg.disk_cache.release_size = cfg.disk_cache.max_size / 100;
    } else {
        cfg.disk_cache.release_size *= 1024 * 1024;
    }
    if cfg.disk_cache.gc_size == 0 {
        cfg.disk_cache.gc_size = 10 * 1024 * 1024; // 10 MB
    } else {
        cfg.disk_cache.gc_size *= 1024 * 1024;
    }

    if cfg.disk_cache.multi_dir.contains('/') {
        return Err(anyhow::anyhow!(
            "ZO_DISK_CACHE_MULTI_DIR only supports a single directory level, can not contains / "
        ));
    }

    if cfg.disk_cache.bucket_num == 0 {
        // because we validate thread_query_num before this
        // we can be sure here that that value is sane.

        // following numbers are imperically decided, users can set the value
        // directly if they know better, otherwise this was the best numbers
        // for bucket_num based on thread count.
        let threads = cfg.limit.query_thread_num;
        if threads <= 16 {
            // for less than 16 threads, same buckets would be good enough
            // with 16 files in parallel we should not run into that many
            // files going into same bucket, so ok.
            cfg.disk_cache.bucket_num = cfg.limit.query_thread_num;
        } else if threads > 16 && threads <= 64 {
            // for 32 -> 64 ish range, there can be a lot of collisions
            // so we set it to double the threads to avoid any collisions
            cfg.disk_cache.bucket_num = 2 * threads;
        } else {
            // for > 64 threads, it was observed that even with 1.5 times buckets
            // it is ok, not that many collisions. This is imperical, no concrete
            // reasoning for 1.5
            cfg.disk_cache.bucket_num = (threads as f64 * 1.5) as usize;
        }
    }
    cfg.disk_cache.bucket_num = max(
        cfg.disk_cache.bucket_num,
        cfg.disk_cache
            .multi_dir
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .count(),
    );
    cfg.disk_cache.max_size /= cfg.disk_cache.bucket_num;
    cfg.disk_cache.release_size /= cfg.disk_cache.bucket_num;
    cfg.disk_cache.gc_size /= cfg.disk_cache.bucket_num;

    // check disk cache with tantivy mode
    cfg.common.inverted_index_tantivy_mode = cfg.common.inverted_index_tantivy_mode.to_lowercase();
    if cfg.common.inverted_index_tantivy_mode.is_empty() {
        cfg.common.inverted_index_tantivy_mode = "puffin".to_string();
    }
    if !cfg.disk_cache.enabled && cfg.common.inverted_index_tantivy_mode == "mmap" {
        return Err(anyhow::anyhow!(
            "Inverted index tantivy mode can not be set to mmap when disk cache is disabled."
        ));
    }

    Ok(())
}

fn check_compact_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.compact.data_retention_days > 0 && cfg.compact.data_retention_days < 3 {
        return Err(anyhow::anyhow!(
            "Data retention is not allowed to be less than 3 days."
        ));
    }
    if cfg.compact.interval < 1 {
        cfg.compact.interval = 60;
    }

    // check compact_max_file_size to MB
    if cfg.compact.max_file_size < 1 {
        cfg.compact.max_file_size = 512;
    }
    cfg.compact.max_file_size *= 1024 * 1024;
    if cfg.compact.delete_files_delay_hours < 1 {
        cfg.compact.delete_files_delay_hours = 2;
    }

    if cfg.compact.old_data_interval < 1 {
        cfg.compact.old_data_interval = 3600;
    }
    if cfg.compact.old_data_max_days < 1 {
        cfg.compact.old_data_max_days = 7;
    }
    if cfg.compact.old_data_min_hours < 1 {
        cfg.compact.old_data_min_hours = 2;
    }
    if cfg.compact.old_data_min_records < 1 {
        cfg.compact.old_data_min_records = 100;
    }
    if cfg.compact.old_data_min_files < 1 {
        cfg.compact.old_data_min_files = 10;
    }

    if cfg.compact.batch_size < 1 {
        cfg.compact.batch_size = 100;
    }
    if cfg.compact.pending_jobs_metric_interval == 0 {
        cfg.compact.pending_jobs_metric_interval = 300;
    }

    Ok(())
}

fn check_sns_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    // Validate endpoint URL if provided
    if !cfg.sns.endpoint.is_empty()
        && !cfg.sns.endpoint.starts_with("http://")
        && !cfg.sns.endpoint.starts_with("https://")
    {
        return Err(anyhow::anyhow!(
            "Invalid SNS endpoint URL. It must start with http:// or https://"
        ));
    }

    // Validate timeouts
    if cfg.sns.connect_timeout == 0 {
        cfg.sns.connect_timeout = 10; // Default to 10 seconds if not set
        log::warn!("SNS connect timeout not specified, defaulting to 10 seconds");
    }
    if cfg.sns.operation_timeout == 0 {
        cfg.sns.operation_timeout = 30; // Default to 30 seconds if not set
        log::warn!("SNS operation timeout not specified, defaulting to 30 seconds");
    }

    Ok(())
}

fn check_s3_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if !cfg.s3.bucket_prefix.is_empty() && !cfg.s3.bucket_prefix.ends_with('/') {
        cfg.s3.bucket_prefix = format!("{}/", cfg.s3.bucket_prefix);
    }
    if cfg.s3.provider.is_empty() {
        if cfg.s3.server_url.contains(".googleapis.com") {
            cfg.s3.provider = "gcs".to_string();
        } else if cfg.s3.server_url.contains(".aliyuncs.com") {
            cfg.s3.provider = "oss".to_string();
            if !cfg
                .s3
                .server_url
                .contains(&format!("://{}.", cfg.s3.bucket_name))
            {
                cfg.s3.server_url = cfg
                    .s3
                    .server_url
                    .replace("://", &format!("://{}.", cfg.s3.bucket_name));
            }
        } else {
            cfg.s3.provider = "aws".to_string();
        }
    }
    cfg.s3.provider = cfg.s3.provider.to_lowercase();
    if cfg.s3.provider.eq("swift") {
        std::env::set_var("AWS_EC2_METADATA_DISABLED", "true");
    }

    if cfg.s3.keepalive_timeout == 0 {
        // reset to default
        cfg.s3.keepalive_timeout = 20;
    }

    Ok(())
}

fn check_pipeline_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    // pipeline
    if cfg.pipeline.remote_stream_wal_dir.is_empty() {
        cfg.pipeline.remote_stream_wal_dir = format!("{}remote_stream_wal/", cfg.common.data_dir);
    }

    if !cfg.pipeline.remote_stream_wal_dir.is_empty()
        && !cfg.pipeline.remote_stream_wal_dir.ends_with('/')
    {
        cfg.pipeline.remote_stream_wal_dir = format!("{}/", cfg.pipeline.remote_stream_wal_dir);
    }

    if cfg.pipeline.offset_flush_interval == 0 {
        cfg.pipeline.offset_flush_interval = 10;
    }
    if cfg.pipeline.remote_request_max_retry_time == 0 {
        cfg.pipeline.remote_request_max_retry_time = 86400; // 24 hours, in seconds
    }

    if cfg.pipeline.wal_size_limit == 0 {
        cfg.pipeline.wal_size_limit = cfg.limit.disk_free as u64 / 2; // 50%
        if cfg.pipeline.wal_size_limit > 1024 * 1024 * 1024 * 100 {
            cfg.pipeline.wal_size_limit = 1024 * 1024 * 1024 * 100; // 100GB
        }
    } else {
        cfg.pipeline.wal_size_limit *= 1024 * 1024;
    }
    Ok(())
}

fn check_health_check_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if cfg.health_check.timeout == 0 {
        cfg.health_check.timeout = 10;
    }
    if cfg.health_check.failed_times == 0 {
        cfg.health_check.failed_times = 5;
    }
    Ok(())
}

#[inline]
pub fn is_local_disk_storage() -> bool {
    let cfg = get_config();
    cfg.common.local_mode
        && (cfg.common.local_mode_storage == "disk" || cfg.common.local_mode_storage == "local")
}

#[inline]
pub fn get_cluster_name() -> String {
    let cfg = get_config();
    if !cfg.common.cluster_name.is_empty() {
        cfg.common.cluster_name.to_string()
    } else {
        INSTANCE_ID.get("instance_id").unwrap().to_string()
    }
}

fn check_encryption_config(cfg: &mut Config) -> Result<(), anyhow::Error> {
    if !cfg.encryption.algorithm.is_empty() {
        if cfg.encryption.algorithm != "aes-256-siv" {
            return Err(anyhow::anyhow!(
                "invalid algorithm specified, only [aes-256-siv] is supported"
            ));
        }
        // this is basically a duplication of code from tables/cipher.rs
        // but we only support one algorithm for now, so ok. Once we support more
        // we have to extract this into proper functions and use the same in both places
        let key = match BASE64_STANDARD.decode(&cfg.encryption.master_key) {
            Ok(v) => v,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "master encryption key is not properly base64 encoded : {e}"
                ));
            }
        };
        match Aes256Siv::new_from_slice(&key) {
            Ok(_) => {}
            Err(e) => {
                return Err(anyhow::anyhow!("invalid master encryption key : {e}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config() {
        let mut cfg = Config::init().unwrap();
        let ret = check_limit_config(&mut cfg);
        assert!(ret.is_ok());

        cfg.s3.server_url = "https://storage.googleapis.com".to_string();
        cfg.s3.provider = "".to_string();
        check_s3_config(&mut cfg).unwrap();
        assert_eq!(cfg.s3.provider, "gcs");
        cfg.s3.server_url = "https://oss-cn-beijing.aliyuncs.com".to_string();
        cfg.s3.provider = "".to_string();
        check_s3_config(&mut cfg).unwrap();
        assert_eq!(cfg.s3.provider, "oss");
        cfg.s3.server_url = "".to_string();
        cfg.s3.provider = "".to_string();
        check_s3_config(&mut cfg).unwrap();
        assert_eq!(cfg.s3.provider, "aws");

        // SNS configuration tests
        // Test default values
        check_sns_config(&mut cfg).unwrap();
        assert_eq!(cfg.sns.connect_timeout, 10);
        assert_eq!(cfg.sns.operation_timeout, 30);
        assert!(cfg.sns.endpoint.is_empty());

        // Test custom endpoint
        cfg.sns.endpoint = "https://sns.us-west-2.amazonaws.com".to_string();
        check_sns_config(&mut cfg).unwrap();
        assert_eq!(cfg.sns.endpoint, "https://sns.us-west-2.amazonaws.com");

        // Test custom timeouts
        cfg.sns.connect_timeout = 15;
        cfg.sns.operation_timeout = 45;
        check_sns_config(&mut cfg).unwrap();
        assert_eq!(cfg.sns.connect_timeout, 15);
        assert_eq!(cfg.sns.operation_timeout, 45);

        // Test zero values (should set to defaults)
        cfg.sns.connect_timeout = 0;
        cfg.sns.operation_timeout = 0;
        check_sns_config(&mut cfg).unwrap();
        assert_eq!(cfg.sns.connect_timeout, 10);
        assert_eq!(cfg.sns.operation_timeout, 30);

        // Test endpoint URL validation
        cfg.sns.endpoint = "invalid-url".to_string();
        assert!(check_sns_config(&mut cfg).is_err());

        cfg.memory_cache.max_size = 1024;
        cfg.memory_cache.release_size = 1024;
        cfg.memory_cache.bucket_num = 1;
        check_memory_config(&mut cfg).unwrap();
        assert_eq!(cfg.memory_cache.max_size, 1024 * 1024 * 1024);
        assert_eq!(cfg.memory_cache.release_size, 1024 * 1024 * 1024);

        cfg.limit.file_push_interval = 0;
        cfg.limit.req_cols_per_record_limit = 0;
        cfg.compact.interval = 0;
        cfg.compact.data_retention_days = 10;
        let ret = check_common_config(&mut cfg);
        assert!(ret.is_ok());
        assert_eq!(cfg.compact.data_retention_days, 10);
        assert_eq!(cfg.limit.req_cols_per_record_limit, 1000);

        cfg.compact.data_retention_days = 2;
        let ret = check_compact_config(&mut cfg);
        assert!(ret.is_err());

        cfg.common.data_dir = "".to_string();
        let ret = check_path_config(&mut cfg);
        assert!(ret.is_ok());

        cfg.common.data_dir = "/abc".to_string();
        cfg.common.data_wal_dir = "/abc".to_string();
        cfg.common.data_stream_dir = "/abc".to_string();
        cfg.common.base_uri = "/abc/".to_string();
        let ret = check_path_config(&mut cfg);
        assert!(ret.is_ok());
        assert_eq!(cfg.common.data_dir, "/abc/".to_string());
        assert_eq!(cfg.common.data_wal_dir, "/abc/".to_string());
        assert_eq!(cfg.common.data_stream_dir, "/abc/".to_string());
        assert_eq!(cfg.common.data_dir, "/abc/".to_string());
        assert_eq!(cfg.common.base_uri, "/abc".to_string());
    }
}
