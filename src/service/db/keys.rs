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

use infra::{
    cluster_coordinator::get_coordinator,
    db::Event,
    errors,
    table::cipher::{CipherEntry, EntryKind},
};
use o2_enterprise::enterprise::cipher::CipherData;

use crate::{cipher::registry::REGISTRY, service::db};

// DBKey to set cipher keys
pub const CIPHER_KEY_PREFIX: &str = "/cipher_keys/";

pub async fn add(entry: CipherEntry) -> Result<(), errors::Error> {
    infra::table::cipher::add(entry.clone()).await?;
    // specifically for cipher keys, we need to notify of this addition
    if entry.kind == EntryKind::CipherKey {
        // trigger watch event by putting value to cluster coordinator
        let cluster_coordinator = get_coordinator().await;
        cluster_coordinator
            .put(
                &format!("{CIPHER_KEY_PREFIX}{}/{}", entry.org, entry.name),
                bytes::Bytes::new(), // no actual data, the receiver can query the db
                true,
                None,
            )
            .await?;

        #[cfg(feature = "enterprise")]
        {
            let config = o2_enterprise::enterprise::common::infra::config::get_config();
            if config.super_cluster.enabled {
                match o2_enterprise::enterprise::super_cluster::queue::keys_put(entry.clone()).await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!(
                "error in sending cipher key add notification to super cluster queue for {}/{} : {e}", entry.org,entry.name
            );
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn update(entry: CipherEntry) -> Result<(), errors::Error> {
    infra::table::cipher::update(entry.clone()).await?;

    // specifically for cipher keys, we need to notify of this addition
    if entry.kind == EntryKind::CipherKey {
        // trigger watch event by putting value to cluster coordinator
        let cluster_coordinator = get_coordinator().await;
        cluster_coordinator
            .put(
                &format!("{CIPHER_KEY_PREFIX}{}/{}", entry.org, entry.name),
                bytes::Bytes::new(), // no actual data, the receiver can query the db
                true,
                None,
            )
            .await?;
        #[cfg(feature = "enterprise")]
        {
            let config = o2_enterprise::enterprise::common::infra::config::get_config();
            if config.super_cluster.enabled {
                match o2_enterprise::enterprise::super_cluster::queue::keys_update(entry.clone())
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!(
                "error in sending cipher key update notification to super cluster queue for {}/{} : {e}",entry.org,entry.name
            );
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn remove(org: &str, kind: EntryKind, name: &str) -> Result<(), errors::Error> {
    infra::table::cipher::remove(org, kind, name).await?;
    // specifically for cipher keys, we need to notify of this deletion
    if kind == EntryKind::CipherKey {
        // trigger watch event by putting value to cluster coordinator
        let cluster_coordinator = get_coordinator().await;
        cluster_coordinator
            .delete(
                &format!("{CIPHER_KEY_PREFIX}{}/{}", org, name),
                false,
                true,
                None,
            )
            .await?;

        #[cfg(feature = "enterprise")]
        {
            let config = o2_enterprise::enterprise::common::infra::config::get_config();
            if config.super_cluster.enabled {
                match o2_enterprise::enterprise::super_cluster::queue::keys_delete(org, name).await
                {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!(
                "error in sending cipher key delete notification to super cluster queue for {org}/{name} : {e}"
            );
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let prefix = CIPHER_KEY_PREFIX;
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(prefix).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching cipher keys");

    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_cipher_keys: event channel closed");
                return Ok(());
            }
        };

        match ev {
            Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(prefix).unwrap();
                let (org, name) = item_key.split_once("/").unwrap();
                let item = match infra::table::cipher::get_data(
                    org,
                    infra::table::cipher::EntryKind::CipherKey,
                    name,
                )
                .await
                {
                    Ok(Some(val)) => val,
                    Ok(None) => {
                        log::error!("unexpected missing cipher key");
                        continue;
                    }
                    Err(e) => {
                        log::error!("Error getting value: {}", e);
                        continue;
                    }
                };
                let cd: CipherData = serde_json::from_str(&item).unwrap();
                let kname = format!("{org}:{name}");
                let key = cd.get_key().await.unwrap();
                {
                    let mut lock = REGISTRY.write();
                    lock.add_key(kname, Box::new(key));
                    drop(lock);
                }
            }
            Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(prefix).unwrap();
                let (org, name) = item_key.split_once("/").unwrap();
                let kname = format!("{org}:{name}");
                {
                    let mut lock = REGISTRY.write();
                    lock.remove_key(&kname);
                    drop(lock);
                }
            }
            Event::Empty => {}
        }
    }
}
