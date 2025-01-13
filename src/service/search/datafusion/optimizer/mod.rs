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

use add_sort_and_limit::AddSortAndLimitRule;
use add_timestamp::AddTimestampRule;
#[cfg(feature = "enterprise")]
use cipher::{RewriteCipherCall, RewriteCipherKey};
use datafusion::optimizer::{
    common_subexpr_eliminate::CommonSubexprEliminate,
    decorrelate_predicate_subquery::DecorrelatePredicateSubquery,
    eliminate_cross_join::EliminateCrossJoin, eliminate_duplicated_expr::EliminateDuplicatedExpr,
    eliminate_filter::EliminateFilter, eliminate_group_by_constant::EliminateGroupByConstant,
    eliminate_join::EliminateJoin, eliminate_limit::EliminateLimit,
    eliminate_nested_union::EliminateNestedUnion, eliminate_one_union::EliminateOneUnion,
    eliminate_outer_join::EliminateOuterJoin, extract_equijoin_predicate::ExtractEquijoinPredicate,
    filter_null_join_keys::FilterNullJoinKeys, optimize_projections::OptimizeProjections,
    propagate_empty_relation::PropagateEmptyRelation, push_down_filter::PushDownFilter,
    push_down_limit::PushDownLimit, replace_distinct_aggregate::ReplaceDistinctWithAggregate,
    scalar_subquery_to_join::ScalarSubqueryToJoin, simplify_expressions::SimplifyExpressions,
    single_distinct_to_groupby::SingleDistinctToGroupBy,
    unwrap_cast_in_comparison::UnwrapCastInComparison, OptimizerRule,
};
use infra::schema::get_stream_setting_fts_fields;
use limit_join_right_side::LimitJoinRightSide;
use rewrite_histogram::RewriteHistogram;
use rewrite_match::RewriteMatch;

use crate::service::search::sql::Sql;

pub mod add_sort_and_limit;
pub mod add_timestamp;
#[cfg(feature = "enterprise")]
pub mod cipher;
pub mod join_reorder;
pub mod limit_join_right_side;
pub mod rewrite_histogram;
pub mod rewrite_match;
pub mod utils;

pub fn generate_optimizer_rules(sql: &Sql) -> Vec<Arc<dyn OptimizerRule + Send + Sync>> {
    let cfg = config::get_config();
    let limit = if sql.limit as i32 > config::QUERY_WITH_NO_LIMIT {
        if sql.limit > 0 {
            Some(sql.limit as usize)
        } else {
            Some(config::get_config().limit.query_default_limit as usize)
        }
    } else {
        None
    };
    let offset = sql.offset as usize;
    let (start_time, end_time) = sql.time_range.unwrap();

    let mut rules: Vec<Arc<dyn OptimizerRule + Send + Sync>> = Vec::with_capacity(64);

    // get full text search fields
    if sql.match_items.is_some() && sql.stream_names.len() == 1 {
        let mut fields = Vec::new();
        let stream_name = &sql.stream_names[0];
        let schema = sql.schemas.get(stream_name).unwrap();
        let stream_settings = infra::schema::unwrap_stream_settings(schema.schema());
        let fts_fields = get_stream_setting_fts_fields(&stream_settings);
        for fts_field in fts_fields {
            if schema.field_with_name(&fts_field).is_none() {
                continue;
            }
            fields.push(fts_field);
        }
        // *********** custom rules ***********
        rules.push(Arc::new(RewriteMatch::new(fields)));
        // ************************************
    }

    rules.push(Arc::new(EliminateNestedUnion::new()));
    rules.push(Arc::new(SimplifyExpressions::new()));
    rules.push(Arc::new(UnwrapCastInComparison::new()));
    rules.push(Arc::new(ReplaceDistinctWithAggregate::new()));
    rules.push(Arc::new(EliminateJoin::new()));
    rules.push(Arc::new(DecorrelatePredicateSubquery::new()));
    rules.push(Arc::new(ScalarSubqueryToJoin::new()));
    rules.push(Arc::new(ExtractEquijoinPredicate::new()));
    // simplify expressions does not simplify expressions in subqueries, so we
    // run it again after running the optimizations that potentially converted
    // subqueries to joins
    rules.push(Arc::new(SimplifyExpressions::new()));
    rules.push(Arc::new(EliminateDuplicatedExpr::new()));
    rules.push(Arc::new(EliminateFilter::new()));
    rules.push(Arc::new(EliminateCrossJoin::new()));
    rules.push(Arc::new(CommonSubexprEliminate::new()));
    rules.push(Arc::new(EliminateLimit::new()));
    rules.push(Arc::new(PropagateEmptyRelation::new()));
    // Must be after PropagateEmptyRelation
    rules.push(Arc::new(EliminateOneUnion::new()));
    rules.push(Arc::new(FilterNullJoinKeys::default()));
    rules.push(Arc::new(EliminateOuterJoin::new()));

    // *********** custom rules ***********
    rules.push(Arc::new(RewriteHistogram::new(start_time, end_time)));
    if let Some(limit) = limit {
        rules.push(Arc::new(AddSortAndLimitRule::new(limit, offset)));
    };
    rules.push(Arc::new(AddTimestampRule::new(start_time, end_time)));
    #[cfg(feature = "enterprise")]
    rules.push(Arc::new(RewriteCipherCall::new()));
    #[cfg(feature = "enterprise")]
    rules.push(Arc::new(RewriteCipherKey::new(&sql.org_id)));

    // should after ExtractEquijoinPredicate, because LimitJoinRightSide will
    // require the join's on columns
    if cfg.common.feature_join_match_one_enabled && cfg.common.feature_join_right_side_max_rows > 0
    {
        rules.push(Arc::new(LimitJoinRightSide::new(
            cfg.common.feature_join_right_side_max_rows,
        )));
    }
    // ************************************

    // Filters can't be pushed down past Limits, we should do PushDownFilter after
    // PushDownLimit
    rules.push(Arc::new(PushDownLimit::new()));
    rules.push(Arc::new(PushDownFilter::new()));
    rules.push(Arc::new(SingleDistinctToGroupBy::new()));
    // The previous optimizations added expressions and projections,
    // that might benefit from the following rules
    rules.push(Arc::new(SimplifyExpressions::new()));
    rules.push(Arc::new(UnwrapCastInComparison::new()));
    rules.push(Arc::new(CommonSubexprEliminate::new()));
    rules.push(Arc::new(EliminateGroupByConstant::new()));
    rules.push(Arc::new(OptimizeProjections::new()));

    rules
}
