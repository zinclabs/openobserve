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

#[cfg(feature = "cloud")]
use chrono::{Duration, Utc};
#[cfg(feature = "cloud")]
use config::{get_config, SMTP_CLIENT};
use config::{
    ider,
    meta::{
        dashboards::ListDashboardsParams,
        pipeline::components::PipelineSource,
        stream::StreamType,
        user::{UserOrg, UserRole},
    },
    utils::rand::generate_random_string,
};
#[cfg(feature = "cloud")]
use lettre::{message::SinglePart, AsyncTransport, Message};
#[cfg(feature = "cloud")]
use o2_enterprise::enterprise::cloud::{org_invites, OrgInviteStatus};
#[cfg(feature = "enterprise")]
use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;

use super::{db::org_users, users::add_admin_to_org};
#[cfg(feature = "cloud")]
use crate::common::meta::organization::{
    OrganizationInviteResponse, OrganizationInviteUserRecord, OrganizationInvites,
};
use crate::{
    common::{
        meta::organization::{
            AlertSummary, IngestionPasscode, IngestionTokensContainer, OrgSummary, Organization,
            PipelineSummary, RumIngestionToken, StreamSummary, CUSTOM, DEFAULT_ORG,
        },
        utils::auth::{delete_org_tuples, is_root_user, save_org_tuples},
    },
    service::{db, stream::get_streams},
};

pub async fn get_summary(org_id: &str) -> OrgSummary {
    let streams = get_streams(org_id, None, false, None).await;
    let mut stream_summary = StreamSummary::default();
    for stream in streams.iter() {
        if !stream.stream_type.eq(&StreamType::Index)
            && !stream.stream_type.eq(&StreamType::Metadata)
        {
            stream_summary.num_streams += 1;
            stream_summary.total_records += stream.stats.doc_num;
            stream_summary.total_storage_size += stream.stats.storage_size;
            stream_summary.total_compressed_size += stream.stats.compressed_size;
            stream_summary.total_index_size += stream.stats.index_size;
        }
    }

    let pipelines = db::pipeline::list_by_org(org_id).await.unwrap_or_default();
    let pipeline_summary = PipelineSummary {
        num_realtime: pipelines
            .iter()
            .filter(|p| matches!(p.source, PipelineSource::Realtime(_)))
            .count() as i64,
        num_scheduled: pipelines
            .iter()
            .filter(|p| matches!(p.source, PipelineSource::Scheduled(_)))
            .count() as i64,
    };

    let alerts = db::alerts::alert::list(org_id, None, None).await.unwrap();
    let alert_summary = AlertSummary {
        num_realtime: alerts.iter().filter(|a| a.is_real_time).count() as i64,
        num_scheduled: alerts.iter().filter(|a| !a.is_real_time).count() as i64,
    };

    let functions = db::functions::list(org_id).await.unwrap_or_default();
    let dashboards = super::dashboards::list_dashboards(ListDashboardsParams::new(org_id))
        .await
        .unwrap_or_default();

    OrgSummary {
        streams: stream_summary,
        pipelines: pipeline_summary,
        alerts: alert_summary,
        total_functions: functions.len() as i64,
        total_dashboards: dashboards.len() as i64,
    }
}

pub async fn get_passcode(
    org_id: Option<&str>,
    user_id: &str,
) -> Result<IngestionPasscode, anyhow::Error> {
    let Ok(Some(user)) = db::user::get(org_id, user_id).await else {
        return Err(anyhow::Error::msg("User not found"));
    };
    if user.role.eq(&UserRole::ServiceAccount) && user.is_external {
        return Err(anyhow::Error::msg(
            "Not allowed for external service accounts",
        ));
    }
    Ok(IngestionPasscode {
        user: user.email,
        passcode: user.token,
    })
}

pub async fn get_rum_token(
    org_id: Option<&str>,
    user_id: &str,
) -> Result<RumIngestionToken, anyhow::Error> {
    let Ok(Some(user)) = db::user::get(org_id, user_id).await else {
        return Err(anyhow::Error::msg("User not found"));
    };
    Ok(RumIngestionToken {
        user: user.email,
        rum_token: user.rum_token,
    })
}

