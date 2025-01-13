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

//! This migration recreates the `folders`, `dashboards`, and `alerts` tables.
//!
//! Rather than using auto-incrementing integer indices, as they were before, the
//! `folders` and `dashboards` tables are recrated using KSUIDs as their primary
//! keys. Additionally, since the `dashboards` and `alerts` tables both had a
//! foreign key reference on the `folders` table's auto-incrementing integer primary
//! key, each of those folders is recreated using a KSUID as the foreign key on
//! the `folders` table.instead of using auto-incremented integers as primary keys
//!
//! The migration strategy to implement these changes follows these steps:
//! 1. Rename the legacy `folders`, `dashboards`, and `alerts` tables to `legacy_folders`,
//!    `legacy_dashboards`, and `legacy_alerts` so that later we can create new tables using these
//!    original names.
//! 2. Add a temporary `ksuid` column to the `legacy_folders` table and the `dashboards` table and
//!    populate the column. These KSUIDs will act as the primary keys for records that are copied to
//!    new `folders` and `dashboards` tables.
//! 3. Create new `folders`, `dashboards`, and `alerts` tables and their indices. These new tables
//!    should all use KSUIDs as their primary keys and foreign keys.
//! 4. Select all records from the `legacy_folders` table. For each record create a new record in
//!    the `folders` table using `legacy_folders.ksuid` as `folders.id`.
//! 5. Select all records from the `legacy_dashboards` table joined on the `legacy_folders` table.
//!    For each result create a new record in the `dashboards` table using `legacy_dashboards.ksuid`
//!    as `dashboards.id` and `legacy_folders.ksuid` as `dashboard.folder_id`.
//! 6. Select all records from the `legacy_alerts` table joined on the `legacy_folders` table. For
//!    each result create a new record in the `alerts` table using `legacy_alerts.ksuid` as
//!    `alerts.id` and `legacy_folders.ksuid` as `alerts.folder_id`.
//! 7. Delete the `legacy_folders`, `legacy_dashboards`, and `legacy_alerts` tables.

use itertools::Itertools;
use sea_orm::{
    sea_query::{Table, TableCreateStatement},
    DeriveIden, EntityTrait, IntoActiveModel, PaginatorTrait, QueryOrder, Set, TransactionTrait,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Rename the legacy tables so that we can create new tables using the
        // original table names
        manager
            .rename_table(legacy_folders::rename_to_legacy_folders())
            .await?;
        manager
            .rename_table(legacy_dashboards::rename_to_legacy_dashboards())
            .await?;
        manager
            .rename_table(legacy_alerts::rename_to_legacy_alerts())
            .await?;

        // Add a temporary `ksuid` column to `legacy_folders` and
        // `legacy_dasbhoards` columns and populate the KSUIDs. `legacy_alerts`
        // already has a KSUID primary key so we don't need to create a
        // temporary `ksuid` column on that table.
        manager
            .alter_table(legacy_folders::add_ksuid_column())
            .await?;
        manager
            .alter_table(legacy_dashboards::add_ksuid_column())
            .await?;
        let txn = manager.get_connection().begin().await?;
        legacy_folders::populate_ksuid_column(&txn, 100).await?;
        legacy_dashboards::populate_ksuid_column(&txn, 100).await?;
        txn.commit().await?;

        // Create the new `folders`, `dashboards`, and `alerts` tables which use
        // KSUIDs as their primary keys and foreign keys. And create the new
        // indices for the tables.
        manager
            .create_table(new_folders::create_folders_table_statement())
            .await?;
        manager
            .create_index(new_folders::create_folders_org_idx_stmnt())
            .await?;
        manager
            .create_index(new_folders::create_folders_org_type_folder_id_idx_stmnt())
            .await?;
        manager
            .create_table(new_dashboards::create_dashboards_table_statement())
            .await?;
        manager
            .create_index(new_dashboards::create_dashboards_folder_id_dashboard_id_idx_stmnt())
            .await?;
        manager
            .create_table(new_alerts::create_alerts_table_statement())
            .await?;
        manager
            .create_index(new_alerts::create_alerts_folder_id_idx_stmnt())
            .await?;
        manager
            .create_index(new_alerts::create_alerts_org_stream_type_stream_name_name_idx_stmnt())
            .await?;

        // Populate the new `folders`, `dashboards`, and `alerts` tables with
        // data from the legacy tables.
        let txn = manager.get_connection().begin().await?;
        new_folders::populate(&txn).await?;
        new_dashboards::populate(&txn).await?;
        new_alerts::populate(&txn).await?;
        txn.commit().await?;

        // Delete each of the legacy tables.
        manager.drop_table(legacy_dashboards::drop_table()).await?;
        manager.drop_table(legacy_alerts::drop_table()).await?;
        manager.drop_table(legacy_folders::drop_table()).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "Downgrading this migration is not supported".to_string(),
        ))
    }
}

/// Data structures and migration statements for the legacy folders table.
mod legacy_folders {
    use sea_orm::{ActiveModelTrait, IntoActiveModel, QueryOrder};
    use svix_ksuid::KsuidLike;

    use super::*;

    const OLD_TABLE_NAME: &str = "folders";
    const NEW_TABLE_NAME: &str = "legacy_folders";

