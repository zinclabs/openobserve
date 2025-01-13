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

use std::sync::Arc;

use datafusion::{
    common::{
        tree_node::{Transformed, TreeNode, TreeNodeRewriter},
        Result,
    },
    error::DataFusionError,
    logical_expr::{expr::ScalarFunction, BinaryExpr, Expr, LogicalPlan, Operator},
    optimizer::{optimizer::ApplyOrder, utils::NamePreserver, OptimizerConfig, OptimizerRule},
    scalar::ScalarValue,
};

use crate::service::search::datafusion::udf::cipher_udf::{
    DECRYPT_UDF, DECRYPT_UDF_NAME, ENCRYPT_UDF, ENCRYPT_UDF_NAME,
};

/// Optimization rule that rewrite decrypt if applicable
/// This will convert the following and the other way around
/// - decrypt(field,key_name) = value -> field = encrypt(value,key_name)
/// - decrypt(field,key_name) != value -> field != encrypt(value,key_name)
/// - encrypt(field,key_name) = value -> field = decrypt(value,key_name)
/// - encrypt(field,key_name) != value -> field != decrypt(value,key_name)
///
/// The value must be static string, as otherwise there is no benefit to
/// converting from decrypt->encrypt, both will have same const.
///
/// When the value is static string, it is quite beneficial to do it this way,
/// as datafusion will call encrypt at time of physical plan construction and
/// basically inline the encrypted value in the plan, making the comparison
/// a standard string comparison, instead of calling decrypt on each value and
/// then doing a string comparison on it.
///
/// This is assuming both have same cost. If we later find they don't, we'll
/// have to deal with that separately.
#[derive(Default, Debug)]
pub struct RewriteCipherCall {}

impl RewriteCipherCall {
    pub fn new() -> Self {
        Self {}
    }
}

impl OptimizerRule for RewriteCipherCall {
    fn name(&self) -> &str {
        "rewrite_cipher_key"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        Some(ApplyOrder::BottomUp)
    }

    fn supports_rewrite(&self) -> bool {
        true
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        let mut expr_rewriter = CipherReplace::new();

        let name_preserver = NamePreserver::new(&plan);
        plan.map_expressions(|expr| {
            let original_name = name_preserver.save(&expr);
            expr.rewrite(&mut expr_rewriter)
                .map(|transformed| transformed.update_data(|e| original_name.restore(e)))
        })
    }
}

fn is_rewritable_cipher_call(expr: &Expr) -> bool {
    if let Expr::BinaryExpr(BinaryExpr {
        left,
        op: Operator::Eq | Operator::NotEq,
        right,
    }) = expr
    {
        // we are specifically looking for binary op, where the one side
        // is encrypt/decrypt invocation and other side is a constant string

        // check one side is encrypt/decrypt call
        let left_is_cipher = match left.as_ref() {
            Expr::ScalarFunction(ScalarFunction { func, .. }) => {
                func.name() == DECRYPT_UDF_NAME || func.name() == ENCRYPT_UDF_NAME
            }
            _ => false,
        };

        let right_is_cipher = match right.as_ref() {
            Expr::ScalarFunction(ScalarFunction { func, .. }) => {
                func.name() == DECRYPT_UDF_NAME || func.name() == ENCRYPT_UDF_NAME
            }
            _ => false,
        };

        // check one side is const string
        let right_is_literal = matches!(right.as_ref(), Expr::Literal(ScalarValue::Utf8(_)));
        let left_is_literal = matches!(left.as_ref(), Expr::Literal(ScalarValue::Utf8(_)));

        (left_is_cipher && right_is_literal) || (left_is_literal && right_is_cipher)
    } else {
        false
    }
}

// Rewriter for decrypt
#[derive(Debug, Clone)]
pub struct CipherReplace {}

impl CipherReplace {
    pub fn new() -> Self {
        Self {}
    }
}
impl TreeNodeRewriter for CipherReplace {
    type Node = Expr;

