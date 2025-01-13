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
#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
use std::str::FromStr;

#[cfg(feature = "enterprise")]
use jsonwebtoken::TokenData;
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::openfga::authorizer::authz::{
    get_user_creation_tuples, get_user_org_tuple, update_tuples,
};
#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
use o2_enterprise::enterprise::openfga::authorizer::roles::{
    check_and_get_crole_tuple_for_new_user, get_roles_for_user, get_user_crole_removal_tuples,
};
#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
use once_cell::sync::Lazy;
#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
use regex::Regex;
#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
use {
    crate::{
        common::meta::user::RoleOrg,
        service::{organization, users},
    },
    config::meta::user::{UserOrg, UserRole},
};
#[cfg(feature = "enterprise")]
use {
    crate::{common::meta::user::TokenValidationResponse, service::db},
    config::meta::user::DBUser,
    o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config,
};
#[cfg(feature = "enterprise")]
use {serde_json::Value, std::collections::HashMap};

#[cfg(feature = "cloud")]
use crate::{
    common::meta::organization::{Organization, USER_DEFAULT},
    service::organization::{accept_invitation, list_orgs_by_user},
};

#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
static RE_ROLE_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^a-zA-Z0-9_]+").unwrap());

#[cfg(feature = "enterprise")]
pub async fn process_token(
    res: (
        TokenValidationResponse,
        Option<TokenData<HashMap<String, Value>>>,
    ),
) {
    let dec_token = res.1.unwrap();

    let user_email = res.0.user_email.to_owned();

    let name = match dec_token.claims.get("name") {
        None => res.0.user_email.to_owned(),
        Some(name) => name.as_str().unwrap().to_string(),
    };

    #[cfg(feature = "cloud")]
    {
        check_and_add_to_org(&user_email, &name).await;
    }

    #[cfg(not(feature = "cloud"))]
    {
        use config::get_config;
        use o2_enterprise::enterprise::openfga::authorizer::authz::{
            get_user_role_creation_tuple, get_user_role_deletion_tuple,
        };

        use crate::common::meta::user::UserOrgRole;

        let o2cfg = get_o2_config();
        let groups = match dec_token.claims.get(&o2cfg.dex.group_claim) {
            None => vec![],
            Some(groups) => {
                if !groups.is_array() {
                    vec![]
                } else {
                    groups.as_array().unwrap().to_vec()
                }
            }
        };

        let mut source_orgs: Vec<UserOrg> = vec![];
        let mut custom_roles: Vec<String> = vec![];
        let mut tuples_to_add = HashMap::new();
        if groups.is_empty() {
            let role = if let Some(role) = &res.0.user_role {
                role.clone()
            } else {
                UserRole::from_str(&o2cfg.dex.default_role).unwrap()
            };
            source_orgs.push(UserOrg {
                role,
                name: o2cfg.dex.default_org.clone(),
                token: Default::default(),
                rum_token: Default::default(),
            });
        } else {
            for group in groups {
                let role_org = parse_dn(group.as_str().unwrap()).unwrap();
                if o2cfg.openfga.map_group_to_role {
                    custom_roles.push(format_role_name(
                        &role_org.org,
                        role_org.custom_role.unwrap(),
                    ));
                } else {
                    source_orgs.push(UserOrg {
                        role: role_org.role,
                        name: role_org.org,
                        token: Default::default(),
                        rum_token: Default::default(),
                    });
                }
            }
        }

        // Assign users custom roles in RBAC
        if o2cfg.openfga.map_group_to_role {
            map_group_to_custom_role(&user_email, &name, custom_roles, res.0.user_role).await;
            return;
        }

        // Check if the user exists in the database
        let db_user = db::user::get_user_by_email(&user_email).await;

        if db_user.is_none() {
            log::info!("User does not exist in the database");

            if o2cfg.openfga.enabled {
                for (index, org) in source_orgs.iter().enumerate() {
                    // Assuming all the relevant tuples for this org exist
                    let mut tuples = vec![];
                    get_user_creation_tuples(
                        &org.name,
                        &user_email,
                        &org.role.to_string(),
                        &mut tuples,
                    );

                    // Create the org if it does not exist. `org.name` is the id of the org.
                    // Also it creates necessary ofga tuples for the newly created org
                    let _ = organization::check_and_create_org(&org.name).await;

                    if index == 0 {
                        // this is to allow user call organization api with org
                        tuples.push(get_user_org_tuple(&user_email, &user_email));
                    }

                    tuples_to_add.insert(org.name.to_owned(), tuples);
                }
            }
            let updated_db_user = DBUser {
                email: user_email.to_owned(),
                first_name: name.to_owned(),
                last_name: "".to_owned(),
                password: "".to_owned(),
                salt: "".to_owned(),
                organizations: source_orgs,
                is_external: true,
                password_ext: Some("".to_owned()),
            };

            match users::create_new_user(updated_db_user).await {
                Ok(_) => {
                    log::info!("User added to the database");
                    if o2cfg.openfga.enabled {
                        for (_, tuples) in tuples_to_add {
                            match update_tuples(tuples, vec![]).await {
                                Ok(_) => {
                                    log::info!("User updated to the openfga");
                                }
                                Err(e) => {
                                    log::error!("Error updating user to the openfga: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error adding user to the database: {}", e);
                }
            }
        } else {
            log::info!("User exists in the database perform check for role change");
            let existing_db_user = db_user.unwrap();
            let existing_orgs = existing_db_user.organizations;
            let mut orgs_removed = Vec::new();
            let mut orgs_role_changed = HashMap::new();
            let mut orgs_added = Vec::new();

            let mut write_tuples = Vec::new();
            let mut delete_tuples = Vec::new();

            // Check for newly added organizations
            for source_org in &source_orgs {
                if !existing_orgs
                    .iter()
                    .any(|existing_org| existing_org.name == source_org.name)
                {
                    orgs_added.push(source_org);
                }
            }

            for existing_org in &existing_orgs {
                match source_orgs
                    .iter()
                    .find(|&src_org| src_org.name == existing_org.name)
                {
                    Some(src_org) => {
                        if src_org.role != existing_org.role {
                            // The role has changed for this organization
                            orgs_role_changed.insert(existing_org.role.to_string(), src_org);
                        }
                    }
                    None => {
                        // The organization is not found in source_orgs, hence removed
                        orgs_removed.push(existing_org);
                    }
                }
            }

            // Add the user to the newly added organizations
            for org in orgs_added {
                let _ = organization::check_and_create_org(&org.name).await;

                match users::add_user_to_org(
                    &org.name,
                    &user_email,
                    UserOrgRole {
                        base_role: org.role.clone(),
                        custom_role: None,
                    },
                    &get_config().auth.root_user_email,
                )
                .await
                {
                    Ok(_) => {
                        log::info!("User added to the organization {}", org.name);
                        write_tuples.push(get_user_role_creation_tuple(
                            &org.role.to_string(),
                            &user_email,
                            &org.name,
                        ));
                    }
                    Err(e) => {
                        log::error!("Error adding user to the organization {}: {}", org.name, e);
                    }
                }
            }

            for org in orgs_removed {
                match users::remove_user_from_org(
                    &org.name,
                    &user_email,
                    &get_config().auth.root_user_email,
                )
                .await
                {
                    Ok(_) => {
                        log::info!("User removed from the organization {}", org.name);
                        delete_tuples.push(get_user_role_deletion_tuple(
                            &org.role.to_string(),
                            &user_email,
                            &org.name,
                        ));
                    }
                    Err(e) => {
                        log::error!(
                            "Error removing user to the organization {}: {}",
                            org.name,
                            e
                        );
                    }
                }
            }
            for (existing_role, org) in orgs_role_changed {
                match users::update_user(
                    &org.name,
                    &user_email,
                    false,
                    &get_config().auth.root_user_email,
                    crate::common::meta::user::UpdateUser {
                        role: Some(crate::common::meta::user::UserRoleRequest {
                            role: org.role.to_string(),
                            custom: None,
                        }),
                        ..Default::default()
                    },
                )
                .await
                {
                    Ok(_) => {
                        log::info!("User update for the organization {}", org.name);
                        delete_tuples.push(get_user_role_deletion_tuple(
                            &existing_role,
                            &user_email,
                            &org.name,
                        ));
                        write_tuples.push(get_user_role_creation_tuple(
                            &org.role.to_string(),
                            &user_email,
                            &org.name,
                        ));
                    }
                    Err(e) => {
                        log::error!(
                            "Error updating user to the organization {}: {}",
                            org.name,
                            e
                        );
                    }
                }
            }
            if o2cfg.openfga.enabled {
                if write_tuples.is_empty() && delete_tuples.is_empty() {
                    log::info!("No changes to the user information tuples");
                } else {
                    log::info!(
                        "openfga tuples: wt {:?} ,dt {:?} ",
                        write_tuples,
                        delete_tuples
                    );

                    match update_tuples(write_tuples, delete_tuples).await {
                        Ok(_) => {
                            log::info!("User updated to the openfga");
                        }
                        Err(_) => {
                            log::error!("Error updating user to the openfga");
                        }
                    }
                }
            }
        };
    }
}

#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
fn parse_dn(dn: &str) -> Option<RoleOrg> {
    let mut org = "";
    let mut role = "";
    let mut custom_role = None;

    let o2cfg = get_o2_config();
    if o2cfg.openfga.map_group_to_role {
        custom_role = Some(dn.to_owned());
        org = &o2cfg.dex.default_org;
    } else {
        for part in dn.split(',') {
            let parts: Vec<&str> = part.split('=').collect();
            if parts.len() == 2 {
                if parts[0].eq(&o2cfg.dex.group_attribute) && org.is_empty() {
                    org = parts[1];
                }
                if parts[0].eq(&o2cfg.dex.role_attribute) && role.is_empty() {
                    role = parts[1];
                }
            }
        }
    }
    let role = if role.is_empty() {
        UserRole::from_str(&o2cfg.dex.default_role).unwrap()
    } else {
        UserRole::from_str(role).unwrap()
    };

    if org.is_empty() {
        org = &o2cfg.dex.default_org;
    }
    Some(RoleOrg {
        role,
        org: org.to_owned(),
        custom_role,
    })
}

#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
async fn map_group_to_custom_role(
    user_email: &str,
    name: &str,
    custom_roles: Vec<String>,
    default_role: Option<UserRole>,
) {
    let o2cfg = get_o2_config();
    // Check if the user exists in the database
    let db_user = db::user::get_user_by_email(user_email).await;
    log::debug!("map_group_to_custom_role custom roles: {:#?}", custom_roles);

    if db_user.is_none() {
        let mut tuples = vec![];
        log::info!("group_to_custom_role: User does not exist in the database");

        let role = if let Some(role) = default_role {
            role
        } else {
            UserRole::from_str(&o2cfg.dex.default_role).unwrap()
        };

        if o2cfg.openfga.enabled {
            let _ = organization::check_and_create_org(&o2cfg.dex.default_org).await;
            tuples.push(get_user_org_tuple(&o2cfg.dex.default_org, user_email));
            // this check added to avoid service accounts from logging in
            if !role.eq(&UserRole::ServiceAccount) {
                tuples.push(get_user_org_tuple(user_email, user_email));
            }
            let start = std::time::Instant::now();
            check_and_get_crole_tuple_for_new_user(
                user_email,
                &o2cfg.dex.default_org,
                custom_roles,
                &mut tuples,
            )
            .await;
            log::info!(
                "group_to_custom_role: Time taken to get crole tuple: {:?}",
                start.elapsed()
            );
        }

        let updated_db_user = DBUser {
            email: user_email.to_owned(),
            first_name: name.to_owned(),
            last_name: "".to_owned(),
            password: "".to_owned(),
            salt: "".to_owned(),
            organizations: vec![UserOrg {
                role,
                name: o2cfg.dex.default_org.clone(),
                token: Default::default(),
                rum_token: Default::default(),
            }],
            is_external: true,
            password_ext: Some("".to_owned()),
        };

        match users::create_new_user(updated_db_user).await {
            Ok(_) => {
                log::info!("group_to_custom_role: User added to the database");
                if o2cfg.openfga.enabled {
                    let start = std::time::Instant::now();
                    match update_tuples(tuples, vec![]).await {
                        Ok(_) => {
                            log::info!("group_to_custom_role: User updated to the openfga");
                        }
                        Err(e) => {
                            log::error!(
                                "group_to_custom_role: Error updating user to the openfga: {}",
                                e
                            );
                        }
                    }

                    log::info!(
                        "group_to_custom_role: Time taken to update roles to db for new user: {:?}",
                        start.elapsed()
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "group_to_custom_role: Error adding user to the database: {}",
                    e
                );
            }
        }
    } else {
        log::info!("group_to_custom_role: User exists in the database");
        let mut add_tuples = vec![];
        let mut remove_tuples = vec![];
        // user exists in the db with default org hence skip org creation tuples
        let existing_roles = get_roles_for_user(user_email).await;

        // Find roles to delete: present in existing_role but not in custom_role
        for existing_role in &existing_roles {
            if !custom_roles.contains(existing_role) {
                // delete role
                get_user_crole_removal_tuples(user_email, existing_role, &mut remove_tuples);
            }
        }

        // Find new roles to add: present in custom_role but not in existing_role
        let new_roles = custom_roles
            .iter()
            .filter(|&role| !existing_roles.contains(role))
            .cloned()
            .collect();
        log::debug!("new roles: {:#?}", new_roles);

        check_and_get_crole_tuple_for_new_user(
            user_email,
            &o2cfg.dex.default_org,
            new_roles,
            &mut add_tuples,
        )
        .await;
        log::debug!(
            "add_tuples: {:#?}\nremove_tuples: {:#?}",
            add_tuples,
            remove_tuples
        );

        if o2cfg.openfga.enabled {
            let start = std::time::Instant::now();
            match update_tuples(add_tuples, remove_tuples).await {
                Ok(_) => {
                    log::info!("group_to_custom_role: User updated to the openfga");
                }
                Err(e) => {
                    log::error!(
                        "group_to_custom_role: Error updating user to the openfga: {}",
                        e
                    );
                }
            }
            log::info!(
                "group_to_custom_role: Time taken to update roles to db for existing user: {:?}",
                start.elapsed()
            );
        }
    }
}

#[cfg(all(feature = "enterprise", not(feature = "cloud")))]
fn format_role_name(org: &str, role: String) -> String {
    let role = RE_ROLE_NAME.replace_all(&role, "_").to_string();
    format!("{org}/{role}")
}

#[cfg(feature = "cloud")]
pub async fn check_and_add_to_org(user_email: &str, name: &str) {
    use config::ider;
    use o2_enterprise::enterprise::{
        cloud::org_invites::list_by_invitee, openfga::authorizer::authz::save_org_tuples,
    };

    use crate::service::users::{add_admin_to_org, create_new_user};
    let o2cfg = get_o2_config();

    let mut tuples_to_add = HashMap::new();
    let (first_name, last_name) = name.split_once(' ').unwrap_or((name, ""));
    let db_user = db::user::get_user_by_email(user_email).await;
    if db_user.is_none() {
        match create_new_user(DBUser {
            email: user_email.to_owned(),
            first_name: first_name.to_owned(),
            last_name: last_name.to_owned(),
            password: "".to_owned(),
            salt: "".to_owned(),
            organizations: vec![], // No org
            is_external: true,
            password_ext: Some("".to_owned()),
        })
        .await
        {
            Ok(_) => {
                let tuples = vec![get_user_org_tuple(user_email, user_email)];
                tuples_to_add.insert(user_email.to_string(), tuples);
                log::info!("User added to the database");
            }
            Err(e) => {
                log::error!("Error adding user to the database: {}", e);
                return;
            }
        }
    }

    match list_by_invitee(user_email).await {
        Ok(invites) => {
            if !invites.is_empty() {
                for invite in invites {
                    let org_id = invite.org_id.clone();
                    let invite_token = invite.token.clone();
                    if let Err(e) = accept_invitation(user_email, &invite_token).await {
                        log::error!(
                            "Error accepting invite for user: {} org: {} error: {}",
                            user_email,
                            org_id,
                            e
                        );
                    } else {
                        let mut tuples = vec![];
                        get_user_creation_tuples(&org_id, user_email, &invite.role, &mut tuples);
                        tuples_to_add.insert(org_id, tuples);
                    }
                }
            }
        }

        Err(e) => {
            log::error!(
                "Error fetching invites for user: {}, error: {e}",
                user_email
            );
        }
    };

    // Check if the user is part of any organization
    let orgs = list_orgs_by_user(user_email).await;
    if orgs.is_err() {
        log::error!("Error fetching orgs for user: {}", user_email);
    }
    if orgs.is_err() || orgs.unwrap().is_empty() {
        // Create a default org for the user
        let org = Organization {
            // id will be overridden by the service function
            identifier: ider::uuid(),
            name: format!("{}'s Organization", name),
            org_type: USER_DEFAULT.to_owned(),
        };
        match db::organization::save_org(&org).await {
            Ok(_) => {
                save_org_tuples(&org.identifier).await;
                if let Err(e) = add_admin_to_org(&org.identifier, user_email).await {
                    log::error!(
                        "Error adding user as admin to org: {} error: {}",
                        org.identifier,
                        e
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "Error creating default org for user: {} error: {}",
                    user_email,
                    e
                );
            }
        }
    }

    if o2cfg.openfga.enabled {
        for (_, tuples) in tuples_to_add {
            match update_tuples(tuples, vec![]).await {
                Ok(_) => {
                    log::info!("User updated to the openfga");
                }
                Err(e) => {
                    log::error!("Error updating user to the openfga: {}", e);
                }
            }
        }
    }
}