pub async fn update_rum_token(
    org_id: Option<&str>,
    user_id: &str,
) -> Result<RumIngestionToken, anyhow::Error> {
    let is_rum_update = true;
    match update_passcode_inner(org_id, user_id, is_rum_update).await {
        Ok(IngestionTokensContainer::RumToken(response)) => Ok(response),
        _ => Err(anyhow::Error::msg("User not found")),
    }
}

pub async fn update_passcode(
    org_id: Option<&str>,
    user_id: &str,
) -> Result<IngestionPasscode, anyhow::Error> {
    let is_rum_update = false;
    match update_passcode_inner(org_id, user_id, is_rum_update).await {
        Ok(IngestionTokensContainer::Passcode(response)) => Ok(response),
        Err(e) => Err(e),
        _ => Err(anyhow::Error::msg("User not found")),
    }
}

async fn update_passcode_inner(
    org_id: Option<&str>,
    user_id: &str,
    is_rum_update: bool,
) -> Result<IngestionTokensContainer, anyhow::Error> {
    let mut local_org_id = "";
    let Ok(db_user) = db::user::get_db_user(user_id).await else {
        return Err(anyhow::Error::msg("User not found"));
    };

    if org_id.is_some() {
        local_org_id = org_id.unwrap();
    }
    let token = generate_random_string(16);
    let rum_token = format!("rum{}", generate_random_string(16));

    if !is_root_user(user_id) {
        let orgs = db_user
            .organizations
            .iter()
            .filter(|org| org.name.eq(local_org_id))
            .collect::<Vec<&UserOrg>>();
        if orgs.is_empty() {
            return Err(anyhow::Error::msg("User not found"));
        }
        let org_to_update = orgs[0];
        if org_to_update.role.eq(&UserRole::ServiceAccount) && db_user.is_external {
            return Err(anyhow::Error::msg(
                "Not allowed for external service accounts",
            ));
        }
    } else {
        local_org_id = DEFAULT_ORG;
    }

    // Update the org with the new token
    if is_rum_update {
        org_users::update_rum_token(local_org_id, user_id, &rum_token).await?;
    } else {
        org_users::update_token(local_org_id, user_id, &token).await?;
    }

    // TODO : Fix for root users
    let ret = if is_rum_update {
        IngestionTokensContainer::RumToken(RumIngestionToken {
            user: db_user.email,
            rum_token: Some(rum_token),
        })
    } else {
        IngestionTokensContainer::Passcode(IngestionPasscode {
            user: db_user.email,
            passcode: token,
        })
    };
    Ok(ret)
}

pub async fn list_all_orgs(limit: Option<i64>) -> Result<Vec<Organization>, anyhow::Error> {
    db::organization::list(limit).await
}

pub async fn list_orgs_by_user(user_email: &str) -> Result<Vec<Organization>, anyhow::Error> {
    let records = db::org_users::list_orgs_by_user(user_email).await?;
    Ok(records
        .into_iter()
        .map(|record| Organization {
            identifier: record.org_id.clone(),
            name: record.org_name.clone(),
            org_type: record.org_type.to_string(),
        })
        .collect())
}

