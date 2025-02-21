// Copyright 2025 OpenObserve Inc.
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

use anyhow::{anyhow, Context};
use bytes::Bytes;
use config::utils::time::now;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{connect_to_orm, ORM_CLIENT},
    orm_err,
    table::{
        entity::rate_limit_rules::{ActiveModel, Column, Entity},
        migration::{Expr, Order},
    },
};

#[derive(FromQueryResult, Default, Debug, Clone, Serialize, Deserialize)]
pub struct RatelimitRule {
    pub org: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    pub resource: String,
    pub threshold: f64,
}
impl TryFrom<&Bytes> for RatelimitRule {
    type Error = anyhow::Error;

    fn try_from(bytes: &Bytes) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes).context("Failed to deserialize RatelimitRule")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RatelimitRuleType {
    Exact,
    Regex,
}

impl From<&str> for RatelimitRuleType {
    fn from(s: &str) -> Self {
        match s {
            "exact" => RatelimitRuleType::Exact,
            "regex" => RatelimitRuleType::Regex,
            _ => panic!("Invalid RatelimitRuleType"),
        }
    }
}

impl std::fmt::Display for RatelimitRuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RatelimitRuleType::Exact => write!(f, "exact"),
            RatelimitRuleType::Regex => write!(f, "regex"),
        }
    }
}

pub const RULE_EXISTS: &str = "Rule already exists";
pub const RULE_NOT_FOUND: &str = "Rule not found";
pub async fn fetch_rules() -> Result<Vec<RatelimitRule>, anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let res = Entity::find()
        .select_only()
        .column(Column::Org)
        .column(Column::RuleId)
        .column(Column::RuleType)
        .column(Column::Resource)
        .column(Column::Threshold)
        .order_by(Column::CreatedAt, Order::Desc);
    let records = res
        .into_model::<RatelimitRule>()
        .all(client)
        .await
        .map_err(|e| anyhow!("DbError# {e}"))?;
    Ok(records)
}

pub async fn fetch_rules_by_id(rule_id: &str) -> Result<Option<RatelimitRule>, anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let res = Entity::find()
        .select_only()
        .column(Column::Org)
        .column(Column::RuleId)
        .column(Column::RuleType)
        .column(Column::Resource)
        .column(Column::Threshold)
        .filter(Column::RuleId.eq(rule_id));
    let record = res
        .into_model::<RatelimitRule>()
        .one(client)
        .await
        .map_err(|e| anyhow!("DbError# {e}"))?;
    Ok(record)
}

pub async fn add(rule: RatelimitRule) -> Result<(), anyhow::Error> {
    let rule_id = rule.rule_id.clone();
    match fetch_rules_by_id(rule_id.unwrap().as_str()).await {
        Ok(None) => {
            let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
            let active_rule = ActiveModel {
                org: Set(rule.org),
                rule_id: Set(rule.rule_id.unwrap()),
                rule_type: Set(rule
                    .rule_type
                    .unwrap_or(RatelimitRuleType::Exact.to_string())),
                resource: Set(rule.resource),
                threshold: Set(rule.threshold),
                created_at: Set(now().timestamp()),
            };
            match Entity::insert(active_rule)
                .exec(client)
                .await
                .map_err(|e| anyhow!("DbError# {e}"))
            {
                Ok(_) => Ok(()),
                Err(e) => orm_err!(format!("Add ratelimit rule error: {e}"))
                    .map_err(|e| anyhow!("DbError# {e}"))?,
            }
        }
        Ok(Some(_)) => Err(anyhow::anyhow!(RULE_EXISTS)),
        Err(e) => {
            log::error!("Add Rule Error fetching rule: {:?}", e.to_string());
            Err(anyhow::anyhow!(e.to_string()))
        }
    }
}

