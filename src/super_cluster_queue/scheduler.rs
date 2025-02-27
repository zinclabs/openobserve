// Copyright 2024 Zinc Labs Inc.
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

use config::{meta::triggers::Trigger, utils::json};
use infra::{
    errors::{Error, Result},
    scheduler,
};
use o2_enterprise::enterprise::super_cluster::queue::{Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::SchedulerPush => {
            push(msg).await?;
        }
        MessageType::SchedulerUpdate => {
            update(msg).await?;
        }
        MessageType::SchedulerUpdateStatus => {
            update_status(msg).await?;
        }
        MessageType::SchedulerDelete => {
            delete(msg).await?;
        }
        _ => {
            log::error!(
                "[SUPER_CLUSTER:DB] Invalid message: type: {:?}, key: {}",
                msg.message_type,
                msg.key
            );
            return Err(Error::Message("Invalid message type".to_string()));
        }
    }
    Ok(())
}

async fn push(msg: Message) -> Result<()> {
    let trigger: Trigger = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) = scheduler::push(trigger.clone()).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to push scheduler: {}/{:?}/{}, error: {}",
            trigger.org,
            trigger.module,
            trigger.module_key,
            e
        );
        return Err(e);
    }
    Ok(())
}

async fn update(msg: Message) -> Result<()> {
    let trigger: Trigger = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) = scheduler::update_trigger(trigger.clone()).await {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to update scheduler: {}/{:?}/{}, error: {}",
            trigger.org,
            trigger.module,
            trigger.module_key,
            e
        );
        return Err(e);
    }
    Ok(())
}

async fn update_status(msg: Message) -> Result<()> {
    let trigger: Trigger = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) = scheduler::update_status(
        &trigger.org,
        trigger.module.clone(),
        &trigger.module_key,
        trigger.status,
        trigger.retries,
        // Only update trigger data if it is not empty
        if trigger.data.is_empty() {
            None
        } else {
            Some(&trigger.data)
        },
    )
    .await
    {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to update status for scheduler: {}/{:?}/{}, error: {}",
            trigger.org,
            trigger.module,
            trigger.module_key,
            e
        );
        return Err(e);
    }
    Ok(())
}

async fn delete(msg: Message) -> Result<()> {
    let trigger: Trigger = json::from_slice(&msg.value.unwrap())?;
    if let Err(e) =
        scheduler::delete(&trigger.org, trigger.module.clone(), &trigger.module_key).await
    {
        log::error!(
            "[SUPER_CLUSTER:sync] Failed to delete scheduler: {}/{:?}/{}, error: {}",
            trigger.org,
            trigger.module,
            trigger.module_key,
            e
        );
        return Err(e);
    }
    Ok(())
}
