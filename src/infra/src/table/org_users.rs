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

use core::convert::Into;

use config::{
    ider,
    meta::{
        organization::OrganizationType,
        user::{UserRole, UserType},
    },
};
use sea_orm::{
    entity::prelude::*, ColumnTrait, EntityTrait, FromQueryResult, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Schema, Set,
};
use serde::{Deserialize, Serialize};

use super::{
    entity::{
        org_users::{ActiveModel, Column, Entity, Model},
        organizations, users,
    },
    get_lock,
};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors::{self, DbError, Error},
};

#[derive(Debug, Clone)]
pub struct OrgUserRecord {
    pub email: String,
    pub org_id: String,
    pub role: UserRole,
    pub token: String,
    pub rum_token: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrgUserPut {
    pub email: String,
    pub org_id: String,
    pub role: UserRole,
    pub token: String,
    pub rum_token: Option<String>,
}

impl OrgUserRecord {
    pub fn new(
        org_id: &str,
        user_email: &str,
        role: UserRole,
        token: &str,
        rum_token: Option<String>,
    ) -> Self {
        Self {
            org_id: org_id.to_string(),
            email: user_email.to_string(),
            role,
            token: token.to_string(),
            rum_token,
            created_at: chrono::Utc::now().timestamp_micros(),
        }
    }
}

impl From<Model> for OrgUserRecord {
    fn from(model: Model) -> Self {
        Self {
            email: model.email,
            org_id: model.org_id,
            role: model.role.into(),
            token: model.token,
            rum_token: model.rum_token,
            created_at: model.created_at,
        }
    }
}

#[derive(Debug)]
pub struct UserOrgExpandedRecord {
    pub email: String,
    pub org_id: String,
    pub role: UserRole,
    pub token: String,
    pub rum_token: Option<String>,
    pub created_at: i64,
    pub org_name: String,
    pub org_type: OrganizationType,
}

impl FromQueryResult for UserOrgExpandedRecord {
    fn from_query_result(result: &QueryResult, pre: &str) -> Result<Self, DbErr> {
        let email = result.try_get(pre, "email")?;
        let org_id = result.try_get(pre, "org_id")?;
        let role: i16 = result.try_get(pre, "role")?;
        let token = result.try_get(pre, "token")?;
        let rum_token = result.try_get(pre, "rum_token")?;
        let created_at = result.try_get(pre, "created_at")?;
        let org_name = result.try_get(pre, "org_name")?;
        let org_type: i16 = result.try_get(pre, "org_type")?;

        Ok(Self {
            email,
            org_id,
            role: role.into(),
            token,
            rum_token,
            created_at,
            org_name,
            org_type: org_type.into(),
        })
    }
}

#[derive(Debug)]
pub struct OrgUserExpandedRecord {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub user_type: UserType,
    pub password: String,
    pub salt: String,
    pub password_ext: Option<String>,
    pub org_id: String,
    pub role: UserRole,
    pub token: String,
    pub rum_token: Option<String>,
    pub created_at: i64,
}

impl FromQueryResult for OrgUserExpandedRecord {
    fn from_query_result(res: &QueryResult, pre: &str) -> Result<Self, DbErr> {
        let email = res.try_get(pre, "email")?;
        let first_name = res.try_get(pre, "first_name")?;
        let last_name = res.try_get(pre, "last_name")?;
        let user_type: i16 = res.try_get(pre, "user_type")?;
        let password = res.try_get(pre, "password")?;
        let salt = res.try_get(pre, "salt")?;
        let password_ext = res.try_get(pre, "password_ext")?;
        let org_id = res.try_get(pre, "org_id")?;
        let role: i16 = res.try_get(pre, "role")?;
        let token = res.try_get(pre, "token")?;
        let rum_token = res.try_get(pre, "rum_token")?;
        let created_at = res.try_get(pre, "created_at")?;

        Ok(Self {
            email,
            first_name,
            last_name,
            user_type: user_type.into(),
            password,
            salt,
            password_ext,
            org_id,
            role: role.into(),
            token,
            rum_token,
            created_at,
        })
    }
}

pub async fn create_table() -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let builder = client.get_database_backend();

    let schema = Schema::new(builder);
    let create_table_stmt = schema
        .create_table_from_entity(Entity)
        .if_not_exists()
        .take();

