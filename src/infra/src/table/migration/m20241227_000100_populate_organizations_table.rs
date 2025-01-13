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

use hashbrown::HashSet;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const DEFAULT_ORG: &str = "default";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let txn = db.begin().await?;
        let mut org_set = HashSet::new();
        // Migrate pages of 100 records at a time to avoid loading too many
        // records into memory.
        // txn.execute()
        let mut meta_pages = meta::Entity::find()
            .filter(meta::Column::Module.eq("schema"))
            .order_by_asc(meta::Column::Id)
            .paginate(&txn, 100);

        while let Some(metas) = meta_pages.fetch_and_next().await? {
            let mut orgs = vec![];
            log::debug!("Processing {} records", metas.len());
            for schema in metas {
                log::debug!(
                    "Processing record -> id: {}, key1: {}, key2: {}",
                    schema.id,
                    schema.key1,
                    schema.key2
                );
                if org_set.contains(&schema.key1) {
                    continue;
                }
                let org_id = schema.key1;
                let org_type = if PartialEq::eq(DEFAULT_ORG, &org_id) {
                    0
                } else {
                    1
                };
                let now = chrono::Utc::now().timestamp_micros() as u64;
                log::debug!("Creating organization: {}", org_id);
                orgs.push(organizations::ActiveModel {
                    identifier: Set(org_id.clone()),
                    org_name: Set(org_id.clone()),
                    org_type: Set(org_type),
                    created_at: Set(now),
                    updated_at: Set(now),
                });
                org_set.insert(org_id);
            }
            if orgs.is_empty() {
                continue;
            }
            organizations::Entity::insert_many(orgs).exec(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        organizations::Entity::delete_many().exec(db).await?;
        Ok(())
    }
}

/// Representation of the meta table at the time this migration executes.
mod meta {
    use sea_orm::entity::prelude::*;

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
}

/// Representation of the folder table at the time this migration executes.
mod organizations {
    use sea_orm::entity::prelude::*;

    // define the organizations table
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "organizations")]
    pub struct Model {
        #[sea_orm(
            primary_key,
            auto_increment = false,
            column_type = "String(StringLen::N(255))"
        )]
        pub identifier: String,
        #[sea_orm(column_type = "String(StringLen::N(255))")]
        pub org_name: String,
        pub org_type: i16,
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
