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

use config::cluster::LOCAL_NODE;
use infra::file_list as infra_file_list;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;
use regex::Regex;

use crate::{
    common::{
        infra::config::SYSLOG_ENABLED,
        meta::{organization::DEFAULT_ORG, user::UserRequest},
    },
    service::{db, self_reporting, users},
};

mod alert_manager;
#[cfg(feature = "enterprise")]
mod cipher;
mod compactor;
pub(crate) mod files;
mod flatten_compactor;
pub mod metrics;
mod mmdb_downloader;
mod promql;
mod promql_self_consume;
mod stats;
pub(crate) mod syslog_server;
mod telemetry;

pub use mmdb_downloader::MMDB_INIT_NOTIFIER;

pub async fn init() -> Result<(), anyhow::Error> {
    let email_regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.-]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )
    .expect("Email regex is valid");

    let cfg = config::get_config();
    // init root user
    if !db::user::root_user_exists().await {
        if cfg.auth.root_user_email.is_empty()
            || !email_regex.is_match(&cfg.auth.root_user_email)
            || cfg.auth.root_user_password.is_empty()
        {
            panic!(
                "Please set root user email-id & password using ZO_ROOT_USER_EMAIL & ZO_ROOT_USER_PASSWORD environment variables. This can also indicate an invalid email ID. Email ID must comply with ([a-z0-9_+]([a-z0-9_+.-]*[a-z0-9_+])?)@([a-z0-9]+([\\-\\.]{{1}}[a-z0-9]+)*\\.[a-z]{{2,6}})"
            );
        }
        let _ = users::create_root_user(
            DEFAULT_ORG,
            UserRequest {
                email: cfg.auth.root_user_email.clone(),
                password: cfg.auth.root_user_password.clone(),
                role: crate::common::meta::user::UserRole::Root,
                first_name: "root".to_owned(),
                last_name: "".to_owned(),
                is_external: false,
            },
        )
        .await;
    }

    if !cfg.common.mmdb_disable_download {
        // Try to download the mmdb files, if its not disabled.
        tokio::task::spawn(async move { mmdb_downloader::run().await });
    }
    // cache users
    tokio::task::spawn(async move { db::user::watch().await });
    db::user::cache().await.expect("user cache failed");

    db::organization::cache()
        .await
        .expect("organization cache sync failed");

    // check version
    db::version::set().await.expect("db version set failed");

    // Auth auditing should be done by router also
    #[cfg(feature = "enterprise")]
    tokio::task::spawn(async move { self_reporting::run_audit_publish().await });

    tokio::task::spawn(async move { promql_self_consume::run().await });
    // Router doesn't need to initialize job
    if LOCAL_NODE.is_router() {
        return Ok(());
    }

    // telemetry run
    if cfg.common.telemetry_enabled && LOCAL_NODE.is_querier() {
        tokio::task::spawn(async move { telemetry::run().await });
    }

    tokio::task::spawn(async move { self_reporting::run().await });

    // cache short_urls
    tokio::task::spawn(async move { db::short_url::watch().await });
    db::short_url::cache()
        .await
        .expect("short url cache failed");

    // initialize metadata watcher
    tokio::task::spawn(async move { db::schema::watch().await });
    tokio::task::spawn(async move { db::functions::watch().await });
    tokio::task::spawn(async move { db::compact::retention::watch().await });
    tokio::task::spawn(async move { db::metrics::watch_prom_cluster_leader().await });
    tokio::task::spawn(async move { db::alerts::templates::watch().await });
    tokio::task::spawn(async move { db::alerts::destinations::watch().await });
    tokio::task::spawn(async move { db::alerts::realtime_triggers::watch().await });
    tokio::task::spawn(async move { db::alerts::alert::watch().await });
    tokio::task::spawn(async move { db::dashboards::reports::watch().await });
    tokio::task::spawn(async move { db::organization::watch().await });
    #[cfg(feature = "enterprise")]
    tokio::task::spawn(async move { db::ofga::watch().await });

    #[cfg(feature = "enterprise")]
    if !LOCAL_NODE.is_compactor() || LOCAL_NODE.is_single_node() {
        tokio::task::spawn(async move { db::session::watch().await });
    }
    if !LOCAL_NODE.is_compactor() && !LOCAL_NODE.is_router() {
        tokio::task::spawn(async move { db::enrichment_table::watch().await });
    }

    tokio::task::yield_now().await;

    // cache core metadata
    db::schema::cache().await.expect("stream cache failed");
    db::functions::cache()
        .await
        .expect("functions cache failed");
    db::compact::retention::cache()
        .await
        .expect("compact delete cache failed");
    db::metrics::cache_prom_cluster_leader()
        .await
        .expect("prom cluster leader cache failed");

    // cache alerts
    db::alerts::templates::cache()
        .await
        .expect("alerts templates cache failed");
    db::alerts::destinations::cache()
        .await
        .expect("alerts destinations cache failed");
    db::alerts::realtime_triggers::cache()
        .await
        .expect("alerts realtime triggers cache failed");
    db::alerts::alert::cache()
        .await
        .expect("alerts cache failed");
    db::dashboards::reports::cache()
        .await
        .expect("reports cache failed");
    db::syslog::cache().await.expect("syslog cache failed");
    db::syslog::cache_syslog_settings()
        .await
        .expect("syslog settings cache failed");

    // cache pipeline
    db::pipeline::cache().await.expect("Pipeline cache failed");

    infra_file_list::create_table_index().await?;
    infra_file_list::LOCAL_CACHE.create_table_index().await?;
    tokio::task::spawn(async move { db::file_list::cache_stats().await });

    #[cfg(feature = "enterprise")]
    db::ofga::cache().await.expect("ofga model cache failed");

    #[cfg(feature = "enterprise")]
    if !LOCAL_NODE.is_compactor() {
        db::session::cache()
            .await
            .expect("user session cache failed");
    }

    // check wal directory
    if LOCAL_NODE.is_ingester() {
        // create wal dir
        if let Err(e) = std::fs::create_dir_all(&cfg.common.data_wal_dir) {
            log::error!("Failed to create wal dir: {}", e);
        }
    }

    tokio::task::spawn(async move { files::run().await });
    tokio::task::spawn(async move { stats::run().await });
    tokio::task::spawn(async move { compactor::run().await });
    tokio::task::spawn(async move { flatten_compactor::run().await });
    tokio::task::spawn(async move { metrics::run().await });
    tokio::task::spawn(async move { promql::run().await });
    tokio::task::spawn(async move { alert_manager::run().await });

    #[cfg(feature = "enterprise")]
    o2_enterprise::enterprise::openfga::authorizer::authz::init_open_fga().await;

    #[cfg(feature = "enterprise")]
    tokio::task::spawn(async move { cipher::run().await });
    #[cfg(feature = "enterprise")]
    tokio::task::spawn(async move { db::keys::watch().await });

    // RBAC model
    #[cfg(feature = "enterprise")]
    if get_o2_config().openfga.enabled {
        if let Err(e) = crate::common::infra::ofga::init().await {
            log::error!("OFGA init failed: {}", e);
        }
    }

    // Shouldn't serve request until initialization finishes
    log::info!("Job initialization complete");

    // Syslog server start
    tokio::task::spawn(async move { db::syslog::watch().await });
    tokio::task::spawn(async move { db::syslog::watch_syslog_settings().await });

    let start_syslog = *SYSLOG_ENABLED.read();
    if start_syslog {
        syslog_server::run(start_syslog, true)
            .await
            .expect("syslog server run failed");
    }

    Ok(())
}
