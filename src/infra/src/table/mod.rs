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

pub mod action_scripts;
pub mod alerts;
pub mod cipher;
pub mod dashboards;
pub mod destinations;
pub mod distinct_values;
#[allow(unused_imports)]
pub mod entity;
pub mod folders;
mod migration;
pub mod ratelimit;
pub mod search_job;
pub mod search_queue;
pub mod short_urls;
pub mod templates;
pub mod timed_annotation_panels;
pub mod timed_annotations;

pub async fn init() -> Result<(), anyhow::Error> {
    distinct_values::init().await?;
    short_urls::init().await?;
    Ok(())
}

pub async fn migrate() -> Result<(), anyhow::Error> {
    let locker = dist_lock::lock("/database/migration", 0).await?;
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    // This is a hack to fix the failing alerts migration
    // For postgres, we need to run the migration that populates the alerts table first.
    // Otherwise, the `m20250109_092400_recreate_tables_with_ksuids` migration will fail.
    let first_stage = get_alerts_populate_migration_index().await?;
    Migrator::up(client, Some(first_stage)).await?; // hack for failing alerts migration
    Migrator::up(client, None).await?;
    dist_lock::unlock(&locker).await?;
    Ok(())
}

/// Get the index of the migration that populates the alerts table.
/// This index is used as the first stage of the migration process.
async fn get_alerts_populate_migration_index() -> Result<u32, anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let migrations = Migrator::get_pending_migrations(client).await?;
    let mut index: u32 = 0;
    for (i, migration) in migrations.iter().enumerate() {
        if migration.name() == "m20241217_155000_populate_alerts_table" {
            index = i as u32 + 1;
            break;
        }
    }
    // If the migration is not found, it is already applied so return 0
    log::debug!(
        "Migration m20241217_155000_populate_alerts_table at step {} (0 means already applied)",
        index
    );
    Ok(index)
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