    /// Statement to rename the legacy folders table from `folders` to
    /// `legacy_folders`.
    pub fn rename_to_legacy_folders() -> TableRenameStatement {
        Table::rename()
            .table(Alias::new(OLD_TABLE_NAME), Alias::new(NEW_TABLE_NAME))
            .to_owned()
    }

    /// Statement to add a new optional `ksuid` column to the `legacy_folders` table.
    ///
    /// This should be ran AFTER the legacy folders table is renamed from
    /// `folders` to `legacy_folders`.
    pub fn add_ksuid_column() -> TableAlterStatement {
        Table::alter()
            .table(Alias::new(NEW_TABLE_NAME))
            .add_column(ColumnDef::new(Alias::new("ksuid")).char_len(27).null())
            .to_owned()
    }

    /// Populates the `ksuid` column for every record in the `legacy_folders` table.
    ///
    /// This should be ran AFTER the legacy folders table is renamed from
    /// `folders` to `legacy_folders`.
    pub async fn populate_ksuid_column<C: ConnectionTrait>(
        conn: &C,
        page_size: u64,
    ) -> Result<(), sea_orm_migration::DbErr> {
        let mut pages = legacy_entities::legacy_folders::Entity::find()
            .order_by_asc(legacy_entities::legacy_folders::Column::Id)
            .paginate(conn, page_size);

        while let Some(folders) = pages.fetch_and_next().await? {
            for folder in folders {
                let mut am = folder.into_active_model();
                let ksuid = svix_ksuid::Ksuid::new(None, None).to_string();
                println!("folder ksuid: {}", ksuid);
                am.ksuid = Set(Some(ksuid));
                am.update(conn).await?;
            }
        }

        Ok(())
    }

    /// Statement to drop the legacy folders table.
    ///
    /// This should be ran AFTER the legacy folders table is renamed from
    /// `folders` to `legacy_folders`.
    pub fn drop_table() -> TableDropStatement {
        Table::drop().table(Alias::new(NEW_TABLE_NAME)).to_owned()
    }
}

/// Data structures and migration statements for the legacy dashboards table.
mod legacy_dashboards {
    use sea_orm::{ActiveModelTrait, IntoActiveModel, QueryOrder};
    use svix_ksuid::KsuidLike;

    use super::*;

    const OLD_TABLE_NAME: &str = "dashboards";
    const NEW_TABLE_NAME: &str = "legacy_dashboards";

    /// Statement to rename the legacy dashboards table from `dashboards` to
    /// `legacy_dashboards`.
    pub fn rename_to_legacy_dashboards() -> TableRenameStatement {
        Table::rename()
            .table(Alias::new(OLD_TABLE_NAME), Alias::new(NEW_TABLE_NAME))
            .to_owned()
    }

    /// Statement to add a new optional `ksuid` column to the
    /// `legacy_dashboards` table.
    ///
    /// This should be ran AFTER the legacy dashboards table is renamed from
    /// `dashboards` to `legacy_dashboards`.
    pub fn add_ksuid_column() -> TableAlterStatement {
        Table::alter()
            .table(Alias::new(NEW_TABLE_NAME))
            .add_column(ColumnDef::new(Alias::new("ksuid")).char_len(27).null())
            .to_owned()
    }

    /// Populates the `ksuid` column for every record in the `legacy_dashboards`
    /// table.
    ///
    /// This should be ran AFTER the legacy dashboards table is renamed from
    /// `dashboards` to `legacy_dashboards`.
    pub async fn populate_ksuid_column<C: ConnectionTrait>(
        conn: &C,
        page_size: u64,
    ) -> Result<(), sea_orm_migration::DbErr> {
        let mut pages = legacy_entities::legacy_dashboards::Entity::find()
            .order_by_asc(legacy_entities::legacy_dashboards::Column::Id)
            .paginate(conn, page_size);

        while let Some(dashboards) = pages.fetch_and_next().await? {
            for dashboard in dashboards {
                let mut am = dashboard.into_active_model();
                let ksuid = svix_ksuid::Ksuid::new(None, None).to_string();
                am.ksuid = Set(Some(ksuid));
                am.update(conn).await?;
            }
        }

        Ok(())
    }

    /// Statement to drop the legacy dashboards table.
    ///
    /// This should be ran AFTER the legacy dashboards table is renamed from
    /// `dashboards` to `legacy_dashboards`.
    pub fn drop_table() -> TableDropStatement {
        Table::drop().table(Alias::new(NEW_TABLE_NAME)).to_owned()
    }
}

/// Data structures and migration statements for the legacy alerts table.
mod legacy_alerts {
    use super::*;

    const OLD_TABLE_NAME: &str = "alerts";
    const NEW_TABLE_NAME: &str = "legacy_alerts";
    /// Statement to rename the legacy alerts table from `alerts` to
    /// `legacy_alerts`.
    pub fn rename_to_legacy_alerts() -> TableRenameStatement {
        Table::rename()
            .table(Alias::new(OLD_TABLE_NAME), Alias::new(NEW_TABLE_NAME))
            .to_owned()
    }

