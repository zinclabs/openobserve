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

use config::{ider, utils::json};
use hashbrown::HashSet;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let txn = db.begin().await?;
        // Migrate pages of 100 records at a time to avoid loading too many
        // records into memory.
        // txn.execute()
        let mut meta_pages = meta::Entity::find()
            .filter(meta::Column::Module.eq("user"))
            .order_by_asc(meta::Column::Id)
            .paginate(&txn, 100);
        let mut orgs_set = HashSet::new();
        let mut org_pages = organization::Entity::find().paginate(&txn, 100);
        while let Some(orgs) = org_pages.fetch_and_next().await? {
            log::debug!("Orgs found: {:?}", orgs);
            for org in orgs {
                orgs_set.insert(org.identifier.clone());
            }
        }

        while let Some(metas) = meta_pages.fetch_and_next().await? {
            let mut users = vec![];
            let mut org_users = vec![];
            for m in metas {
                let json_user: meta::DBUser =
                    json::from_str(&m.value).map_err(|e| DbErr::Migration(e.to_string()))?;
                log::debug!("json_user: {:#?}", json_user);
                for org in json_user.organizations.iter() {
                    if !orgs_set.contains(&org.name) {
                        log::debug!("Inserting org: {:#?}", org.name);
                        let now = chrono::Utc::now().timestamp_micros();
                        organization::Entity::insert(organization::ActiveModel {
                            identifier: Set(org.name.clone()),
                            org_name: Set(org.name.clone()),
                            org_type: Set(1),
                            created_at: Set(now),
                            updated_at: Set(now),
                        })
                        .exec(&txn)
                        .await?;
                        orgs_set.insert(org.name.clone());
                    }
                }
                let all_org_users = json_user.get_all_users();
                let is_root_user = if all_org_users.is_empty() {
                    false
                } else {
                    all_org_users[0].role.eq(&meta::UserRole::Root)
                };

                let now = chrono::Utc::now().timestamp_micros() as u64;
                users.push(users::ActiveModel {
                    email: Set(json_user.email.clone()),
                    first_name: Set(json_user.first_name.clone()),
                    last_name: Set(json_user.last_name.clone()),
                    password: Set(json_user.password.clone()),
                    salt: Set(json_user.salt.clone()),
                    is_root: Set(is_root_user),
                    password_ext: Set(json_user.password_ext.clone()),
                    user_type: Set(json_user.is_external as i16),
                    created_at: Set(now),
                    updated_at: Set(now),
                    id: Set(ider::uuid()),
                });

                for org_user in all_org_users {
                    org_users.push(org_users::ActiveModel {
                        org_id: Set(org_user.org),
                        email: Set(org_user.email),
                        role: Set(org_user.role.into()),
                        token: Set(org_user.token),
                        rum_token: Set(org_user.rum_token),
                        created_at: Set(now),
                        updated_at: Set(now),
                        id: Set(ider::uuid()),
                    });
                }
            }
            users::Entity::insert_many(users).exec(&txn).await?;
            org_users::Entity::insert_many(org_users).exec(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        org_users::Entity::delete_many().exec(db).await?;
        users::Entity::delete_many().exec(db).await?;
        Ok(())
    }
}

/// Representation of the meta table at the time this migration executes.
mod meta {
    use std::{fmt, str::FromStr};

    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "meta")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub module: String,
        pub key1: String,
        pub key2: String,
        pub start_dt: i64,
        #[sea_orm(column_type = "Text")]
        pub value: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Clone, Debug, Serialize, Deserialize)]
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

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct UserOrg {
        pub name: String,
        #[serde(default)]
        pub token: String,
        #[serde(default)]
        pub rum_token: Option<String>,
        pub role: UserRole,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

    #[derive(Clone, Debug, Serialize, Deserialize)]
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
}

/// Representation of the users table at the time this migration executes.
mod users {
    use sea_orm::entity::prelude::*;

    // define the organizations table
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        #[sea_orm(column_type = "String(StringLen::N(100))")]
        pub email: String,
        #[sea_orm(column_type = "String(StringLen::N(100))")]
        pub first_name: String,
        #[sea_orm(column_type = "String(StringLen::N(100))")]
        pub last_name: String,
        #[sea_orm(column_type = "Text")]
        pub password: String,
        pub salt: String,
        pub is_root: bool,
        pub password_ext: Option<String>,
        pub user_type: i16,
        pub created_at: u64,
        pub updated_at: u64,
    }

    #[derive(Copy, Clone, Debug, EnumIter)]
    pub enum Relation {}

    impl RelationTrait for Relation {
        fn def(&self) -> RelationDef {
            panic!("No relations defined")
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Representation of the org_users table at the time this migration executes.
mod org_users {
    use sea_orm::entity::prelude::*;

    // define the organizations table
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "org_users")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        #[sea_orm(column_type = "String(StringLen::N(256))")]
        pub org_id: String,
        #[sea_orm(column_type = "String(StringLen::N(100))")]
        pub email: String,
        pub role: i16,
        pub token: String,
        pub rum_token: Option<String>,
        pub created_at: u64,
        pub updated_at: u64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

mod organization {
    //! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.0

    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "organizations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub identifier: String,
        pub org_name: String,
        pub org_type: i16,
        pub created_at: i64,
        pub updated_at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
