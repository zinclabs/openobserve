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

use config::meta::user::{DBUser, User, UserOrg, UserRole};
#[cfg(feature = "cloud")]
use o2_enterprise::enterprise::cloud::OrgInviteStatus;
use serde::{Deserialize, Serialize};
#[cfg(feature = "enterprise")]
use strum::IntoEnumIterator;
use utoipa::ToSchema;

use super::organization::OrgRoleMapping;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserRequest {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub password: String,
    #[serde(skip_serializing)]
    pub role: UserOrgRole,
    /// Is the user created via ldap flow.
    #[serde(default)]
    pub is_external: bool,
}

impl UserRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn to_new_dbuser(
        &self,
        password: String,
        salt: String,
        org: String,
        token: String,
        rum_token: String,
        is_external: bool,
        password_ext: String,
    ) -> DBUser {
        DBUser {
            email: self.email.clone(),
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            password,
            salt,
            organizations: vec![UserOrg {
                name: org,
                token,
                rum_token: Some(rum_token),
                role: self.role.base_role.clone(),
            }],
            is_external,
            password_ext: Some(password_ext),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PostUserRequest {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub password: String,
    #[serde(skip_serializing, flatten)]
    pub role: UserRoleRequest,
    /// Is the user created via ldap flow.
    #[serde(default)]
    pub is_external: bool,
}

impl From<&PostUserRequest> for UserRequest {
    fn from(user: &PostUserRequest) -> Self {
        UserRequest {
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            password: user.password.clone(),
            role: UserOrgRole::from(&user.role),
            is_external: user.is_external,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserOrgRole {
    #[serde(rename = "role")]
    pub base_role: UserRole,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub custom_role: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, Eq, PartialEq, Default)]
pub struct UpdateUser {
    #[serde(default)]
    pub change_password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub role: Option<UserRoleRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

pub fn get_default_user_org() -> UserOrg {
    UserOrg {
        name: "".to_string(),
        token: "".to_string(),
        rum_token: None,
        role: get_default_user_role(),
    }
}

#[cfg(feature = "enterprise")]
pub fn get_default_user_role() -> UserRole {
    let mut role = UserRole::Admin;
    use o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config;
    if get_o2_config().openfga.enabled {
        role = get_o2_config().dex.default_role.parse().unwrap();
    }
    role
}

#[cfg(not(feature = "enterprise"))]
pub fn get_default_user_role() -> UserRole {
    UserRole::Admin
}

#[cfg(feature = "enterprise")]
pub fn get_roles() -> Vec<UserRole> {
    UserRole::iter().collect()
}

#[cfg(not(feature = "enterprise"))]
pub fn get_roles() -> Vec<UserRole> {
    vec![UserRole::Admin, UserRole::Root, UserRole::ServiceAccount]
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub role: String,
    #[serde(default)]
    pub is_external: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orgs: Option<Vec<OrgRoleMapping>>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserList {
    pub data: Vec<UserResponse>,
}

#[cfg(feature = "cloud")]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub enum InviteStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "expired")]
    Expired,
}

#[cfg(feature = "cloud")]
impl From<&OrgInviteStatus> for InviteStatus {
    fn from(status: &OrgInviteStatus) -> Self {
        match status {
            OrgInviteStatus::Pending => InviteStatus::Pending,
            OrgInviteStatus::Accepted => InviteStatus::Accepted,
            OrgInviteStatus::Rejected => InviteStatus::Rejected,
            OrgInviteStatus::Expired => InviteStatus::Expired,
        }
    }
}

#[cfg(feature = "cloud")]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserInvite {
    pub org_id: String,
    pub token: String,
    pub role: String,
    pub status: InviteStatus,
    pub expires_at: i64,
}

#[cfg(feature = "cloud")]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserInviteList {
    pub data: Vec<UserInvite>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SignInUser {
    pub name: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct SignInResponse {
    pub status: bool,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TokenValidationResponse {
    pub is_valid: bool,
    pub user_email: String,
    pub user_name: String,
    pub family_name: String,
    pub given_name: String,
    pub is_internal_user: bool,
    pub user_role: Option<UserRole>,
}
pub struct TokenValidationResponseBuilder {
    pub response: TokenValidationResponse,
}

/// Builder for creating a `TokenValidationResponse` from a `DBUser`.
impl TokenValidationResponseBuilder {
    /// Creates a new `TokenValidationResponseBuilder` from a `DBUser`.
    ///
    /// # Arguments
    ///
    /// * `user` - The `DBUser` object used to build the `TokenValidationResponse`.
    ///
    /// # Returns
    ///
    /// A `TokenValidationResponseBuilder` object.
    pub fn from_db_user(user: &DBUser) -> TokenValidationResponseBuilder {
        Self {
            response: TokenValidationResponse {
                is_valid: true,
                user_email: user.email.clone(),
                is_internal_user: !user.is_external,
                user_role: None,
                user_name: user.first_name.clone(),
                given_name: user.first_name.clone(),
                family_name: user.last_name.clone(),
            },
        }
    }

    /// Creates a new `TokenValidationResponseBuilder` from a `User`.
    ///
    /// Arguments
    ///
    /// * `user` - The `User` object used to build the `TokenValidationResponse`.
    pub fn from_user(user: &User) -> TokenValidationResponseBuilder {
        Self {
            response: TokenValidationResponse {
                is_valid: true,
                user_email: user.email.clone(),
                is_internal_user: !user.is_external,
                user_role: Some(user.role.clone()),
                user_name: user.first_name.clone(),
                given_name: user.first_name.clone(),
                family_name: user.last_name.clone(),
            },
        }
    }

    pub fn new() -> TokenValidationResponseBuilder {
        Self {
            response: TokenValidationResponse::default(),
        }
    }

    pub fn is_valid(mut self, is_valid: bool) -> Self {
        self.response.is_valid = is_valid;
        self
    }

    pub fn user_email(mut self, user_email: String) -> Self {
        self.response.user_email = user_email;
        self
    }

    pub fn user_name(mut self, user_name: String) -> Self {
        self.response.user_name = user_name;
        self
    }

    pub fn family_name(mut self, family_name: String) -> Self {
        self.response.family_name = family_name;
        self
    }

    pub fn given_name(mut self, given_name: String) -> Self {
        self.response.given_name = given_name;
        self
    }

    pub fn is_internal_user(mut self, is_internal_user: bool) -> Self {
        self.response.is_internal_user = is_internal_user;
        self
    }

    pub fn user_role(mut self, user_role: Option<UserRole>) -> Self {
        self.response.user_role = user_role;
        self
    }

    pub fn build(self) -> TokenValidationResponse {
        TokenValidationResponse {
            is_valid: self.response.is_valid,
            user_email: self.response.user_email,
            user_name: self.response.user_name,
            family_name: self.response.family_name,
            given_name: self.response.given_name,
            is_internal_user: self.response.is_internal_user,
            user_role: self.response.user_role,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleOrg {
    pub role: UserRole,
    pub org: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_role: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct UserGroup {
    pub name: String,
    pub users: Option<std::collections::HashSet<String>>,
    pub roles: Option<std::collections::HashSet<String>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct UserGroupRequest {
    pub add_users: Option<std::collections::HashSet<String>>,
    pub remove_users: Option<std::collections::HashSet<String>>,
    pub add_roles: Option<std::collections::HashSet<String>>,
    pub remove_roles: Option<std::collections::HashSet<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize, ToSchema)]
pub struct UserRoleRequest {
    pub role: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "custom_role"
    )]
    pub custom: Option<Vec<String>>,
}

impl From<&UserRoleRequest> for UserOrgRole {
    fn from(role: &UserRoleRequest) -> Self {
        let mut standard_role = get_default_user_role();
        let mut custom_role = role.custom.clone();
        let mut is_role_name_standard = false;
        for user_role in get_roles() {
            if user_role.to_string().eq(&role.role) {
                standard_role = user_role;
                is_role_name_standard = true;
                break;
            }
        }
        if !is_role_name_standard && custom_role.is_none() {
            custom_role = Some(vec![role.role.clone()]);
        }
        UserOrgRole {
            base_role: standard_role,
            custom_role,
        }
    }
}

#[cfg(feature = "enterprise")]
pub fn is_standard_role(role: &str) -> bool {
    for user_role in UserRole::iter() {
        if user_role.to_string().eq_ignore_ascii_case(role) {
            return true;
        }
    }
    false
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct RolesResponse {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct AuthTokensExt {
    pub auth_ext: String,
    pub refresh_token: String,
    pub request_time: i64,
    pub expires_in: i64,
}