    /// Statement to drop the legacy alerts table.
    ///
    /// This should be ran AFTER the legacy alerts table is renamed from
    /// `alerts` to `legacy_alerts`.
    pub fn drop_table() -> TableDropStatement {
        Table::drop().table(Alias::new(NEW_TABLE_NAME)).to_owned()
    }
}

mod new_folders {

    use super::*;

    const FOLDERS_ORG_IDX: &str = "folders_org_idx_2";
    const FOLDERS_ORG_TYPE_FOLDER_ID_IDX: &str = "folders_org_type_folder_id_idx_2";

    /// Identifiers used in queries on the new folders table.
    #[derive(DeriveIden)]
    pub enum Folders {
        Table,
        Id,
        Org,
        FolderId,
        Name,
        Description,
        Type,
    }

    /// Statement to create the new folders table.
    pub fn create_folders_table_statement() -> TableCreateStatement {
        Table::create()
        .table(Folders::Table)
        // The ID is 27-character human readable KSUID.
        .col(
            ColumnDef::new(Folders::Id)
                .char_len(27)
                .not_null()
                .primary_key(),
        )
        .col(ColumnDef::new(Folders::Org).string_len(100).not_null())
        // A user-facing ID for the folder. This value can be a 64-bit signed
        // integer "snowflake" or the string "default" to indicate the
        // organization's special default folder.
        .col(ColumnDef::new(Folders::FolderId).string_len(256).not_null())
        .col(ColumnDef::new(Folders::Name).string_len(256).not_null())
        .col(ColumnDef::new(Folders::Description).text())
        // Folder type where...
        // - 0 is a dashboards folder
        // - 1 is an alerts folder
        // - 2 is a reports folder
        .col(ColumnDef::new(Folders::Type).small_integer().not_null())
        .to_owned()
    }

    /// Statement to create index on org.
    pub fn create_folders_org_idx_stmnt() -> IndexCreateStatement {
        sea_query::Index::create()
            .if_not_exists()
            .name(FOLDERS_ORG_IDX)
            .table(Folders::Table)
            .col(Folders::Org)
            .to_owned()
    }

    /// Statement to create  unique index on org, type, and folder_id.
    pub fn create_folders_org_type_folder_id_idx_stmnt() -> IndexCreateStatement {
        sea_query::Index::create()
            .if_not_exists()
            .name(FOLDERS_ORG_TYPE_FOLDER_ID_IDX)
            .table(Folders::Table)
            .col(Folders::Org)
            .col(Folders::Type)
            .col(Folders::FolderId)
            .unique()
            .to_owned()
    }

    /// Select each record from the `legacy_folders` table and use it to
    /// create a new record in the new `folders` table.
    pub async fn populate<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
        let mut legacy_folder_pages = legacy_entities::legacy_folders::Entity::find()
            .order_by_asc(legacy_entities::legacy_folders::Column::Id)
            .paginate(conn, 100);
        while let Some(legacy_folders) = legacy_folder_pages.fetch_and_next().await? {
            let conversions_rslt: Result<Vec<_>, _> = legacy_folders
                .into_iter()
                .map(new_entities::folders::Model::try_from)
                .collect();
            let active_models = conversions_rslt?
                .into_iter()
                .map(|m| m.into_active_model())
                .collect_vec();
            new_entities::folders::Entity::insert_many(active_models)
                .exec(conn)
                .await?;
        }
        Ok(())
    }
}

mod new_dashboards {
    use sea_orm::QueryOrder;

    use super::*;

    const DASHBOARDS_FOLDERS_FK: &str = "dashboards_folders_fk_2";
    const DASHBOARDS_FOLDER_ID_DASHBOARD_ID_IDX: &str = "dashboards_folder_id_dashboard_id_idx_2";

    /// Identifiers used in queries on the new dashboards table.
    #[derive(DeriveIden)]
    enum Dashboards {
        Table,
        Id,
        DashboardId,
        FolderId,
        Owner,
        Role,
        Title,
        Description,
        Data,
        Version,
        CreatedAt,
    }

    /// Statement to create the new dashboards table.
    pub fn create_dashboards_table_statement() -> TableCreateStatement {
        Table::create()
        .table(Dashboards::Table)
        // The ID is a 27-character human readable KSUID.
        .col(
            ColumnDef::new(Dashboards::Id)
                .char_len(27)
                .not_null()
                .primary_key(),
        )
        // A user-facing ID for the folder. This value can be a 64-bit signed
        // integer "snowflake".
        .col(ColumnDef::new(Dashboards::DashboardId).string_len(256).not_null())
        // Foreign key to the folders table. This is a 27-character human readable
        // KSUID.
        .col(ColumnDef::new(Dashboards::FolderId).char_len(27).not_null())
        // Identifier of the user that owns the dashboard.
        .col(ColumnDef::new(Dashboards::Owner).string_len(256).not_null())
        .col(ColumnDef::new(Dashboards::Role).string_len(256).null())
        .col(ColumnDef::new(Dashboards::Title).string_len(256).not_null())
        .col(ColumnDef::new(Dashboards::Description).text().null())
        .col(ColumnDef::new(Dashboards::Data).json().not_null())
        .col(ColumnDef::new(Dashboards::Version).integer().not_null())
        .col(
            ColumnDef::new(Dashboards::CreatedAt)
                .big_integer()
                .not_null(),
        )
        .foreign_key(
            sea_query::ForeignKey::create()
                    .name(DASHBOARDS_FOLDERS_FK)
                    .from(Dashboards::Table, Dashboards::FolderId)
                    .to(new_folders::Folders::Table, new_folders::Folders::Id)
        )
        .to_owned()
    }

