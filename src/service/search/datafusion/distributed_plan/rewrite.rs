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

use config::meta::{cluster::NodeInfo, inverted_index::InvertedIndexOptimizeMode, stream::FileKey};
use datafusion::{
    common::{
        tree_node::{Transformed, TreeNode, TreeNodeRecursion, TreeNodeRewriter, TreeNodeVisitor},
        Result, TableReference,
    },
    physical_expr::LexOrdering,
    physical_plan::{
        repartition::RepartitionExec,
        sorts::{sort::SortExec, sort_preserving_merge::SortPreservingMergeExec},
        ExecutionPlan, ExecutionPlanProperties, Partitioning,
    },
};
use hashbrown::HashMap;
use proto::cluster_rpc::KvItem;

use super::{
    empty_exec::NewEmptyExec, node::RemoteScanNodes, remote_scan::RemoteScanExec,
    streaming_aggs_exec,
};
use crate::service::search::{index::IndexCondition, request::Request};

// add remote scan to physical plan
pub struct RemoteScanRewriter {
    pub remote_scan_nodes: RemoteScanNodes,
    pub is_changed: bool,
}

impl RemoteScanRewriter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        req: Request,
        nodes: Vec<Arc<dyn NodeInfo>>,
        file_id_lists: HashMap<TableReference, Vec<Vec<i64>>>,
        idx_file_list: Vec<FileKey>,
        equal_keys: HashMap<TableReference, Vec<KvItem>>,
        match_all_keys: Vec<String>,
        index_condition: Option<IndexCondition>,
        index_optimizer_mode: Option<InvertedIndexOptimizeMode>,
        is_leader: bool,
        opentelemetry_context: opentelemetry::Context,
    ) -> Self {
        Self {
            remote_scan_nodes: RemoteScanNodes::new(
                req,
                nodes,
                file_id_lists,
                idx_file_list,
                equal_keys,
                match_all_keys,
                index_condition,
                index_optimizer_mode,
                is_leader,
                opentelemetry_context,
            ),
            is_changed: false,
        }
    }
}

impl TreeNodeRewriter for RemoteScanRewriter {
    type Node = Arc<dyn ExecutionPlan>;

    fn f_up(&mut self, node: Arc<dyn ExecutionPlan>) -> Result<Transformed<Self::Node>> {
        if node.name() == "RepartitionExec" || node.name() == "CoalescePartitionsExec" {
            let mut visitor = TableNameVisitor::new();
            node.visit(&mut visitor)?;
            if visitor.is_remote_scan {
                let table_name = visitor.table_name.clone().unwrap();
                let input = node.children()[0];
                let remote_scan = Arc::new(RemoteScanExec::new(
                    input.clone(),
                    self.remote_scan_nodes.get_remote_node(&table_name),
                )?);
                let output_partitioning =
                    Partitioning::RoundRobinBatch(input.output_partitioning().partition_count());
                let repartition =
                    Arc::new(RepartitionExec::try_new(remote_scan, output_partitioning)?);
                let new_node = node.with_new_children(vec![repartition])?;
                self.is_changed = true;
                return Ok(Transformed::yes(new_node));
            }
        } else if node.name() == "SortPreservingMergeExec" {
            let mut visitor = TableNameVisitor::new();
            node.visit(&mut visitor)?;
            if visitor.is_remote_scan {
                let table_name = visitor.table_name.clone().unwrap();
                let follow_merge_node = node.clone();
                let new_input =
                    follow_merge_node.with_new_children(vec![node.children()[0].clone()])?;

                let remote_scan = Arc::new(RemoteScanExec::new(
                    new_input,
                    self.remote_scan_nodes.get_remote_node(&table_name),
                )?);
                let new_node = node.with_new_children(vec![remote_scan])?;
                self.is_changed = true;
                return Ok(Transformed::yes(new_node));
            }
        } else if node.name() == "UnionExec" {
            let mut visitor = TableNameVisitor::new();
            node.visit(&mut visitor)?;
            // add each remote scan for each child
            if visitor.is_remote_scan {
                let mut new_children: Vec<Arc<dyn ExecutionPlan>> = vec![];
                for child in node.children() {
                    let mut visitor = TableNameVisitor::new();
                    child.visit(&mut visitor)?;
                    // For sort, we should add a SortPreservingMergeExec
                    if child.name() == "SortExec" {
                        let table_name = visitor.table_name.clone().unwrap();
                        let sort = child.as_any().downcast_ref::<SortExec>().unwrap();
                        let sort_merge = Arc::new(
                            SortPreservingMergeExec::new(
                                LexOrdering::new(sort.expr().to_vec()),
                                Arc::new(sort.clone()),
                            )
                            .with_fetch(sort.fetch()),
                        );
                        let remote_scan = Arc::new(RemoteScanExec::new(
                            sort_merge,
                            self.remote_scan_nodes.get_remote_node(&table_name),
                        )?);
                        new_children.push(remote_scan);
                    } else {
                        let table_name = visitor.table_name.clone().unwrap();
                        let remote_scan = Arc::new(RemoteScanExec::new(
                            child.clone(),
                            self.remote_scan_nodes.get_remote_node(&table_name),
                        )?);
                        new_children.push(remote_scan);
                    }
                }
                let new_node = node.with_new_children(new_children)?;
                self.is_changed = true;
                return Ok(Transformed::yes(new_node));
            }
        }
        Ok(Transformed::no(node))
    }
}

