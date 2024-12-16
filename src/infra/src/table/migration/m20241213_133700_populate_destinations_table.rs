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

use std::collections::HashMap;

use config::{ider, utils::json};
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use sea_orm_migration::prelude::*;

use crate::table::entity::templates;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;

        // Get all templates first
        let templates: HashMap<String, String> = templates::Entity::find()
            .all(&txn)
            .await?
            .into_iter()
            .map(|model| (format!("{}-{}", model.org, model.name), model.id))
            .collect();

        // Migrate pages of 100 records at a time to avoid loading too many
        // records into memory.
        let mut meta_pages = meta::Entity::find()
            .filter(meta::Column::Module.eq("destinations"))
            .order_by_asc(meta::Column::Id)
            .paginate(&txn, 100);

        while let Some(metas) = meta_pages.fetch_and_next().await? {
            let new_temp_results: Result<Vec<_>, DbErr> = metas
                .into_iter()
                .map(|meta| {
                    let old_dest: meta_destinations::Destination =
                        json::from_str(&meta.value).map_err(|e| DbErr::Migration(e.to_string()))?;

                    let new_type = match old_dest.destination_type {
                        meta_destinations::DestinationType::Http => {
                            destinations::DestinationType::Http(destinations::Endpoint {
                                url: old_dest.url,
                                method: old_dest.method.into(),
                                skip_tls_verify: old_dest.skip_tls_verify,
                                headers: old_dest.headers,
                            })
                        }
                        meta_destinations::DestinationType::Email => {
                            destinations::DestinationType::Email(destinations::Email {
                                recipients: old_dest.emails,
                            })
                        }
                        meta_destinations::DestinationType::Sns => {
                            destinations::DestinationType::Sns(destinations::AwsSns {
                                sns_topic_arn: old_dest.sns_topic_arn.ok_or(DbErr::Migration(
                                    "SNS destination missing sns_topic_arn".to_string(),
                                ))?,
                                aws_region: old_dest.aws_region.ok_or(DbErr::Migration(
                                    "SNS destination missing aws region info".to_string(),
                                ))?,
                            })
                        }
                    };
                    // let new_type =
                    //     json::to_value(new_type).map_err(|e| DbErr::Migration(e.to_string()))?;

                    let template_id = templates
                        .get(&format!("{}-{}", meta.key1, old_dest.template))
                        .cloned();

                    Ok(destinations::ActiveModel {
                        id: Set(ider::uuid()),
                        org: Set(meta.key1),
                        name: Set(old_dest.name),
                        module: Set("alert".to_string()), // currently only alerts destinations
                        template_id: Set(template_id),
                        pipeline_id: Set(None), // currently only alerts destinations
                        r#type: Set(new_type),  // this doesn't not have the enum tag!!! fix!
                    })
                })
                .collect();
            let new_temps = new_temp_results?;
            destinations::Entity::insert_many(new_temps)
                .exec(&txn)
                .await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        destinations::Entity::delete_many().exec(db).await?;
        Ok(())
    }
}

// The schemas of tables might change after subsequent migrations. Therefore
// this migration only references ORM models in private submodules that should
// remain unchanged rather than ORM models in the `entity` module that will be
// updated to reflect the latest changes to table schemas.

mod meta_destinations {

    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Destination {
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub url: String,
        #[serde(default)]
        pub method: HTTPType,
        #[serde(default)]
        pub skip_tls_verify: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub headers: Option<HashMap<String, String>>,
        pub template: String,
        #[serde(default)]
        pub emails: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub sns_topic_arn: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub aws_region: Option<String>,
        #[serde(rename = "type")]
        #[serde(default)]
        pub destination_type: DestinationType,
    }

    #[derive(Serialize, Debug, Default, PartialEq, Eq, Deserialize, Clone)]
    #[serde(rename_all = "snake_case")]
    pub enum DestinationType {
        #[default]
        Http,
        Email,
        Sns,
    }

    impl std::fmt::Display for DestinationType {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                DestinationType::Http => write!(f, "http"),
                DestinationType::Email => write!(f, "email"),
                DestinationType::Sns => write!(f, "sns"),
            }
        }
    }

    #[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum HTTPType {
        #[default]
        Post,
        Put,
        Get,
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

/// Representation of the destinations table at the time this migration executes.
mod destinations {

    use std::collections::HashMap;

    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "destinations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub org: String,
        pub name: String,
        pub module: String,
        pub template_id: Option<String>,
        pub pipeline_id: Option<String>,
        pub r#type: Json,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::super::super::entity::templates::Entity",
            from = "Column::TemplateId",
            to = "super::super::super::entity::templates::Column::Id",
            on_update = "NoAction",
            on_delete = "NoAction"
        )]
        Templates,
    }

    impl Related<super::super::super::entity::templates::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Templates.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Destination {
        pub id: String,
        pub org_id: String,
        pub name: String,
        pub module: Module,
    }

    #[derive(Serialize, Debug, Deserialize, Clone)]
    #[serde(rename_all = "snake_case")]
    pub enum Module {
        Alert {
            template_id: String,
            destination_type: DestinationType,
        },
        Pipeline {
            pipeline_id: String,
            endpoint: Endpoint,
        },
    }

    #[derive(Serialize, Debug, Deserialize, Clone)]
    #[serde(tag = "type")]
    #[serde(rename_all = "snake_case")]
    pub enum DestinationType {
        Http(Endpoint),
        Email(Email),
        Sns(AwsSns),
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Email {
        pub recipients: Vec<String>,
    }

    #[derive(Serialize, Debug, PartialEq, Eq, Deserialize, Clone)]
    pub struct Endpoint {
        pub url: String,
        #[serde(default)]
        pub method: HTTPType,
        #[serde(default)]
        pub skip_tls_verify: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub headers: Option<HashMap<String, String>>,
    }

    #[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum HTTPType {
        #[default]
        Post,
        Put,
        Get,
    }

    impl std::fmt::Display for HTTPType {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                HTTPType::Post => write!(f, "post"),
                HTTPType::Put => write!(f, "put"),
                HTTPType::Get => write!(f, "get"),
            }
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AwsSns {
        pub sns_topic_arn: String,
        pub aws_region: String,
    }

    #[cfg(test)]
    mod tests {
        use config::utils::json;

        use super::*;

        #[test]
        fn test_enum() {
            let obj: DestinationType = DestinationType::Email(Email {
                recipients: vec!["hello".to_string()],
            });
            let json = json::to_value(obj).unwrap();
            println!("{:?}", json);
        }
    }
}

impl From<meta_destinations::HTTPType> for destinations::HTTPType {
    fn from(value: meta_destinations::HTTPType) -> Self {
        match value {
            meta_destinations::HTTPType::Get => Self::Get,
            meta_destinations::HTTPType::Put => Self::Put,
            meta_destinations::HTTPType::Post => Self::Post,
        }
    }
}