    /// Statement to create unique index on dashboard_id.
    pub fn create_dashboards_folder_id_dashboard_id_idx_stmnt() -> IndexCreateStatement {
        sea_query::Index::create()
            .if_not_exists()
            .name(DASHBOARDS_FOLDER_ID_DASHBOARD_ID_IDX)
            .table(Dashboards::Table)
            .col(Dashboards::FolderId)
            .col(Dashboards::DashboardId)
            .unique()
            .to_owned()
    }

    /// Select each record from the `legacy_dashboards` table joined on the
    /// `legacy_folders` table and use it to create a new record in the new
    /// `dashboards` table.
    pub async fn populate<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
        let mut legacy_dashboard_pages = legacy_entities::legacy_dashboards::Entity::find()
            .find_also_related(legacy_entities::legacy_folders::Entity)
            .order_by_asc(legacy_entities::legacy_dashboards::Column::Id)
            .paginate(conn, 100);
        while let Some(legacy_dashboards) = legacy_dashboard_pages.fetch_and_next().await? {
            let conversions_rslt: Result<Vec<_>, _> = legacy_dashboards
                .into_iter()
                .map(new_entities::dashboards::Model::try_from)
                .collect();
            let active_models = conversions_rslt?
                .into_iter()
                .map(|m| m.into_active_model())
                .collect_vec();
            new_entities::dashboards::Entity::insert_many(active_models)
                .exec(conn)
                .await?;
        }
        Ok(())
    }
}

mod new_alerts {
    use sea_orm::QueryOrder;

    use super::*;
    const ALERTS_FOLDERS_FK: &str = "alerts_folders_fk_2";
    const ALERTS_ORG_STREAM_TYPE_STREAM_NAME_NAME_IDX: &str =
        "alerts_org_stream_type_stream_name_name_idx_2";
    const ALERTS_FOLDER_ID_IDX: &str = "alerts_folder_id_idx_2";

    /// Identifiers used in queries on the alerts table.
    #[derive(DeriveIden)]
    enum Alerts {
        Table,
        Id,
        Org,
        FolderId,
        Name,
        StreamType,
        StreamName,
        IsRealTime,
        Destinations,
        ContextAttributes,
        RowTemplate,
        Description,
        Enabled,
        TzOffset,
        LastTriggeredAt,
        LastSatisfiedAt,
        // Query condition
        QueryType,
        QueryConditions,
        QuerySql,
        QueryPromql,
        QueryPromqlCondition,
        QueryAggregation,
        QueryVrlFunction,
        QuerySearchEventType,
        QueryMultiTimeRange,
        // Trigger condition
        TriggerThresholdOperator,
        TriggerThresholdCount,
        TriggerFrequencyType,
        TriggerFrequencySeconds,
        TriggerFrequencyCron,
        TriggerFrequencyCronTimezone,
        TriggerPeriodSeconds,
        TriggerSilenceSeconds,
        TriggerToleranceSeconds,
        Owner,
        UpdatedAt,
        LastEditedBy,
    }

