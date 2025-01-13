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

use config::meta::timed_annotations::{TimedAnnotation, TimedAnnotationDelete, TimedAnnotationReq};
use infra::table;

#[tracing::instrument]
pub async fn create_timed_annotations(
    org_id: &str,
    req: TimedAnnotationReq,
) -> Result<Vec<TimedAnnotation>, anyhow::Error> {
    let dashboard_id = req.dashboard_id.clone();
    let res =
        table::timed_annotations::add_many(&dashboard_id, org_id, req.timed_annotations).await?;
    // TODO: send WATCH for super cluster
    Ok(res)
}

#[tracing::instrument]
pub async fn get_timed_annotations(
    dashboard_id: &str,
    panels: Option<Vec<String>>,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<TimedAnnotation>, anyhow::Error> {
    let annotations =
        table::timed_annotations::get(dashboard_id, panels, start_time, end_time).await?;
    // TODO: send WATCH for super cluster
    Ok(annotations)
}

#[tracing::instrument]
pub async fn delete_timed_annotations(req: TimedAnnotationDelete) -> Result<(), anyhow::Error> {
    if req.annotation_ids.is_empty() {
        return Err(anyhow::anyhow!("annotation_ids cannot be empty"));
    }
    table::timed_annotations::delete_many(&req.dashboard_id, &req.annotation_ids).await?;
    // TODO: send WATCH for super cluster
    Ok(())
}