/// Always creates a new org. Also, makes the user an admin of the org
pub async fn create_org(
    org: &mut Organization,
    user_email: &str,
) -> Result<Organization, anyhow::Error> {
    #[cfg(not(feature = "enterprise"))]
    let is_allowed = false;
    #[cfg(feature = "enterprise")]
    let is_allowed = if get_o2_config().openfga.enabled {
        // In this case, openfga takes care of permission checks
        // If the request reaches here, it means the user is allowed
        true
    } else {
        false
    };
    if !is_allowed && !is_root_user(user_email) {
        return Err(anyhow::anyhow!("Only root user can create organization"));
    }
    org.name = org.name.trim().to_owned();

    #[cfg(feature = "cloud")]
    if !is_add_user_allowed_for_org("", user_email).await {
        return Err(anyhow::anyhow!(
            "User can not be member of more than one free organization"
        ));
    }

    org.identifier = ider::uuid();
    #[cfg(not(feature = "cloud"))]
    let org_type = CUSTOM.to_owned();
    #[cfg(feature = "cloud")]
    let org_type = org.org_type.clone();
    org.org_type = org_type;
    match db::organization::save_org(org).await {
        Ok(_) => {
            save_org_tuples(&org.identifier).await;
            add_admin_to_org(&org.identifier, user_email).await?;
            Ok(org.clone())
        }
        Err(e) => {
            log::error!("Error creating org: {}", e);
            Err(anyhow::anyhow!("Error creating org: {}", e))
        }
    }
}

/// Checks if the org exists, otherwise creates the org. Does not associate any user
/// with the org, only saves the org in the meta and creates org tuples.
pub async fn check_and_create_org(org_id: &str) -> Result<Organization, anyhow::Error> {
    let org_id = org_id.trim();
    if let Some(org) = get_org(org_id).await {
        return Ok(org);
    }

    let org = &Organization {
        identifier: org_id.to_owned(),
        name: org_id.to_owned(),
        org_type: if org_id.eq(DEFAULT_ORG) {
            DEFAULT_ORG.to_owned()
        } else {
            CUSTOM.to_owned()
        },
    };
    match db::organization::save_org(org).await {
        Ok(_) => {
            save_org_tuples(&org.identifier).await;
            Ok(org.clone())
        }
        Err(e) => {
            log::error!("Error creating org: {}", e);
            Err(anyhow::anyhow!("Error creating org: {}", e))
        }
    }
}

pub async fn check_and_create_org_without_ofga(
    org_id: &str,
) -> Result<Organization, anyhow::Error> {
    let org_id = org_id.trim();
    if let Some(org) = get_org(org_id).await {
        return Ok(org);
    }

    let org = &Organization {
        identifier: org_id.to_owned(),
        name: org_id.to_owned(),
        org_type: if org_id.eq(DEFAULT_ORG) {
            DEFAULT_ORG.to_owned()
        } else {
            CUSTOM.to_owned()
        },
    };
    match db::organization::save_org(org).await {
        Ok(_) => Ok(org.clone()),
        Err(e) => {
            log::error!("Error creating org: {}", e);
            Err(anyhow::anyhow!("Error creating org: {}", e))
        }
    }
}

pub async fn rename_org(
    org_id: &str,
    name: &str,
    user_email: &str,
) -> Result<Organization, anyhow::Error> {
    #[cfg(not(feature = "enterprise"))]
    let is_allowed = false;
    #[cfg(feature = "enterprise")]
    let is_allowed = if get_o2_config().openfga.enabled {
        // In this case, openfga takes care of permission checks
        // If the request reaches here, it means the user is allowed
        true
    } else {
        false
    };
    if !is_allowed && !is_root_user(user_email) {
        return Err(anyhow::anyhow!("Not allowed to rename org"));
    }

    if get_org(org_id).await.is_none() {
        return Err(anyhow::anyhow!("Organization doesn't exist"));
    }
    let mut org = get_org(org_id).await.unwrap();
    org.name = name.trim().to_owned();
    match db::organization::save_org(&org).await {
        Ok(_) => Ok(org),
        Err(e) => {
            log::error!("Error creating org: {}", e);
            Err(anyhow::anyhow!("Error creating org: {}", e))
        }
    }
}

pub async fn remove_org(org_id: &str) -> Result<(), anyhow::Error> {
    if org_id.eq(DEFAULT_ORG) {
        return Err(anyhow::anyhow!("Cannot delete default organization"));
    }
    if get_org(org_id).await.is_none() {
        return Err(anyhow::anyhow!("Organization does not exist"));
    }
    match db::organization::delete_org(org_id).await {
        Ok(_) => {
            delete_org_tuples(org_id).await;
            Ok(())
        }
        Err(e) => {
            log::error!("Error deleting org: {}", e);
            Err(anyhow::anyhow!("Error deleting org: {}", e))
        }
    }
}

