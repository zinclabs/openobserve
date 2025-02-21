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

use std::sync::Arc;

use config::{
    meta::{cluster::NodeInfo, inverted_index::InvertedIndexOptimizeMode, stream::FileKey},
    utils::json,
};
use datafusion::common::TableReference;
use hashbrown::HashMap;
use proto::cluster_rpc::{
    IdxFileName, IndexInfo, KvItem, QueryIdentifier, SearchInfo, SuperClusterInfo,
};

use crate::service::search::{
    index::IndexCondition,
    request::{FlightSearchRequest, Request},
};

pub struct RemoteScanNodes {
    pub req: Request,
    pub nodes: Vec<Arc<dyn NodeInfo>>,
    pub file_id_lists: HashMap<TableReference, Vec<Vec<i64>>>,
    pub idx_file_list: Vec<FileKey>,
    pub equal_keys: HashMap<TableReference, Vec<KvItem>>,
    pub match_all_keys: Vec<String>,
    pub index_condition: Option<IndexCondition>,
    pub index_optimize_mode: Option<InvertedIndexOptimizeMode>,
    pub is_leader: bool, // for super cluster
    pub opentelemetry_context: opentelemetry::Context,
}

impl RemoteScanNodes {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        req: Request,
        nodes: Vec<Arc<dyn NodeInfo>>,
        file_id_lists: HashMap<TableReference, Vec<Vec<i64>>>,
        idx_file_list: Vec<FileKey>,
        equal_keys: HashMap<TableReference, Vec<KvItem>>,
        match_all_keys: Vec<String>,
        index_condition: Option<IndexCondition>,
        index_optimize_mode: Option<InvertedIndexOptimizeMode>,
        is_leader: bool,
        opentelemetry_context: opentelemetry::Context,
    ) -> Self {
        Self {
            req,
            nodes,
            file_id_lists,
            idx_file_list,
            equal_keys,
            match_all_keys,
            index_condition,
            index_optimize_mode,
            is_leader,
            opentelemetry_context,
        }
    }

    pub fn get_remote_node(&self, table_name: &TableReference) -> RemoteScanNode {
        let query_identifier = QueryIdentifier {
            trace_id: self.req.trace_id.clone(),
            org_id: self.req.org_id.clone(),
            stream_type: self.req.stream_type.to_string(),
            partition: 0,           // set in FlightSearchRequest
            job_id: "".to_string(), // set in FlightSearchRequest
        };

        let search_infos = SearchInfos {
            plan: vec![], // set in RemoteScanNode
            file_id_list: self
                .file_id_lists
                .get(table_name)
                .unwrap_or(&vec![])
                .clone(),
            idx_file_list: self.idx_file_list.clone(),
            start_time: self.req.time_range.as_ref().map(|x| x.0).unwrap_or(0),
            end_time: self.req.time_range.as_ref().map(|x| x.1).unwrap_or(0),
            timeout: self.req.timeout as u64,
        };

        let index_condition = match &self.index_condition {
            Some(index_condition) => json::to_string(&index_condition).unwrap(),
            None => "".to_string(),
        };

        let index_info = IndexInfo {
            use_inverted_index: self.req.use_inverted_index,
            index_condition,
            equal_keys: self.equal_keys.get(table_name).unwrap_or(&vec![]).clone(),
            match_all_keys: self.match_all_keys.clone(),
            index_optimize_mode: self.index_optimize_mode.clone().map(|x| x.into()),
        };

        let super_cluster_info = SuperClusterInfo {
            is_super_cluster: self.is_leader,
            user_id: self.req.user_id.clone(),
            work_group: self.req.work_group.clone(),
            search_event_type: self.req.search_event_type.clone(),
        };

        RemoteScanNode {
            nodes: self.nodes.clone(),
            opentelemetry_context: self.opentelemetry_context.clone(),
            query_identifier,
            search_infos,
            index_info,
            super_cluster_info,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteScanNode {
    pub nodes: Vec<Arc<dyn NodeInfo>>,
    pub opentelemetry_context: opentelemetry::Context,
    pub query_identifier: QueryIdentifier,
    pub search_infos: SearchInfos,
    pub index_info: IndexInfo,
    pub super_cluster_info: SuperClusterInfo,
}

impl RemoteScanNode {
    pub fn get_flight_search_request(&self, partition: usize) -> FlightSearchRequest {
        FlightSearchRequest {
            query_identifier: self.query_identifier.clone(),
            search_info: self.search_infos.get_search_info(partition), // add is_querier
            index_info: self.index_info.clone(),
            super_cluster_info: self.super_cluster_info.clone(),
        }
    }

    pub fn is_querier(&self, partition: usize) -> bool {
        let file_id_list = if self.search_infos.file_id_list.is_empty() {
            vec![]
        } else {
            self.search_infos.file_id_list[partition].clone()
        };
        !file_id_list.is_empty()
    }

    // used in RemoteScanExec
    // need to set plan before send to follow
    pub fn set_plan(&mut self, plan: Vec<u8>) {
        self.search_infos.plan = plan;
    }

    // need to set to true when from super cluster leader to super cluster follower
    // otherwise set to false
    // used in super cluster follower.rs
    #[cfg(feature = "enterprise")]
    pub fn set_is_super_cluster(&mut self, is_leader: bool) {
        self.super_cluster_info.is_super_cluster = is_leader;
    }

    #[cfg(feature = "enterprise")]
    pub fn from_flight_search_request(
        request: &FlightSearchRequest,
        search_infos: SearchInfos,
        nodes: Vec<Arc<dyn NodeInfo>>,
        opentelemetry_context: opentelemetry::Context,
    ) -> Self {
        Self {
            nodes,
            opentelemetry_context,
            query_identifier: request.query_identifier.clone(),
            search_infos,
            index_info: request.index_info.clone(),
            super_cluster_info: request.super_cluster_info.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchInfos {
    pub plan: Vec<u8>,
    pub file_id_list: Vec<Vec<i64>>,
    pub idx_file_list: Vec<FileKey>,
    pub start_time: i64,
    pub end_time: i64,
    pub timeout: u64,
}

impl SearchInfos {
    pub fn get_search_info(&self, partition: usize) -> SearchInfo {
        let file_id_list = if self.file_id_list.is_empty() {
            vec![]
        } else {
            self.file_id_list[partition].clone()
        };
        let idx_file_list = if !file_id_list.is_empty() {
            self.idx_file_list
                .iter()
                .map(|f| IdxFileName {
                    key: f.key.clone(),
                    segment_ids: f.segment_ids.clone().map(|s| s.into_vec()),
                })
                .collect()
        } else {
            vec![]
        };
        SearchInfo {
            plan: self.plan.clone(),
            file_id_list,
            idx_file_list,
            start_time: self.start_time,
            end_time: self.end_time,
            timeout: self.timeout as i64,
        }
    }
}
