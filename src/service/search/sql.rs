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

use std::{collections::HashSet, ops::ControlFlow, sync::Arc};

use arrow_schema::FieldRef;
use chrono::Duration;
use config::{
    get_config,
    meta::{
        inverted_index::InvertedIndexOptimizeMode,
        sql::{resolve_stream_names_with_type, OrderBy, Sql as MetaSql, TableReferenceExt},
        stream::StreamType,
    },
    utils::sql::AGGREGATE_UDF_LIST,
    ID_COL_NAME, ORIGINAL_DATA_COL_NAME,
};
use datafusion::{arrow::datatypes::Schema, common::TableReference};
use hashbrown::HashMap;
use infra::{
    errors::{Error, ErrorCodes},
    schema::{
        get_stream_setting_fts_fields, get_stream_setting_index_fields, unwrap_stream_settings,
        SchemaCache,
    },
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use proto::cluster_rpc::SearchQuery;
use regex::Regex;
use sqlparser::{
    ast::{
        BinaryOperator, DuplicateTreatment, Expr, Function, FunctionArg, FunctionArgExpr,
        FunctionArgumentList, FunctionArguments, GroupByExpr, Ident, ObjectName, Query, Select,
        SelectItem, SetExpr, Statement, TableFactor, TableWithJoins, VisitMut, VisitorMut,
    },
    dialect::PostgreSqlDialect,
    parser::Parser,
};

#[cfg(feature = "enterprise")]
use super::datafusion::udf::cipher_udf::{DECRYPT_UDF_NAME, ENCRYPT_UDF_NAME};
use super::{
    datafusion::udf::match_all_udf::{
        FUZZY_MATCH_ALL_UDF_NAME, MATCH_ALL_RAW_IGNORE_CASE_UDF_NAME, MATCH_ALL_RAW_UDF_NAME,
        MATCH_ALL_UDF_NAME,
    },
    index::{get_index_condition_from_expr, IndexCondition},
    request::Request,
    utils::{is_field, is_value, split_conjunction, trim_quotes},
};

pub static RE_ONLY_SELECT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)select[ ]+\*").unwrap());
pub static RE_SELECT_FROM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)SELECT (.*) FROM").unwrap());
pub static RE_WHERE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i) where (.*)").unwrap());

pub static RE_HISTOGRAM: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)histogram\(([^\)]*)\)").unwrap());

#[derive(Clone, Debug)]
pub struct Sql {
    pub sql: String,
    pub org_id: String,
    pub stream_type: StreamType,
    pub stream_names: Vec<TableReference>,
    pub match_items: Option<Vec<String>>, // match_all, only for single stream
    pub equal_items: HashMap<TableReference, Vec<(String, String)>>, /* table_name ->
                                           * [(field_name, value)] */
    pub prefix_items: HashMap<TableReference, Vec<(String, String)>>, /* table_name -> [(field_name, value)] */
    pub columns: HashMap<TableReference, HashSet<String>>,            // table_name -> [field_name]
    pub aliases: Vec<(String, String)>,                               // field_name, alias
    pub schemas: HashMap<TableReference, Arc<SchemaCache>>,
    pub limit: i64,
    pub offset: i64,
    pub time_range: Option<(i64, i64)>,
    pub group_by: Vec<String>,
    pub order_by: Vec<(String, OrderBy)>,
    pub histogram_interval: Option<i64>,
    pub sorted_by_time: bool,     // if only order by _timestamp
    pub use_inverted_index: bool, // if can use inverted index
    pub index_condition: Option<IndexCondition>, // use for tantivy index
    pub index_optimize_mode: Option<InvertedIndexOptimizeMode>,
}

impl Sql {
    pub async fn new_from_req(req: &Request, query: &SearchQuery) -> Result<Sql, Error> {
        Self::new(query, &req.org_id, req.stream_type).await
    }

