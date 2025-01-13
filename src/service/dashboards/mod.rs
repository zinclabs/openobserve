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

use config::{
    ider,
    meta::{
        dashboards::{Dashboard, ListDashboardsParams},
        folder::{Folder, DEFAULT_FOLDER},
        stream::{DistinctField, StreamType},
    },
};
use hashbrown::HashMap;
use infra::table::{
    self,
    distinct_values::{DistinctFieldRecord, OriginType},
    folders::FolderType,
};

use super::{db::distinct_values, folders, stream::save_stream_settings};
use crate::common::{
    meta::authz::Authz,
    utils::auth::{remove_ownership, set_ownership},
};
pub mod reports;
pub mod timed_annotations;

#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::{
    common::infra::config::get_config as get_o2_config,
    openfga::authorizer::authz::{get_ofga_type, remove_parent_relation, set_parent_relation},
};

/// An error that occurs interacting with dashboards.
#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    /// An error that occurs while interacting with the database through the
    /// [infra] crate.
    #[error("InfraError# {0}")]
    InfraError(#[from] infra::errors::Error),

    /// Error that occurs when trying to access a dashboard that cannot be
    /// found.
    #[error("dashboard not found")]
    DashboardNotFound,

    /// Error that occurs when trying to update a dashboard using a missing hash
    /// value or a hash value that cannot be parsed as an [i64].
    #[error("tried to update dashboard using hash that cannot be parsed")]
    UpdateMissingHash,

    /// Error that occurs when trying to update a dashboard using a stale hash.
    /// This occurs when the two clients attempt to update the dashboard
    /// concurrently.
    #[error("tried to update dashboard using conflicting hash")]
    UpdateConflictingHash,

    /// Error that occurs when trying to create an update a dashboard but not
    /// title is provided.
    #[error("dashboard cannot have empty title")]
    PutMissingTitle,

    /// Error that occurs when trying to move a dashboard but either the source
    /// or destination folder is not specified.
    #[error("missing source or destination folder for dashboard move")]
    MoveMissingFolderParam,

    /// Error that occurs when trying to move a dashboard but either the source
    /// or destination folder is not specified.
    #[error("error deleting the dashboard {0} from old folder {1} : {2}")]
    MoveDashboardDeleteOld(String, String, String),

    /// Error that occurs when trying to move a dashboard to a destination
    /// folder that cannot be found.
    #[error("error moving dashboard to folder that cannot be found")]
    MoveDestinationFolderNotFound,

    /// Error that occurs when trying to create a dashboard in a folder that
    /// cannot be found.
    #[error("error creating dashboard in folder that cannot be found")]
    CreateFolderNotFound,

    /// Error that occurs when trying to create a destination folder for a new
    /// dashboard.
    #[error("error creating default folder")]
    CreateDefaultFolder,

    /// Error that occurs when updating the distinct values using the
    /// dashboard variables
    #[error("error in updating distinct values")]
    DistinctValueError,
}

async fn add_distinct_field_entry(
    dashboard_id: &str,
    org: &str,
    stream: &str,
    stype: String,
    field: &str,
) -> Result<(), anyhow::Error> {
    log::info!(
        "adding distinct field {stype} {org}/{stream} : {field} for dashboard {dashboard_id}"
    );
    let record = DistinctFieldRecord::new(
        OriginType::Dashboard,
        dashboard_id,
        org,
        stream,
        stype,
        field,
    );
    distinct_values::add(record)
        .await
        .map_err(|e| anyhow::anyhow!("error in adding distinct value record : {e}"))
}

async fn remove_distinct_field_entry(
    dashboard_id: &str,
    org: &str,
    stream: &str,
    stype: String,
    field: &str,
) -> Result<(), anyhow::Error> {
    log::info!(
        "removing distinct field {stype} {org}/{stream} : {field} for dashboard {dashboard_id}"
    );
    let record = DistinctFieldRecord::new(
        OriginType::Dashboard,
        dashboard_id,
        org,
        stream,
        stype,
        field,
    );
    distinct_values::remove(record)
        .await
        .map_err(|e| anyhow::anyhow!("error in removing distinct value record : {e}"))
}

