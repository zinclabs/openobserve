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

use infra::{
    errors::Error,
    table::ratelimit::{RatelimitRule, RULE_EXISTS, RULE_NOT_FOUND},
};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};

pub async fn process(msg: Message) -> Result<(), anyhow::Error> {
    log::debug!(
        "[SUPER_CLUSTER:RATELIMIT] LOCAL_NODE:{:?}, Processing message: {:?}",
        config::cluster::LOCAL_NODE,
        msg.key
    );

    let bytes = msg
        .value
        .ok_or(Error::Message("Message missing value".to_string()))?;
    let rule = RatelimitRule::try_from(&bytes)?;

    match msg.message_type {
        MessageType::RatelimitAdd => match infra::table::ratelimit::add(rule).await {
            Ok(_) => Ok(()),
            Err(e) if e.to_string() == RULE_EXISTS => {
                log::warn!("[SUPER_CLUSTER:RATELIMIT] Rule already exists, ignoring");
                Ok(())
            }
            Err(e) => Err(e),
        },
        MessageType::RatelimitUpdate => match infra::table::ratelimit::update(rule).await {
            Ok(_) => Ok(()),
            Err(e) if e.to_string() == RULE_NOT_FOUND => {
                log::warn!("[SUPER_CLUSTER:RATELIMIT] Rule not found for update, ignoring");
                Ok(())
            }
            Err(e) => Err(e),
        },
        MessageType::RatelimitDelete => {
            match infra::table::ratelimit::delete(rule.rule_id.unwrap()).await {
                Ok(_) => Ok(()),
                Err(e) if e.to_string() == RULE_NOT_FOUND => {
                    log::warn!("[SUPER_CLUSTER:RATELIMIT] Rule not found for deletion, ignoring");
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        _ => {
            log::error!(
                "[SUPER_CLUSTER:RATELIMIT] Invalid message: type: {:?}, key: {}",
                msg.message_type,
                msg.key
            );
            Err(anyhow::anyhow!("Invalid message type".to_string()))
        }
    }
}