    fn f_up(&mut self, expr: Expr) -> Result<Transformed<Expr>, DataFusionError> {
        if !is_rewritable_cipher_call(&expr) {
            return Ok(Transformed::no(expr));
        }

        // start with extracting individual sides of binary op
        let (left, op, right) = match &expr {
            Expr::BinaryExpr(BinaryExpr { left, op, right })
                if matches!(op, Operator::Eq | Operator::NotEq) =>
            {
                (left, op, right)
            }
            _ => return Ok(Transformed::no(expr)),
        };

        // Here we can be certain that exactly one of the left and right is
        // cipher call and the other is literal because of the is_rewritable_cipher check above

        let (cipher, literal) = if matches!(left.as_ref(), Expr::ScalarFunction(_)) {
            (left.as_ref(), right.as_ref())
        } else {
            (right.as_ref(), left.as_ref())
        };

        // extract args from the cipher call, so we get the table col and key name
        let (cipher_args, cipher_type) = match cipher {
            Expr::ScalarFunction(ScalarFunction { func, args }) => {
                if func.name() != DECRYPT_UDF_NAME && func.name() != ENCRYPT_UDF_NAME {
                    return Ok(Transformed::no(expr));
                }
                (args, func.name())
            }
            _ => return Ok(Transformed::no(expr)),
        };

        // sanity check
        if cipher_args.len() != 2 {
            return Err(DataFusionError::Internal(
                "encrypt/decrypt functions requires 2 arguments".to_string(),
            ));
        }

        let col_expr = cipher_args[0].clone();
        let key_name = cipher_args[1].clone();

        let _cipher = match cipher_type {
            DECRYPT_UDF_NAME => ENCRYPT_UDF.clone(),
            ENCRYPT_UDF_NAME => DECRYPT_UDF.clone(),
            _ => unreachable!("we made sure that type will be only one of these two"),
        };

        // construct the cipher call over the const value
        let cipher_call = Expr::ScalarFunction(ScalarFunction {
            func: Arc::new(_cipher),
            args: vec![literal.to_owned(), key_name],
        });

        // construct the new binary op, where we compare col to the cipher-ed value
        // instead of de-ciphering col and comparing it to plain value
        // it doesn't matter where the cipher was in original (left/right)
        // we always construct in order col op cipher . Because the op can be
        // only eq/neq , it will hold no matter operand order
        let binary_op = BinaryExpr {
            left: Box::new(col_expr),
            op: *op,
            right: Box::new(cipher_call),
        };

        Ok(Transformed::yes(Expr::BinaryExpr(binary_op)))
    }
}

#[derive(Debug)]
pub struct RewriteCipherKey {
    org: String,
}

impl RewriteCipherKey {
    pub fn new(org: &str) -> Self {
        if org.is_empty() {
            panic!("org must not be empty in cipher key re-write");
        }
        Self {
            org: org.to_string(),
        }
    }
}

impl OptimizerRule for RewriteCipherKey {
    fn name(&self) -> &str {
        "rewrite_cipher_key"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        Some(ApplyOrder::BottomUp)
    }

    fn supports_rewrite(&self) -> bool {
        true
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        let mut expr_rewriter = CipherKeyRewrite::new(&self.org);

        let name_preserver = NamePreserver::new(&plan);
        plan.map_expressions(|expr| {
            let original_name = name_preserver.save(&expr);
            expr.rewrite(&mut expr_rewriter)
                .map(|transformed| transformed.update_data(|e| original_name.restore(e)))
        })
    }
}

// Rewriter for decrypt
#[derive(Debug, Clone)]
pub struct CipherKeyRewrite {
    org: String,
}

impl CipherKeyRewrite {
    pub fn new(org: &str) -> Self {
        Self {
            org: org.to_string(),
        }
    }
}
impl TreeNodeRewriter for CipherKeyRewrite {
    type Node = Expr;