    /// Statement to create the alerts table.
    pub fn create_alerts_table_statement() -> TableCreateStatement {
        Table::create()
        .table(Alerts::Table)
        .if_not_exists()
        // The ID is a 27-character human readable KSUID.
        .col(
            ColumnDef::new(Alerts::Id)
                .char_len(27)
                .not_null()
                .primary_key(),
        )
        // The org field is redundant and should always match the org of the
        // associated folder. However we need the org column on this table in
        // order to enforce a uniqueness constraint that includeds org but not
        // folder_id.
        .col(ColumnDef::new(Alerts::Org).string_len(100).not_null())
        // Foreign key to the folders table. This is a 27-character human
        // readable KSUID.
        .col(ColumnDef::new(Alerts::FolderId).char_len(27).not_null())
        .col(ColumnDef::new(Alerts::Name).string_len(256).not_null())
        .col(ColumnDef::new(Alerts::StreamType).string_len(50).not_null())
        .col(ColumnDef::new(Alerts::StreamName).string_len(256).not_null())
        .col(ColumnDef::new(Alerts::IsRealTime).boolean().not_null())
        .col(ColumnDef::new(Alerts::Destinations).json().not_null())
        .col(ColumnDef::new(Alerts::ContextAttributes).json().null())
        .col(ColumnDef::new(Alerts::RowTemplate).text().null())
        .col(ColumnDef::new(Alerts::Description).text().null())
        .col(ColumnDef::new(Alerts::Enabled).boolean().not_null())
        .col(ColumnDef::new(Alerts::TzOffset).integer().not_null())
        .col(ColumnDef::new(Alerts::LastTriggeredAt).big_integer().null())
        .col(ColumnDef::new(Alerts::LastSatisfiedAt).big_integer().null())
        // Query condition
        .col(
            ColumnDef::new(Alerts::QueryType).small_integer().not_null(),
        )
        .col(ColumnDef::new(Alerts::QueryConditions).json().null())
        .col(ColumnDef::new(Alerts::QuerySql).text().null())
        .col(ColumnDef::new(Alerts::QueryPromql).text().null())
        .col(
            ColumnDef::new(Alerts::QueryPromqlCondition)
                .json()
                .null(),
        )
        .col(ColumnDef::new(Alerts::QueryAggregation).json().null())
        .col(ColumnDef::new(Alerts::QueryVrlFunction).text().null())
        .col(
            ColumnDef::new(Alerts::QuerySearchEventType).small_integer().null(),
        )
        .col(
            ColumnDef::new(Alerts::QueryMultiTimeRange)
                .json()
                .null(),
        )
        // Trigger condition
        .col(
            ColumnDef::new(Alerts::TriggerThresholdOperator)
                .string_len(50)
                .not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerPeriodSeconds)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerThresholdCount)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerFrequencyType).small_integer().not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerFrequencySeconds)
                .big_integer()
                .not_null(),
        )
        .col(ColumnDef::new(Alerts::TriggerFrequencyCron).text().null())
        .col(
            ColumnDef::new(Alerts::TriggerFrequencyCronTimezone)
                .string_len(256)
                .null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerSilenceSeconds)
                .big_integer()
                .not_null(),
        )
        .col(
            ColumnDef::new(Alerts::TriggerToleranceSeconds)
                .big_integer()
                .null(),
        )
        // Ownership and update information.
        .col(ColumnDef::new(Alerts::Owner).string_len(256).null())
        .col(ColumnDef::new(Alerts::LastEditedBy).string_len(256).null())
        .col(ColumnDef::new(Alerts::UpdatedAt).big_integer().null())
        .foreign_key(
            sea_query::ForeignKey::create()
                    .name(ALERTS_FOLDERS_FK)
                    .from(Alerts::Table, Alerts::FolderId)
                    .to(new_folders::Folders::Table, new_folders::Folders::Id)
        )
        .to_owned()
    }

    /// Statement to create unique index on the org, stream_type, stream_name, and
    /// name columns of the alerts table.
    pub fn create_alerts_org_stream_type_stream_name_name_idx_stmnt() -> IndexCreateStatement {
        sea_query::Index::create()
            .if_not_exists()
            .name(ALERTS_ORG_STREAM_TYPE_STREAM_NAME_NAME_IDX)
            .table(Alerts::Table)
            .col(Alerts::Org)
            .col(Alerts::StreamType)
            .col(Alerts::StreamName)
            .col(Alerts::Name)
            .unique()
            .to_owned()
    }

    /// Statement to create index on the folder_id column of the alerts table.
    pub fn create_alerts_folder_id_idx_stmnt() -> IndexCreateStatement {
        sea_query::Index::create()
            .if_not_exists()
            .name(ALERTS_FOLDER_ID_IDX)
            .table(Alerts::Table)
            .col(Alerts::FolderId)
            .to_owned()
    }

    /// Select each record from the `legacy_alerts` table joined on the
    /// `legacy_folders` table and use it to create a new record in the new
    /// `alerts` table.
    pub async fn populate<C: ConnectionTrait>(conn: &C) -> Result<(), DbErr> {
        let mut legacy_alerts_pages = legacy_entities::legacy_alerts::Entity::find()
            .find_also_related(legacy_entities::legacy_folders::Entity)
            .order_by_asc(legacy_entities::legacy_alerts::Column::Id)
            .paginate(conn, 100);
        while let Some(legacy_alerts) = legacy_alerts_pages.fetch_and_next().await? {
            let conversions_rslt: Result<Vec<_>, _> = legacy_alerts
                .into_iter()
                .map(new_entities::alerts::Model::try_from)
                .collect();
            let active_models = conversions_rslt?
                .into_iter()
                .map(|m| m.into_active_model())
                .collect_vec();
            new_entities::alerts::Entity::insert_many(active_models)
                .exec(conn)
                .await?;
        }
        Ok(())
    }
}

