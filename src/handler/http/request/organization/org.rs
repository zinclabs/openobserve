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

use std::{
    collections::{HashMap, HashSet},
    io::Error,
};

use actix_web::{get, http, post, put, web, HttpRequest, HttpResponse, Result};
#[cfg(feature = "cloud")]
use o2_enterprise::enterprise::cloud::org_usage;

#[cfg(feature = "cloud")]
use crate::common::meta::organization::OrganizationInvites;
use crate::{
    common::{
        meta::{
            http::HttpResponse as MetaHttpResponse,
            organization::{
                OrgDetails, OrgUser, Organization, OrganizationResponse, PasscodeResponse,
                RumIngestionResponse, THRESHOLD,
            },
        },
        utils::auth::{is_root_user, UserEmail},
    },
    service::organization::{self, get_passcode, get_rum_token, update_passcode, update_rum_token},
};

/// GetOrganizations
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "GetUserOrganizations",
    security(
        ("Authorization"= [])
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = OrganizationResponse),
    )
)]
#[get("/organizations")]
pub async fn organizations(user_email: UserEmail, req: HttpRequest) -> Result<HttpResponse, Error> {
    let user_id = user_email.user_id.as_str();
    let mut id = 0;
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();

    let mut orgs: Vec<OrgDetails> = vec![];
    let mut org_names = HashSet::new();
    let user_detail = OrgUser {
        first_name: "".to_string(),
        last_name: "".to_string(),
        email: user_id.to_string(),
    };

    let limit = query
        .get("limit")
        .unwrap_or(&"100".to_string())
        .parse::<i64>()
        .ok();
    let is_root_user = is_root_user(user_id);
    let all_orgs = if is_root_user {
        let Ok(records) = organization::list_all_orgs(limit).await else {
            return Ok(
                HttpResponse::InternalServerError().json(MetaHttpResponse::error(
                    http::StatusCode::INTERNAL_SERVER_ERROR.into(),
                    "Something went wrong".to_string(),
                )),
            );
        };
        records
    } else {
        let Ok(records) = organization::list_orgs_by_user(user_id).await else {
            return Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
                http::StatusCode::NOT_FOUND.into(),
                "Something went wrong".to_string(),
            )));
        };
        records
    };
    for org in all_orgs {
        id += 1;
        #[cfg(feature = "cloud")]
        let org_subscription = {
            if let Ok(Some(subscription)) = org_usage::get(org.identifier.as_str()).await {
                subscription.subscription_type as i32
            } else {
                0
            }
        };
        #[cfg(not(feature = "cloud"))]
        let org_subscription = 0;
        let org = OrgDetails {
            id,
            identifier: org.identifier.clone(),
            name: org.name,
            user_email: user_id.to_string(),
            ingest_threshold: THRESHOLD,
            search_threshold: THRESHOLD,
            org_type: org.org_type,
            user_obj: user_detail.clone(),
            plan: org_subscription,
        };
        if !org_names.contains(&org.identifier) {
            org_names.insert(org.identifier.clone());
            orgs.push(org)
        }
    }
    let org_response = OrganizationResponse { data: orgs };

    Ok(HttpResponse::Ok().json(org_response))
}

/// GetOrganizationSummary
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "GetOrganizationSummary",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = OrgSummary),
    )
)]
#[get("/{org_id}/summary")]
async fn org_summary(org_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let org_summary = organization::get_summary(&org).await;
    Ok(HttpResponse::Ok().json(org_summary))
}

