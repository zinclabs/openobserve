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

use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};

use async_trait::async_trait;
use config::{
    meta::search::ScanStats,
    utils::time::{now_micros, second_micros},
};
use datafusion::{arrow::datatypes::Schema, error::DataFusionError, prelude::SessionContext};
use infra::{cache::tmpfs, errors::Result};
use promql_parser::{label::Matchers, parser};
use proto::cluster_rpc;

use super::Value;
use crate::service::{
    promql::{value, PromqlContext, TableProvider, DEFAULT_LOOKBACK},
    search,
};

mod storage;
mod wal;

struct StorageProvider {
    trace_id: String,
    need_wal: bool,
}

#[async_trait]
impl TableProvider for StorageProvider {
    async fn create_context(
        &self,
        org_id: &str,
        stream_name: &str,
        time_range: (i64, i64),
        matchers: Matchers,
        label_selector: Option<HashSet<String>>,
        filters: &mut [(String, Vec<String>)],
    ) -> datafusion::error::Result<Vec<(SessionContext, Arc<Schema>, ScanStats)>> {
        let mut resp = Vec::new();
        // register storage table
        let trace_id = self.trace_id.to_owned() + "-storage-" + stream_name;
        let ctx = storage::create_context(
            &trace_id,
            org_id,
            stream_name,
            time_range,
            matchers.clone(),
            filters,
        )
        .await?;
        resp.push(ctx);
        // register Wal table
        if self.need_wal {
            let trace_id = self.trace_id.to_owned() + "-wal-" + stream_name;
            let wal_ctx_list = wal::create_context(
                &trace_id,
                org_id,
                stream_name,
                time_range,
                matchers,
                label_selector,
            )
            .await?;
            for ctx in wal_ctx_list {
                resp.push(ctx);
            }
        }
        Ok(resp)
    }
}

#[tracing::instrument(name = "promql:search:grpc:search", skip_all, fields(org_id = req.org_id))]
pub async fn search(
    req: &cluster_rpc::MetricsQueryRequest,
) -> Result<cluster_rpc::MetricsQueryResponse> {
    let cfg = config::get_config();
    let start_time = std::time::Instant::now();
    let query = req.query.as_ref().unwrap();
    let max_interval = cfg.limit.metrics_max_search_interval_per_group * 3_600_000_000; // convert hours to microseconds

    let start = query.start;
    let end = query.end;
    let step = query.step;

    let mut results = Vec::new();
    if start == end {
        results.push(search_inner(req).await?);
    } else {
        let group_interval = max_interval - max_interval % step;
        let group = generate_search_group(start, end, step, group_interval);
        for (start, end) in group {
            let mut req = req.clone();
            req.need_wal =
                end >= now_micros() - second_micros(cfg.limit.max_file_retention_time as i64 * 3);
            req.query.as_mut().unwrap().start = start;
            req.query.as_mut().unwrap().end = end;
            let resp = search_inner(&req).await?;
            results.push(resp);
        }
    }

    let mut resp = cluster_rpc::MetricsQueryResponse {
        job: req.job.clone(),
        took: start_time.elapsed().as_millis() as i32,
        result_type: results[0].1.clone(),
        ..Default::default()
    };

    let mut scan_stats = ScanStats::default();
    for (value, _, stats) in results {
        add_value(&mut resp, value);
        scan_stats.add(&stats);
    }
    resp.scan_stats = Some(cluster_rpc::ScanStats::from(&scan_stats));

    Ok(resp)
}

#[tracing::instrument(name = "promql:search:grpc:search_inner", skip_all, fields(org_id = req.org_id))]
pub async fn search_inner(
    req: &cluster_rpc::MetricsQueryRequest,
) -> Result<(Value, String, ScanStats)> {
    let trace_id = req.job.as_ref().unwrap().trace_id.to_string();

    let org_id = &req.org_id;
    let query = req.query.as_ref().unwrap();
    let prom_expr = parser::parse(&query.query).map_err(|e| {
        log::error!("promQL parse query error: {e}");
        DataFusionError::Execution(e)
    })?;

    let eval_stmt = parser::EvalStmt {
        expr: prom_expr,
        start: UNIX_EPOCH
            .checked_add(Duration::from_micros(query.start as _))
            .unwrap(),
        end: UNIX_EPOCH
            .checked_add(Duration::from_micros(query.end as _))
            .unwrap(),
        interval: Duration::from_micros(query.step as _),
        lookback_delta: DEFAULT_LOOKBACK,
    };

    let timeout = if req.timeout > 0 {
        req.timeout as u64
    } else {
        config::get_config().limit.query_timeout
    };

    let mut ctx = PromqlContext::new(
        org_id,
        StorageProvider {
            trace_id: trace_id.to_string(),
            need_wal: req.need_wal,
        },
        query.query_exemplars,
        timeout,
    );

    let (value, result_type, mut scan_stats) = if query.query_exemplars {
        ctx.query_exemplars(eval_stmt).await?
    } else {
        ctx.exec(eval_stmt).await?
    };
    let result_type = match result_type {
        Some(v) => v,
        None => value.get_type().to_string(),
    };

    // clear session
    search::datafusion::storage::file_list::clear(&trace_id);
    // clear tmpfs
    tmpfs::delete(&trace_id, true).unwrap();

    scan_stats.format_to_mb();
    Ok((value, result_type, scan_stats))
}