/// ORM models of the legacy folders, dashboards, and alerts tables after they
/// have been renamed to `legacy_folders`, `legacy_dashboards`, and
/// `legacy_alerts` and after a new `ksuid` column has been added to each of the
/// `legacy_folders` and `legacy_dashboards` tables.
mod legacy_entities {
    pub mod legacy_folders {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "legacy_folders")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub org: String,
            pub folder_id: String,
            pub name: String,
            #[sea_orm(column_type = "Text", nullable)]
            pub description: Option<String>,
            pub r#type: i16,
            pub ksuid: Option<String>, // This column is newly added by this migration.
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(has_many = "super::legacy_alerts::Entity")]
            LegacyAlerts,
            #[sea_orm(has_many = "super::legacy_dashboards::Entity")]
            LegacyDashboards,
        }

        impl Related<super::legacy_alerts::Entity> for Entity {
            fn to() -> RelationDef {
                Relation::LegacyAlerts.def()
            }
        }

        impl Related<super::legacy_dashboards::Entity> for Entity {
            fn to() -> RelationDef {
                Relation::LegacyDashboards.def()
            }
        }

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub mod legacy_alerts {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "legacy_alerts")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: String,
            pub org: String,
            pub folder_id: i64,
            pub name: String,
            pub stream_type: String,
            pub stream_name: String,
            pub is_real_time: bool,
            pub destinations: Json,
            pub context_attributes: Option<Json>,
            #[sea_orm(column_type = "Text", nullable)]
            pub row_template: Option<String>,
            #[sea_orm(column_type = "Text", nullable)]
            pub description: Option<String>,
            pub enabled: bool,
            pub tz_offset: i32,
            pub last_triggered_at: Option<i64>,
            pub last_satisfied_at: Option<i64>,
            pub query_type: i16,
            pub query_conditions: Option<Json>,
            #[sea_orm(column_type = "Text", nullable)]
            pub query_sql: Option<String>,
            #[sea_orm(column_type = "Text", nullable)]
            pub query_promql: Option<String>,
            pub query_promql_condition: Option<Json>,
            pub query_aggregation: Option<Json>,
            #[sea_orm(column_type = "Text", nullable)]
            pub query_vrl_function: Option<String>,
            pub query_search_event_type: Option<i16>,
            pub query_multi_time_range: Option<Json>,
            pub trigger_threshold_operator: String,
            pub trigger_period_seconds: i64,
            pub trigger_threshold_count: i64,
            pub trigger_frequency_type: i16,
            pub trigger_frequency_seconds: i64,
            #[sea_orm(column_type = "Text", nullable)]
            pub trigger_frequency_cron: Option<String>,
            pub trigger_frequency_cron_timezone: Option<String>,
            pub trigger_silence_seconds: i64,
            pub trigger_tolerance_seconds: Option<i64>,
            pub owner: Option<String>,
            pub last_edited_by: Option<String>,
            pub updated_at: Option<i64>,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(
                belongs_to = "super::legacy_folders::Entity",
                from = "Column::FolderId",
                to = "super::legacy_folders::Column::Id",
                on_update = "NoAction",
                on_delete = "NoAction"
            )]
            LegacyFolders,
        }

        impl Related<super::legacy_folders::Entity> for Entity {
            fn to() -> RelationDef {
                Relation::LegacyFolders.def()
            }
        }

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub mod legacy_dashboards {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "legacy_dashboards")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub dashboard_id: String,
            pub folder_id: i64,
            pub owner: String,
            pub role: Option<String>,
            pub title: String,
            #[sea_orm(column_type = "Text", nullable)]
            pub description: Option<String>,
            pub data: Json,
            pub version: i32,
            pub created_at: i64,
            pub ksuid: Option<String>, // This column is newly added by this migration.
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {
            #[sea_orm(
                belongs_to = "super::legacy_folders::Entity",
                from = "Column::FolderId",
                to = "super::legacy_folders::Column::Id",
                on_update = "NoAction",
                on_delete = "NoAction"
            )]
            LegacyFolders,
        }

        impl Related<super::legacy_folders::Entity> for Entity {
            fn to() -> RelationDef {
                Relation::LegacyFolders.def()
            }
        }

        impl ActiveModelBehavior for ActiveModel {}
    }
}

/// ORM models of the new folders, dashboards, and alerts tables.
mod new_entities {
    pub mod folders {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "folders")]
        pub struct Model {
            // The new KSUID primary key.
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: String,
            pub org: String,
            pub folder_id: String,
            pub name: String,
            #[sea_orm(column_type = "Text", nullable)]
            pub description: Option<String>,
            pub r#type: i16,
        }

        // There are relations but they are not important to this migration.
        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub mod alerts {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "alerts")]
        pub struct Model {
            // The ID was a KSUID in the legacy table so this column is
            // unchanged in the new table.
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: String,
            pub org: String,
            // The new KSUID foreign key reference to the folders table.
            pub folder_id: String,
            pub name: String,
            pub stream_type: String,
            pub stream_name: String,
            pub is_real_time: bool,
            pub destinations: Json,
            pub context_attributes: Option<Json>,
            #[sea_orm(column_type = "Text", nullable)]
            pub row_template: Option<String>,
            #[sea_orm(column_type = "Text", nullable)]
            pub description: Option<String>,
            pub enabled: bool,
            pub tz_offset: i32,
            pub last_triggered_at: Option<i64>,
            pub last_satisfied_at: Option<i64>,
            pub query_type: i16,
            pub query_conditions: Option<Json>,
            #[sea_orm(column_type = "Text", nullable)]
            pub query_sql: Option<String>,
            #[sea_orm(column_type = "Text", nullable)]
            pub query_promql: Option<String>,
            pub query_promql_condition: Option<Json>,
            pub query_aggregation: Option<Json>,
            #[sea_orm(column_type = "Text", nullable)]
            pub query_vrl_function: Option<String>,
            pub query_search_event_type: Option<i16>,
            pub query_multi_time_range: Option<Json>,
            pub trigger_threshold_operator: String,
            pub trigger_period_seconds: i64,
            pub trigger_threshold_count: i64,
            pub trigger_frequency_type: i16,
            pub trigger_frequency_seconds: i64,
            #[sea_orm(column_type = "Text", nullable)]
            pub trigger_frequency_cron: Option<String>,
            pub trigger_frequency_cron_timezone: Option<String>,
            pub trigger_silence_seconds: i64,
            pub trigger_tolerance_seconds: Option<i64>,
            pub owner: Option<String>,
            pub last_edited_by: Option<String>,
            pub updated_at: Option<i64>,
        }

        // There are relations but they are not important to this migration.
        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub mod dashboards {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
        #[sea_orm(table_name = "dashboards")]
        pub struct Model {
            // The new KSUID primary key.
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: String,
            pub dashboard_id: String,
            // The new KSUID foreign key reference to the folders table.
            pub folder_id: String,
            pub owner: String,
            pub role: Option<String>,
            pub title: String,
            #[sea_orm(column_type = "Text", nullable)]
            pub description: Option<String>,
            pub data: Json,
            pub version: i32,
            pub created_at: i64,
        }

        // There are relations but they are not important to this migration.
        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }
}

