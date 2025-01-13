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

use std::cmp::Ordering;

use hashbrown::HashSet;
use infra::dist_lock;
use o2_enterprise::enterprise::{
    common::infra::config::get_config as get_o2_config,
    openfga::{
        authorizer::authz::{
            add_tuple_for_pipeline, get_index_creation_tuples, get_org_creation_tuples,
            get_ownership_all_org_tuple, get_user_role_tuple, update_tuples,
        },
        meta::mapping::{NON_OWNING_ORG, OFGA_MODELS},
    },
    super_cluster::kv::ofga::{get_model, set_model},
};

use crate::{
    common::{
        infra::config::USERS,
        meta::{organization::DEFAULT_ORG, user::UserRole},
    },
    service::db,
};

pub async fn init() -> Result<(), anyhow::Error> {
    use o2_enterprise::enterprise::openfga::{
        authorizer::authz::get_tuple_for_new_index, get_all_init_tuples,
    };

    log::info!("[OFGA] Initializing OFGA model");

    let mut init_tuples = vec![];
    let mut migrate_native_objects = false;
    let mut need_migrate_index_streams = false;
    let mut need_pipeline_migration = false;
    let mut existing_meta = match db::ofga::get_ofga_model().await {
        Ok(Some(model)) => Some(model),
        Ok(None) | Err(_) => {
            migrate_native_objects = true;
            None
        }
    };

    // sync with super cluster
    if get_o2_config().super_cluster.enabled {
        let meta_in_super = get_model().await?;
        log::info!(
            "[OFGA] meta_in_super: {:?}, existing_model: {:?}",
            &meta_in_super,
            &existing_meta,
        );
        match (meta_in_super, &existing_meta) {
            (None, Some(existing_model)) => {
                // set to super cluster
                log::info!("[OFGA] meta_in_super is None, existing_model is Some: setting existing model to super cluster");
                set_model(Some(existing_model.clone())).await?;
            }
            (Some(model), None) => {
                // set to local
                log::info!(
                    "[OFGA] meta_in_super is Some, existing_model is None: setting it to local",
                );
                existing_meta = Some(model.clone());
                migrate_native_objects = false;
                db::ofga::set_ofga_model_to_db(model).await?;
            }
            (Some(model), Some(existing_model)) => {
                log::info!(
                    "[OFGA] meta_in_super is Some, existing_model is Some: comparing versions",
                );
                log::info!(
                    "[OFGA] meta_in_super version: {}, existing_model version: {}",
                    model.version,
                    existing_model.version,
                );
                match model.version.cmp(&existing_model.version) {
                    Ordering::Less => {
                        // update version in super cluster
                        log::info!("[OFGA] setting existing_model to super cluster");
                        set_model(Some(existing_model.clone())).await?;
                    }
                    Ordering::Greater => {
                        // update version in local
                        log::info!("[OFGA] setting meta_in_super to local");
                        existing_meta = Some(model.clone());
                        migrate_native_objects = false;
                        db::ofga::set_ofga_model_to_db(model).await?;
                    }
                    Ordering::Equal => {}
                }
            }
            _ => {}
        }
    }

    let meta = o2_enterprise::enterprise::openfga::model::read_ofga_model().await;
    get_all_init_tuples(&mut init_tuples).await;
    if let Some(existing_model) = &existing_meta {
        if meta.version == existing_model.version {
            log::info!("[OFGA] model already exists & no changes required");
            if !init_tuples.is_empty() {
                match update_tuples(init_tuples, vec![]).await {
                    Ok(_) => {
                        log::info!("[OFGA] Data migrated to openfga");
                    }
                    Err(e) => {
                        log::error!(
                            "Error writing init ofga tuples to the openfga during migration: {}",
                            e
                        );
                    }
                }
            }
            return Ok(());
        }
        // Check if ofga migration of index streams are needed
        let meta_version = version_compare::Version::from(&meta.version).unwrap();
        let existing_model_version =
            version_compare::Version::from(&existing_model.version).unwrap();
        let v0_0_4 = version_compare::Version::from("0.0.4").unwrap();
        let v0_0_5 = version_compare::Version::from("0.0.5").unwrap();
        let v0_0_6 = version_compare::Version::from("0.0.6").unwrap();
        if meta_version > v0_0_4 && existing_model_version < v0_0_5 {
            need_migrate_index_streams = true;
        }
        if meta_version > v0_0_5 && existing_model_version < v0_0_6 {
            need_pipeline_migration = true;
        }
    }

    // 1. create a cluster lock
    let locker = dist_lock::lock("/ofga/model/", 0)
        .await
        .expect("Failed to acquire lock for openFGA");
    match db::ofga::set_ofga_model(existing_meta).await {
        Ok(store_id) => {
            log::info!("[OFGA] set store_id: {}", store_id);
            if store_id.is_empty() {
                log::error!("OFGA store id is empty");
            }
            o2_enterprise::enterprise::common::infra::config::OFGA_STORE_ID
                .insert("store_id".to_owned(), store_id);

            let mut tuples = vec![];
            let r = infra::schema::STREAM_SCHEMAS.read().await;
            let mut orgs = HashSet::new();
            for key in r.keys() {
                if !key.contains('/') {
                    continue;
                }
                let split_key = key.split('/').collect::<Vec<&str>>();
                let org_name = split_key[0];
                orgs.insert(org_name);
                if need_migrate_index_streams
                    && split_key.len() > 2
                    && split_key[1] == "index"
                    && !migrate_native_objects
                {
                    get_tuple_for_new_index(org_name, split_key[2], &mut tuples);
                }
            }
            if migrate_native_objects {
                for org_name in orgs {
                    get_org_creation_tuples(
                        org_name,
                        &mut tuples,
                        OFGA_MODELS
                            .iter()
                            .map(|(_, fga_entity)| fga_entity.key)
                            .collect(),
                        NON_OWNING_ORG.to_vec(),
                    )
                    .await;
                }
                // No Data Ingested hence STREAM_SCHEMAS is empty , so we need to create at
                // least default org
                if tuples.is_empty() {
                    get_org_creation_tuples(
                        DEFAULT_ORG,
                        &mut tuples,
                        OFGA_MODELS
                            .iter()
                            .map(|(_, fga_entity)| fga_entity.key)
                            .collect(),
                        NON_OWNING_ORG.to_vec(),
                    )
                    .await;
                }

                for user_key_val in USERS.iter() {
                    let user = user_key_val.value();
                    if user.is_external {
                        continue;
                    } else {
                        let role = if user.role.eq(&UserRole::Root) {
                            UserRole::Admin.to_string()
                        } else {
                            user.role.to_string()
                        };

                        get_user_role_tuple(&role, &user.email, &user.org, &mut tuples);
                    }
                }
            } else {
                for org_name in orgs {
                    if need_migrate_index_streams {
                        get_index_creation_tuples(org_name, &mut tuples).await;
                    }
                    if need_pipeline_migration {
                        get_ownership_all_org_tuple(org_name, "pipelines", &mut tuples);
                        match db::pipeline::list_by_org(org_name).await {
                            Ok(pipelines) => {
                                for pipeline in pipelines {
                                    add_tuple_for_pipeline(org_name, &pipeline.id, &mut tuples);
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to migrate RBAC for org {} pipelines: {}",
                                    org_name,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            // Check if there are init ofga tuples that needs to be added now
            for tuple in init_tuples {
                tuples.push(tuple);
            }

            if tuples.is_empty() {
                log::info!("No orgs to update to the openfga");
            } else {
                log::debug!("tuples not empty: {:?}", tuples);
                match update_tuples(tuples, vec![]).await {
                    Ok(_) => {
                        log::info!("Data migrated to openfga");
                    }
                    Err(e) => {
                        log::error!(
                            "Error updating orgs & users to the openfga during migration: {}",
                            e
                        );
                    }
                }
            }
            drop(r);
        }
        Err(e) => {
            log::error!("Error setting OFGA model: {:?}", e);
        }
    }
    // release lock
    dist_lock::unlock(&locker)
        .await
        .expect("Failed to release lock");

    Ok(())
}
