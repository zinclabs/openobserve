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

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(create_organizations_table_statement())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Organizations::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the folder table.
fn create_organizations_table_statement() -> TableCreateStatement {
    Table::create()
        .table(Organizations::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Organizations::Identifier)
                .string_len(256)
                .not_null()
                .primary_key(),
        )
        .col(ColumnDef::new(Organizations::OrgName).string_len(200).not_null())
        // Organizations type where...
        // - 0 is a default organization
        // - 1 is a custom organization
        .col(ColumnDef::new(Organizations::OrgType).small_integer().not_null())
        .col(
            ColumnDef::new(Organizations::CreatedAt)
            .big_integer()
                .not_null(),
        )
        .col(ColumnDef::new(Organizations::UpdatedAt).big_integer().not_null())
        .to_owned()
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
pub(super) enum Organizations {
    Table,
    Identifier,
    OrgName,
    OrgType,
    CreatedAt,
    UpdatedAt,
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn postgres() {
        collapsed_eq!(
            &create_organizations_table_statement().to_string(PostgresQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "organizations" (
                "identifier" varchar(256) NOT NULL PRIMARY KEY,
                "org_name" varchar(200) NOT NULL,
                "org_type" smallint NOT NULL,
                "created_at" bigint NOT NULL,
                "updated_at" bigint NOT NULL
            )"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &create_organizations_table_statement().to_string(MysqlQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS `organizations` (
                `identifier` varchar(256) NOT NULL PRIMARY KEY,
                `org_name` varchar(200) NOT NULL,
                `org_type` smallint NOT NULL,
                `created_at` bigint NOT NULL,
                `updated_at` bigint NOT NULL
            )"#
        );
    }

    #[test]
    fn sqlite() {
        collapsed_eq!(
            &create_organizations_table_statement().to_string(SqliteQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "organizations" (
                "identifier" varchar(256) NOT NULL PRIMARY KEY,
                "org_name" varchar(200) NOT NULL,
                "org_type" smallint NOT NULL,
                "created_at" bigint NOT NULL,
                "updated_at" bigint NOT NULL
            )"#
        );
    }
}