pub async fn update(rule: RatelimitRule) -> Result<(), anyhow::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let _ = match Entity::update_many()
        .col_expr(Column::Org, Expr::value(&rule.org))
        .col_expr(
            Column::RuleType,
            Expr::value(
                rule.rule_type
                    .unwrap_or(RatelimitRuleType::Exact.to_string()),
            ),
        )
        .col_expr(Column::Resource, Expr::value(&rule.resource))
        .col_expr(Column::Threshold, Expr::value(rule.threshold))
        .filter(Column::RuleId.eq(rule.rule_id.unwrap()))
        .exec(client)
        .await
        .map_err(|e| anyhow!("DbError# {e}"))
    {
        Ok(res) => res,
        Err(e) => {
            return orm_err!(format!("update ratelimit rule error: {e}"))
                .map_err(|e| anyhow!("DbError# {e}"))?
        }
    };

    Ok(())
}

pub async fn delete(rule_id: String) -> Result<(), anyhow::Error> {
    match fetch_rules_by_id(rule_id.as_str()).await {
        Ok(Some(_)) => {
            let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
            match Entity::delete_many()
                .filter(Column::RuleId.eq(rule_id))
                .exec(client)
                .await
                .map_err(|e| anyhow!("DbError# {e}"))
            {
                Ok(_) => Ok(()),
                Err(e) => orm_err!(format!("Delete ratelimit rule error: {e}"))
                    .map_err(|e| anyhow!("DbError# {e}"))?,
            }
        }
        Ok(None) => Err(anyhow::anyhow!(RULE_NOT_FOUND)),
        Err(e) => {
            log::error!("Delete rule Error fetching rule: {:?}", e.to_string());
            Err(anyhow::anyhow!(e.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use sea_orm::{DatabaseBackend, DbErr, MockDatabase, MockExecResult, Transaction};
    use serde_json::json;

    use super::*;
    use crate::table::entity::rate_limit_rules::Model;

    fn create_test_rule() -> RatelimitRule {
        RatelimitRule {
            org: "test_org".to_string(),
            rule_type: Some("exact".to_string()),
            rule_id: Some("test_rule".to_string()),
            resource: "test_resource".to_string(),
            threshold: 100.0,
        }
    }

    fn create_test_model() -> Model {
        Model {
            org: "test_org".to_string(),
            rule_type: "exact".to_string(),
            rule_id: "test_rule".to_string(),
            resource: "test_resource".to_string(),
            threshold: 100.0,
            created_at: 0,
        }
    }

    #[tokio::test]
    async fn test_fetch_rules() -> Result<(), DbErr> {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_test_model()]])
            .into_connection();

        // Override the ORM_CLIENT for testing
        let rules = Entity::find()
            .select_only()
            .column(Column::Org)
            .column(Column::RuleId)
            .column(Column::RuleType)
            .column(Column::Resource)
            .column(Column::Threshold)
            .order_by(Column::CreatedAt, Order::Desc)
            .into_model::<RatelimitRule>()
            .all(&db)
            .await?;

        assert_eq!(rules.len(), 1);
        let rule = &rules[0];
        assert_eq!(rule.org, "test_org");
        assert_eq!(rule.rule_type, Some("exact".to_string()));
        assert_eq!(rule.rule_id, Some("test_rule".to_string()));
        assert_eq!(rule.resource, "test_resource");
        assert_eq!(rule.threshold, 100.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_rules_by_id() {
        let test_rule = create_test_model();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![test_rule.clone()], vec![]])
            .into_connection();

        let result = Entity::find()
            .select_only()
            .column(Column::Org)
            .column(Column::RuleId)
            .column(Column::RuleType)
            .column(Column::Resource)
            .column(Column::Threshold)
            .filter(Column::RuleId.eq("test_rule"))
            .into_model::<RatelimitRule>()
            .one(&db)
            .await;

        assert!(result.is_ok());
        let rule = result.unwrap().unwrap();
        assert_eq!(rule.rule_id, Some("test_rule".to_string()));
        assert_eq!(rule.org, "test_org");

        let result = Entity::find()
            .select_only()
            .column(Column::Org)
            .column(Column::RuleId)
            .column(Column::RuleType)
            .column(Column::Resource)
            .column(Column::Threshold)
            .filter(Column::RuleId.eq("non_existent"))
            .into_model::<RatelimitRule>()
            .one(&db)
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_add_rule() -> Result<(), DbErr> {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![create_test_model()]])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let rule = create_test_rule();
        let active_rule = ActiveModel {
            org: Set(rule.org.clone()),
            rule_id: Set(rule.rule_id.clone().unwrap()),
            rule_type: Set(rule.rule_type.clone().unwrap()),
            resource: Set(rule.resource.clone()),
            threshold: Set(rule.threshold),
            created_at: Default::default(),
        };

        // Test insert
        let result = Entity::insert(active_rule).exec(&db).await;

        // Verify the result
        assert!(result.is_ok());

        // Verify the mock database received the expected queries
        assert_eq!(
            db.into_transaction_log(),
            vec![Transaction::from_sql_and_values(
                DatabaseBackend::Postgres,
                r#"INSERT INTO "rate_limit_rules" ("rule_id", "org", "rule_type", "resource", "threshold") VALUES ($1, $2, $3, $4, $5) RETURNING "rule_id""#,
                vec![
                    rule.rule_id.into(),
                    rule.org.into(),
                    rule.rule_type.into(),
                    rule.resource.into(),
                    rule.threshold.into(),
                ]
            ),]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_update_rule() -> Result<(), DbErr> {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection();

        let rule = create_test_rule();

        // Test update
        let result = Entity::update_many()
            .col_expr(Column::Org, Expr::value(&rule.org))
            .col_expr(Column::RuleType, Expr::value(&rule.rule_type.unwrap()))
            .col_expr(Column::Resource, Expr::value(&rule.resource))
            .col_expr(Column::Threshold, Expr::value(rule.threshold))
            .filter(Column::RuleId.eq(&rule.rule_id.unwrap()))
            .exec(&db)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_rule() -> Result<(), DbErr> {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection();

        let rule = create_test_rule();

        // Test delete
        let result = Entity::delete_many()
            .filter(Column::RuleId.eq(&rule.rule_id.unwrap()))
            .exec(&db)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().rows_affected, 1);

        Ok(())
    }

    #[test]
    fn test_try_from_bytes() {
        // Test valid JSON
        let json_data = json!({
            "org": "test_org",
            "rule_type": "exact",
            "rule_id": "test_rule",
            "resource": "test_resource",
            "threshold": 100.0
        });
        let bytes = Bytes::from(json_data.to_string());

        let result = RatelimitRule::try_from(&bytes);
        assert!(result.is_ok());

        let rule = result.unwrap();
        assert_eq!(rule.org, "test_org");
        assert_eq!(rule.rule_type, Some("exact".to_string()));
        assert_eq!(rule.rule_id, Some("test_rule".to_string()));
        assert_eq!(rule.resource, "test_resource");
        assert_eq!(rule.threshold, 100.0);

        // Test invalid JSON
        let invalid_bytes = Bytes::from("invalid json");
        let result = RatelimitRule::try_from(&invalid_bytes);
        assert!(result.is_err());

        // Test missing fields
        let invalid_json = json!({
            "org": "test_org"
        });
        let bytes = Bytes::from(invalid_json.to_string());
        let result = RatelimitRule::try_from(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_ratelimit_rule_type() {
        assert!(matches!(
            RatelimitRuleType::from("exact"),
            RatelimitRuleType::Exact
        ));
        assert!(matches!(
            RatelimitRuleType::from("regex"),
            RatelimitRuleType::Regex
        ));

        assert_eq!(RatelimitRuleType::Exact.to_string(), "exact");
        assert_eq!(RatelimitRuleType::Regex.to_string(), "regex");
    }
}
