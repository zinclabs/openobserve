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

use infra::table::enrichment_table_jobs::EnrichmentTableJobsRecord;
use serde::{Deserialize, Serialize};

use crate::service::enrichment_table::parse_key;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrichmentTableReq {
    pub file_link: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrichmentTableResp {
    pub task_id: String,
    pub org_id: String,
    pub file_link: String,
    pub task_status: String,
    pub stream_name: String,
    pub created_at: i64,
}

impl TryFrom<EnrichmentTableJobsRecord> for EnrichmentTableResp {
    type Error = anyhow::Error;

    fn try_from(value: EnrichmentTableJobsRecord) -> Result<Self, Self::Error> {
        let (_, stream_name, _) = parse_key(value.file_key.as_str())?;
        Ok(EnrichmentTableResp {
            task_id: value.task_id,
            org_id: value.org_id,
            file_link: value.file_link,
            task_status: value.task_status.into(),
            stream_name,
            created_at: value.created_ts,
        })
    }
}