// visit physical plan to get underlying table name and check is add a remote scan after current
// physical plan
struct TableNameVisitor {
    table_name: Option<TableReference>,
    is_remote_scan: bool, // is add remote scan after current physical plan
}

impl TableNameVisitor {
    pub fn new() -> Self {
        Self {
            table_name: None,
            is_remote_scan: true,
        }
    }
}

impl<'n> TreeNodeVisitor<'n> for TableNameVisitor {
    type Node = Arc<dyn ExecutionPlan>;

    fn f_up(&mut self, node: &'n Self::Node) -> Result<TreeNodeRecursion> {
        let name = node.name();
        if name == "RemoteScanExec" {
            self.is_remote_scan = false;
            Ok(TreeNodeRecursion::Stop)
        } else if name == "NewEmptyExec" {
            let table = node.as_any().downcast_ref::<NewEmptyExec>().unwrap();
            self.table_name = Some(TableReference::from(table.name()));
            Ok(TreeNodeRecursion::Continue)
        } else {
            Ok(TreeNodeRecursion::Continue)
        }
    }
}

pub struct StreamingAggsRewriter {
    id: String,
    start_time: i64,
    end_time: i64,
}

impl StreamingAggsRewriter {
    pub fn new(id: String, start_time: i64, end_time: i64) -> Self {
        Self {
            id,
            start_time,
            end_time,
        }
    }
}

impl TreeNodeRewriter for StreamingAggsRewriter {
    type Node = Arc<dyn ExecutionPlan>;

    fn f_up(&mut self, node: Arc<dyn ExecutionPlan>) -> Result<Transformed<Self::Node>> {
        if node.name() == "RemoteScanExec"
            && !node.children().is_empty()
            && node.children().first().unwrap().name() == "AggregateExec"
            && config::get_config().common.feature_query_streaming_aggs
        {
            let cached_data = streaming_aggs_exec::GLOBAL_CACHE
                .get(&self.id)
                .unwrap_or_default();
            let streaming_node: Arc<dyn ExecutionPlan> =
                Arc::new(streaming_aggs_exec::StreamingAggsExec::new(
                    self.id.clone(),
                    self.start_time,
                    self.end_time,
                    cached_data,
                    node,
                )) as _;
            return Ok(Transformed::yes(streaming_node));
        }
        Ok(Transformed::no(node))
    }
}