impl TryFrom<legacy_entities::legacy_folders::Model> for new_entities::folders::Model {
    type Error = DbErr;

    fn try_from(value: legacy_entities::legacy_folders::Model) -> Result<Self, Self::Error> {
        let folder_ksuid = value.ksuid.ok_or(DbErr::Migration(
            "The legacy folder's `ksuid` column is not populated.".to_string(),
        ))?;
        let folder = Self {
            id: folder_ksuid, // This is the new KSUID primary key.
            org: value.org,
            folder_id: value.folder_id,
            name: value.name,
            description: value.description,
            r#type: value.r#type,
        };
        Ok(folder)
    }
}

impl
    TryFrom<(
        legacy_entities::legacy_alerts::Model,
        Option<legacy_entities::legacy_folders::Model>,
    )> for new_entities::alerts::Model
{
    type Error = DbErr;

    fn try_from(
        value: (
            legacy_entities::legacy_alerts::Model,
            Option<legacy_entities::legacy_folders::Model>,
        ),
    ) -> Result<Self, Self::Error> {
        let folder = value.1.ok_or(DbErr::Migration(
            "The legacy alert references a folder that doesn't exist.".to_string(),
        ))?;
        let folder_ksuid = folder.ksuid.ok_or(DbErr::Migration(
            "The legacy folder's `ksuid` column is not populated.".to_string(),
        ))?;
        let alert = Self {
            id: value.0.id,
            org: value.0.org,
            folder_id: folder_ksuid, // This is the new KSUID foreign key.
            name: value.0.name,
            stream_type: value.0.stream_type,
            stream_name: value.0.stream_name,
            is_real_time: value.0.is_real_time,
            destinations: value.0.destinations,
            context_attributes: value.0.context_attributes,
            row_template: value.0.row_template,
            description: value.0.description,
            enabled: value.0.enabled,
            tz_offset: value.0.tz_offset,
            last_triggered_at: value.0.last_triggered_at,
            last_satisfied_at: value.0.last_satisfied_at,
            query_type: value.0.query_type,
            query_conditions: value.0.query_conditions,
            query_sql: value.0.query_sql,
            query_promql: value.0.query_promql,
            query_promql_condition: value.0.query_promql_condition,
            query_aggregation: value.0.query_aggregation,
            query_vrl_function: value.0.query_vrl_function,
            query_search_event_type: value.0.query_search_event_type,
            query_multi_time_range: value.0.query_multi_time_range,
            trigger_threshold_operator: value.0.trigger_threshold_operator,
            trigger_period_seconds: value.0.trigger_period_seconds,
            trigger_threshold_count: value.0.trigger_threshold_count,
            trigger_frequency_type: value.0.trigger_frequency_type,
            trigger_frequency_seconds: value.0.trigger_frequency_seconds,
            trigger_frequency_cron: value.0.trigger_frequency_cron,
            trigger_frequency_cron_timezone: value.0.trigger_frequency_cron_timezone,
            trigger_silence_seconds: value.0.trigger_silence_seconds,
            trigger_tolerance_seconds: value.0.trigger_tolerance_seconds,
            owner: value.0.owner,
            last_edited_by: value.0.last_edited_by,
            updated_at: value.0.updated_at,
        };
        Ok(alert)
    }
}