    pub async fn new(
        query: &SearchQuery,
        org_id: &str,
        stream_type: StreamType,
    ) -> Result<Sql, Error> {
        let cfg = get_config();
        let sql = query.sql.clone();
        let limit = query.size as i64;
        let offset = query.from as i64;

        // 1. get table name
        let stream_names =
            resolve_stream_names_with_type(&sql).map_err(|e| Error::Message(e.to_string()))?;
        let mut total_schemas = HashMap::with_capacity(stream_names.len());
        for stream in stream_names.iter() {
            let stream_name = stream.stream_name();
            let stream_type = stream.get_stream_type(stream_type);
            let schema = infra::schema::get(org_id, &stream_name, stream_type)
                .await
                .unwrap_or_else(|_| Schema::empty());
            total_schemas.insert(stream.clone(), Arc::new(SchemaCache::new(schema)));
        }

        let mut statement = Parser::parse_sql(&PostgreSqlDialect {}, &sql)
            .map_err(|e| Error::Message(e.to_string()))?
            .pop()
            .unwrap();

        // 2. rewrite track_total_hits
        if query.track_total_hits {
            let mut trace_total_hits_visitor = TrackTotalHitsVisitor::new();
            statement.visit(&mut trace_total_hits_visitor);
        }

        // 3. get column name, alias, group by, order by
        let mut column_visitor = ColumnVisitor::new(&total_schemas);
        statement.visit(&mut column_visitor);

        let columns = column_visitor.columns.clone();
        let aliases = column_visitor
            .columns_alias
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let group_by = column_visitor.group_by;
        let mut order_by = column_visitor.order_by;

        // check if need sort by time
        if order_by.is_empty()
            && !query.track_total_hits
            && stream_names.len() == 1
            && group_by.is_empty()
            && !column_visitor.has_agg_function
            && !column_visitor.is_distinct
        {
            order_by.push((cfg.common.column_timestamp.clone(), OrderBy::Desc));
        }
        let need_sort_by_time = order_by.len() == 1
            && order_by[0].0 == cfg.common.column_timestamp
            && order_by[0].1 == OrderBy::Desc;
        let use_inverted_index = column_visitor.use_inverted_index;

        // 4. get match_all() value
        let mut match_visitor = MatchVisitor::new();
        statement.visit(&mut match_visitor);

        // 5. check if have full text search filed in stream
        if stream_names.len() == 1 && match_visitor.match_items.is_some() {
            let schema = total_schemas.values().next().unwrap();
            let stream_settings = infra::schema::unwrap_stream_settings(schema.schema());
            let fts_fields = get_stream_setting_fts_fields(&stream_settings);
            // check if schema don't have full text search field
            if fts_fields
                .into_iter()
                .all(|field| !schema.contains_field(&field))
            {
                return Err(Error::Message(
                    "Using match_all() function in a stream that don't have full text search field"
                        .to_string(),
                ));
            }
        }

        // 6. generate used schema
        let mut used_schemas = HashMap::with_capacity(total_schemas.len());
        if column_visitor.is_wildcard {
            let has_original_column = has_original_column(&column_visitor.columns);
            used_schemas = generate_select_star_schema(total_schemas, has_original_column);
        } else {
            for (stream, schema) in total_schemas.iter() {
                let columns = match column_visitor.columns.get(stream) {
                    Some(columns) => columns.clone(),
                    None => {
                        used_schemas.insert(stream.clone(), schema.clone());
                        continue;
                    }
                };
                let fields =
                    generate_schema_fields(columns, schema, match_visitor.match_items.is_some());
                let schema = Schema::new(fields).with_metadata(schema.schema().metadata().clone());
                used_schemas.insert(stream.clone(), Arc::new(SchemaCache::new(schema)));
            }
        }

        // 7. get partition column value
        let mut partition_column_visitor = PartitionColumnVisitor::new(&used_schemas);
        statement.visit(&mut partition_column_visitor);

        // 8. get prefix column value
        let mut prefix_column_visitor = PrefixColumnVisitor::new(&used_schemas);
        statement.visit(&mut prefix_column_visitor);

        // 9. pick up histogram interval
        let mut histogram_interval_visitor =
            HistogramIntervalVistor::new(Some((query.start_time, query.end_time)));
        statement.visit(&mut histogram_interval_visitor);

        // NOTE: only this place modify the sql
        // 10. add _timestamp and _o2_id if need
        if !is_complex_query(&mut statement) {
            let mut add_timestamp_visitor = AddTimestampVisitor::new();
            statement.visit(&mut add_timestamp_visitor);
            if o2_id_is_needed(&used_schemas) {
                let mut add_o2_id_visitor = AddO2IdVisitor::new();
                statement.visit(&mut add_o2_id_visitor);
            }
        }

        // NOTE: only this place modify the sql
        // 11. generate tantivy query
        let mut index_condition = None;
        let mut can_optimize = false;
        if cfg.common.inverted_index_search_format.eq("tantivy")
            && stream_names.len() == 1
            && cfg.common.inverted_index_enabled
            && use_inverted_index
        {
            let is_remove_filter = cfg.common.feature_query_remove_filter_with_index;
            let mut index_visitor = IndexVisitor::new(&used_schemas, is_remove_filter);
            statement.visit(&mut index_visitor);
            index_condition = index_visitor.index_condition;
            can_optimize = index_visitor.can_optimize;
        }

        // 12. check `select * from table where match_all()` optimizer
        let mut index_optimize_mode = None;
        if !is_complex_query(&mut statement)
            && order_by.len() == 1
            && order_by[0].0 == cfg.common.column_timestamp
            && can_optimize
        {
            index_optimize_mode = Some(InvertedIndexOptimizeMode::SimpleSelect(
                (offset + limit) as usize,
                order_by[0].1 == OrderBy::Asc,
            ));
        }

        // 13. check `select count(*) from table where match_all` optimizer
        if can_optimize
            && is_simple_count_query(&mut statement)
            && cfg.common.inverted_index_count_optimizer_enabled
        {
            index_optimize_mode = Some(InvertedIndexOptimizeMode::SimpleCount);
        }

        Ok(Sql {
            sql: statement.to_string(),
            org_id: org_id.to_string(),
            stream_type,
            stream_names,
            match_items: match_visitor.match_items,
            equal_items: partition_column_visitor.equal_items,
            prefix_items: prefix_column_visitor.prefix_items,
            columns,
            aliases,
            schemas: used_schemas,
            limit,
            offset,
            time_range: Some((query.start_time, query.end_time)),
            group_by,
            order_by,
            histogram_interval: histogram_interval_visitor.interval,
            sorted_by_time: need_sort_by_time,
            use_inverted_index,
            index_condition,
            index_optimize_mode,
        })
    }
}

impl std::fmt::Display for Sql {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "sql: {}, time_range: {:?}, stream: {}/{}/{:?}, match_items: {:?}, equal_items: {:?}, prefix_items: {:?}, aliases: {:?}, limit: {}, offset: {}, group_by: {:?}, order_by: {:?}, histogram_interval: {:?}, sorted_by_time: {}, use_inverted_index: {}, index_condition: {:?}",
            self.sql,
            self.time_range,
            self.org_id,
            self.stream_type,
            self.stream_names,
            self.match_items,
            self.equal_items,
            self.prefix_items,
            self.aliases,
            self.limit,
            self.offset,
            self.group_by,
            self.order_by,
            self.histogram_interval,
            self.sorted_by_time,
            self.use_inverted_index,
            self.index_condition,
        )
    }
}

fn generate_select_star_schema(
    schemas: HashMap<TableReference, Arc<SchemaCache>>,
    has_original_column: HashMap<TableReference, bool>,
) -> HashMap<TableReference, Arc<SchemaCache>> {
    let mut used_schemas = HashMap::new();
    for (name, schema) in schemas {
        let stream_settings = unwrap_stream_settings(schema.schema()).unwrap_or_default();
        let defined_schema_fields = stream_settings.defined_schema_fields.unwrap_or_default();
        let has_original_column = *has_original_column.get(&name).unwrap_or(&false);
        // check if it is user defined schema
        if defined_schema_fields.is_empty() && !has_original_column {
            if schema.contains_field(ORIGINAL_DATA_COL_NAME) {
                // skip selecting "_original" column if `SELECT * ...`
                let mut fields = schema.schema().fields().iter().cloned().collect::<Vec<_>>();
                fields.retain(|field| field.name() != ORIGINAL_DATA_COL_NAME);
                let schema = Arc::new(SchemaCache::new(
                    Schema::new(fields).with_metadata(schema.schema().metadata().clone()),
                ));
                used_schemas.insert(name, schema);
            } else {
                used_schemas.insert(name, schema);
            }
        } else {
            used_schemas.insert(
                name,
                generate_user_defined_schema(schema.as_ref(), defined_schema_fields),
            );
        }
    }
    used_schemas
}

fn generate_user_defined_schema(
    schema: &SchemaCache,
    defined_schema_fields: Vec<String>,
) -> Arc<SchemaCache> {
    let cfg = get_config();
    let mut fields: HashSet<String> = defined_schema_fields.iter().cloned().collect();
    if !fields.contains(&cfg.common.column_timestamp) {
        fields.insert(cfg.common.column_timestamp.to_string());
    }
    if !cfg.common.feature_query_exclude_all && !fields.contains(&cfg.common.column_all) {
        fields.insert(cfg.common.column_all.to_string());
    }
    if !fields.contains(ID_COL_NAME) {
        fields.insert(ID_COL_NAME.to_string());
    }
    let new_fields = fields
        .iter()
        .filter_map(|name| schema.field_with_name(name).cloned())
        .collect::<Vec<_>>();

    Arc::new(SchemaCache::new(
        Schema::new(new_fields).with_metadata(schema.schema().metadata().clone()),
    ))
}