async fn update_distinct_variables(
    org_id: &str,
    old_dash: Option<Dashboard>,
    new_dash: &Dashboard,
) -> Result<(), anyhow::Error> {
    let mut old_variables = get_query_variables(old_dash.as_ref());
    let new_variables = get_query_variables(Some(new_dash));

    let dashboard_id = new_dash.dashboard_id().unwrap();

    if !new_variables.is_empty() {
        for ((name, typ), fields) in new_variables.into_iter() {
            let mut stream_settings = infra::schema::get_settings(org_id, &name, typ)
                .await
                .unwrap_or_default();
            // we only store distinct values for logs and traces -
            // if anything else, we can ignore.
            if !matches!(typ, StreamType::Logs | StreamType::Traces) {
                continue;
            }
            // get entry from previous variables corresponding to this stream
            let old_fields = old_variables
                .remove(&(name.to_owned(), typ))
                .unwrap_or_default();
            let mut _new_added = false;

            for f in fields.iter() {
                // we ignore full text search no matter what
                if stream_settings.full_text_search_keys.contains(f) {
                    continue;
                }
                // we add entry for all the fields, because we need mappings for each individual
                // origin-stream-field mapping. The duplicates are handled in add function, so
                // we can call it for each field without issues
                add_distinct_field_entry(dashboard_id, org_id, &name, typ.to_string(), f).await?;
                let _temp = DistinctField {
                    name: f.to_owned(),
                    added_ts: chrono::Utc::now().timestamp_micros(),
                };
                if !stream_settings.distinct_value_fields.contains(&_temp) {
                    stream_settings.distinct_value_fields.push(_temp);
                    _new_added = true;
                }
            }
            // here we check if any of the fields used in previous version are no longer used
            // if so, remove their entry.
            for f in old_fields {
                if !fields.contains(&f) {
                    remove_distinct_field_entry(dashboard_id, org_id, &name, typ.to_string(), &f)
                        .await?;
                }
            }
            if _new_added {
                save_stream_settings(org_id, &name, typ, stream_settings).await?;
            }
        }

        // finally, whatever stream remains in the old variables
        // it has all corresponding fields removed, so remove those as well
        for ((name, typ), fields) in old_variables.into_iter() {
            for f in fields {
                remove_distinct_field_entry(dashboard_id, org_id, &name, typ.to_string(), &f)
                    .await?;
            }
        }
    } else {
        // I guess all the variables were removed from the dashboard.
        // we can batch remove all entries belonging to this dashboard.
        distinct_values::batch_remove(OriginType::Dashboard, dashboard_id).await?
    }
    Ok(())
}

macro_rules! _get_variables {
    ($map:ident, $dash:ident) => {
        if let Some(vars) = &$dash.variables {
            for v in vars.list.iter() {
                if let Some(ref qd) = v.query_data {
                    $map.entry((qd.stream.clone(), qd.stream_type))
                        .or_default()
                        .push(qd.field.clone());
                }
            }
        }
    };
}

