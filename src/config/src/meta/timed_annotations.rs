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

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TimedAnnotationReq {
    pub dashboard_id: String,
    pub timed_annotations: Vec<TimedAnnotation>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TimedAnnotationRes {
    pub timed_annotation_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TimedAnnotation {
    pub annotation_id: Option<String>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub title: String,
    pub text: Option<String>,
    pub tags: Option<String>,
    pub panels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TimedAnnotationDelete {
    pub dashboard_id: String,
    pub annotation_ids: Vec<String>,
}
