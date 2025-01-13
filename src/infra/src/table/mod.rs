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

use config::get_config;
use migration::Migrator;
use sea_orm_migration::MigratorTrait;

use crate::{
    db::{connect_to_orm, sqlite::CLIENT_RW, ORM_CLIENT, SQLITE_STORE},
    dist_lock,
};

pub mod alerts;
pub mod dashboards;
pub mod distinct_values;
pub mod enrichment_table_jobs;
#[allow(unused_imports)]
pub mod entity;
pub mod folders;
mod migration;
pub mod search_job;
pub mod search_queue;
pub mod short_urls;

pub async fn init() -> Result<(), anyhow::Error> {
    distinct_values::init().await?;
    short_urls::init().await?;
    enrichment_table_jobs::init().await?;
    Ok(())
}

pub async fn migrate() -> Result<(), anyhow::Error> {
    let locker = dist_lock::lock("/database/migration", 0).await?;
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Migrator::up(client, None).await?;
    dist_lock::unlock(&locker).await?;
    Ok(())
}

pub async fn down(steps: Option<u32>) -> Result<(), anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    Migrator::down(client, steps).await?;
    Ok(())
}

/// Acquires a lock on the SQLite client if SQLite is configured as the meta store.
///
/// # Returns
/// - `Some(MutexGuard)` if SQLite is configured
/// - `None` if a different store is configured
pub async fn get_lock() -> Option<tokio::sync::MutexGuard<'static, sqlx::Pool<sqlx::Sqlite>>> {
    if get_config()
        .common
        .meta_store
        .eq_ignore_ascii_case(SQLITE_STORE)
    {
        Some(CLIENT_RW.lock().await)
    } else {
        None
    }
}

#[macro_export]
macro_rules! orm_err {
    ($e:expr) => {
        Err($crate::errors::Error::DbError(
            $crate::errors::DbError::SeaORMError($e.to_string()),
        ))
    };
}
