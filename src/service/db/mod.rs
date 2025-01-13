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

use bytes::Bytes;
use hashbrown::HashMap;
use infra::{db as infra_db, errors::Result};
#[cfg(feature = "enterprise")]
use {
    infra::errors::Error,
    o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config,
};

pub mod alerts;
pub mod compact;
pub mod dashboards;
pub mod distinct_values;
pub mod enrichment_table;
pub mod file_list;
pub mod functions;
pub mod instance;
#[cfg(feature = "enterprise")]
pub mod keys;
pub mod kv;
pub mod metrics;
#[cfg(feature = "enterprise")]
pub mod ofga;
pub mod organization;
pub mod pipeline;
pub mod saved_view;
pub mod scheduler;
pub mod schema;
pub mod search_job;
pub mod session;
pub mod short_url;
pub mod syslog;
pub mod user;
pub mod version;

pub(crate) use infra_db::{get_coordinator, Event, NEED_WATCH, NO_NEED_WATCH};

#[inline]
pub(crate) async fn get(key: &str) -> Result<Bytes> {
    let db = infra_db::get_db().await;
    db.get(key).await
}

#[inline]
pub(crate) async fn put(
    key: &str,
    value: Bytes,
    need_watch: bool,
    start_dt: Option<i64>,
) -> Result<()> {
    let db = infra_db::get_db().await;
    db.put(key, value.clone(), need_watch, start_dt).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        o2_enterprise::enterprise::super_cluster::queue::put(key, value, need_watch, start_dt)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(())
}

#[inline]
pub(crate) async fn delete(
    key: &str,
    with_prefix: bool,
    need_watch: bool,
    start_dt: Option<i64>,
) -> Result<()> {
    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        o2_enterprise::enterprise::super_cluster::queue::delete(
            key,
            with_prefix,
            need_watch,
            start_dt,
        )
        .await
        .map_err(|e| Error::Message(e.to_string()))?;
    }

    let db = infra_db::get_db().await;
    db.delete(key, with_prefix, need_watch, start_dt).await?;
    Ok(())
}

#[inline]
pub(crate) async fn delete_if_exists(key: &str, with_prefix: bool, need_watch: bool) -> Result<()> {
    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        o2_enterprise::enterprise::super_cluster::queue::delete(key, with_prefix, need_watch, None)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
    }

    let db = infra_db::get_db().await;
    db.delete_if_exists(key, with_prefix, need_watch).await
}

#[inline]
pub(crate) async fn list(prefix: &str) -> Result<HashMap<String, Bytes>> {
    let db = infra_db::get_db().await;
    db.list(prefix).await
}

#[inline]
pub(crate) async fn list_keys(prefix: &str) -> Result<Vec<String>> {
    let db = infra_db::get_db().await;
    db.list_keys(prefix).await
}

#[inline]
pub(crate) async fn list_values(prefix: &str) -> Result<Vec<Bytes>> {
    let db = infra_db::get_db().await;
    db.list_values(prefix).await
}

#[inline]
pub(crate) async fn list_values_by_start_dt(
    prefix: &str,
    start_dt: Option<(i64, i64)>,
) -> Result<Vec<(i64, Bytes)>> {
    let db = infra_db::get_db().await;
    db.list_values_by_start_dt(prefix, start_dt).await
}