// add field from full text search
fn generate_schema_fields(
    columns: HashSet<String>,
    schema: &SchemaCache,
    has_match_all: bool,
) -> Vec<FieldRef> {
    let mut columns = columns;

    // 1. add timestamp field
    if !columns.contains(&get_config().common.column_timestamp) {
        columns.insert(get_config().common.column_timestamp.clone());
    }

    // 2. check _o2_id
    if !columns.contains(ID_COL_NAME) {
        columns.insert(ID_COL_NAME.to_string());
    }

    // 3. add field from full text search
    if has_match_all {
        let stream_settings = infra::schema::unwrap_stream_settings(schema.schema());
        let fts_fields = get_stream_setting_fts_fields(&stream_settings);
        for fts_field in fts_fields {
            if schema.field_with_name(&fts_field).is_none() {
                continue;
            }
            columns.insert(fts_field);
        }
    }

    // 4. generate fields
    let mut fields = Vec::with_capacity(columns.len());
    for column in columns {
        if let Some(field) = schema.field_with_name(&column) {
            fields.push(field.clone());
        }
    }
    fields
}

// check if has original column in sql
fn has_original_column(
    columns: &HashMap<TableReference, HashSet<String>>,
) -> HashMap<TableReference, bool> {
    let mut has_original_column = HashMap::with_capacity(columns.len());
    for (name, column) in columns.iter() {
        if column.contains(ORIGINAL_DATA_COL_NAME) {
            has_original_column.insert(name.clone(), true);
        } else {
            has_original_column.insert(name.clone(), false);
        }
    }
    has_original_column
}

/// visit a sql to get all columns
struct ColumnVisitor<'a> {
    columns: HashMap<TableReference, HashSet<String>>,
    columns_alias: HashSet<(String, String)>,
    schemas: &'a HashMap<TableReference, Arc<SchemaCache>>,
    group_by: Vec<String>,
    order_by: Vec<(String, OrderBy)>, // field_name, order_by
    is_wildcard: bool,
    is_distinct: bool,
    has_agg_function: bool,
    use_inverted_index: bool,
}

impl<'a> ColumnVisitor<'a> {
    fn new(schemas: &'a HashMap<TableReference, Arc<SchemaCache>>) -> Self {
        Self {
            columns: HashMap::new(),
            columns_alias: HashSet::new(),
            schemas,
            group_by: Vec::new(),
            order_by: Vec::new(),
            is_wildcard: false,
            is_distinct: false,
            has_agg_function: false,
            use_inverted_index: false,
        }
    }
}

impl VisitorMut for ColumnVisitor<'_> {
    type Break = ();

    fn pre_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<Self::Break> {
        match expr {
            Expr::Identifier(ident) => {
                let field_name = ident.value.clone();
                for (name, schema) in self.schemas.iter() {
                    if schema.contains_field(&field_name) {
                        self.columns
                            .entry(name.clone())
                            .or_default()
                            .insert(field_name.clone());
                    }
                }
            }
            Expr::CompoundIdentifier(idents) => {
                let name = idents
                    .iter()
                    .map(|ident| ident.value.clone())
                    .collect::<Vec<_>>();
                let field_name = name.last().unwrap().clone();
                // check if table_name is in schemas, otherwise the table_name maybe is a alias
                for (name, schema) in self.schemas.iter() {
                    if schema.contains_field(&field_name) {
                        self.columns
                            .entry(name.clone())
                            .or_default()
                            .insert(field_name.clone());
                    }
                }
            }
            Expr::Function(f) => {
                if AGGREGATE_UDF_LIST
                    .contains(&trim_quotes(&f.name.to_string().to_lowercase()).as_str())
                {
                    self.has_agg_function = true;
                }
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let Some(order_by) = query.order_by.as_mut() {
            for order in order_by.exprs.iter_mut() {
                let mut name_visitor = FieldNameVisitor::new();
                order.expr.visit(&mut name_visitor);
                if name_visitor.field_names.len() == 1 {
                    let expr_name = name_visitor.field_names.iter().next().unwrap().to_string();
                    self.order_by.push((
                        expr_name,
                        if order.asc.unwrap_or(true) {
                            OrderBy::Asc
                        } else {
                            OrderBy::Desc
                        },
                    ));
                }
            }
        }
        if let sqlparser::ast::SetExpr::Select(select) = query.body.as_mut() {
            for select_item in select.projection.iter_mut() {
                match select_item {
                    SelectItem::ExprWithAlias { expr, alias } => {
                        self.columns_alias
                            .insert((expr.to_string(), alias.value.to_string()));
                    }
                    SelectItem::Wildcard(_) => {
                        self.is_wildcard = true;
                    }
                    _ => {}
                }
            }
            if let GroupByExpr::Expressions(exprs, _) = &mut select.group_by {
                for expr in exprs.iter_mut() {
                    let mut name_visitor = FieldNameVisitor::new();
                    expr.visit(&mut name_visitor);
                    if name_visitor.field_names.len() == 1 {
                        let expr_name = name_visitor.field_names.iter().next().unwrap().to_string();
                        self.group_by.push(expr_name);
                    }
                }
            }
            if select.distinct.is_some() {
                self.is_distinct = true;
            }
            if let Some(expr) = select.selection.as_ref() {
                // TODO: match_all only support single stream
                if self.schemas.len() == 1 {
                    for (_, schema) in self.schemas.iter() {
                        let stream_settings = unwrap_stream_settings(schema.schema());
                        let fts_fields = get_stream_setting_fts_fields(&stream_settings);
                        let index_fields = get_stream_setting_index_fields(&stream_settings);
                        let index_fields = itertools::chain(fts_fields.iter(), index_fields.iter())
                            .collect::<HashSet<_>>();
                        self.use_inverted_index =
                            checking_inverted_index_inner(&index_fields, expr);
                    }
                }
            }
        }
        ControlFlow::Continue(())
    }
}

// generate tantivy from sql and remove filter when we can
struct IndexVisitor {
    index_fields: HashSet<String>,
    is_remove_filter: bool,
    index_condition: Option<IndexCondition>,
    pub can_optimize: bool,
}

impl IndexVisitor {
    fn new(schemas: &HashMap<TableReference, Arc<SchemaCache>>, is_remove_filter: bool) -> Self {
        let index_fields = if let Some((_, schema)) = schemas.iter().next() {
            let stream_settings = unwrap_stream_settings(schema.schema());
            let index_fields = get_stream_setting_index_fields(&stream_settings);
            index_fields.into_iter().collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };
        Self {
            index_fields,
            is_remove_filter,
            index_condition: None,
            can_optimize: false,
        }
    }

    #[allow(dead_code)]
    fn new_from_index_fields(index_fields: HashSet<String>, is_remove_filter: bool) -> Self {
        Self {
            index_fields,
            is_remove_filter,
            index_condition: None,
            can_optimize: false,
        }
    }
}

impl VisitorMut for IndexVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let sqlparser::ast::SetExpr::Select(select) = query.body.as_mut() {
            if let Some(expr) = select.selection.as_mut() {
                let (index, other_expr) = get_index_condition_from_expr(&self.index_fields, expr);
                self.index_condition = index;
                // make sure all filter in where clause can be used in inverted index
                if other_expr.is_none() && select.selection.is_some() {
                    self.can_optimize = true;
                }
                if self.is_remove_filter {
                    select.selection = other_expr;
                }
            }
        }
        ControlFlow::Continue(())
    }
}

