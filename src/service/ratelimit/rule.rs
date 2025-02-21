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

use std::future::Future;

use anyhow::Context;
use infra::table::ratelimit::RatelimitRule;
use o2_enterprise::enterprise::super_cluster::queue_item::ratelimit::{
    ratelimit_rule_delete, ratelimit_rule_put, ratelimit_rule_update, SUPER_CLUSTER_TOPIC_RATELIMIT,
};

#[derive(Debug, thiserror::Error)]
pub enum RatelimitError {
    #[error("ratelimit rule with ID {0} not found.")]
    NotFound(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug)]
enum RuleOperation {
    Save,
    Update,
    Delete,
}

impl RuleOperation {
    fn as_str(&self) -> &'static str {
        match self {
            RuleOperation::Save => "add",
            RuleOperation::Update => "update",
            RuleOperation::Delete => "delete",
        }
    }

    async fn execute_cluster_operation(
        &self,
        key: String,
        value: bytes::Bytes,
    ) -> Result<(), anyhow::Error> {
        match self {
            RuleOperation::Save => ratelimit_rule_put(key, value).await,
            RuleOperation::Update => ratelimit_rule_update(key, value).await,
            RuleOperation::Delete => ratelimit_rule_delete(key, value).await,
        }
    }
}

async fn handle_rule_operation(
    rule: RatelimitRule,
    operation: RuleOperation,
    db_operation: impl Future<Output = Result<(), anyhow::Error>>,
) -> Result<(), RatelimitError> {
    let db_result = db_operation.await.map_err(RatelimitError::Other);

    #[cfg(feature = "enterprise")]
    if o2_enterprise::enterprise::common::infra::config::get_config()
        .super_cluster
        .enabled
    {
        if let Err(e) = sync_to_super_cluster(&rule, &operation).await {
            log::error!(
                "[Ratelimit] error triggering super cluster event to {} rule: {}",
                operation.as_str(),
                e
            );
        }
    }

    db_result
}

async fn sync_to_super_cluster(
    rule: &RatelimitRule,
    operation: &RuleOperation,
) -> Result<(), anyhow::Error> {
    let value_vec = config::utils::json::to_vec(rule)
        .context("[Ratelimit] error serializing ratelimit rule for super_cluster event")?;

    let key = format!(
        "/{SUPER_CLUSTER_TOPIC_RATELIMIT}/{}",
        rule.rule_id.clone().unwrap_or("".to_string()),
    );

    operation
        .execute_cluster_operation(key, value_vec.into())
        .await
}

pub async fn save(rule: RatelimitRule) -> Result<(), RatelimitError> {
    handle_rule_operation(
        rule.clone(),
        RuleOperation::Save,
        infra::table::ratelimit::add(rule),
    )
    .await
}

pub async fn update(rule: RatelimitRule) -> Result<(), RatelimitError> {
    handle_rule_operation(
        rule.clone(),
        RuleOperation::Update,
        infra::table::ratelimit::update(rule),
    )
    .await
}

pub async fn delete(rule: RatelimitRule) -> Result<(), RatelimitError> {
    handle_rule_operation(
        rule.clone(),
        RuleOperation::Delete,
        infra::table::ratelimit::delete(rule.rule_id.unwrap()),
    )
    .await
}