/// GetIngestToken
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "GetOrganizationUserIngestToken",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = PasscodeResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/passcode")]
async fn get_user_passcode(
    user_email: UserEmail,
    org_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let user_id = user_email.user_id.as_str();
    let mut org_id = Some(org.as_str());
    if is_root_user(user_id) {
        org_id = None;
    }
    match get_passcode(org_id, user_id).await {
        Ok(passcode) => Ok(HttpResponse::Ok().json(PasscodeResponse { data: passcode })),
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}

/// UpdateIngestToken
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "UpdateOrganizationUserIngestToken",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = PasscodeResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/passcode")]
async fn update_user_passcode(
    user_email: UserEmail,
    org_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let user_id = user_email.user_id.as_str();
    let mut org_id = Some(org.as_str());
    if is_root_user(user_id) {
        org_id = None;
    }
    match update_passcode(org_id, user_id).await {
        Ok(passcode) => Ok(HttpResponse::Ok().json(PasscodeResponse { data: passcode })),
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}

/// GetRumIngestToken
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "GetOrganizationUserRumIngestToken",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = RumIngestionResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/rumtoken")]
async fn get_user_rumtoken(
    user_email: UserEmail,
    org_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let user_id = user_email.user_id.as_str();
    let mut org_id = Some(org.as_str());
    if is_root_user(user_id) {
        org_id = None;
    }
    match get_rum_token(org_id, user_id).await {
        Ok(rumtoken) => Ok(HttpResponse::Ok().json(RumIngestionResponse { data: rumtoken })),
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}

/// UpdateRumIngestToken
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "UpdateOrganizationUserRumIngestToken",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = RumIngestionResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[put("/{org_id}/rumtoken")]
async fn update_user_rumtoken(
    user_email: UserEmail,
    org_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let user_id = user_email.user_id.as_str();
    let mut org_id = Some(org.as_str());
    if is_root_user(user_id) {
        org_id = None;
    }
    match update_rum_token(org_id, user_id).await {
        Ok(rumtoken) => Ok(HttpResponse::Ok().json(RumIngestionResponse { data: rumtoken })),
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}

/// CreateRumIngestToken
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "CreateOrganizationUserRumIngestToken",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = RumIngestionResponse),
        (status = 404, description = "NotFound", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/rumtoken")]
async fn create_user_rumtoken(
    user_email: UserEmail,
    org_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let org = org_id.into_inner();
    let user_id = user_email.user_id.as_str();
    let mut org_id = Some(org.as_str());
    if is_root_user(user_id) {
        org_id = None;
    }
    match update_rum_token(org_id, user_id).await {
        Ok(rumtoken) => Ok(HttpResponse::Ok().json(RumIngestionResponse { data: rumtoken })),
        Err(e) => Ok(HttpResponse::NotFound().json(MetaHttpResponse::error(
            http::StatusCode::NOT_FOUND.into(),
            e.to_string(),
        ))),
    }
}

/// CreateOrganization
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "CreateOrganization",
    security(
        ("Authorization"= [])
    ),
    request_body(content = Organization, description = "Organization data", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = RumIngestionResponse),
    )
)]
#[post("/organizations")]
async fn create_org(
    user_email: UserEmail,
    org: web::Json<Organization>,
) -> Result<HttpResponse, Error> {
    let mut org = org.into_inner();

    let result = organization::create_org(&mut org, &user_email.user_id).await;
    match result {
        Ok(_) => Ok(HttpResponse::Ok().json(org)),
        Err(err) => Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            err.to_string(),
        ))),
    }
}

/// RenameOrganization
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "RenameOrganization",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization id"),
        ("new_name" = String, Path, description = "New organization name"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = Organization),
    )
)]
#[put("/{org_id}/rename/{new_name}")]
async fn rename_org(
    user_email: UserEmail,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    let (org, new_name) = path.into_inner();
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            "New name cannot be empty".to_string(),
        )));
    }

    let result = organization::rename_org(&org, new_name, &user_email.user_id).await;
    match result {
        Ok(org) => Ok(HttpResponse::Ok().json(org)),
        Err(err) => Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            err.to_string(),
        ))),
    }
}

/// InviteOrganizationMembers
#[cfg(feature = "cloud")]
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "GetOrganizationMemberInvites",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization id"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = OrganizationInviteUserRecord),
    )
)]
#[get("/{org_id}/invites")]
pub async fn get_org_invites(path: web::Path<String>) -> Result<HttpResponse, Error> {
    let org = path.into_inner();

    let result = organization::get_invitations_for_org(&org).await;
    match result {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(err) => Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            err.to_string(),
        ))),
    }
}

/// InviteOrganizationMembers
#[cfg(feature = "cloud")]
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "InviteOrganizationMembers",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization id"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = Organization),
    )
)]
#[post("/{org_id}/invites")]
pub async fn generate_org_invite(
    user_email: UserEmail,
    path: web::Path<String>,
    invites: web::Json<OrganizationInvites>,
) -> Result<HttpResponse, Error> {
    let org = path.into_inner();
    let invites = invites.into_inner();

    let result = organization::generate_invitation(&org, &user_email.user_id, invites).await;
    match result {
        Ok(org) => Ok(HttpResponse::Ok().json(org)),
        Err(err) => Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            err.to_string(),
        ))),
    }
}

/// AcceptOrganizationInvite
#[cfg(feature = "cloud")]
#[utoipa::path(
    context_path = "/api",
    tag = "Organizations",
    operation_id = "AcceptOrganizationInvite",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization id"),
        ("invite_token" = String, Path, description = "The token sent to the user"),
      ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = Organization),
    )
)]
#[put("/{org_id}/member_subscription/{invite_token}")]
async fn accept_org_invite(
    user_email: UserEmail,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    let (_org, invite_token) = path.into_inner();

    let result = organization::accept_invitation(&user_email.user_id, &invite_token).await;
    match result {
        Ok(_) => Ok(MetaHttpResponse::ok("Invitation accepted successfully")),
        Err(err) => Ok(HttpResponse::BadRequest().json(MetaHttpResponse::error(
            http::StatusCode::BAD_REQUEST.into(),
            err.to_string(),
        ))),
    }
}