/// generate search group
/// if the last group is less than group_interval * 25%, it will be merged into the previous group
fn generate_search_group(start: i64, end: i64, step: i64, group_interval: i64) -> Vec<(i64, i64)> {
    let mut resp = Vec::new();
    let mut start = start;
    while start < end {
        let next = start + group_interval;
        if next >= end - group_interval / 4 {
            resp.push((start, end));
            break;
        }
        resp.push((start, next));
        start = next + step;
    }
    resp
}

pub(crate) fn add_value(resp: &mut cluster_rpc::MetricsQueryResponse, value: Value) {
    match value {
        value::Value::None => {}
        value::Value::Instant(v) => {
            resp.result.push(cluster_rpc::Series {
                metric: v.labels.iter().map(|x| x.as_ref().into()).collect(),
                sample: Some((&v.sample).into()),
                ..Default::default()
            });
        }
        value::Value::Range(v) => {
            resp.result.push(cluster_rpc::Series {
                metric: v.labels.iter().map(|x| x.as_ref().into()).collect(),
                samples: v.samples.iter().map(|x| x.into()).collect(),
                ..Default::default()
            });
        }
        value::Value::Vector(v) => {
            v.iter().for_each(|v| {
                resp.result.push(cluster_rpc::Series {
                    metric: v.labels.iter().map(|x| x.as_ref().into()).collect(),
                    sample: Some((&v.sample).into()),
                    ..Default::default()
                });
            });
        }
        value::Value::Matrix(v) => {
            v.iter().for_each(|v| {
                let samples = v
                    .samples
                    .iter()
                    .filter_map(|x| if x.is_nan() { None } else { Some(x.into()) })
                    .collect::<Vec<_>>();
                let exemplars = v.exemplars.as_ref().map(|v| {
                    let exemplars = v.iter().map(|x| x.into()).collect::<Vec<_>>();
                    cluster_rpc::Exemplars { exemplars }
                });
                if !samples.is_empty() || exemplars.is_some() {
                    resp.result.push(cluster_rpc::Series {
                        metric: v.labels.iter().map(|x| x.as_ref().into()).collect(),
                        samples,
                        exemplars,
                        ..Default::default()
                    });
                }
            });
        }
        value::Value::Sample(v) => {
            resp.result.push(cluster_rpc::Series {
                sample: Some((&v).into()),
                ..Default::default()
            });
        }
        value::Value::Float(v) => {
            resp.result.push(cluster_rpc::Series {
                scalar: Some(v),
                ..Default::default()
            });
        }
        value::Value::String(v) => {
            resp.result.push(cluster_rpc::Series {
                stringliteral: Some(v),
                ..Default::default()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_search_group() {
        // test case 1: normal case
        let resp = generate_search_group(0, 10, 2, 4);
        let expected = vec![(0, 4), (6, 10)];
        assert_eq!(resp, expected);

        // test case 2: start == end
        let resp = generate_search_group(0, 0, 2, 4);
        let expected = vec![];
        assert_eq!(resp, expected);

        // test case 3: start > end
        let resp = generate_search_group(10, 0, 2, 4);
        let expected = vec![];
        assert_eq!(resp, expected);

        // test case 4, the last group is greater than group_interval * 25%
        let resp = generate_search_group(0, 11, 2, 8);
        let expected = vec![(0, 8), (10, 11)];
        assert_eq!(resp, expected);

        // test case 5, the last group is less than group_interval * 25%
        let resp = generate_search_group(0, 10, 2, 8);
        let expected = vec![(0, 10)];
        assert_eq!(resp, expected);
    }
}