impl
    TryFrom<(
        legacy_entities::legacy_dashboards::Model,
        Option<legacy_entities::legacy_folders::Model>,
    )> for new_entities::dashboards::Model
{
    type Error = DbErr;

    fn try_from(
        value: (
            legacy_entities::legacy_dashboards::Model,
            Option<legacy_entities::legacy_folders::Model>,
        ),
    ) -> Result<Self, Self::Error> {
        let dashboard_ksuid = value.0.ksuid.ok_or(DbErr::Migration(
            "The legacy dashboard's `ksuid` column is not populated.".to_string(),
        ))?;
        let folder = value.1.ok_or(DbErr::Migration(
            "The legacy dashboard references a folder that doesn't exist.".to_string(),
        ))?;
        let folder_ksuid = folder.ksuid.ok_or(DbErr::Migration(
            "The legacy folder's `ksuid` column is not populated.".to_string(),
        ))?;
        let dashboard = Self {
            id: dashboard_ksuid, // This is the new KSUID primary key.
            dashboard_id: value.0.dashboard_id,
            folder_id: folder_ksuid, // This is the new KSUID foreign key.
            owner: value.0.owner,
            role: value.0.role,
            title: value.0.title,
            description: value.0.description,
            data: value.0.data,
            version: value.0.version,
            created_at: value.0.created_at,
        };
        Ok(dashboard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres() {
        legacy_folders::add_ksuid_column().to_string(PostgresQueryBuilder);
        legacy_folders::drop_table().to_string(PostgresQueryBuilder);
        legacy_folders::rename_to_legacy_folders().to_string(PostgresQueryBuilder);

        legacy_dashboards::add_ksuid_column().to_string(PostgresQueryBuilder);
        legacy_dashboards::drop_table().to_string(PostgresQueryBuilder);
        legacy_dashboards::rename_to_legacy_dashboards().to_string(PostgresQueryBuilder);

        legacy_alerts::drop_table().to_string(PostgresQueryBuilder);
        legacy_alerts::rename_to_legacy_alerts().to_string(PostgresQueryBuilder);

        new_folders::create_folders_org_idx_stmnt().to_string(PostgresQueryBuilder);
        new_folders::create_folders_org_type_folder_id_idx_stmnt().to_string(PostgresQueryBuilder);
        new_folders::create_folders_table_statement().to_string(PostgresQueryBuilder);

        new_dashboards::create_dashboards_folder_id_dashboard_id_idx_stmnt()
            .to_string(PostgresQueryBuilder);
        new_dashboards::create_dashboards_table_statement().to_string(PostgresQueryBuilder);

        new_alerts::create_alerts_folder_id_idx_stmnt().to_string(PostgresQueryBuilder);
        new_alerts::create_alerts_org_stream_type_stream_name_name_idx_stmnt()
            .to_string(PostgresQueryBuilder);
        new_alerts::create_alerts_table_statement().to_string(PostgresQueryBuilder);
    }

    #[test]
    fn mysql() {
        legacy_folders::add_ksuid_column().to_string(MysqlQueryBuilder);
        legacy_folders::drop_table().to_string(MysqlQueryBuilder);
        legacy_folders::rename_to_legacy_folders().to_string(MysqlQueryBuilder);

        legacy_dashboards::add_ksuid_column().to_string(MysqlQueryBuilder);
        legacy_dashboards::drop_table().to_string(MysqlQueryBuilder);
        legacy_dashboards::rename_to_legacy_dashboards().to_string(MysqlQueryBuilder);

        legacy_alerts::drop_table().to_string(MysqlQueryBuilder);
        legacy_alerts::rename_to_legacy_alerts().to_string(MysqlQueryBuilder);

        new_folders::create_folders_org_idx_stmnt().to_string(MysqlQueryBuilder);
        new_folders::create_folders_org_type_folder_id_idx_stmnt().to_string(MysqlQueryBuilder);
        new_folders::create_folders_table_statement().to_string(MysqlQueryBuilder);

        new_dashboards::create_dashboards_folder_id_dashboard_id_idx_stmnt()
            .to_string(MysqlQueryBuilder);
        new_dashboards::create_dashboards_table_statement().to_string(MysqlQueryBuilder);

        new_alerts::create_alerts_folder_id_idx_stmnt().to_string(MysqlQueryBuilder);
        new_alerts::create_alerts_org_stream_type_stream_name_name_idx_stmnt()
            .to_string(MysqlQueryBuilder);
        new_alerts::create_alerts_table_statement().to_string(MysqlQueryBuilder);
    }

    #[test]
    fn sqlite() {
        legacy_folders::add_ksuid_column().to_string(SqliteQueryBuilder);
        legacy_folders::drop_table().to_string(SqliteQueryBuilder);
        legacy_folders::rename_to_legacy_folders().to_string(SqliteQueryBuilder);

        legacy_dashboards::add_ksuid_column().to_string(SqliteQueryBuilder);
        legacy_dashboards::drop_table().to_string(SqliteQueryBuilder);
        legacy_dashboards::rename_to_legacy_dashboards().to_string(SqliteQueryBuilder);

        legacy_alerts::drop_table().to_string(SqliteQueryBuilder);
        legacy_alerts::rename_to_legacy_alerts().to_string(SqliteQueryBuilder);

        new_folders::create_folders_org_idx_stmnt().to_string(SqliteQueryBuilder);
        new_folders::create_folders_org_type_folder_id_idx_stmnt().to_string(SqliteQueryBuilder);
        new_folders::create_folders_table_statement().to_string(SqliteQueryBuilder);

        new_dashboards::create_dashboards_folder_id_dashboard_id_idx_stmnt()
            .to_string(SqliteQueryBuilder);
        new_dashboards::create_dashboards_table_statement().to_string(SqliteQueryBuilder);

        new_alerts::create_alerts_folder_id_idx_stmnt().to_string(SqliteQueryBuilder);
        new_alerts::create_alerts_org_stream_type_stream_name_name_idx_stmnt()
            .to_string(SqliteQueryBuilder);
        new_alerts::create_alerts_table_statement().to_string(SqliteQueryBuilder);
    }
}
