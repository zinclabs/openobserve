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
    common::{internal_err, Result},
    error::DataFusionError,
    execution::FunctionRegistry,
    logical_expr::ScalarUDF,
    physical_plan::ExecutionPlan,
};
use datafusion_proto::{
    convert_required,
    logical_plan::{
        from_proto::parse_expr, to_proto::serialize_exprs, DefaultLogicalExtensionCodec,
    },
    physical_plan::PhysicalExtensionCodec,
    protobuf::proto_error,
};
use prost::Message;
use proto::cluster_rpc;

use super::empty_exec::NewEmptyExec;

/// A PhysicalExtensionCodec that can serialize and deserialize ChildExec
#[derive(Debug)]
pub struct EmptyExecPhysicalExtensionCodec;

impl PhysicalExtensionCodec for EmptyExecPhysicalExtensionCodec {
    fn try_decode(
        &self,
        buf: &[u8],
        _inputs: &[Arc<dyn ExecutionPlan>],
        registry: &dyn FunctionRegistry,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let proto = cluster_rpc::NewEmptyExecNode::decode(buf).map_err(|e| {
            DataFusionError::Internal(format!(
                "failed to decode NewEmptyExecNode writer execution plan: {e:?}"
            ))
        })?;
        let schema = Arc::new(convert_required!(proto.schema)?);
        let full_schema = Arc::new(convert_required!(proto.full_schema)?);
        let extension_codec = DefaultLogicalExtensionCodec {};
        let filters = proto
            .filters
            .iter()
            .map(|v| parse_expr(v, registry, &extension_codec))
            .collect::<Result<Vec<_>, _>>()?;
        let projection = if proto.projection.is_empty() {
            None
        } else {
            Some(
                proto
                    .projection
                    .iter()
                    .map(|v| *v as usize)
                    .collect::<Vec<_>>(),
            )
        };
        Ok(Arc::new(NewEmptyExec::new(
            &proto.name,
            schema,
            projection.as_ref(),
            &filters,
            proto.limit.map(|v| v as usize),
            proto.sorted_by_time,
            full_schema,
        )))
    }

    fn try_encode(&self, node: Arc<dyn ExecutionPlan>, buf: &mut Vec<u8>) -> Result<()> {
        let Some(node) = node.as_any().downcast_ref::<NewEmptyExec>() else {
            return internal_err!("Not supported");
        };
        let extension_codec = DefaultLogicalExtensionCodec {};
        let filters = serialize_exprs(node.filters(), &extension_codec)?;
        let proto = cluster_rpc::NewEmptyExecNode {
            name: node.name().to_string(),
            schema: Some(node.schema().as_ref().try_into()?),
            projection: match node.projection() {
                Some(v) => v.iter().map(|v| *v as u64).collect(),
                None => vec![],
            },
            filters,
            limit: node.limit().map(|v| v as u64),
            sorted_by_time: node.sorted_by_time(),
            full_schema: Some(node.full_schema().as_ref().try_into()?),
        };
        proto.encode(buf).map_err(|e| {
            DataFusionError::Internal(format!(
                "failed to encode NewEmptyExecNode writer execution plan: {e:?}"
            ))
        })?;
        Ok(())
    }
}

/// A PhysicalExtensionCodec that tries one of multiple inner codecs
/// until one works
#[derive(Debug)]
pub struct ComposedPhysicalExtensionCodec {
    pub codecs: Vec<Arc<dyn PhysicalExtensionCodec>>,
}

impl PhysicalExtensionCodec for ComposedPhysicalExtensionCodec {
    fn try_decode(
        &self,
        buf: &[u8],
        inputs: &[Arc<dyn ExecutionPlan>],
        registry: &dyn FunctionRegistry,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let mut last_err = None;
        for codec in &self.codecs {
            match codec.try_decode(buf, inputs, registry) {
                Ok(plan) => return Ok(plan),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap())
    }

    fn try_encode(&self, node: Arc<dyn ExecutionPlan>, buf: &mut Vec<u8>) -> Result<()> {
        let mut last_err = None;
        for codec in &self.codecs {
            match codec.try_encode(node.clone(), buf) {
                Ok(_) => return Ok(()),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap())
    }

    fn try_decode_udf(&self, name: &str, _buf: &[u8]) -> Result<Arc<ScalarUDF>> {
        let mut last_err = None;
        for codec in &self.codecs {
            match codec.try_decode_udf(name, _buf) {
                Ok(plan) => return Ok(plan),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap())
    }

    fn try_encode_udf(&self, _node: &ScalarUDF, _buf: &mut Vec<u8>) -> Result<()> {
        let mut last_err = None;
        for codec in &self.codecs {
            match codec.try_encode_udf(_node, _buf) {
                Ok(_) => return Ok(()),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use datafusion_proto::bytes::{
        physical_plan_from_bytes_with_extension_codec, physical_plan_to_bytes_with_extension_codec,
    };

    use super::*;

    #[tokio::test]
    async fn test_datafusion_codec() -> Result<()> {
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
        let plan: Arc<dyn ExecutionPlan> = Arc::new(NewEmptyExec::new(
            "test",
            Arc::clone(&schema),
            Some(&vec![0]),
            &[],
            Some(10),
            false,
            Arc::clone(&schema),
        ));

        // encode
        let proto = ComposedPhysicalExtensionCodec {
            codecs: vec![Arc::new(EmptyExecPhysicalExtensionCodec {})],
        };
        let plan_bytes = physical_plan_to_bytes_with_extension_codec(plan.clone(), &proto).unwrap();

        // decode
        let ctx = datafusion::prelude::SessionContext::new();
        let plan2 = physical_plan_from_bytes_with_extension_codec(&plan_bytes, &ctx, &proto)?;
        let plan2 = plan2.as_any().downcast_ref::<NewEmptyExec>().unwrap();
        let plan = plan.as_any().downcast_ref::<NewEmptyExec>().unwrap();

        // check
        assert_eq!(plan.name(), plan2.name());
        assert_eq!(plan.schema(), plan2.schema());
        assert_eq!(plan.projection(), plan2.projection());
        assert_eq!(plan.filters(), plan2.filters());
        assert_eq!(plan.limit(), plan2.limit());
        assert_eq!(plan.full_schema(), plan2.full_schema());

        Ok(())
    }
}