#[cfg(feature = "cloud")]
pub async fn get_invitations_for_org(
    org_id: &str,
) -> Result<Vec<OrganizationInviteUserRecord>, anyhow::Error> {
    use crate::common::meta::user::InviteStatus;

    let invites = org_invites::get_invite_list_for_org(org_id).await?;
    Ok(invites
        .into_iter()
        .map(|invite| OrganizationInviteUserRecord {
            email: invite.invitee_id,
            first_name: "".to_owned(),
            last_name: "".to_owned(),
            status: InviteStatus::from(&invite.status),
            expires_at: invite.expires_at,
            is_external: true,
            role: invite.role,
        })
        .collect())
}

#[cfg(feature = "cloud")]
pub async fn generate_invitation(
    org_id: &str,
    user_email: &str,
    invites: OrganizationInvites,
) -> Result<OrganizationInviteResponse, anyhow::Error> {
    use org_invites::{get_invite_email_body, get_invite_email_subject};

    use super::users::get_user;

    let mut inviter_name = "".to_owned();
    let cfg = get_config();
    if !is_root_user(user_email) {
        match get_user(Some(org_id), user_email).await {
            Some(user) => {
                inviter_name = format!("{} {}", user.first_name, user.last_name);
                if !(user.role.eq(&UserRole::Admin) || user.role.eq(&UserRole::Root)) {
                    return Err(anyhow::anyhow!("Unauthorized access"));
                }
            }
            None => return Err(anyhow::anyhow!("Unauthorized access")),
        }
    }
    if let Some(org) = get_org(org_id).await {
        let invite_token = config::ider::generate();
        let expires_at = Utc::now().timestamp_micros()
            + Duration::days(cfg.common.org_invite_expiry as i64)
                .num_microseconds()
                .unwrap();

        org_invites::add_many(
            &invites.role.to_string(),
            user_email,
            org_id,
            &invite_token,
            expires_at,
            invites.invites.clone(),
        )
        .await?;

        if cfg.smtp.smtp_enabled {
            // TODO: Use an env to decide whether to send email or not
            let mut email = Message::builder()
                .from(cfg.smtp.smtp_from_email.parse()?)
                .subject(get_invite_email_subject(&org.name));
            for invite in invites.invites.iter() {
                email = email.to(invite.parse()?);
            }
            if !cfg.smtp.smtp_reply_to.is_empty() {
                email = email.reply_to(cfg.smtp.smtp_reply_to.parse()?);
            }
            let msg = get_invite_email_body(org_id, &org.name, &inviter_name, &invite_token);
            let email = email.singlepart(SinglePart::html(msg)).unwrap();

            // Send the email
            match SMTP_CLIENT.as_ref().unwrap().send(email).await {
                Ok(resp) => {
                    log::info!("sent invite email response code: {}", resp.code());
                }
                Err(e) => {
                    log::error!("Error sending email for invitation: {}", e);
                }
            }
        }
        Ok(OrganizationInviteResponse {
            data: Default::default(),
            message: "Member invitation created successfully".to_owned(),
        })
    } else {
        Err(anyhow::anyhow!("Organization doesn't exist"))
    }
}

