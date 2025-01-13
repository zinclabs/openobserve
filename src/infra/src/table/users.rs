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
    meta::user::{DBUser, UserRole, UserType},
};
use sea_orm::{
    entity::prelude::*, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Schema, Set,
};
use serde::{Deserialize, Serialize};

use super::{
    entity::users::{ActiveModel, Column, Entity, Model},
    get_lock,
};
use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    errors::{self, DbError, Error},
};

impl From<Model> for UserRecord {
    fn from(model: Model) -> Self {
        Self {
            email: model.email,
            first_name: model.first_name,
            last_name: model.last_name,
            password: model.password,
            salt: model.salt,
            is_root: model.is_root,
            password_ext: model.password_ext,
            user_type: model.user_type.into(),
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserRecord {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub password: String,
    pub salt: String,
    pub is_root: bool,
    #[serde(default)]
    pub password_ext: Option<String>,
    pub user_type: UserType,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserUpdate {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub password: String,
    pub password_ext: Option<String>,
}

impl From<&DBUser> for UserRecord {
    fn from(user: &DBUser) -> Self {
        let is_root = user
            .organizations
            .iter()
            .any(|org| org.role.eq(&UserRole::Root));
        let email = user.email.to_lowercase();
        Self {
            email,
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            password: user.password.clone(),
            salt: user.salt.clone(),
            is_root,
            password_ext: user.password_ext.clone(),
            user_type: if user.is_external {
                UserType::External
            } else {
                UserType::Internal
            },
            created_at: 0,
            updated_at: 0,
        }
    }
}

impl From<&UserRecord> for DBUser {
    fn from(user: &UserRecord) -> Self {
        DBUser {
            email: user.email.clone(),
            password: user.password.clone(),
            salt: user.salt.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            is_external: user.user_type.is_external(),
            organizations: vec![],
            password_ext: user.password_ext.clone(),
        }
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

pub async fn add(user: UserRecord) -> Result<(), errors::Error> {
    let now = chrono::Utc::now().timestamp_micros();
    let record = ActiveModel {
        email: Set(user.email),
        first_name: Set(user.first_name.to_string()),
        last_name: Set(user.last_name.to_string()),
        password: Set(user.password.to_string()),
        salt: Set(user.salt.to_string()),
        is_root: Set(user.is_root),
        password_ext: Set(user.password_ext),
        user_type: Set(user.user_type.into()),
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

pub async fn update(
    email: &str,
    first_name: &str,
    last_name: &str,
    password: &str,
    password_ext: Option<String>,
) -> Result<u64, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let result = Entity::update_many()
        .col_expr(Column::FirstName, Expr::value(first_name))
        .col_expr(Column::LastName, Expr::value(last_name))
        .col_expr(Column::Password, Expr::value(password))
        .col_expr(Column::PasswordExt, Expr::value(password_ext))
        .col_expr(
            Column::UpdatedAt,
            Expr::value(chrono::Utc::now().timestamp_micros()),
        )
        .filter(Column::Email.eq(email))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(result.rows_affected)
}

pub async fn remove(email: &str) -> Result<(), errors::Error> {
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

pub async fn get(email: &str) -> Result<UserRecord, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .filter(Column::Email.eq(email))
        .one(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?
        .ok_or_else(|| Error::DbError(DbError::SeaORMError("User not found".to_string())))?;

    Ok(UserRecord::from(record))
}

pub async fn get_root_user() -> Result<UserRecord, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let record = Entity::find()
        .filter(Column::IsRoot.eq(true))
        .one(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?
        .ok_or_else(|| Error::DbError(DbError::SeaORMError("Root user not found".to_string())))?;

    Ok(UserRecord::from(record))
}

pub async fn list(limit: Option<i64>) -> Result<Vec<UserRecord>, errors::Error> {
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
        .map(UserRecord::from)
        .collect();

    Ok(records)
}

pub async fn len() -> usize {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let len = Entity::find().count(client).await;

    match len {
        Ok(len) => len as usize,
        Err(e) => {
            log::error!("users len error: {}", e);
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

pub async fn batch_remove(emails: Vec<String>) -> Result<(), errors::Error> {
    // make sure only one client is writing to the database(only for sqlite)
    let _lock = get_lock().await;

    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Entity::delete_many()
        .filter(Column::Email.is_in(emails))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}