/// get all equal items from where clause
struct PartitionColumnVisitor<'a> {
    equal_items: HashMap<TableReference, Vec<(String, String)>>, // filed = value
    schemas: &'a HashMap<TableReference, Arc<SchemaCache>>,
}

impl<'a> PartitionColumnVisitor<'a> {
    fn new(schemas: &'a HashMap<TableReference, Arc<SchemaCache>>) -> Self {
        Self {
            equal_items: HashMap::new(),
            schemas,
        }
    }
}

impl VisitorMut for PartitionColumnVisitor<'_> {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
            if let Some(expr) = select.selection.as_ref() {
                let exprs = split_conjunction(expr);
                for e in exprs {
                    match e {
                        Expr::BinaryOp {
                            left,
                            op: BinaryOperator::Eq,
                            right,
                        } => {
                            let (left, right) = if is_value(left) && is_field(right) {
                                (right, left)
                            } else if is_value(right) && is_field(left) {
                                (left, right)
                            } else {
                                continue;
                            };
                            match left.as_ref() {
                                Expr::Identifier(ident) => {
                                    let mut count = 0;
                                    let field_name = ident.value.clone();
                                    let mut table_name = "".to_string();
                                    for (name, schema) in self.schemas.iter() {
                                        if schema.contains_field(&field_name) {
                                            count += 1;
                                            table_name = name.to_string();
                                        }
                                    }
                                    if count == 1 {
                                        self.equal_items
                                            .entry(TableReference::from(table_name))
                                            .or_default()
                                            .push((
                                                field_name,
                                                trim_quotes(right.to_string().as_str()),
                                            ));
                                    }
                                }
                                Expr::CompoundIdentifier(idents) => {
                                    let (table_name, field_name) = generate_table_reference(idents);
                                    // check if table_name is in schemas, otherwise the table_name
                                    // maybe is a alias
                                    if self.schemas.contains_key(&table_name) {
                                        self.equal_items.entry(table_name).or_default().push((
                                            field_name,
                                            trim_quotes(right.to_string().as_str()),
                                        ));
                                    }
                                }
                                _ => {}
                            }
                        }
                        Expr::InList {
                            expr,
                            list,
                            negated: false,
                        } => {
                            match expr.as_ref() {
                                Expr::Identifier(ident) => {
                                    let mut count = 0;
                                    let field_name = ident.value.clone();
                                    let mut table_name = "".to_string();
                                    for (name, schema) in self.schemas.iter() {
                                        if schema.contains_field(&field_name) {
                                            count += 1;
                                            table_name = name.to_string();
                                        }
                                    }
                                    if count == 1 {
                                        let entry = self
                                            .equal_items
                                            .entry(TableReference::from(table_name))
                                            .or_default();
                                        for val in list.iter() {
                                            entry.push((
                                                field_name.clone(),
                                                trim_quotes(val.to_string().as_str()),
                                            ));
                                        }
                                    }
                                }
                                Expr::CompoundIdentifier(idents) => {
                                    let (table_name, field_name) = generate_table_reference(idents);
                                    // check if table_name is in schemas, otherwise the table_name
                                    // maybe is a alias
                                    if self.schemas.contains_key(&table_name) {
                                        let entry = self.equal_items.entry(table_name).or_default();
                                        for val in list.iter() {
                                            entry.push((
                                                field_name.clone(),
                                                trim_quotes(val.to_string().as_str()),
                                            ));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        ControlFlow::Continue(())
    }
}

/// get all equal items from where clause
struct PrefixColumnVisitor<'a> {
    prefix_items: HashMap<TableReference, Vec<(String, String)>>, // filed like 'value%'
    schemas: &'a HashMap<TableReference, Arc<SchemaCache>>,
}

impl<'a> PrefixColumnVisitor<'a> {
    fn new(schemas: &'a HashMap<TableReference, Arc<SchemaCache>>) -> Self {
        Self {
            prefix_items: HashMap::new(),
            schemas,
        }
    }
}

impl VisitorMut for PrefixColumnVisitor<'_> {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
            if let Some(expr) = select.selection.as_ref() {
                let exprs = split_conjunction(expr);
                for e in exprs {
                    if let Expr::Like {
                        negated: false,
                        expr,
                        pattern,
                        escape_char: _,
                    } = e
                    {
                        match expr.as_ref() {
                            Expr::Identifier(ident) => {
                                let mut count = 0;
                                let field_name = ident.value.clone();
                                let mut table_name = "".to_string();
                                for (name, schema) in self.schemas.iter() {
                                    if schema.contains_field(&field_name) {
                                        count += 1;
                                        table_name = name.to_string();
                                    }
                                }
                                if count == 1 {
                                    let pattern = trim_quotes(pattern.to_string().as_str());
                                    if !pattern.starts_with('%') && pattern.ends_with('%') {
                                        self.prefix_items
                                            .entry(TableReference::from(table_name))
                                            .or_default()
                                            .push((
                                                field_name,
                                                pattern.trim_end_matches('%').to_string(),
                                            ));
                                    }
                                }
                            }
                            Expr::CompoundIdentifier(idents) => {
                                let (table_name, field_name) = generate_table_reference(idents);
                                // check if table_name is in schemas, otherwise the table_name
                                // maybe is a alias
                                if self.schemas.contains_key(&table_name) {
                                    let pattern = trim_quotes(pattern.to_string().as_str());
                                    if !pattern.starts_with('%') && pattern.ends_with('%') {
                                        self.prefix_items.entry(table_name).or_default().push((
                                            field_name,
                                            pattern.trim_end_matches('%').to_string(),
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        ControlFlow::Continue(())
    }
}

/// get all item from match_all functions
struct MatchVisitor {
    pub match_items: Option<Vec<String>>, // filed = value
}

impl MatchVisitor {
    fn new() -> Self {
        Self { match_items: None }
    }
}

impl VisitorMut for MatchVisitor {
    type Break = ();

    fn pre_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<Self::Break> {
        if let Expr::Function(func) = expr {
            let name = func.name.to_string().to_lowercase();
            if name == MATCH_ALL_UDF_NAME
                || name == MATCH_ALL_RAW_IGNORE_CASE_UDF_NAME
                || name == MATCH_ALL_RAW_UDF_NAME
                || name == FUZZY_MATCH_ALL_UDF_NAME
            {
                if let FunctionArguments::List(list) = &func.args {
                    if !list.args.is_empty() {
                        let value = trim_quotes(list.args[0].to_string().as_str());
                        match &mut self.match_items {
                            Some(items) => items.push(value),
                            None => self.match_items = Some(vec![value]),
                        }
                    }
                }
            }
        }
        ControlFlow::Continue(())
    }
}

struct FieldNameVisitor {
    pub field_names: HashSet<String>,
}

impl FieldNameVisitor {
    fn new() -> Self {
        Self {
            field_names: HashSet::new(),
        }
    }
}

impl VisitorMut for FieldNameVisitor {
    type Break = ();

    fn pre_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<Self::Break> {
        if let Expr::Identifier(ident) = expr {
            self.field_names.insert(ident.value.clone());
        }
        ControlFlow::Continue(())
    }
}

// add _timestamp to the query like `SELECT name FROM t` -> `SELECT _timestamp, name FROM t`
struct AddTimestampVisitor {}

impl AddTimestampVisitor {
    fn new() -> Self {
        Self {}
    }
}

impl VisitorMut for AddTimestampVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let SetExpr::Select(select) = query.body.as_mut() {
            let mut has_timestamp = false;
            for item in select.projection.iter_mut() {
                match item {
                    SelectItem::UnnamedExpr(expr) => {
                        let mut visitor = FieldNameVisitor::new();
                        expr.visit(&mut visitor);
                        if visitor
                            .field_names
                            .contains(&get_config().common.column_timestamp)
                        {
                            has_timestamp = true;
                            break;
                        }
                    }
                    SelectItem::ExprWithAlias { expr, alias: _ } => {
                        let mut visitor = FieldNameVisitor::new();
                        expr.visit(&mut visitor);
                        if visitor
                            .field_names
                            .contains(&get_config().common.column_timestamp)
                        {
                            has_timestamp = true;
                            break;
                        }
                    }
                    SelectItem::Wildcard(_) => {
                        has_timestamp = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !has_timestamp {
                select.projection.insert(
                    0,
                    SelectItem::UnnamedExpr(Expr::Identifier(Ident::new(
                        get_config().common.column_timestamp.clone(),
                    ))),
                );
            }
        }
        ControlFlow::Continue(())
    }
}

// add _o2_id to the query like `SELECT name FROM t` -> `SELECT _o2_id, name FROM t`
struct AddO2IdVisitor {}

impl AddO2IdVisitor {
    fn new() -> Self {
        Self {}
    }
}

impl VisitorMut for AddO2IdVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        if let SetExpr::Select(select) = query.body.as_mut() {
            let mut has_o2_id = false;
            for item in select.projection.iter_mut() {
                match item {
                    SelectItem::UnnamedExpr(expr) => {
                        let mut visitor = FieldNameVisitor::new();
                        expr.visit(&mut visitor);
                        if visitor.field_names.contains(ID_COL_NAME) {
                            has_o2_id = true;
                            break;
                        }
                    }
                    SelectItem::ExprWithAlias { expr, alias: _ } => {
                        let mut visitor = FieldNameVisitor::new();
                        expr.visit(&mut visitor);
                        if visitor.field_names.contains(ID_COL_NAME) {
                            has_o2_id = true;
                            break;
                        }
                    }
                    SelectItem::Wildcard(_) => {
                        has_o2_id = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !has_o2_id {
                select.projection.insert(
                    0,
                    SelectItem::UnnamedExpr(Expr::Identifier(Ident::new(ID_COL_NAME.to_string()))),
                );
            }
        }
        ControlFlow::Continue(())
    }
}

fn is_simple_count_query(statement: &mut Statement) -> bool {
    let mut visitor = SimpleCountVisitor::new();
    statement.visit(&mut visitor);
    visitor.is_simple_count
}

// check if the query is simple count query
// 1. don't has subquery
// 2. don't has join
// 3. don't has group by
// 4. only has count(*)
// 5. don't has SetOperation(UNION/EXCEPT/INTERSECT of two queries)
// 6. don't has distinct
struct SimpleCountVisitor {
    pub is_simple_count: bool,
}

impl SimpleCountVisitor {
    fn new() -> Self {
        Self {
            is_simple_count: false,
        }
    }
}

impl VisitorMut for SimpleCountVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        // Check if the query is select *
        if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
            if select.projection.len() != 1
                || select.distinct.is_some()
                || select.from.len() > 1
                || select.from.iter().any(|from| !from.joins.is_empty())
                || !matches!(select.group_by, GroupByExpr::Expressions(ref expr, _) if expr.is_empty())
            {
                return ControlFlow::Break(());
            }

            self.is_simple_count = match &select.projection[0] {
                SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => {
                    if let Expr::Function(func) = expr {
                        let name = trim_quotes(&func.name.to_string().to_lowercase());
                        name == "count"
                            && func.filter.is_none()
                            && func.over.is_none()
                            && func.within_group.is_empty()
                            && matches!(
                                &func.args,
                                FunctionArguments::List(list) if list.args.len() == 1 && trim_quotes(&list.args[0].to_string()) == "*"
                            )
                    } else {
                        false
                    }
                }
                _ => false,
            };
        }
        if !self.is_simple_count {
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<Self::Break> {
        if matches!(
            expr,
            Expr::Subquery(_) | Expr::Exists { .. } | Expr::InSubquery { .. }
        ) {
            self.is_simple_count = false;
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }
}

fn is_complex_query(statement: &mut Statement) -> bool {
    let mut visitor = ComplexQueryVisitor::new();
    statement.visit(&mut visitor);
    visitor.is_complex
}

// check if the query is complex query
// 1. has subquery
// 2. has join
// 3. has group by
// 4. has aggregate
// 5. has SetOperation(UNION/EXCEPT/INTERSECT of two queries)
// 6. has distinct
// 7. has wildcard
struct ComplexQueryVisitor {
    pub is_complex: bool,
}

impl ComplexQueryVisitor {
    fn new() -> Self {
        Self { is_complex: false }
    }
}

impl VisitorMut for ComplexQueryVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        match query.body.as_ref() {
            sqlparser::ast::SetExpr::Select(select) => {
                // check if has group by
                match select.group_by {
                    GroupByExpr::Expressions(ref expr, _) => self.is_complex = !expr.is_empty(),
                    _ => self.is_complex = true,
                }
                // check if has join
                if select.from.len() > 1 || select.from.iter().any(|from| !from.joins.is_empty()) {
                    self.is_complex = true;
                }
                if select.distinct.is_some() {
                    self.is_complex = true;
                }
                if self.is_complex {
                    return ControlFlow::Break(());
                }
            }
            // check if SetOperation
            sqlparser::ast::SetExpr::SetOperation { .. } => {
                self.is_complex = true;
                return ControlFlow::Break(());
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<Self::Break> {
        match expr {
            // check if has subquery
            Expr::Subquery(_) | Expr::Exists { .. } | Expr::InSubquery { .. } => {
                self.is_complex = true;
            }
            // check if has aggregate function or window function
            Expr::Function(func) => {
                if AGGREGATE_UDF_LIST
                    .contains(&trim_quotes(&func.name.to_string().to_lowercase()).as_str())
                    || func.filter.is_some()
                    || func.over.is_some()
                    || !func.within_group.is_empty()
                {
                    self.is_complex = true;
                }
            }
            // check select * from table
            Expr::Wildcard => self.is_complex = true,
            _ => {}
        }
        if self.is_complex {
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    }
}

struct HistogramIntervalVistor {
    pub interval: Option<i64>,
    time_range: Option<(i64, i64)>,
}

impl HistogramIntervalVistor {
    fn new(time_range: Option<(i64, i64)>) -> Self {
        Self {
            interval: None,
            time_range,
        }
    }
}

impl VisitorMut for HistogramIntervalVistor {
    type Break = ();

    fn pre_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<Self::Break> {
        if let Expr::Function(func) = expr {
            if func.name.to_string().to_lowercase() == "histogram" {
                if let FunctionArguments::List(list) = &func.args {
                    let mut args = list.args.iter();
                    // first is field
                    let _ = args.next();
                    // second is interval
                    let interval = if let Some(interval) = args.next() {
                        let interval = interval
                            .to_string()
                            .trim_matches(|v| v == '\'' || v == '"')
                            .to_string();
                        match interval.parse::<u16>() {
                            Ok(v) => generate_histogram_interval(self.time_range, v),
                            Err(_) => interval,
                        }
                    } else {
                        generate_histogram_interval(self.time_range, 0)
                    };
                    self.interval =
                        Some(convert_histogram_interval_to_seconds(&interval).unwrap_or_default());
                }
            }
        }
        ControlFlow::Break(())
    }
}

struct TrackTotalHitsVisitor {}

impl TrackTotalHitsVisitor {
    fn new() -> Self {
        Self {}
    }
}

impl VisitorMut for TrackTotalHitsVisitor {
    type Break = ();

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<Self::Break> {
        match query.body.as_mut() {
            SetExpr::Select(select) => {
                let (field_expr, duplicate_treatment) = if select.distinct.is_some() {
                    match select.projection.first() {
                        Some(SelectItem::UnnamedExpr(expr)) => (
                            FunctionArgExpr::Expr(expr.clone()),
                            Some(DuplicateTreatment::Distinct),
                        ),
                        Some(SelectItem::ExprWithAlias { expr, alias: _ }) => (
                            FunctionArgExpr::Expr(expr.clone()),
                            Some(DuplicateTreatment::Distinct),
                        ),
                        _ => (FunctionArgExpr::Wildcard, None),
                    }
                } else {
                    (FunctionArgExpr::Wildcard, None)
                };

                select.group_by = GroupByExpr::Expressions(vec![], vec![]);
                select.having = None;
                select.sort_by = vec![];
                select.projection = vec![SelectItem::ExprWithAlias {
                    expr: Expr::Function(Function {
                        name: ObjectName(vec![Ident::new("count")]),
                        parameters: FunctionArguments::None,
                        args: FunctionArguments::List(FunctionArgumentList {
                            args: vec![FunctionArg::Unnamed(field_expr)],
                            duplicate_treatment,
                            clauses: vec![],
                        }),
                        filter: None,
                        null_treatment: None,
                        over: None,
                        within_group: vec![],
                    }),
                    alias: Ident::new("zo_sql_num"),
                }];
                select.distinct = None;
                query.order_by = None;
            }
            SetExpr::SetOperation { .. } => {
                let select = Box::new(SetExpr::Select(Box::new(Select {
                    distinct: None,
                    top: None,
                    projection: vec![SelectItem::ExprWithAlias {
                        expr: Expr::Function(Function {
                            name: ObjectName(vec![Ident::new("count")]),
                            parameters: FunctionArguments::None,
                            args: FunctionArguments::List(FunctionArgumentList {
                                args: vec![FunctionArg::Unnamed(FunctionArgExpr::Wildcard)],
                                duplicate_treatment: None,
                                clauses: vec![],
                            }),
                            filter: None,
                            null_treatment: None,
                            over: None,
                            within_group: vec![],
                        }),
                        alias: Ident::new("zo_sql_num"),
                    }],
                    into: None,
                    from: vec![TableWithJoins {
                        relation: TableFactor::Derived {
                            lateral: false,
                            subquery: Box::new(query.clone()),
                            alias: None,
                        },
                        joins: vec![],
                    }],
                    lateral_views: vec![],
                    selection: None,
                    group_by: GroupByExpr::Expressions(vec![], vec![]),
                    having: None,
                    prewhere: None,
                    sort_by: vec![],
                    cluster_by: vec![],
                    distribute_by: vec![],
                    named_window: vec![],
                    qualify: None,
                    window_before_qualify: false,
                    connect_by: None,
                    value_table_mode: None,
                })));
                *query = Query {
                    with: None,
                    body: select,
                    order_by: None,
                    limit: None,
                    offset: None,
                    fetch: None,
                    limit_by: vec![],
                    for_clause: None,
                    locks: vec![],
                    settings: None,
                    format_clause: None,
                };
            }
            _ => {}
        }
        ControlFlow::Break(())
    }
}

fn generate_table_reference(idents: &[Ident]) -> (TableReference, String) {
    if idents.len() == 2 {
        let table_name = idents[0].value.clone();
        let field_name = idents[1].value.clone();
        (TableReference::from(table_name), field_name)
    } else {
        let stream_type = idents[0].value.clone();
        let table_name = idents[1].value.clone();
        let field_name = idents[2].value.clone();
        (TableReference::partial(stream_type, table_name), field_name)
    }
}

fn checking_inverted_index_inner(index_fields: &HashSet<&String>, expr: &Expr) -> bool {
    match expr {
        Expr::Identifier(Ident {
            value,
            quote_style: _,
        }) => index_fields.contains(value),
        Expr::Nested(expr) => checking_inverted_index_inner(index_fields, expr),
        Expr::BinaryOp { left, op, right } => match op {
            BinaryOperator::And => true,
            BinaryOperator::Or => {
                checking_inverted_index_inner(index_fields, left)
                    && checking_inverted_index_inner(index_fields, right)
            }
            BinaryOperator::Eq => checking_inverted_index_inner(index_fields, left),
            _ => false,
        },
        Expr::InList {
            expr,
            list: _,
            negated: _,
        } => checking_inverted_index_inner(index_fields, expr),
        Expr::Like {
            negated: _,
            expr,
            pattern: _,
            escape_char: _,
        } => checking_inverted_index_inner(index_fields, expr),
        Expr::Function(f) => {
            let f = f.name.to_string().to_lowercase();
            f == MATCH_ALL_UDF_NAME || f == FUZZY_MATCH_ALL_UDF_NAME
        }
        _ => false,
    }
}

pub fn generate_histogram_interval(time_range: Option<(i64, i64)>, num: u16) -> String {
    if time_range.is_none() || time_range.unwrap().eq(&(0, 0)) {
        return "1 hour".to_string();
    }
    let time_range = time_range.unwrap();
    if num > 0 {
        return format!(
            "{} second",
            std::cmp::max(
                (time_range.1 - time_range.0)
                    / Duration::try_seconds(1)
                        .unwrap()
                        .num_microseconds()
                        .unwrap()
                    / num as i64,
                1
            )
        );
    }

    let intervals = [
        (Duration::try_hours(24 * 60), "1 day"),
        (Duration::try_hours(24 * 30), "12 hour"),
        (Duration::try_hours(24 * 28), "6 hour"),
        (Duration::try_hours(24 * 21), "3 hour"),
        (Duration::try_hours(24 * 15), "2 hour"),
        (Duration::try_hours(6), "1 hour"),
        (Duration::try_hours(2), "1 minute"),
        (Duration::try_hours(1), "30 second"),
        (Duration::try_minutes(30), "15 second"),
        (Duration::try_minutes(15), "10 second"),
    ];
    for (time, interval) in intervals.iter() {
        let time = time.unwrap().num_microseconds().unwrap();
        if (time_range.1 - time_range.0) >= time {
            return interval.to_string();
        }
    }
    "10 second".to_string()
}

pub fn convert_histogram_interval_to_seconds(interval: &str) -> Result<i64, Error> {
    let Some((num, unit)) = interval.splitn(2, ' ').collect_tuple() else {
        return Err(Error::Message("Invalid interval format".to_string()));
    };
    let seconds = match unit.to_lowercase().as_str() {
        "second" | "seconds" => num.parse::<i64>(),
        "minute" | "minutes" => num.parse::<i64>().map(|n| n * 60),
        "hour" | "hours" => num.parse::<i64>().map(|n| n * 3600),
        "day" | "days" => num.parse::<i64>().map(|n| n * 86400),
        _ => {
            return Err(Error::Message(
                "Unsupported histogram interval unit".to_string(),
            ));
        }
    };
    seconds.map_err(|_| Error::Message("Invalid number format".to_string()))
}

pub fn pickup_where(sql: &str, meta: Option<MetaSql>) -> Result<Option<String>, Error> {
    let meta = match meta {
        Some(v) => v,
        None => match MetaSql::new(sql) {
            Ok(meta) => meta,
            Err(_) => {
                return Err(Error::ErrorCode(ErrorCodes::SearchSQLNotValid(
                    sql.to_string(),
                )));
            }
        },
    };
    let Some(caps) = RE_WHERE.captures(sql) else {
        return Ok(None);
    };
    let mut where_str = caps.get(1).unwrap().as_str().to_string();
    if !meta.group_by.is_empty() {
        where_str = where_str[0..where_str.to_lowercase().rfind(" group ").unwrap()].to_string();
    } else if meta.having {
        where_str = where_str[0..where_str.to_lowercase().rfind(" having ").unwrap()].to_string();
    } else if !meta.order_by.is_empty() {
        where_str = where_str[0..where_str.to_lowercase().rfind(" order ").unwrap()].to_string();
    } else if meta.limit > 0 || where_str.to_lowercase().ends_with(" limit 0") {
        where_str = where_str[0..where_str.to_lowercase().rfind(" limit ").unwrap()].to_string();
    } else if meta.offset > 0 {
        where_str = where_str[0..where_str.to_lowercase().rfind(" offset ").unwrap()].to_string();
    }
    Ok(Some(where_str))
}

fn o2_id_is_needed(schemas: &HashMap<TableReference, Arc<SchemaCache>>) -> bool {
    schemas.values().any(|schema| {
        let stream_setting = unwrap_stream_settings(schema.schema());
        stream_setting.map_or(false, |setting| setting.store_original_data)
    })
}

#[cfg(feature = "enterprise")]
struct ExtractKeyNamesVisitor {
    keys: Vec<String>,
    error: Option<Error>,
}

#[cfg(feature = "enterprise")]
impl ExtractKeyNamesVisitor {
    fn new() -> Self {
        Self {
            keys: Vec::new(),
            error: None,
        }
    }
}

#[cfg(feature = "enterprise")]
impl VisitorMut for ExtractKeyNamesVisitor {
    type Break = ();

    fn pre_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<Self::Break> {
        if let Expr::Function(Function {
            name: ObjectName(names),
            args,
            ..
        }) = expr
        {
            // cipher functions will always be 1-part names
            if names.len() != 1 {
                return ControlFlow::Continue(());
            }
            let fname = names.first().unwrap();
            if fname.value == ENCRYPT_UDF_NAME || fname.value == DECRYPT_UDF_NAME {
                let list = match args {
                    FunctionArguments::List(list) => list,
                    _ => {
                        self.error = Some(Error::Message(
                            "invalid arguments to cipher function".to_string(),
                        ));
                        return ControlFlow::Continue(());
                    }
                };
                if list.args.len() != 2 {
                    self.error = Some(Error::Message(
                        "invalid number of arguments to cipher function".to_string(),
                    ));
                    return ControlFlow::Continue(());
                }
                let arg = match &list.args[1] {
                    FunctionArg::Named { arg, .. } => arg,
                    FunctionArg::Unnamed(arg) => arg,
                };
                match arg {
                    FunctionArgExpr::Expr(Expr::Value(
                        sqlparser::ast::Value::SingleQuotedString(s),
                    )) => {
                        self.keys.push(s.to_owned());
                    }
                    _ => {
                        self.error = Some(Error::Message(
                            "key name must be a static string in cipher function".to_string(),
                        ));
                        return ControlFlow::Continue(());
                    }
                }
            }
        }
        ControlFlow::Continue(())
    }
}

#[cfg(feature = "enterprise")]
pub fn get_cipher_key_names(sql: &str) -> Result<Vec<String>, Error> {
    let dialect = &PostgreSqlDialect {};
    let mut statement = Parser::parse_sql(dialect, sql)
        .map_err(|e| Error::Message(e.to_string()))?
        .pop()
        .unwrap();
    let mut visitor = ExtractKeyNamesVisitor::new();
    statement.visit(&mut visitor);
    if let Some(e) = visitor.error {
        Err(e)
    } else {
        Ok(visitor.keys)
    }
}

#[cfg(test)]
mod tests {

    use sqlparser::dialect::GenericDialect;

    use super::*;

    #[test]
    fn test_index_visitor1() {
        let sql = "SELECT * FROM t WHERE name = 'a' AND age = 1 AND (name = 'b' OR (match_all('good') AND match_all('bar'))) AND (match_all('foo') OR age = 2)";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut index_fields = HashSet::new();
        index_fields.insert("name".to_string());
        let mut index_visitor = IndexVisitor::new_from_index_fields(index_fields, true);
        statement.visit(&mut index_visitor);
        let expected = "name=a AND (name=b OR (_all:good AND _all:bar))";
        let expected_sql = "SELECT * FROM t WHERE age = 1 AND (match_all('foo') OR age = 2)";
        assert_eq!(
            index_visitor.index_condition.clone().unwrap().to_query(),
            expected
        );
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_index_visitor2() {
        let sql = "SELECT * FROM t WHERE name is not null AND age > 1 AND (match_all('foo') OR abs(age) = 2)";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut index_fields = HashSet::new();
        index_fields.insert("name".to_string());
        let mut index_visitor = IndexVisitor::new_from_index_fields(index_fields, true);
        statement.visit(&mut index_visitor);
        let expected = "";
        let expected_sql = "SELECT * FROM t WHERE name IS NOT NULL AND (age > 1) AND (match_all('foo') OR abs(age) = 2)";
        assert_eq!(
            index_visitor
                .index_condition
                .clone()
                .unwrap_or_default()
                .to_query(),
            expected
        );
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_index_visitor3() {
        let sql = "SELECT * FROM t WHERE (name = 'b' OR (match_all('good') AND match_all('bar'))) OR (match_all('foo') OR age = 2)";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut index_fields = HashSet::new();
        index_fields.insert("name".to_string());
        let mut index_visitor = IndexVisitor::new_from_index_fields(index_fields, true);
        statement.visit(&mut index_visitor);
        let expected = "";
        let expected_sql = "SELECT * FROM t WHERE (name = 'b' OR (match_all('good') AND match_all('bar'))) OR (match_all('foo') OR age = 2)";
        assert_eq!(
            index_visitor
                .index_condition
                .clone()
                .unwrap_or_default()
                .to_query(),
            expected
        );
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_index_visitor4() {
        let sql = "SELECT * FROM t WHERE (name = 'b' OR (match_all('good') AND match_all('bar'))) OR (match_all('foo') AND name = 'c')";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut index_fields = HashSet::new();
        index_fields.insert("name".to_string());
        let mut index_visitor = IndexVisitor::new_from_index_fields(index_fields, true);
        statement.visit(&mut index_visitor);
        let expected = "((name=b OR (_all:good AND _all:bar)) OR (_all:foo AND name=c))";
        let expected_sql = "SELECT * FROM t";
        assert_eq!(
            index_visitor.index_condition.clone().unwrap().to_query(),
            expected
        );
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_track_total_hits1() {
        let sql = "SELECT * FROM t WHERE name = 'a'";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut track_total_hits_visitor = TrackTotalHitsVisitor::new();
        statement.visit(&mut track_total_hits_visitor);
        let expected_sql = "SELECT count(*) AS zo_sql_num FROM t WHERE name = 'a'";
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_track_total_hits2() {
        let sql = "SELECT name, count(*) FROM t WHERE name = 'a' group by name order by name";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut track_total_hits_visitor = TrackTotalHitsVisitor::new();
        statement.visit(&mut track_total_hits_visitor);
        let expected_sql = "SELECT count(*) AS zo_sql_num FROM t WHERE name = 'a'";
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_track_total_hits3() {
        let sql = "SELECT t1.name, t2.name from t1 join t2 on t1.name = t2.name where t1.name = 'openobserve' group by t1.name, t2.name order by t1.name, t2.name";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut track_total_hits_visitor = TrackTotalHitsVisitor::new();
        statement.visit(&mut track_total_hits_visitor);
        let expected_sql = "SELECT count(*) AS zo_sql_num FROM t1 JOIN t2 ON t1.name = t2.name WHERE t1.name = 'openobserve'";
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_track_total_hits4() {
        let sql = "SELECT name from t1 where name not in (select name from t2)";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut track_total_hits_visitor = TrackTotalHitsVisitor::new();
        statement.visit(&mut track_total_hits_visitor);
        let expected_sql =
            "SELECT count(*) AS zo_sql_num FROM t1 WHERE name NOT IN (SELECT name FROM t2)";
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_track_total_hits5() {
        let sql = "SELECT name from t1 union select name from t2";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut track_total_hits_visitor = TrackTotalHitsVisitor::new();
        statement.visit(&mut track_total_hits_visitor);
        let expected_sql =
            "SELECT count(*) AS zo_sql_num FROM (SELECT name FROM t1 UNION SELECT name FROM t2)";
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_track_total_hits6() {
        let sql = "(SELECT name from t1) union (select name from t2)";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut track_total_hits_visitor = TrackTotalHitsVisitor::new();
        statement.visit(&mut track_total_hits_visitor);
        let expected_sql = "SELECT count(*) AS zo_sql_num FROM ((SELECT name FROM t1) UNION (SELECT name FROM t2))";
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_track_total_hits7() {
        let sql = "SELECT name from t1 union select name from t2 union select name from t3";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        let mut track_total_hits_visitor = TrackTotalHitsVisitor::new();
        statement.visit(&mut track_total_hits_visitor);
        let expected_sql = "SELECT count(*) AS zo_sql_num FROM (SELECT name FROM t1 UNION SELECT name FROM t2 UNION SELECT name FROM t3)";
        assert_eq!(statement.to_string(), expected_sql);
    }

    #[test]
    fn test_is_simple_count_visit1() {
        let sql = "SELECT count(*) from t";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(is_simple_count_query(&mut statement), true);
    }

    #[test]
    fn test_is_simple_count_visit2() {
        let sql = "SELECT count(*) as cnt from t";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(is_simple_count_query(&mut statement), true);
    }

    #[test]
    fn test_is_simple_count_visit3() {
        let sql = "SELECT count(*) as cnt from t where name = 'a'";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(is_simple_count_query(&mut statement), true);
    }

    #[test]
    fn test_is_simple_count_visit4() {
        let sql = "SELECT name, count(*) as cnt from t group by name order by name";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(is_simple_count_query(&mut statement), false);
    }

    #[test]
    fn test_is_simple_count_visit5() {
        let sql = "SELECT count(_timestamp) as cnt from t";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(is_simple_count_query(&mut statement), false);
    }

    #[test]
    fn test_is_simple_count_visit6() {
        let sql = "SELECT count(*) as cnt from (select * from t)";
        let mut statement = sqlparser::parser::Parser::parse_sql(&GenericDialect {}, &sql)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(is_simple_count_query(&mut statement), false);
    }
}
