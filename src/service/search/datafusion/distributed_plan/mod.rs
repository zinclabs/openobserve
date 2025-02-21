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

use datafusion::{
    common::{
        tree_node::{Transformed, TreeNodeRecursion, TreeNodeRewriter, TreeNodeVisitor},
        Result,
    },
    physical_plan::ExecutionPlan,
};

pub mod codec;
pub mod empty_exec;
pub mod node;
pub mod remote_scan;
pub mod rewrite;
pub mod streaming_aggs_exec;

pub struct NewEmptyExecVisitor {
    data: Option<Arc<dyn ExecutionPlan>>,
}

impl NewEmptyExecVisitor {
    pub fn new() -> Self {
        Self { data: None }
    }

    pub fn get_data(&self) -> Option<&Arc<dyn ExecutionPlan>> {
        self.data.as_ref()
    }
}

impl Default for NewEmptyExecVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl<'n> TreeNodeVisitor<'n> for NewEmptyExecVisitor {
    type Node = Arc<dyn ExecutionPlan>;

    fn f_up(&mut self, node: &'n Self::Node) -> Result<TreeNodeRecursion> {
        if node.name() == "NewEmptyExec" {
            self.data = Some(node.clone());
            Ok(TreeNodeRecursion::Stop)
        } else {
            Ok(TreeNodeRecursion::Continue)
        }
    }
}

pub struct EmptyExecVisitor {
    data: Option<Arc<dyn ExecutionPlan>>,
}

impl EmptyExecVisitor {
    pub fn new() -> Self {
        Self { data: None }
    }

    pub fn get_data(&self) -> Option<&Arc<dyn ExecutionPlan>> {
        self.data.as_ref()
    }
}

impl Default for EmptyExecVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl<'n> TreeNodeVisitor<'n> for EmptyExecVisitor {
    type Node = Arc<dyn ExecutionPlan>;

    fn f_up(&mut self, node: &'n Self::Node) -> Result<TreeNodeRecursion> {
        if node.name() == "EmptyExec" {
            self.data = Some(node.clone());
            Ok(TreeNodeRecursion::Stop)
        } else {
            Ok(TreeNodeRecursion::Continue)
        }
    }
}

pub struct ReplaceTableScanExec {
    input: Arc<dyn ExecutionPlan>,
}

impl ReplaceTableScanExec {
    pub fn new(input: Arc<dyn ExecutionPlan>) -> Self {
        Self { input }
    }
}

impl TreeNodeRewriter for ReplaceTableScanExec {
    type Node = Arc<dyn ExecutionPlan>;

    fn f_up(&mut self, node: Self::Node) -> Result<Transformed<Self::Node>> {
        let name = node.name().to_string();
        let mut transformed = if name == "NewEmptyExec" {
            Transformed::yes(self.input.clone())
        } else {
            Transformed::no(node)
        };
        if name == "NewEmptyExec" {
            transformed.tnr = TreeNodeRecursion::Stop;
        } else {
            transformed.tnr = TreeNodeRecursion::Continue;
        }
        Ok(transformed)
    }
}