pub fn get_query_variables(
    dashboard: Option<&Dashboard>,
) -> HashMap<(String, StreamType), Vec<String>> {
    let mut map: HashMap<(String, StreamType), Vec<String>> = HashMap::new();
    let dashboard = if let Some(d) = dashboard {
        d
    } else {
        return map;
    };
    match dashboard.version {
        1 => {
            let dash = dashboard.v1.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        2 => {
            let dash = dashboard.v2.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        3 => {
            let dash = dashboard.v3.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        4 => {
            let dash = dashboard.v4.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        5 => {
            let dash = dashboard.v5.as_ref().unwrap();
            _get_variables!(map, dash);
        }
        _ => {
            unreachable!("we only have 5 dashboard versions")
        }
    }
    map
}

#[tracing::instrument(skip(dashboard))]
pub async fn create_dashboard(
    org_id: &str,
    folder_id: &str,
    dashboard: Dashboard,
) -> Result<Dashboard, DashboardError> {
    // NOTE: Overwrite whatever `dashboard_id` the client has sent us
    // If folder is default folder & doesn't exist then create it

    let dashboard = if table::folders::exists(org_id, folder_id, FolderType::Dashboards).await? {
        let dashboard_id = ider::generate();
        let saved = put(org_id, &dashboard_id, folder_id, dashboard, None).await?;
        set_ownership(
            org_id,
            "dashboards",
            Authz {
                obj_id: dashboard_id,
                parent_type: "folders".to_owned(),
                parent: folder_id.to_owned(),
            },
        )
        .await;
        Ok(saved)
    } else if folder_id == DEFAULT_FOLDER {
        let folder = Folder {
            folder_id: DEFAULT_FOLDER.to_string(),
            name: DEFAULT_FOLDER.to_string(),
            description: DEFAULT_FOLDER.to_string(),
        };
        folders::save_folder(org_id, folder, FolderType::Dashboards, true)
            .await
            .map_err(|_| DashboardError::CreateDefaultFolder)?;
        let dashboard_id = ider::generate();
        let saved = put(org_id, &dashboard_id, folder_id, dashboard, None).await?;
        set_ownership(
            org_id,
            "dashboards",
            Authz {
                obj_id: dashboard_id,
                parent_type: "folders".to_owned(),
                parent: folder_id.to_owned(),
            },
        )
        .await;
        Ok(saved)
    } else {
        Err(DashboardError::CreateFolderNotFound)
    }?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        let _ = o2_enterprise::enterprise::super_cluster::queue::dashboards_put(
            org_id,
            folder_id,
            dashboard.clone(),
        )
        .await;
    }

    Ok(dashboard)
}

#[tracing::instrument(skip(dashboard))]
pub async fn update_dashboard(
    org_id: &str,
    dashboard_id: &str,
    folder_id: &str,
    dashboard: Dashboard,
    hash: Option<&str>,
) -> Result<Dashboard, DashboardError> {
    let dashboard = put(org_id, dashboard_id, folder_id, dashboard, hash).await?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        let _ = o2_enterprise::enterprise::super_cluster::queue::dashboards_put(
            org_id,
            folder_id,
            dashboard.clone(),
        )
        .await;
    }

    Ok(dashboard)
}

#[tracing::instrument]
pub async fn list_dashboards(
    params: ListDashboardsParams,
) -> Result<Vec<(Folder, Dashboard)>, DashboardError> {
    let dashboards = table::dashboards::list(params).await?;
    Ok(dashboards)
}

#[tracing::instrument]
pub async fn get_dashboard(org_id: &str, dashboard_id: &str) -> Result<Dashboard, DashboardError> {
    table::dashboards::get_by_id(org_id, dashboard_id)
        .await?
        .ok_or(DashboardError::DashboardNotFound)
        .map(|(_f, d)| d)
}

#[tracing::instrument]
pub async fn delete_dashboard(org_id: &str, dashboard_id: &str) -> Result<(), DashboardError> {
    let Some((folder, _dashboard)) = table::dashboards::get_by_id(org_id, dashboard_id).await?
    else {
        return Err(DashboardError::DashboardNotFound);
    };
    table::dashboards::delete_from_folder(org_id, &folder.folder_id, dashboard_id).await?;
    distinct_values::batch_remove(OriginType::Dashboard, dashboard_id).await?;
    remove_ownership(
        org_id,
        "dashboards",
        Authz {
            obj_id: dashboard_id.to_owned(),
            parent_type: "folders".to_owned(),
            parent: folder.folder_id.clone(),
        },
    )
    .await;

    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        let _ = o2_enterprise::enterprise::super_cluster::queue::dashboards_delete(
            org_id,
            &folder.folder_id,
            dashboard_id,
        )
        .await;
    }

    Ok(())
}

#[tracing::instrument]
pub async fn move_dashboard(
    org_id: &str,
    dashboard_id: &str,
    from_folder: &str,
    to_folder: &str,
) -> Result<(), DashboardError> {
    if from_folder.is_empty() || to_folder.is_empty() {
        return Err(DashboardError::MoveMissingFolderParam);
    };

    let Some(dashboard) =
        table::dashboards::get_from_folder(org_id, from_folder, dashboard_id).await?
    else {
        return Err(DashboardError::DashboardNotFound);
    };

    // make sure the destination folder exists
    if !table::folders::exists(org_id, to_folder, FolderType::Dashboards).await? {
        return Err(DashboardError::MoveDestinationFolderNotFound);
    };

    // add the dashboard to the destination folder
    put(org_id, dashboard_id, to_folder, dashboard, None).await?;
    // OFGA ownership
    #[cfg(feature = "enterprise")]
    if get_o2_config().openfga.enabled {
        set_parent_relation(
            dashboard_id,
            &get_ofga_type("dashboards"),
            to_folder,
            &get_ofga_type("folders"),
        )
        .await;
    }

    // delete the dashboard from the source folder
    table::dashboards::delete_from_folder(org_id, from_folder, dashboard_id)
        .await
        .map_err(|e| {
            DashboardError::MoveDashboardDeleteOld(
                dashboard_id.to_string(),
                from_folder.to_string(),
                e.to_string(),
            )
        })?;

    #[cfg(feature = "enterprise")]
    if get_o2_config().openfga.enabled {
        remove_parent_relation(
            dashboard_id,
            &get_ofga_type("dashboards"),
            from_folder,
            &get_ofga_type("folders"),
        )
        .await;
    }
    Ok(())
}

#[tracing::instrument(skip(dashboard))]
async fn put(
    org_id: &str,
    dashboard_id: &str,
    folder_id: &str,
    mut dashboard: Dashboard,
    hash: Option<&str>,
) -> Result<Dashboard, DashboardError> {
    let old_version = table::dashboards::get_from_folder(org_id, folder_id, dashboard_id).await?;
    if let Some(existing_dash) = &old_version {
        let existing_dash_hash = &existing_dash.hash;

        let Some(Ok(hash_val)) = hash.map(|hash_str| hash_str.parse::<u64>()) else {
            return Err(DashboardError::UpdateMissingHash);
        };
        if hash_val.to_string() != *existing_dash_hash {
            return Err(DashboardError::UpdateConflictingHash);
        }
    };

    match update_distinct_variables(org_id, old_version, &dashboard).await {
        Ok(_) => {}
        Err(e) => {
            log::error!("error in updating distinct values while updating dashboard : {e}");
            return Err(DashboardError::DistinctValueError);
        }
    }

    let title = dashboard
        .title()
        .map(|t| t.trim().to_string())
        .and_then(|t| if t.is_empty() { None } else { Some(t) })
        .ok_or_else(|| DashboardError::PutMissingTitle)?;
    dashboard.set_title(title);

    dashboard.set_dashboard_id(dashboard_id.to_owned());
    let dash = table::dashboards::put(org_id, folder_id, dashboard).await?;
    Ok(dash)
}

/// Internal helper function find dashboard and its folder by id.
///
/// Used by self_reporting to enrich dashboard SearchEventContext
pub(crate) async fn get_folder_and_dashboard(
    org_id: &str,
    dashboard_id: &str,
) -> Result<(Folder, Dashboard), DashboardError> {
    table::dashboards::get_by_id(org_id, dashboard_id)
        .await?
        .ok_or(DashboardError::DashboardNotFound)
}
