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

use config::meta::{alerts::alert as meta_alerts, folder as meta_folders, triggers::Trigger};
use serde::Serialize;
use svix_ksuid::Ksuid;
use utoipa::ToSchema;

use super::{Alert, QueryCondition};

/// HTTP response body for `GetAlert` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GetAlertResponseBody(pub Alert);

/// HTTP response body for `ListAlerts` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListAlertsResponseBody {
    pub list: Vec<ListAlertsResponseBodyItem>,
}

/// An item in the list returned by the `ListDashboards` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListAlertsResponseBodyItem {
    pub alert_id: Ksuid,
    pub folder_id: String,
    pub folder_name: String,
    pub name: String,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub condition: QueryCondition,
    pub last_triggered_at: Option<i64>,
    pub last_satisfied_at: Option<i64>,
}

/// HTTP response body for `EnableAlert` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct EnableAlertResponseBody {
    pub enabled: bool,
}

impl From<(meta_alerts::Alert, Option<Trigger>)> for GetAlertResponseBody {
    fn from(value: (meta_alerts::Alert, Option<Trigger>)) -> Self {
        Self(value.into())
    }
}

impl TryFrom<Vec<(meta_folders::Folder, meta_alerts::Alert, Option<Trigger>)>>
    for ListAlertsResponseBody
{
    type Error = ();

    fn try_from(
        value: Vec<(meta_folders::Folder, meta_alerts::Alert, Option<Trigger>)>,
    ) -> Result<Self, Self::Error> {
        let rslt: Result<Vec<_>, _> = value
            .into_iter()
            .map(ListAlertsResponseBodyItem::try_from)
            .collect();
        Ok(Self { list: rslt? })
    }
}

impl TryFrom<(meta_folders::Folder, meta_alerts::Alert, Option<Trigger>)>
    for ListAlertsResponseBodyItem
{
    type Error = ();

    fn try_from(
        value: (meta_folders::Folder, meta_alerts::Alert, Option<Trigger>),
    ) -> Result<Self, Self::Error> {
        let folder = value.0;
        let alert = value.1;
        let trigger = value.2;
        let (last_triggered_at, last_satisfied_at) = (
            alert.get_last_triggered_at(trigger.as_ref()),
            alert.get_last_satisfied_at(trigger.as_ref()),
        );
        Ok(Self {
            alert_id: alert.id.ok_or(())?,
            folder_id: folder.folder_id,
            folder_name: folder.name,
            name: alert.name,
            owner: alert.owner,
            description: Some(alert.description).filter(|d| !d.is_empty()),
            condition: alert.query_condition.into(),
            last_triggered_at,
            last_satisfied_at,
        })
    }
}