    fn f_up(&mut self, expr: Expr) -> Result<Transformed<Expr>, DataFusionError> {
        // get function and args from the call
        let (func, args) = match &expr {
            Expr::ScalarFunction(ScalarFunction { func, args }) => {
                if func.name() != DECRYPT_UDF_NAME && func.name() != ENCRYPT_UDF_NAME {
                    return Ok(Transformed::no(expr));
                }
                (func, args)
            }
            _ => return Ok(Transformed::no(expr)),
        };

        // sanity check
        if args.len() != 2 {
            return Err(DataFusionError::Internal(
                "encrypt/decrypt functions requires 2 arguments".to_string(),
            ));
        }

        // extract args
        let col_expr = args[0].clone();
        let key_name = args[1].clone();

        let new_key_name = match key_name {
            Expr::Literal(ScalarValue::Utf8(Some(s))) => {
                let org_prefix = format!("{}:", self.org);
                // because datafusion applies optimizer rules multiple times,
                // we have to check if we have appended the prefix already.
                // to prevent user trying to abuse this by adding another org's name
                // in key name itself, we disallow key name to contain `:` character
                if s.starts_with(&org_prefix) {
                    return Ok(Transformed::no(expr));
                }
                // construct new key name containing the org name
                let new_key_name = format!("{}:{}", self.org, s);
                Expr::Literal(ScalarValue::Utf8(Some(new_key_name)))
            }
            _ => return Ok(Transformed::no(expr)),
        };

        // construct the cipher call with new key name
        let cipher_call = Expr::ScalarFunction(ScalarFunction {
            func: func.clone(),
            args: vec![col_expr, new_key_name],
        });

        Ok(Transformed::yes(cipher_call))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{Int64Array, StringArray};
    use arrow_schema::DataType;
    use datafusion::{
        arrow::{
            datatypes::{Field, Schema},
            record_batch::RecordBatch,
        },
        datasource::MemTable,
        logical_expr::LogicalPlan,
        optimizer::{Optimizer, OptimizerContext, OptimizerRule},
        prelude::SessionContext,
    };

    use super::{RewriteCipherCall, RewriteCipherKey};
    use crate::service::search::datafusion::udf::{
        cipher_udf::{DECRYPT_UDF, ENCRYPT_UDF},
        match_all_udf::MATCH_ALL_UDF,
    };

    #[tokio::test]
    async fn test_rewrite_cipher() {
        let sqls = [
            // equal to operator gets re-written
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') = 'test_val'",
                "Projection: t._timestamp\n  Filter: t.name = encrypt(Utf8(\"test_val\"), Utf8(\"test_key\"))\n    TableScan: t"
            ),
            // not equal operator gets rewritten
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') != 'test_val'",
                "Projection: t._timestamp\n  Filter: t.name != encrypt(Utf8(\"test_val\"), Utf8(\"test_key\"))\n    TableScan: t"
            ),
            // like operator does not get rewritten
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') LIKE '%test%'",
                "Projection: t._timestamp\n  Filter: decrypt(t.name, Utf8(\"test_key\")) LIKE Utf8(\"%test%\")\n    TableScan: t"
            ),
            // match all does not get re-written
            (
                "SELECT _timestamp FROM t where match_all(decrypt(name, 'test_key'), 'test')",
                "Projection: t._timestamp\n  Filter: match_all(decrypt(t.name, Utf8(\"test_key\")), Utf8(\"test\"))\n    TableScan: t"
            ),
            // if comparison is to non-literal, it does not get re-written
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') = other_col",
                "Projection: t._timestamp\n  Filter: decrypt(t.name, Utf8(\"test_key\")) = t.other_col\n    TableScan: t"
            ),
            // similar checks for encrypt
            (
                "SELECT _timestamp FROM t where encrypt(name, 'test_key') = 'test_val'",
                "Projection: t._timestamp\n  Filter: t.name = decrypt(Utf8(\"test_val\"), Utf8(\"test_key\"))\n    TableScan: t"
            ),
            (
                "SELECT _timestamp FROM t where encrypt(name, 'test_key') != 'test_val'",
                "Projection: t._timestamp\n  Filter: t.name != decrypt(Utf8(\"test_val\"), Utf8(\"test_key\"))\n    TableScan: t"
            ),
            (
                "SELECT _timestamp FROM t where encrypt(name, 'test_key') = other_col",
                "Projection: t._timestamp\n  Filter: encrypt(t.name, Utf8(\"test_key\")) = t.other_col\n    TableScan: t"
            ),
            // checks for revered order in query
            (
                "SELECT _timestamp FROM t where 'test_val' = decrypt(name, 'test_key')",
                "Projection: t._timestamp\n  Filter: t.name = encrypt(Utf8(\"test_val\"), Utf8(\"test_key\"))\n    TableScan: t"
            ),
            (
                "SELECT _timestamp FROM t where 'test_val' != encrypt(name, 'test_key')",
                "Projection: t._timestamp\n  Filter: t.name != decrypt(Utf8(\"test_val\"), Utf8(\"test_key\"))\n    TableScan: t"
            ),
            // no op sanity check
            (
                "SELECT _timestamp FROM t where match_all(name, 'test')",
                "Projection: t._timestamp\n  Filter: match_all(t.name, Utf8(\"test\"))\n    TableScan: t"
            ),
        ];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("_timestamp", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("other_col", DataType::Utf8, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
                Arc::new(StringArray::from(vec![
                    "open",
                    "observe",
                    "openobserve",
                    "o2",
                    "oo",
                ])),
                Arc::new(StringArray::from(vec![
                    "open",
                    "observe",
                    "openobserve",
                    "o2",
                    "oo",
                ])),
            ],
        )
        .unwrap();

        fn observe(_plan: &LogicalPlan, _rule: &dyn OptimizerRule) {}

        let ctx = SessionContext::new();
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();
        ctx.register_udf(DECRYPT_UDF.clone());
        ctx.register_udf(ENCRYPT_UDF.clone());
        ctx.register_udf(MATCH_ALL_UDF.clone());
        ctx.add_optimizer_rule(Arc::new(RewriteCipherCall::new()));

        for (sql, res) in sqls {
            let plan = ctx.state().create_logical_plan(sql).await.unwrap();
            let optimizer = Optimizer::with_rules(vec![Arc::new(RewriteCipherCall::new())]);
            let optimized_plan = optimizer
                .optimize(plan, &OptimizerContext::new(), observe)
                .unwrap();
            let formatted_plan = format!("{optimized_plan}");
            assert_eq!(res, formatted_plan);
        }
    }

    #[tokio::test]
    async fn test_rewrite_cipher_key() {
        let sqls = [
            // check key is re-written
            (
                "SELECT _timestamp FROM t where decrypt(name, 'test_key') = 'test_val'",
                "Projection: t._timestamp\n  Filter: decrypt(t.name, Utf8(\"org1:test_key\")) = Utf8(\"test_val\")\n    TableScan: t"
            ),
            (
                "SELECT _timestamp FROM t where 'test_val' = encrypt(name, 'test_key')",
                "Projection: t._timestamp\n  Filter: Utf8(\"test_val\") = encrypt(t.name, Utf8(\"org1:test_key\"))\n    TableScan: t"
            ),
        ];

        // define a schema.
        let schema = Arc::new(Schema::new(vec![
            Field::new("_timestamp", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        // define data.
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5])),
                Arc::new(StringArray::from(vec![
                    "open",
                    "observe",
                    "openobserve",
                    "o2",
                    "oo",
                ])),
            ],
        )
        .unwrap();

        fn observe(_plan: &LogicalPlan, _rule: &dyn OptimizerRule) {}

        let ctx = SessionContext::new();
        let provider = MemTable::try_new(schema, vec![vec![batch]]).unwrap();
        ctx.register_table("t", Arc::new(provider)).unwrap();
        ctx.register_udf(DECRYPT_UDF.clone());
        ctx.register_udf(ENCRYPT_UDF.clone());
        ctx.add_optimizer_rule(Arc::new(RewriteCipherKey::new("org1")));

        for (sql, res) in sqls {
            let plan = ctx.state().create_logical_plan(sql).await.unwrap();
            let optimizer = Optimizer::with_rules(vec![Arc::new(RewriteCipherKey::new("org1"))]);
            let optimized_plan = optimizer
                .optimize(plan, &OptimizerContext::new(), observe)
                .unwrap();
            let formatted_plan = format!("{optimized_plan}");
            assert_eq!(res, formatted_plan);
        }
    }
}
