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

use config::{meta::stream::StreamParams, utils::schema::format_stream_name};
use infra::errors::Result;
pub mod alerts;
pub mod circuit_breaker;
pub mod compact;
pub mod dashboards;
pub mod db;
pub mod enrichment;
pub mod enrichment_table;
pub mod exporter;
pub mod file_list;
pub mod folders;
pub mod functions;
pub mod grpc;
pub mod ingestion;
pub mod kv;
pub mod logs;
pub mod metadata;
pub mod metrics;
pub mod organization;
pub mod pipeline;
pub mod promql;
#[cfg(feature = "enterprise")]
pub mod ratelimit;
pub mod schema;
pub mod search;
#[cfg(feature = "enterprise")]
pub mod search_jobs;
pub mod self_reporting;
pub mod session;
pub mod short_url;
pub mod stream;
pub mod syslogs_route;
pub mod tls;
pub mod traces;
pub mod users;

// format stream name
pub async fn get_formatted_stream_name(params: StreamParams) -> Result<String> {
    let stream_name = params.stream_name.to_string();
    let schema = infra::schema::get_cache(&params.org_id, &stream_name, params.stream_type).await?;
    Ok(if schema.fields_map().is_empty() {
        format_stream_name(&stream_name)
    } else {
        stream_name
    })
}