    client
        .execute(builder.build(&create_table_stmt))
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn add(
    org_id: &str,
    user_email: &str,
    role: UserRole,
    token: &str,
    rum_token: Option<String>,
) -> Result<(), errors::Error> {
    let now = chrono::Utc::now().timestamp_micros();
    let role: i16 = role.into();
    let record = ActiveModel {
        org_id: Set(org_id.to_string()),
        email: Set(user_email.to_string()),
        role: Set(role),
        token: Set(token.to_string()),
        rum_token: Set(rum_token),
        created_at: Set(now),
        updated_at: Set(now),
        id: Set(ider::uuid()),
    };

    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::insert(record)
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn remove(org_id: &str, email: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::OrgId.eq(org_id))
        .filter(Column::Email.eq(email))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn remove_by_user(email: &str) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::Email.eq(email))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn update(
    org_id: &str,
    email: &str,
    role: UserRole,
    token: &str,
    rum_token: Option<String>,
) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    // There can be only one record with one org_id and email.
    // Hence the below updates only one record.
    Entity::update_many()
        .col_expr(Column::Role, Expr::value(role as i16))
        .col_expr(Column::Token, Expr::value(token.to_string()))
        .col_expr(Column::RumToken, Expr::value(rum_token))
        .col_expr(
            Column::UpdatedAt,
            Expr::value(chrono::Utc::now().timestamp_micros()),
        )
        .filter(Column::OrgId.eq(org_id))
        .filter(Column::Email.eq(email))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn update_rum_token(
    org_id: &str,
    email: &str,
    rum_token: &str,
) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    // There can be only one record with one org_id and email.
    // Hence the below updates only one record.
    Entity::update_many()
        .col_expr(Column::RumToken, Expr::value(Some(rum_token.to_string())))
        .col_expr(
            Column::UpdatedAt,
            Expr::value(chrono::Utc::now().timestamp_micros()),
        )
        .filter(Column::OrgId.eq(org_id))
        .filter(Column::Email.eq(email))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn update_token(org_id: &str, email: &str, token: &str) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    // There can be only one record with one org_id and email.
    // Hence the below updates only one record.
    Entity::update_many()
        .col_expr(Column::Token, Expr::value(token.to_string()))
        .col_expr(
            Column::UpdatedAt,
            Expr::value(chrono::Utc::now().timestamp_micros()),
        )
        .filter(Column::OrgId.eq(org_id))
        .filter(Column::Email.eq(email))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn get(org_id: &str, email: &str) -> Result<OrgUserRecord, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .filter(Column::OrgId.eq(org_id))
        .filter(Column::Email.eq(email))
        .one(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?
        .ok_or_else(|| Error::DbError(DbError::SeaORMError("User not found".to_string())))?;

    Ok(OrgUserRecord::from(record))
}

pub async fn get_expanded_user_org(
    org_id: &str,
    email: &str,
) -> Result<OrgUserExpandedRecord, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .filter(Column::OrgId.eq(org_id))
        .filter(Column::Email.eq(email))
        .inner_join(users::Entity)
        .select_only()
        .column(users::Column::Email)
        .column(users::Column::FirstName)
        .column(users::Column::LastName)
        .column(users::Column::UserType)
        .column(users::Column::Password)
        .column(users::Column::Salt)
        .column(users::Column::PasswordExt)
        .column(Column::OrgId)
        .column(Column::Role)
        .column(Column::Token)
        .column(Column::RumToken)
        .column(Column::CreatedAt)
        .into_model::<OrgUserExpandedRecord>()
        .one(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?
        .ok_or_else(|| {
            Error::DbError(DbError::SeaORMError(
                "Organization user not found".to_string(),
            ))
        })?;

    Ok(record)
}

pub async fn get_user_by_rum_token(
    org_id: &str,
    rum_token: &str,
) -> Result<OrgUserExpandedRecord, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .filter(Column::RumToken.eq(rum_token))
        .filter(Column::OrgId.eq(org_id))
        .inner_join(users::Entity)
        .select_only()
        .column(users::Column::Email)
        .column(users::Column::FirstName)
        .column(users::Column::LastName)
        .column(users::Column::UserType)
        .column(users::Column::Password)
        .column(users::Column::Salt)
        .column(users::Column::PasswordExt)
        .column(Column::OrgId)
        .column(Column::Role)
        .column(Column::Token)
        .column(Column::RumToken)
        .column(Column::CreatedAt)
        .into_model::<OrgUserExpandedRecord>()
        .one(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?
        .ok_or_else(|| {
            Error::DbError(DbError::SeaORMError(
                "Organization user not found".to_string(),
            ))
        })?;
    Ok(record)
}

pub async fn list_users_by_org(org_id: &str) -> Result<Vec<OrgUserRecord>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let records = Entity::find()
        .filter(Column::OrgId.eq(org_id))
        .all(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?
        .into_iter()
        .map(OrgUserRecord::from)
        .collect();

    Ok(records)
}

pub async fn list_orgs_by_user(email: &str) -> Result<Vec<UserOrgExpandedRecord>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let records = Entity::find()
        .filter(Column::Email.eq(email))
        .order_by(Column::CreatedAt, Order::Desc)
        .inner_join(super::entity::organizations::Entity)
        .select_only()
        .column(Column::Email)
        .column(Column::OrgId)
        .column(Column::Role)
        .column(Column::Token)
        .column(Column::RumToken)
        .column(Column::CreatedAt)
        .column(organizations::Column::OrgName)
        .column(organizations::Column::OrgType)
        .into_model::<UserOrgExpandedRecord>()
        .all(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(records)
}

pub async fn list(limit: Option<i64>) -> Result<Vec<OrgUserRecord>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let mut res = Entity::find().order_by(Column::CreatedAt, Order::Desc);
    if let Some(limit) = limit {
        res = res.limit(limit as u64);
    }
    let records = res
        .all(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?
        .into_iter()
        .map(OrgUserRecord::from)
        .collect();

    Ok(records)
}

pub async fn len() -> usize {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let len = Entity::find().count(client).await;

    match len {
        Ok(len) => len as usize,
        Err(e) => {
            log::error!("org_users len error: {}", e);
            0
        }
    }
}

pub async fn clear() -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub async fn is_empty() -> bool {
    len().await == 0
}