#[cfg(feature = "cloud")]
pub async fn accept_invitation(user_email: &str, invite_token: &str) -> Result<(), anyhow::Error> {
    use std::str::FromStr;

    use crate::service::db::org_users::get_cached_user_org;

    let invite = org_invites::get_by_token_user(invite_token, user_email).await?;

    let now = chrono::Utc::now().timestamp_micros();
    if !is_add_user_allowed_for_org(&invite.org_id, user_email).await {
        return Err(anyhow::anyhow!(
            "User is already part of a free organization"
        ));
    }
    if invite.expires_at < now {
        return Err(anyhow::anyhow!("Invalid token"));
    }
    let org_id = invite.org_id.clone();

    if get_org(&org_id).await.is_none() {
        return Err(anyhow::anyhow!("Organization doesn't exist"));
    }

    // Check if user is already part of the org
    if get_cached_user_org(&org_id, user_email).is_some() {
        return Ok(()); // User is already part of the org
    }

    let invite_role = UserRole::from_str(&invite.role)
        .map_err(|_| anyhow::anyhow!("Invalid role: {}", invite.role))?;
    org_users::add(
        &org_id,
        user_email,
        invite_role,
        &generate_random_string(16),
        Some(format!("rum{}", generate_random_string(16))),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Failed to add user to org"))?;

    if let Err(e) =
        org_invites::update_invite_status(invite_token, user_email, OrgInviteStatus::Accepted).await
    {
        log::error!("Error updating the invite status in the db: {e}");
    }
    Ok(())
}

#[cfg(feature = "cloud")]
pub async fn is_add_user_allowed_for_org(org_id: &str, user_email: &str) -> bool {
    use o2_enterprise::enterprise::cloud::org_usage;

    let org = org_usage::get(org_id).await;
    let mut is_already_part_of_free_org = false;
    let member_orgs = list_orgs_by_user(user_email).await.unwrap_or_default();
    if member_orgs.is_empty() {
        return true;
    }
    for member_org in member_orgs {
        if let Ok(subscription) = org_usage::get(&member_org.identifier).await {
            if subscription.is_some() {
                let subscription = subscription.unwrap();
                if subscription.is_free_sub() {
                    is_already_part_of_free_org = true;
                    break;
                }
            }
        }
    }

    if org.is_ok() {
        let org = org.unwrap();
        if org.is_some() {
            let org = org.unwrap();
            if org.is_free_sub() && is_already_part_of_free_org {
                return false;
            }
        } else if is_already_part_of_free_org {
            return false;
        }
    } else if is_already_part_of_free_org {
        return false;
    }
    true
}

pub async fn get_org(org: &str) -> Option<Organization> {
    db::organization::get_org(org).await.ok()
}

#[cfg(test)]
mod tests {
    use infra::{db as infra_db, table as infra_table};

    use super::*;
    use crate::{common::meta::user::UserRequest, service::users};

    // TODO: move these tests to integration tests,
    // the below test case will fail as is_root_user()
    // will not work as watchers are not initialized
    #[tokio::test]
    #[ignore]
    async fn test_organization() {
        let org_id = "default";
        let user_id = "user-org-1@example.com";
        let init_user = "root@example.com";
        let pwd = "Complexpass#123";

        infra_db::create_table().await.unwrap();
        infra_table::create_user_tables().await.unwrap();
        check_and_create_org_without_ofga(org_id).await.unwrap();
        users::create_root_user_if_not_exists(
            org_id,
            UserRequest {
                email: init_user.to_string(),
                password: pwd.to_string(),
                role: crate::common::meta::user::UserOrgRole {
                    base_role: config::meta::user::UserRole::Root,
                    custom_role: None,
                },
                first_name: "root".to_owned(),
                last_name: "".to_owned(),
                is_external: false,
            },
        )
        .await
        .unwrap();

        let resp = users::post_user(
            org_id,
            UserRequest {
                email: user_id.to_string(),
                password: "pass".to_string(),
                role: crate::common::meta::user::UserOrgRole {
                    base_role: config::meta::user::UserRole::Admin,
                    custom_role: None,
                },
                first_name: "admin".to_owned(),
                last_name: "".to_owned(),
                is_external: false,
            },
            init_user,
        )
        .await;
        assert!(resp.is_ok());
        let resp = resp.unwrap();
        println!("Test organization resp: {:?}", resp.body());
        assert!(resp.status().is_success());

        let resp = get_passcode(Some(org_id), user_id).await.unwrap();
        let passcode = resp.passcode.clone();
        assert!(!resp.passcode.is_empty());

        let resp = update_passcode(Some(org_id), user_id).await.unwrap();
        assert_ne!(resp.passcode, passcode);
    }
}
