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

pub use config::meta::triggers::{Trigger, TriggerModule, TriggerStatus};
pub use infra::scheduler::TRIGGERS_KEY;
use infra::{
    errors::Result,
    scheduler::{self as infra_scheduler},
};
#[cfg(feature = "enterprise")]
use {
    infra::errors::Error,
    o2_enterprise::enterprise::common::infra::config::get_config as get_o2_config,
    o2_enterprise::enterprise::super_cluster,
};

#[inline]
pub async fn push(trigger: Trigger) -> Result<()> {
    #[cfg(feature = "enterprise")]
    let trigger_clone = trigger.clone();
    infra_scheduler::push(trigger).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::scheduler_push(trigger_clone)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(())
}

#[inline]
pub async fn delete(org: &str, module: TriggerModule, key: &str) -> Result<()> {
    infra_scheduler::delete(org, module.clone(), key).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::scheduler_delete(org, module, key)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(())
}

#[inline]
pub async fn update_trigger(trigger: Trigger) -> Result<()> {
    #[cfg(feature = "enterprise")]
    let trigger_clone = trigger.clone();
    infra_scheduler::update_trigger(trigger).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::scheduler_update(trigger_clone)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(())
}

#[inline]
pub async fn update_status(
    org: &str,
    module: TriggerModule,
    key: &str,
    status: TriggerStatus,
    retries: i32,
    data: Option<&str>,
) -> Result<()> {
    infra_scheduler::update_status(org, module.clone(), key, status.clone(), retries, data).await?;

    // super cluster
    #[cfg(feature = "enterprise")]
    if get_o2_config().super_cluster.enabled {
        super_cluster::queue::scheduler_update_status(org, module, key, status, retries, data)
            .await
            .map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(())
}

#[inline]
pub async fn pull(
    concurrency: i64,
    alert_timeout: i64,
    report_timeout: i64,
) -> Result<Vec<Trigger>> {
    infra_scheduler::pull(concurrency, alert_timeout, report_timeout).await
}

/// Returns the scheduled job associated with the given id in read-only fashion
#[inline]
pub async fn get(org: &str, module: TriggerModule, key: &str) -> Result<Trigger> {
    infra_scheduler::get(org, module, key).await
}

/// Returns the scheduled job associated with the given id in read-only fashion
#[inline]
pub async fn exists(org: &str, module: TriggerModule, key: &str) -> bool {
    infra_scheduler::get(org, module, key).await.is_ok()
}

/// The count of jobs for the given module (Report/Alert etc.)
#[inline]
pub async fn len_module(module: TriggerModule) -> usize {
    infra_scheduler::len_module(module).await
}

#[inline]
pub async fn len() -> usize {
    infra_scheduler::len().await
}

/// List the jobs for the given module
#[inline]
pub async fn list(module: Option<TriggerModule>) -> Result<Vec<Trigger>> {
    infra_scheduler::list(module).await
}

/// List the jobs for the given module
#[inline]
pub async fn list_by_org(org: &str, module: Option<TriggerModule>) -> Result<Vec<Trigger>> {
    infra_scheduler::list_by_org(org, module).await
}

#[inline]
pub async fn is_empty() -> bool {
    infra_scheduler::is_empty().await
}

#[inline]
pub async fn clear() -> Result<()> {
    infra_scheduler::clear().await
}
