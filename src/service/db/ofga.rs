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

use config::utils::json;
use o2_enterprise::enterprise::{
    common::infra::config::*,
    openfga::{
        meta::mapping::OFGAModel,
        model::{create_open_fga_store, read_ofga_model, write_auth_models},
    },
};

use crate::service::db;

pub async fn set_ofga_model(existing_meta: Option<OFGAModel>) -> Result<String, anyhow::Error> {
    let meta = read_ofga_model().await;
    if let Some(existing_model) = existing_meta {
        if meta.version == existing_model.version {
            log::info!("OFGA model already exists & no changes required");
            Ok(existing_model.store_id)
        } else {
            let store_id = if existing_model.store_id.is_empty() {
                log::info!("[OFGA DB] Creating new store_id, as existing store_id is empty");
                create_open_fga_store().await.unwrap()
            } else {
                existing_model.store_id
            };
            match write_auth_models(&meta, &store_id).await {
                Ok(_) => {
                    let mut loc_meta = meta.clone();
                    loc_meta.store_id = store_id;
                    loc_meta.model = None;
                    set_ofga_model_to_db(loc_meta).await
                }
                Err(e) => Err(anyhow::anyhow!(e)),
            }
        }
    } else {
        log::info!("[OFGA DB] Creating new store_id, as existing store_id is empty");
        let store_id = create_open_fga_store().await.unwrap();
        match write_auth_models(&meta, &store_id).await {
            Ok(_) => {
                let mut loc_meta = meta.clone();
                loc_meta.store_id = store_id;
                loc_meta.model = None;
                set_ofga_model_to_db(loc_meta).await
            }
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
}

pub async fn set_ofga_model_to_db(mut meta: OFGAModel) -> Result<String, anyhow::Error> {
    let key = "/ofga/model";
    meta.model = None;
    match db::put(
        key,
        json::to_vec(&meta).unwrap().into(),
        db::NEED_WATCH,
        None,
    )
    .await
    {
        Ok(_) => Ok(meta.store_id),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

pub async fn get_ofga_model() -> Result<Option<OFGAModel>, anyhow::Error> {
    let key = "/ofga/model";
    let ret = db::get(key).await?;
    let loc_value = json::from_slice(&ret).unwrap();
    let value = Some(loc_value);
    Ok(value)
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = "/ofga/model";
    let cluster_coordinator = db::get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching /ofga/model");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_ofga_model: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_value: OFGAModel = if config::get_config().common.meta_store_external {
                    match db::get(&ev.key).await {
                        Ok(val) => match json::from_slice(&val) {
                            Ok(val) => val,
                            Err(e) => {
                                log::error!("Error getting value: {}", e);
                                continue;
                            }
                        },
                        Err(e) => {
                            log::error!("Error getting value: {}", e);
                            continue;
                        }
                    }
                } else {
                    json::from_slice(&ev.value.unwrap()).unwrap()
                };
                log::info!("[WATCH] Got store id {}", &item_value.store_id);
                OFGA_STORE_ID.insert("store_id".to_owned(), item_value.store_id);
            }
            db::Event::Delete(_) => {
                OFGA_STORE_ID.remove("store_id");
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let key = "/ofga/model";
    let ret = db::list(key).await?;
    for (_, item_value) in ret {
        let json_val: OFGAModel = json::from_slice(&item_value).unwrap();
        log::info!("Caching store id {}", &json_val.store_id);
        OFGA_STORE_ID.insert("store_id".to_owned(), json_val.store_id);
    }
    log::info!("/ofga/model Cached ");
    Ok(())
}
