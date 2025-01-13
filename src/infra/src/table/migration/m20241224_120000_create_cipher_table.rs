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
            .create_table(create_cipher_table_statement())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CipherKeys::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Statement to create the cipher keys table.
fn create_cipher_table_statement() -> TableCreateStatement {
    Table::create()
        .table(CipherKeys::Table)
        .if_not_exists()
        .col(ColumnDef::new(CipherKeys::Org).string_len(100).not_null())
        .col(ColumnDef::new(CipherKeys::CreatedBy).string_len(256).not_null())
        .col(ColumnDef::new(CipherKeys::CreatedAt).big_integer().not_null())
        .col(ColumnDef::new(CipherKeys::Name).string_len(256).not_null())
        // kind is more of an extension mechanism, if we want to store things other than
        // cipher keys, we can differentiate based on the kind
        .col(ColumnDef::new(CipherKeys::Kind).string_len(100).not_null())
        .col(ColumnDef::new(CipherKeys::Data).text().not_null())
        .primary_key(Index::create().col(CipherKeys::Org).col(CipherKeys::Name).col(CipherKeys::Kind))
        .to_owned()
}

/// Identifiers used in queries on the cipher keys table.
#[derive(DeriveIden)]
enum CipherKeys {
    Table,
    Org,
    CreatedBy,
    CreatedAt,
    Name,
    Kind,
    Data,
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn postgres() {
        collapsed_eq!(
            &create_cipher_table_statement().to_string(PostgresQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS "cipher_keys" ( 
            "org" varchar(100) NOT NULL, 
            "created_by" varchar(256) NOT NULL, 
            "created_at" bigint NOT NULL, 
            "name" varchar(256) NOT NULL, 
            "kind" varchar(100) NOT NULL, 
            "data" text NOT NULL,
            PRIMARY KEY ("org", "name", "kind")
            )"#
        );
    }

    #[test]
    fn mysql() {
        collapsed_eq!(
            &create_cipher_table_statement().to_string(MysqlQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS `cipher_keys` ( 
            `org` varchar(100) NOT NULL, 
            `created_by` varchar(256) NOT NULL, 
            `created_at` bigint NOT NULL, 
            `name` varchar(256) NOT NULL, 
            `kind` varchar(100) NOT NULL, 
            `data` text NOT NULL,
            PRIMARY KEY (`org`, `name`, `kind`)
            )"#
        );
    }

    #[test]
    fn sqlite() {
        collapsed_eq!(
            &create_cipher_table_statement().to_string(SqliteQueryBuilder),
            r#"CREATE TABLE IF NOT EXISTS "cipher_keys" ( 
            "org" varchar(100) NOT NULL, 
            "created_by" varchar(256) NOT NULL, 
            "created_at" bigint NOT NULL, 
            "name" varchar(256) NOT NULL, 
            "kind" varchar(100) NOT NULL, 
            "data" text NOT NULL,
            PRIMARY KEY ("org", "name", "kind")
            )"#
        );
    }
}
