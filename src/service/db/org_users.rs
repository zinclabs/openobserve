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

use bytes::Bytes;
use config::meta::user::{User, UserRole};
#[cfg(feature = "enterprise")]
use infra::table::org_users::OrgUserPut;
use infra::{
    db::{self, delete_from_db_coordinator, get_coordinator, put_into_db_coordinator},
    table::org_users::{self, OrgUserExpandedRecord, OrgUserRecord, UserOrgExpandedRecord},
};

use crate::common::infra::config::{ORG_USERS, ROOT_USER, USERS, USERS_RUM_TOKEN};

pub const ORG_USERS_KEY_PREFIX: &str = "/org_users/";

pub async fn add(
    org_id: &str,
    user_email: &str,
    role: UserRole,
    token: &str,
    rum_token: Option<String>,
) -> Result<(), anyhow::Error> {
    let user_email = user_email.to_lowercase();
    let key = format!("{ORG_USERS_KEY_PREFIX}single/{}/{}", org_id, user_email);
    org_users::add(org_id, &user_email, role.clone(), token, rum_token.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to add user to org: {}", e))?;

    log::debug!("Put into db_coordinator: {user_email}");
    let _ = put_into_db_coordinator(&key, Bytes::new(), true, None).await;

    #[cfg(feature = "enterprise")]
    super_cluster::org_user_add(
        &key,
        &OrgUserPut {
            org_id: org_id.to_string(),
            email: user_email.to_string(),
            role,
            token: token.to_string(),
            rum_token,
        },
    )
    .await?;
    Ok(())
}

pub async fn update(
    org_id: &str,
    user_email: &str,
    role: UserRole,
    token: &str,
    rum_token: Option<String>,
) -> Result<(), anyhow::Error> {
    let user_email = user_email.to_lowercase();
    let key = format!("{ORG_USERS_KEY_PREFIX}single/{}/{}", org_id, user_email);
    org_users::update(org_id, &user_email, role.clone(), token, rum_token.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update user role: {}", e))?;

    let _ = put_into_db_coordinator(&key, Bytes::new(), true, None).await;

    #[cfg(feature = "enterprise")]
    super_cluster::org_user_update(
        &key,
        &OrgUserPut {
            org_id: org_id.to_string(),
            email: user_email.to_string(),
            role,
            token: token.to_string(),
            rum_token,
        },
    )
    .await?;
    Ok(())
}

pub async fn remove(org_id: &str, user_email: &str) -> Result<(), anyhow::Error> {
    let user_email = user_email.to_lowercase();
    let key = format!("{ORG_USERS_KEY_PREFIX}single/{}/{}", org_id, user_email);
    org_users::remove(org_id, &user_email)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to remove user from org: {}", e))?;
    let _ = delete_from_db_coordinator(&key, false, true, None).await;

    #[cfg(feature = "enterprise")]
    super_cluster::org_user_remove(&key).await?;
    Ok(())
}

pub async fn remove_by_user(email: &str) -> Result<(), anyhow::Error> {
    let email = email.to_lowercase();
    let key = format!("{ORG_USERS_KEY_PREFIX}many/user/{}", email);
    org_users::remove_by_user(&email)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to remove user from org: {}", e))?;
    let _ = delete_from_db_coordinator(&key, false, true, None).await;

    #[cfg(feature = "enterprise")]
    super_cluster::org_user_remove(&key).await?;
    Ok(())
}

pub fn get_cached_user_org(org_id: &str, user_email: &str) -> Option<User> {
    let user_email = user_email.to_lowercase();
    match ORG_USERS.get(&format!("{}/{}", org_id, user_email)) {
        Some(org_user) => match USERS.get(&user_email) {
            Some(user) => Some(User {
                email: user.email.clone(),
                password: user.password.clone(),
                role: org_user.role.clone(),
                salt: user.salt.clone(),
                first_name: user.first_name.clone(),
                last_name: user.last_name.clone(),
                password_ext: user.password_ext.clone(),
                token: org_user.token.clone(),
                rum_token: org_user.rum_token.clone(),
                org: org_user.org_id.clone(),
                is_external: user.user_type.is_external(),
            }),
            None => None,
        },
        None => None,
    }
}

pub async fn get_from_db(org_id: &str, user_email: &str) -> Result<OrgUserRecord, anyhow::Error> {
    let user_email = user_email.to_lowercase();
    let org_user = org_users::get(org_id, &user_email)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user role: {}", e))?;
    Ok(org_user)
}

pub async fn get(org_id: &str, user_email: &str) -> Result<OrgUserRecord, anyhow::Error> {
    let user_email = user_email.to_lowercase();
    if let Some(org_user) = ORG_USERS.get(&format!("{}/{}", org_id, user_email)) {
        return Ok(org_user.value().clone());
    }
    let org_user = org_users::get(org_id, &user_email)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user role: {}", e))?;
    Ok(org_user)
}

pub async fn get_expanded_user_org(
    org_id: &str,
    user_email: &str,
) -> Result<OrgUserExpandedRecord, anyhow::Error> {
    let user_email = user_email.to_lowercase();
    let org_user = org_users::get_expanded_user_org(org_id, &user_email)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user role: {}", e))?;
    Ok(org_user)
}

pub async fn get_user_by_rum_token(
    org_id: &str,
    rum_token: &str,
) -> Result<OrgUserExpandedRecord, anyhow::Error> {
    let org_user = org_users::get_user_by_rum_token(org_id, rum_token)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user role: {}", e))?;
    Ok(org_user)
}

pub async fn list_orgs_by_user(
    user_email: &str,
) -> Result<Vec<UserOrgExpandedRecord>, anyhow::Error> {
    let user_email = user_email.to_lowercase();
    let orgs = org_users::list_orgs_by_user(&user_email)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch orgs: {}", e))?;
    Ok(orgs)
}

pub async fn list_users_by_org(org_id: &str) -> Result<Vec<OrgUserRecord>, anyhow::Error> {
    let users = org_users::list_users_by_org(org_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch users: {}", e))?;
    Ok(users)
}

pub async fn list(limit: Option<i64>) -> Result<Vec<OrgUserRecord>, anyhow::Error> {
    let users = org_users::list(limit)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch users: {}", e))?;
    Ok(users)
}

pub async fn update_rum_token(
    org_id: &str,
    user_email: &str,
    rum_token: &str,
) -> Result<(), anyhow::Error> {
    let user_email = user_email.to_lowercase();
    let key = format!("{ORG_USERS_KEY_PREFIX}single/{}/{}", org_id, user_email);
    org_users::update_rum_token(org_id, &user_email, rum_token)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update rum token: {}", e))?;
    let _ = put_into_db_coordinator(&key, Bytes::new(), true, None).await;

    #[cfg(feature = "enterprise")]
    super_cluster::org_user_update_rum_token(&key, rum_token).await?;
    Ok(())
}

pub async fn update_token(
    org_id: &str,
    user_email: &str,
    token: &str,
) -> Result<(), anyhow::Error> {
    let user_email = user_email.to_lowercase();
    let key = format!("{ORG_USERS_KEY_PREFIX}single/{}/{}", org_id, user_email);
    org_users::update_token(org_id, &user_email, token)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to update token: {}", e))?;
    let _ = put_into_db_coordinator(&key, Bytes::new(), true, None).await;

    #[cfg(feature = "enterprise")]
    super_cluster::org_user_update_token(&key, token).await?;
    Ok(())
}

pub async fn watch() -> Result<(), anyhow::Error> {
    let key = ORG_USERS_KEY_PREFIX;
    let cluster_coordinator = get_coordinator().await;
    let mut events = cluster_coordinator.watch(key).await?;
    let events = Arc::get_mut(&mut events).unwrap();
    log::info!("Start watching org_users");
    loop {
        let ev = match events.recv().await {
            Some(ev) => ev,
            None => {
                log::error!("watch_org_users: event channel closed");
                break;
            }
        };
        match ev {
            db::Event::Put(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                if item_key.starts_with("single") {
                    let item_key = item_key.strip_prefix("single/").unwrap();
                    let (org_id, user_id) = item_key.split_once('/').unwrap();
                    let item_value = match get_from_db(org_id, user_id).await {
                        Ok(val) => val,
                        Err(e) => {
                            log::error!("Error getting value: {}", e);
                            continue;
                        }
                    };
                    ORG_USERS.insert(item_key.to_string(), item_value.clone());
                    if let Some(rum_token) = &item_value.rum_token {
                        USERS_RUM_TOKEN
                            .clone()
                            .insert(format!("{}/{}", org_id, rum_token), item_value);
                    }
                } else if item_key.starts_with("many/user/") {
                    let item_key = item_key.strip_prefix("many/user/").unwrap();

                    let item_value: Vec<UserOrgExpandedRecord> =
                        match list_orgs_by_user(item_key).await {
                            Ok(val) => val,
                            Err(e) => {
                                log::error!("Error getting value: {}", e);
                                continue;
                            }
                        };
                    for item in item_value {
                        ORG_USERS.insert(
                            format!("{}/{}", item.org_id, item.email),
                            OrgUserRecord {
                                org_id: item.org_id.clone(),
                                email: item.email.clone(),
                                role: item.role.clone(),
                                token: item.token.clone(),
                                rum_token: item.rum_token.clone(),
                                created_at: item.created_at,
                            },
                        );
                        if let Some(rum_token) = &item.rum_token {
                            USERS_RUM_TOKEN.clone().insert(
                                format!("{}/{}", item.org_id, rum_token),
                                OrgUserRecord {
                                    org_id: item.org_id,
                                    email: item.email,
                                    role: item.role,
                                    token: item.token,
                                    rum_token: item.rum_token,
                                    created_at: item.created_at,
                                },
                            );
                        }
                    }
                }
            }
            db::Event::Delete(ev) => {
                let item_key = ev.key.strip_prefix(key).unwrap();
                if item_key.starts_with("single") {
                    let item_key = item_key.strip_prefix("single/").unwrap();
                    let item_value = ORG_USERS.remove(item_key);
                    if let Some((_, item_value)) = item_value {
                        USERS_RUM_TOKEN.clone().remove(&format!(
                            "{}/{}",
                            item_value.org_id,
                            item_value.rum_token.as_ref().unwrap()
                        ));
                    }
                } else if item_key.starts_with("many/user/") {
                    let item_key = item_key.strip_prefix("many/user/").unwrap();

                    let item_value: Vec<UserOrgExpandedRecord> =
                        match list_orgs_by_user(item_key).await {
                            Ok(val) => val,
                            Err(e) => {
                                log::error!("Error getting value: {}", e);
                                continue;
                            }
                        };
                    for item in item_value {
                        ORG_USERS.remove(&format!("{}/{item_key}", item.org_id));
                        if let Some(rum_token) = &item.rum_token {
                            USERS_RUM_TOKEN
                                .clone()
                                .remove(&format!("{}/{}", item.org_id, rum_token));
                        }
                    }
                }
            }
            db::Event::Empty => {}
        }
    }
    Ok(())
}

pub async fn cache() -> Result<(), anyhow::Error> {
    let org_user_records = list(None).await?;
    for user in org_user_records {
        ORG_USERS.insert(
            format!("{}/{}", user.org_id.clone(), user.email.clone()),
            user.clone(),
        );
        if let Some(rum_token) = &user.rum_token {
            USERS_RUM_TOKEN.clone().insert(
                format!("{}/{}", user.org_id.clone(), rum_token),
                user.clone(),
            );
        }
        if user.role.eq(&UserRole::Root) {
            let root = match USERS.get(&user.email) {
                Some(root) => root.clone(),
                None => super::user::get_user_record(&user.email).await?,
            };

            ROOT_USER.insert(
                "root".to_string(),
                User {
                    email: root.email,
                    password: root.password,
                    role: UserRole::Root,
                    salt: root.salt,
                    first_name: root.first_name,
                    last_name: root.last_name,
                    password_ext: root.password_ext,
                    token: user.token,
                    rum_token: user.rum_token,
                    org: user.org_id,
                    is_external: root.user_type.is_external(),
                },
            );
        }
    }
    log::info!("Organizations users Cached");
    Ok(())
}

#[cfg(feature = "enterprise")]
mod super_cluster {
    use core::convert::Into;

    use config::utils::json;
    use infra::table::org_users::OrgUserPut;
    use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;

    pub async fn org_user_add(
        key: &str,
        org_user: &OrgUserPut,
    ) -> Result<(), infra::errors::Error> {
        let value = json::to_vec(org_user)?.into();
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::org_user_add(
                key,
                value,
                infra::db::NEED_WATCH,
                None,
            )
            .await
            .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn org_user_update(
        key: &str,
        org_user: &OrgUserPut,
    ) -> Result<(), infra::errors::Error> {
        let value = json::to_vec(org_user)?.into();
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::org_user_update(
                key,
                value,
                infra::db::NEED_WATCH,
                None,
            )
            .await
            .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn org_user_remove(key: &str) -> Result<(), infra::errors::Error> {
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::org_user_remove(
                key,
                infra::db::NEED_WATCH,
                None,
            )
            .await
            .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn org_user_update_token(key: &str, token: &str) -> Result<(), infra::errors::Error> {
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::org_user_update_token(
                key,
                token.to_string().into(),
                infra::db::NEED_WATCH,
                None,
            )
            .await
            .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn org_user_update_rum_token(
        key: &str,
        rum_token: &str,
    ) -> Result<(), infra::errors::Error> {
        if get_o2_config().super_cluster.enabled {
            o2_enterprise::enterprise::super_cluster::queue::org_user_update_rum_token(
                key,
                rum_token.to_string().into(),
                infra::db::NEED_WATCH,
                None,
            )
            .await
            .map_err(|e| infra::errors::Error::Message(e.to_string()))?;
        }
        Ok(())
    }
}
