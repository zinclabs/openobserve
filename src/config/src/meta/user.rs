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

use core::clone::Clone;
use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use strum::EnumIter;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DBUser {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub password: String,
    #[serde(default)]
    pub salt: String,
    pub organizations: Vec<UserOrg>,
    #[serde(default)]
    pub is_external: bool,
    pub password_ext: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UserOrg {
    pub name: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub rum_token: Option<String>,
    pub role: UserRole,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, ToSchema, EnumIter)]
pub enum UserRole {
    #[serde(rename = "root")]
    Root = 0,
    #[serde(rename = "admin")]
    Admin = 1,
    #[serde(rename = "editor")]
    Editor = 2,
    #[serde(rename = "viewer")] // read only user
    Viewer = 3,
    #[serde(rename = "user")] // No access only login user
    User = 4,
    #[serde(rename = "service_account")]
    ServiceAccount = 5,
}

impl From<UserRole> for i16 {
    fn from(role: UserRole) -> i16 {
        match role {
            UserRole::Root => 0,
            UserRole::Admin => 1,
            UserRole::Editor => 2,
            UserRole::Viewer => 3,
            UserRole::User => 4,
            UserRole::ServiceAccount => 5,
        }
    }
}

impl From<i16> for UserRole {
    fn from(role: i16) -> Self {
        match role {
            0 => UserRole::Root,
            1 => UserRole::Admin,
            2 => UserRole::Editor,
            3 => UserRole::Viewer,
            4 => UserRole::User,
            5 => UserRole::ServiceAccount,
            _ => UserRole::Admin,
        }
    }
}

impl PartialOrd for UserRole {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let self_val = self.clone() as i16;
        let other_val = other.clone() as i16;
        Some(other_val.cmp(&self_val))
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Root => write!(f, "root"),
            UserRole::Viewer => write!(f, "viewer"),
            UserRole::Editor => write!(f, "editor"),
            UserRole::User => write!(f, "user"),
            UserRole::ServiceAccount => write!(f, "service_account"),
        }
    }
}

impl UserRole {
    pub fn get_label(&self) -> String {
        match self {
            UserRole::Admin => "Admin".to_string(),
            UserRole::Root => "Root".to_string(),
            UserRole::Viewer => "Viewer".to_string(),
            UserRole::Editor => "Editor".to_string(),
            UserRole::User => "User".to_string(),
            UserRole::ServiceAccount => "Service Account".to_string(),
        }
    }
}

// Implementing FromStr for UserRole
impl FromStr for UserRole {
    type Err = ();

    fn from_str(input: &str) -> Result<UserRole, Self::Err> {
        match input {
            "admin" => Ok(UserRole::Admin),
            "root" => Ok(UserRole::Root),
            "viewer" => Ok(UserRole::Viewer),
            "editor" => Ok(UserRole::Editor),
            "user" => Ok(UserRole::User),
            "service_account" => Ok(UserRole::ServiceAccount),
            _ => Ok(UserRole::Admin),
        }
    }
}

impl DBUser {
    pub fn get_user(&self, org_id: String) -> Option<User> {
        if self.organizations.is_empty() {
            return None;
        }

        let mut local = self.clone();
        local.organizations.retain(|org| org.name.eq(&org_id));
        if local.organizations.is_empty() {
            return None;
        }

        let org = local.organizations.first().unwrap();
        Some(User {
            email: local.email,
            first_name: local.first_name,
            last_name: local.last_name,
            password: local.password,
            role: org.role.clone(),
            org: org.name.clone(),
            token: org.token.clone(),
            rum_token: org.rum_token.clone(),
            salt: local.salt,
            is_external: self.is_external,
            password_ext: self.password_ext.clone(),
        })
    }

    pub fn get_all_users(&self) -> Vec<User> {
        let mut ret_val = vec![];
        if self.organizations.is_empty() {
            ret_val
        } else {
            for org in self.organizations.clone() {
                ret_val.push(User {
                    email: self.email.clone(),
                    first_name: self.first_name.clone(),
                    last_name: self.last_name.clone(),
                    password: self.password.clone(),
                    role: org.role,
                    org: org.name,
                    token: org.token,
                    rum_token: org.rum_token,
                    salt: self.salt.clone(),
                    is_external: self.is_external,
                    password_ext: self.password_ext.clone(),
                })
            }
            ret_val
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct User {
    pub email: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    pub password: String,
    #[serde(default)]
    pub salt: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub rum_token: Option<String>,
    pub role: UserRole,
    pub org: String,
    /// Is the user authenticated and created via LDAP
    pub is_external: bool,
    pub password_ext: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserType {
    Internal,
    /// Is the user authenticated and created via LDAP
    External,
}

impl From<i16> for UserType {
    fn from(user_type: i16) -> UserType {
        match user_type {
            0 => UserType::Internal,
            1 => UserType::External,
            _ => UserType::Internal,
        }
    }
}

impl From<UserType> for i16 {
    fn from(user_type: UserType) -> i16 {
        match user_type {
            UserType::Internal => 0,
            UserType::External => 1,
        }
    }
}

impl UserType {
    pub fn is_external(&self) -> bool {
        match self {
            UserType::Internal => false,
            UserType::External => true,
        }
    }
}
