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

const USER_EMAIL_IDX: &str = "users_email_idx";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(create_users_table_statement()).await?;
        manager.create_index(create_users_email_idx_stmnt()).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name(USER_EMAIL_IDX).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the folder table.
fn create_users_table_statement() -> TableCreateStatement {
    Table::create()
        .table(Users::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Users::Id)
                // Ksiud is a unique id string with 27 characters.
                .char_len(27)
                .not_null()
                .primary_key(),
        )
        .col(ColumnDef::new(Users::Email).string_len(100).not_null())
        .col(ColumnDef::new(Users::FirstName).string_len(100).not_null())
        .col(ColumnDef::new(Users::LastName).string_len(100).not_null())
        .col(ColumnDef::new(Users::Password).string_len(256).not_null())
        .col(ColumnDef::new(Users::Salt).string_len(256).not_null())
        .col(ColumnDef::new(Users::IsRoot).boolean().not_null())
        .col(ColumnDef::new(Users::PasswordExt).string_len(256))
        // Users type where...
        // - 0 is a internal user
        // - 1 is a external user
        .col(ColumnDef::new(Users::UserType).small_integer().not_null())
        .col(
            ColumnDef::new(Users::CreatedAt)
            .big_integer()
                .not_null(),
        )
        .col(ColumnDef::new(Users::UpdatedAt).big_integer().not_null())
        .to_owned()
}

/// Statement to create index on org.
fn create_users_email_idx_stmnt() -> IndexCreateStatement {
    sea_query::Index::create()
        .if_not_exists()
        .name(USER_EMAIL_IDX)
        .table(Users::Table)
        .unique()
        .col(Users::Email)
        .to_owned()
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
pub(super) enum Users {
    Table,
    Id,
    Email,
    FirstName,
    LastName,
    Password,
    Salt,
    IsRoot,
    PasswordExt,
    UserType,
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
            &create_users_table_statement().to_string(PostgresQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "users" (
                "id" char(27) NOT NULL PRIMARY KEY,
                "email" varchar(100) NOT NULL,
                "first_name" varchar(100) NOT NULL,
                "last_name" varchar(100) NOT NULL,
                "password" varchar(256) NOT NULL,
                "salt" varchar(256) NOT NULL,
                "is_root" bool NOT NULL,
                "password_ext" varchar(256),
                "user_type" smallint NOT NULL,
                "created_at" bigint NOT NULL,
                "updated_at" bigint NOT NULL
            )"#
        );
        assert_eq!(
            &create_users_email_idx_stmnt().to_string(PostgresQueryBuilder),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "users_email_idx" ON "users" ("email")"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &create_users_table_statement().to_string(MysqlQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS `users` (
                `id` char(27) NOT NULL PRIMARY KEY,
                `email` varchar(100) NOT NULL,
                `first_name` varchar(100) NOT NULL,
                `last_name` varchar(100) NOT NULL,
                `password` varchar(256) NOT NULL,
                `salt` varchar(256) NOT NULL,
                `is_root` bool NOT NULL,
                `password_ext` varchar(256),
                `user_type` smallint NOT NULL,
                `created_at` bigint NOT NULL,
                `updated_at` bigint NOT NULL
            )"#
        );
        assert_eq!(
            &create_users_email_idx_stmnt().to_string(MysqlQueryBuilder),
            r#"CREATE UNIQUE INDEX `users_email_idx` ON `users` (`email`)"#
        );
    }

    #[test]
    fn sqlite() {
        collapsed_eq!(
            &create_users_table_statement().to_string(SqliteQueryBuilder),
            r#"
                CREATE TABLE IF NOT EXISTS "users" (
                "id" char(27) NOT NULL PRIMARY KEY,
                "email" varchar(100) NOT NULL,
                "first_name" varchar(100) NOT NULL,
                "last_name" varchar(100) NOT NULL,
                "password" varchar(256) NOT NULL,
                "salt" varchar(256) NOT NULL,
                "is_root" boolean NOT NULL,
                "password_ext" varchar(256),
                "user_type" smallint NOT NULL,
                "created_at" bigint NOT NULL,
                "updated_at" bigint NOT NULL
            )"#
        );
        assert_eq!(
            &create_users_email_idx_stmnt().to_string(SqliteQueryBuilder),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "users_email_idx" ON "users" ("email")"#
        );
    }
}
