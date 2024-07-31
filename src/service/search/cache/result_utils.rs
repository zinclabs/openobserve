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

use config::utils::{json, time::parse_str_to_timestamp_micros_as_option};
 
use sqlparser::{
    ast::{Expr, Function, Query, SelectItem, SetExpr, Statement},
    dialect::GenericDialect,
    parser::Parser,
};

pub fn is_aggregate_query(query: &str) -> Result<bool, sqlparser::parser::ParserError> {
    let ast = Parser::parse_sql(&GenericDialect {}, query)?;

    for statement in ast {
        if let Statement::Query(query) = statement {
            if is_aggregate_in_select(query) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn is_aggregate_in_select(query: Box<Query>) -> bool {
    if let SetExpr::Select(ref select) = *query.body {
        if select.distinct.is_some() {
            return true;
        }
        for select_item in &select.projection {
            if let SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, alias: _ } =
                select_item
            {
                if is_aggregate_expression(expr) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_aggregate_expression(expr: &Expr) -> bool {
    match expr {
        Expr::Function(Function { name, args: _, .. }) => {
            AGGREGATE_UDF_LIST.contains(&name.to_string().to_lowercase().as_str())
        }

        _ => false,
    }
}

const AGGREGATE_UDF_LIST: [&str; 8] = [
    "min",
    "max",
    "count",
    "sum",
    "avg",
    "median",
    "array_agg",
    "approx_percentile_cont",
];
 
pub fn get_ts_value(ts_column: &str, record: &json::Value) -> i64 {
    match record.get(ts_column) {
        None => 0_i64,
        Some(ts) => match ts {
            serde_json::Value::String(ts) => {
                parse_str_to_timestamp_micros_as_option(ts.as_str()).unwrap()
            }
            serde_json::Value::Number(ts) => ts.as_i64().unwrap(),
            _ => 0_i64,
        },
    }
}

pub fn round_down_to_nearest_minute(microseconds: i64) -> i64 {
    let microseconds_per_second = 1_000_000;
    let seconds_per_minute = 60;
    // Convert microseconds to seconds
    let total_seconds = microseconds / microseconds_per_second;
    // Find how many seconds past the last full minute
    let seconds_past_minute = total_seconds % seconds_per_minute;
    // Calculate the adjustment to round down to the nearest minute
    let adjusted_seconds = total_seconds - seconds_past_minute;
    // Convert the adjusted time back to microseconds
    adjusted_seconds * microseconds_per_second
}
