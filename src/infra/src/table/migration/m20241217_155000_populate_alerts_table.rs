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

//! Populates the `alerts` table by transforming unstructured alert records from
//! the `meta` table.

use itertools::{self, Itertools};
use sea_orm::{
    ActiveValue::NotSet, ColumnTrait, EntityTrait, FromQueryResult, Paginator, PaginatorTrait,
    QueryFilter, SelectModel, Set, Statement, TransactionTrait,
};
use sea_orm_migration::prelude::*;
use svix_ksuid::KsuidLike;

/// The value of the flag used to indicate the alerts folder type in the folders
/// table.
const ALERTS_FOLDER_TYPE: i16 = 1;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let txn = manager.get_connection().begin().await?;

        // Select all orgs that have an alert. Create a default alert folder for
        // each org. Migrate pages of 100 records at a time to avoid loading too
        // many records into memory.
        let mut org_pages = OrgWithAlert::paginate(&txn, 100);
        while let Some(orgs) = org_pages.fetch_and_next().await? {
            let folders = orgs
                .into_iter()
                .map(|o| folders_table::ActiveModel {
                    id: NotSet,
                    org: Set(o.org_name().to_owned()),
                    folder_id: Set("default".to_owned()),
                    name: Set("default".to_owned()),
                    description: Set(Some("default".to_owned())),
                    r#type: Set(ALERTS_FOLDER_TYPE),
                })
                .collect_vec();
            folders_table::Entity::insert_many(folders)
                .exec(&txn)
                .await?;
        }

        // Select each alert from the meta table joined on its associated
        // default folder. For each alert from the meta table insert a
        // corresponding record in the new alerts table. Migrate pages of 100
        // records at a time to avoid loading too many records into memory.
        let mut meta_pages = MetaAlertWithFolder::paginate(&txn, 100);
        while let Some(metas) = meta_pages.fetch_and_next().await? {
            let alerts_rslt: Result<Vec<_>, DbErr> = metas
                .into_iter()
                .map(|m| {
                    let m: alerts_table::ActiveModel = m.try_into().map_err(DbErr::Migration)?;
                    Ok(m)
                })
                .collect();
            let alerts = alerts_rslt?;
            alerts_table::Entity::insert_many(alerts).exec(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        alerts_table::Entity::delete_many().exec(db).await?;
        folders_table::Entity::delete_many()
            .filter(folders_table::Column::Type.eq(ALERTS_FOLDER_TYPE))
            .exec(db)
            .await?;
        Ok(())
    }
}

/// Identifiers used in queries on the meta table.
#[derive(DeriveIden)]
enum Folders {
    Table,
    Id,
    Org,
    FolderId,
    Type,
}

/// Identifiers used in queries on the folders table.
#[derive(DeriveIden)]
enum Meta {
    Table,
    Id,
    Module,
    Key1,
    Value,
}

/// A result from querying for unique names of orgs with alerts from the meta
/// table.
#[derive(Debug, FromQueryResult)]
pub struct OrgWithAlert {
    /// The name of the org.
    key1: String,
}

impl OrgWithAlert {
    pub fn org_name(&self) -> &str {
        &self.key1
    }
}

impl OrgWithAlert {
    /// Returns the select statment to select unique names of orgs with alerts
    /// from the meta table.
    fn statement() -> SelectStatement {
        Query::select()
            .column(Meta::Key1)
            .from(Meta::Table)
            .and_where(Expr::col(Meta::Module).eq("alerts"))
            .group_by_col(Meta::Key1)
            .order_by(Meta::Key1, Order::Asc)
            .to_owned()
    }

    /// Paginate through the results of querying for unique names of orgs with
    /// alerts from the meta table.
    fn paginate<C>(db: &C, page_size: u64) -> Paginator<'_, C, SelectModel<OrgWithAlert>>
    where
        C: ConnectionTrait,
    {
        let select_statement = Self::statement();
        let backend = db.get_database_backend();
        let (sql, values) = match backend {
            sea_orm::DatabaseBackend::MySql => select_statement.build(MysqlQueryBuilder),
            sea_orm::DatabaseBackend::Postgres => select_statement.build(PostgresQueryBuilder),
            sea_orm::DatabaseBackend::Sqlite => select_statement.build(SqliteQueryBuilder),
        };
        let statement = Statement::from_sql_and_values(backend, sql, values);
        Self::find_by_statement(statement).paginate(db, page_size)
    }
}

/// A result from querying for alerts from the meta table and joining on the
/// folders table.
#[derive(Debug, FromQueryResult)]
pub struct MetaAlertWithFolder {
    /// The JSON representation of the alert.
    value: String,

    /// The ID of the folder that the alert belongs to.
    id: Option<i64>,
}

impl MetaAlertWithFolder {
    pub fn alert_json(&self) -> &str {
        &self.value
    }

    pub fn folder_id(&self) -> Option<i64> {
        self.id
    }
}

impl MetaAlertWithFolder {
    /// Returns the select statment to select alerts from the meta table joined
    /// on the folders table.
    fn statement() -> SelectStatement {
        Query::select()
            .column((Meta::Table, Meta::Value))
            .column((Folders::Table, Folders::Id))
            .from(Meta::Table)
            .left_join(
                Folders::Table,
                Expr::col((Folders::Table, Folders::Org)).equals((Meta::Table, Meta::Key1)),
            )
            .and_where(Expr::col((Meta::Table, Meta::Module)).eq("alerts"))
            .and_where(Expr::col((Folders::Table, Folders::Type)).eq(ALERTS_FOLDER_TYPE))
            .and_where(Expr::col((Folders::Table, Folders::FolderId)).eq("default"))
            .order_by((Meta::Table, Meta::Id), Order::Asc)
            .to_owned()
    }

    /// Paginate through the results of querying for dashboards from the meta
    /// table and joining on the folders table.
    fn paginate<C>(db: &C, page_size: u64) -> Paginator<'_, C, SelectModel<MetaAlertWithFolder>>
    where
        C: ConnectionTrait,
    {
        let select_statement = Self::statement();
        let backend = db.get_database_backend();
        let (sql, values) = match backend {
            sea_orm::DatabaseBackend::MySql => select_statement.build(MysqlQueryBuilder),
            sea_orm::DatabaseBackend::Postgres => select_statement.build(PostgresQueryBuilder),
            sea_orm::DatabaseBackend::Sqlite => select_statement.build(SqliteQueryBuilder),
        };
        let statement = Statement::from_sql_and_values(backend, sql, values);
        Self::find_by_statement(statement).paginate(db, page_size)
    }
}

// The schemas of tables might change after subsequent migrations. Therefore
// this migration only references ORM models in private submodules that should
// remain unchanged rather than ORM models in the `entity` module that will be
// updated to reflect the latest changes to table schemas.

/// Representation of the folders table at the time this migration executes.
mod folders_table {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "folders")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub org: String,
        pub folder_id: String,
        pub name: String,
        #[sea_orm(column_type = "Text", nullable)]
        pub description: Option<String>,
        pub r#type: i16,
    }

    // There are relations but they are not important to this migration.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Representation of the alerts table at the time this migration executes.
mod alerts_table {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
    #[sea_orm(table_name = "alerts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub org: String,
        pub folder_id: i64,
        pub name: String,
        pub stream_type: String,
        pub stream_name: String,
        pub is_real_time: bool,
        pub destinations: Json,
        pub context_attributes: Option<Json>,
        #[sea_orm(column_type = "Text", nullable)]
        pub row_template: Option<String>,
        #[sea_orm(column_type = "Text", nullable)]
        pub description: Option<String>,
        pub enabled: bool,
        pub tz_offset: i32,
        pub last_triggered_at: Option<i64>,
        pub last_satisfied_at: Option<i64>,
        pub query_type: i16,
        pub query_conditions: Option<Json>,
        #[sea_orm(column_type = "Text", nullable)]
        pub query_sql: Option<String>,
        #[sea_orm(column_type = "Text", nullable)]
        pub query_promql: Option<String>,
        pub query_promql_condition: Option<Json>,
        pub query_aggregation: Option<Json>,
        #[sea_orm(column_type = "Text", nullable)]
        pub query_vrl_function: Option<String>,
        pub query_search_event_type: Option<i16>,
        pub query_multi_time_range: Option<Json>,
        pub trigger_threshold_operator: String,
        pub trigger_period_seconds: i64,
        pub trigger_threshold_count: i64,
        pub trigger_frequency_type: i16,
        pub trigger_frequency_seconds: i64,
        #[sea_orm(column_type = "Text", nullable)]
        pub trigger_frequency_cron: Option<String>,
        pub trigger_frequency_cron_timezone: Option<String>,
        pub trigger_silence_seconds: i64,
        pub trigger_tolerance_seconds: Option<i64>,
        pub owner: Option<String>,
        pub last_edited_by: Option<String>,
        pub updated_at: Option<i64>,
    }

    // There are relations but they are not important to this migration.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Defines the string and JSON serialization schemas used to store various
/// alert child structures in the new alerts table at the time this migration
/// executes.
mod alerts_table_ser {

    use std::fmt::Display;

    use serde::Serialize;
    use serde_json::Value as JsonValue;

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct CompareHistoricData {
        pub offset: String,
    }

    pub enum FrequencyType {
        Cron,
        Seconds,
    }

    impl From<FrequencyType> for i16 {
        fn from(value: FrequencyType) -> Self {
            match value {
                FrequencyType::Cron => 0,
                FrequencyType::Seconds => 1,
            }
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Aggregation {
        pub group_by: Option<Vec<String>>,
        pub function: AggFunction,
        pub having: Condition,
    }

    #[derive(Serialize)]
    pub enum AggFunction {
        #[serde(rename = "avg")]
        Avg,
        #[serde(rename = "min")]
        Min,
        #[serde(rename = "max")]
        Max,
        #[serde(rename = "sum")]
        Sum,
        #[serde(rename = "count")]
        Count,
        #[serde(rename = "median")]
        Median,
        #[serde(rename = "p50")]
        P50,
        #[serde(rename = "p75")]
        P75,
        #[serde(rename = "p90")]
        P90,
        #[serde(rename = "p95")]
        P95,
        #[serde(rename = "p99")]
        P99,
    }

    pub enum QueryType {
        Custom,
        Sql,
        Promql,
    }

    impl From<QueryType> for i16 {
        fn from(value: QueryType) -> Self {
            match value {
                QueryType::Custom => 0,
                QueryType::Sql => 1,
                QueryType::Promql => 2,
            }
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Condition {
        pub column: String,
        pub operator: ConditionOperator,
        pub value: JsonValue,
        pub ignore_case: bool,
    }

    #[derive(Serialize)]
    pub enum ConditionOperator {
        #[serde(rename = "=")]
        EqualTo,
        #[serde(rename = "!=")]
        NotEqualTo,
        #[serde(rename = ">")]
        GreaterThan,
        #[serde(rename = ">=")]
        GreaterThanEquals,
        #[serde(rename = "<")]
        LessThan,
        #[serde(rename = "<=")]
        LessThanEquals,
        #[serde(rename = "contains")]
        Contains,
        #[serde(rename = "not_contains")]
        NotContains,
    }

    pub enum ThresholdOperator {
        EqualTo,
        NotEqualTo,
        GreaterThan,
        GreaterThanEquals,
        LessThan,
        LessThanEquals,
    }

    impl Display for ThresholdOperator {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let str = match self {
                ThresholdOperator::EqualTo => "=",
                ThresholdOperator::NotEqualTo => "!=",
                ThresholdOperator::GreaterThan => ">",
                ThresholdOperator::GreaterThanEquals => ">=",
                ThresholdOperator::LessThan => "<",
                ThresholdOperator::LessThanEquals => "<=",
            };
            write!(f, "{str}")
        }
    }

    pub enum SearchEventType {
        Ui,
        Dashboards,
        Reports,
        Alerts,
        Values,
        Other,
        Rum,
        DerivedStream,
    }

    impl From<SearchEventType> for i16 {
        fn from(value: SearchEventType) -> Self {
            match value {
                SearchEventType::Ui => 0,
                SearchEventType::Dashboards => 1,
                SearchEventType::Reports => 2,
                SearchEventType::Alerts => 3,
                SearchEventType::Values => 4,
                SearchEventType::Other => 5,
                SearchEventType::Rum => 6,
                SearchEventType::DerivedStream => 7,
            }
        }
    }

    pub enum StreamType {
        Logs,
        Metrics,
        Traces,
        EnrichmentTables,
        Filelist,
        Metadata,
        Index,
    }

    impl Display for StreamType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let str = match self {
                StreamType::Logs => "logs",
                StreamType::Metrics => "metrics",
                StreamType::Traces => "traces",
                StreamType::EnrichmentTables => "enrichment_tables",
                StreamType::Filelist => "file_list",
                StreamType::Metadata => "metadata",
                StreamType::Index => "index",
            };
            write!(f, "{str}")
        }
    }
}

/// Defines the JSON schema used to store alerts and their child structures in
/// the meta table at the time this migration executes.
mod meta_table_alerts {
    use std::collections::HashMap;

    use chrono::{DateTime, FixedOffset};
    use serde::{Deserialize, Serialize};
    use serde_json::Value as JsonValue;

    /// Defines the JSON schema for alerts stored in the meta table.
    #[derive(Deserialize)]
    pub struct Alert {
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub org_id: String,
        #[serde(default)]
        pub stream_type: StreamType,
        #[serde(default)]
        pub stream_name: String,
        #[serde(default)]
        pub is_real_time: bool,
        #[serde(default)]
        pub query_condition: QueryCondition,
        #[serde(default)]
        pub trigger_condition: TriggerCondition,
        pub destinations: Vec<String>,
        pub context_attributes: Option<HashMap<String, String>>,
        #[serde(default)]
        pub row_template: String,
        #[serde(default)]
        pub description: String,
        #[serde(default)]
        pub enabled: bool,
        #[serde(default)]
        /// Timezone offset in minutes.
        /// The negative secs means the Western Hemisphere
        pub tz_offset: i32,
        #[serde(default)]
        pub last_triggered_at: Option<i64>,
        #[serde(default)]
        pub last_satisfied_at: Option<i64>,
        #[serde(default)]
        pub owner: Option<String>,
        pub updated_at: Option<DateTime<FixedOffset>>,
        #[serde(default)]
        pub last_edited_by: Option<String>,
    }

    #[derive(Default, Deserialize)]
    pub struct TriggerCondition {
        pub period: i64, // 10 minutes
        #[serde(default)]
        pub operator: Operator, // >=
        #[serde(default)]
        pub threshold: i64, // 3 times
        #[serde(default)]
        pub frequency: i64, // 1 minute
        #[serde(default)]
        pub cron: String, // Cron Expression
        #[serde(default)]
        pub frequency_type: FrequencyType,
        #[serde(default)]
        pub silence: i64, // silence for 10 minutes after fire an alert
        pub timezone: Option<String>,
        #[serde(default)]
        pub tolerance_in_secs: Option<i64>,
    }

    #[derive(Deserialize, Serialize)]
    pub struct CompareHistoricData {
        #[serde(rename = "offSet")]
        pub offset: String,
    }

    #[derive(Default, Deserialize, Serialize)]
    pub enum FrequencyType {
        #[serde(rename = "cron")]
        Cron,
        #[serde(rename = "minutes")]
        #[default]
        Minutes,
    }

    #[derive(Default, Deserialize, Serialize)]
    pub struct QueryCondition {
        #[serde(default)]
        #[serde(rename = "type")]
        pub query_type: QueryType,
        pub conditions: Option<Vec<Condition>>,
        pub sql: Option<String>,
        pub promql: Option<String>,              // (cpu usage / cpu total)
        pub promql_condition: Option<Condition>, // value >= 80
        pub aggregation: Option<Aggregation>,
        #[serde(default)]
        pub vrl_function: Option<String>,
        #[serde(default)]
        pub search_event_type: Option<SearchEventType>,
        #[serde(default)]
        pub multi_time_range: Option<Vec<CompareHistoricData>>,
    }

    #[derive(Deserialize, Serialize)]
    pub struct Aggregation {
        pub group_by: Option<Vec<String>>,
        pub function: AggFunction,
        pub having: Condition,
    }

    #[derive(Deserialize, Serialize)]
    pub enum AggFunction {
        #[serde(rename = "avg")]
        Avg,
        #[serde(rename = "min")]
        Min,
        #[serde(rename = "max")]
        Max,
        #[serde(rename = "sum")]
        Sum,
        #[serde(rename = "count")]
        Count,
        #[serde(rename = "median")]
        Median,
        #[serde(rename = "p50")]
        P50,
        #[serde(rename = "p75")]
        P75,
        #[serde(rename = "p90")]
        P90,
        #[serde(rename = "p95")]
        P95,
        #[serde(rename = "p99")]
        P99,
    }

    #[derive(Default, Deserialize, Serialize)]
    #[allow(clippy::upper_case_acronyms)]
    pub enum QueryType {
        #[default]
        #[serde(rename = "custom")]
        Custom,
        #[serde(rename = "sql")]
        SQL,
        #[serde(rename = "promql")]
        PromQL,
    }

    #[derive(Deserialize, Serialize)]
    pub struct Condition {
        pub column: String,
        pub operator: Operator,
        pub value: JsonValue,
        #[serde(default)]
        pub ignore_case: bool,
    }

    #[derive(Deserialize, Serialize)]
    pub enum Operator {
        #[serde(rename = "=")]
        EqualTo,
        #[serde(rename = "!=")]
        NotEqualTo,
        #[serde(rename = ">")]
        GreaterThan,
        #[serde(rename = ">=")]
        GreaterThanEquals,
        #[serde(rename = "<")]
        LessThan,
        #[serde(rename = "<=")]
        LessThanEquals,
        Contains,
        NotContains,
    }

    impl Default for Operator {
        fn default() -> Self {
            Self::EqualTo
        }
    }

    #[derive(Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    #[allow(clippy::upper_case_acronyms)]
    pub enum SearchEventType {
        UI,
        Dashboards,
        Reports,
        Alerts,
        Values,
        Other,
        RUM,
        DerivedStream,
    }

    #[derive(Default, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub enum StreamType {
        #[default]
        Logs,
        Metrics,
        Traces,
        #[serde(rename = "enrichment_tables")]
        EnrichmentTables,
        #[serde(rename = "file_list")]
        Filelist,
        Metadata,
        Index,
    }
}

// Implementations of conversion traits for converting from the JSON
// representation of data structures in the old meta table to the JSON or string
// representation of data structures in the new alerts table. Some data
// structures are identical from the old meta table to the new alerts table, but
// others differ so we explicitly specify the conversion logic for each here.

impl TryFrom<MetaAlertWithFolder> for alerts_table::ActiveModel {
    type Error = String;

    fn try_from(m: MetaAlertWithFolder) -> Result<Self, Self::Error> {
        let folder_id = m
            .folder_id()
            .ok_or("Alert in meta table references folder that does not exist")?;
        let meta_alert: meta_table_alerts::Alert = serde_json::from_str(m.alert_json())
            .map_err(|_| "Alert in meta table could not be deserialized")?;
        let id = svix_ksuid::Ksuid::new(None, None).to_string();

        // Transform the parsed stream type from the meta table and serialize
        // into string for storage in DB.
        let stream_type: alerts_table_ser::StreamType = meta_alert.stream_type.into();
        let stream_type = stream_type.to_string();

        // Transform the parsed destinations from the meta table and serialize
        // into JSON for storage in DB.
        let destinations = meta_alert.destinations;
        let destinations = serde_json::to_value(&destinations).map_err(|e| e.to_string())?;

        // Transform the parsed optional context attribute map from the meta
        // table and serialize it into an optional JSON object for storage in
        // DB.
        let context_attributes = meta_alert
            .context_attributes
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| e.to_string())?;

        // Transform the parsed query type from the meta table and serialize
        // into an i16 for storage in DB.
        let query_type: alerts_table_ser::QueryType = meta_alert.query_condition.query_type.into();
        let query_type: i16 = query_type.into();

        // Transform the parsed optional conditions list from the meta table and
        // serialize it into an optional JSON array for storage in DB.
        let query_conditions = meta_alert
            .query_condition
            .conditions
            .map(|cs| {
                cs.into_iter()
                    .map(alerts_table_ser::Condition::from)
                    .collect_vec()
            })
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| e.to_string())?;

        // Transform the parsed optional condition from the meta table and
        // serialize it into an optional JSON object for storage in DB.
        let query_promql_condition = meta_alert
            .query_condition
            .promql_condition
            .map(alerts_table_ser::Condition::from)
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| e.to_string())?;

        // Transform the parsed optional aggregation from the meta table and
        // serialize it into an optional JSON object for storage in DB.
        let query_aggregation = meta_alert
            .query_condition
            .aggregation
            .map(alerts_table_ser::Aggregation::from)
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| e.to_string())?;

        // Transform the parsed optional search event type from the meta table
        // and serialize into an optional i16 for storage in DB.
        let query_search_event_type: Option<i16> = meta_alert
            .query_condition
            .search_event_type
            .map(alerts_table_ser::SearchEventType::from)
            .map(|t| t.into());

        // Transform the parsed optional historic data comparison list from the
        // meta table and serialize it into an optional JSON array for storage
        // in DB.
        let query_multi_time_range = meta_alert
            .query_condition
            .multi_time_range
            .map(|chds| {
                chds.into_iter()
                    .map(alerts_table_ser::CompareHistoricData::from)
                    .collect_vec()
            })
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| e.to_string())?;

        // Transform the parsed threshold operator from the meta table and
        // serialize into string for storage in DB.
        let trigger_threshold_operator: alerts_table_ser::ThresholdOperator =
            meta_alert.trigger_condition.operator.try_into()?;
        let trigger_threshold_operator = trigger_threshold_operator.to_string();

        // Transform the parsed trigger frequency type from the meta table and
        // serialize into an i16 for storage in DB.
        let trigger_frequency_type: alerts_table_ser::FrequencyType =
            meta_alert.trigger_condition.frequency_type.into();
        let trigger_frequency_type: i16 = trigger_frequency_type.into();

        Ok(alerts_table::ActiveModel {
            id: Set(id),
            org: Set(meta_alert.org_id),
            folder_id: Set(folder_id),
            name: Set(meta_alert.name),
            stream_type: Set(stream_type),
            stream_name: Set(meta_alert.stream_name),
            is_real_time: Set(meta_alert.is_real_time),
            destinations: Set(destinations),
            context_attributes: Set(context_attributes),
            row_template: Set(Some(meta_alert.row_template).filter(|s| !s.is_empty())),
            description: Set(Some(meta_alert.description).filter(|s| !s.is_empty())),
            enabled: Set(meta_alert.enabled),
            tz_offset: Set(meta_alert.tz_offset),
            last_triggered_at: Set(meta_alert.last_triggered_at),
            last_satisfied_at: Set(meta_alert.last_satisfied_at),
            query_type: Set(query_type),
            query_conditions: Set(query_conditions),
            query_sql: Set(meta_alert.query_condition.sql.filter(|s| !s.is_empty())),
            query_promql: Set(meta_alert.query_condition.promql.filter(|s| !s.is_empty())),
            query_promql_condition: Set(query_promql_condition),
            query_aggregation: Set(query_aggregation),
            query_vrl_function: Set(meta_alert
                .query_condition
                .vrl_function
                .filter(|s| !s.is_empty())),
            query_search_event_type: Set(query_search_event_type),
            query_multi_time_range: Set(query_multi_time_range),
            trigger_threshold_operator: Set(trigger_threshold_operator),
            trigger_period_seconds: Set(meta_alert.trigger_condition.period * 60),
            trigger_threshold_count: Set(meta_alert.trigger_condition.threshold),
            trigger_frequency_type: Set(trigger_frequency_type),
            trigger_frequency_seconds: Set(meta_alert.trigger_condition.frequency),
            trigger_frequency_cron: Set(
                Some(meta_alert.trigger_condition.cron).filter(|s| !s.is_empty())
            ),
            trigger_frequency_cron_timezone: Set(meta_alert.trigger_condition.timezone),
            trigger_silence_seconds: Set(meta_alert.trigger_condition.silence * 60),
            trigger_tolerance_seconds: Set(meta_alert.trigger_condition.tolerance_in_secs),
            owner: Set(meta_alert.owner),
            last_edited_by: Set(meta_alert.last_edited_by),
            updated_at: Set(meta_alert.updated_at.map(|t| t.timestamp())),
        })
    }
}

impl From<meta_table_alerts::CompareHistoricData> for alerts_table_ser::CompareHistoricData {
    fn from(value: meta_table_alerts::CompareHistoricData) -> Self {
        Self {
            offset: value.offset,
        }
    }
}

impl From<meta_table_alerts::FrequencyType> for alerts_table_ser::FrequencyType {
    fn from(value: meta_table_alerts::FrequencyType) -> Self {
        match value {
            meta_table_alerts::FrequencyType::Cron => Self::Cron,
            meta_table_alerts::FrequencyType::Minutes => Self::Seconds,
        }
    }
}

impl From<meta_table_alerts::Aggregation> for alerts_table_ser::Aggregation {
    fn from(value: meta_table_alerts::Aggregation) -> Self {
        Self {
            group_by: value.group_by,
            function: value.function.into(),
            having: value.having.into(),
        }
    }
}

impl From<meta_table_alerts::AggFunction> for alerts_table_ser::AggFunction {
    fn from(value: meta_table_alerts::AggFunction) -> Self {
        match value {
            meta_table_alerts::AggFunction::Avg => Self::Avg,
            meta_table_alerts::AggFunction::Min => Self::Min,
            meta_table_alerts::AggFunction::Max => Self::Max,
            meta_table_alerts::AggFunction::Sum => Self::Sum,
            meta_table_alerts::AggFunction::Count => Self::Count,
            meta_table_alerts::AggFunction::Median => Self::Median,
            meta_table_alerts::AggFunction::P50 => Self::P50,
            meta_table_alerts::AggFunction::P75 => Self::P75,
            meta_table_alerts::AggFunction::P90 => Self::P90,
            meta_table_alerts::AggFunction::P95 => Self::P95,
            meta_table_alerts::AggFunction::P99 => Self::P99,
        }
    }
}

impl From<meta_table_alerts::QueryType> for alerts_table_ser::QueryType {
    fn from(value: meta_table_alerts::QueryType) -> Self {
        match value {
            meta_table_alerts::QueryType::Custom => Self::Custom,
            meta_table_alerts::QueryType::SQL => Self::Sql,
            meta_table_alerts::QueryType::PromQL => Self::Promql,
        }
    }
}

impl From<meta_table_alerts::Condition> for alerts_table_ser::Condition {
    fn from(value: meta_table_alerts::Condition) -> Self {
        Self {
            column: value.column,
            operator: value.operator.into(),
            value: value.value,
            ignore_case: value.ignore_case,
        }
    }
}

impl From<meta_table_alerts::Operator> for alerts_table_ser::ConditionOperator {
    fn from(value: meta_table_alerts::Operator) -> Self {
        match value {
            meta_table_alerts::Operator::EqualTo => Self::EqualTo,
            meta_table_alerts::Operator::NotEqualTo => Self::NotEqualTo,
            meta_table_alerts::Operator::GreaterThan => Self::GreaterThan,
            meta_table_alerts::Operator::GreaterThanEquals => Self::GreaterThanEquals,
            meta_table_alerts::Operator::LessThan => Self::LessThan,
            meta_table_alerts::Operator::LessThanEquals => Self::LessThanEquals,
            meta_table_alerts::Operator::Contains => Self::Contains,
            meta_table_alerts::Operator::NotContains => Self::NotContains,
        }
    }
}

impl TryFrom<meta_table_alerts::Operator> for alerts_table_ser::ThresholdOperator {
    type Error = String;

    fn try_from(value: meta_table_alerts::Operator) -> Result<Self, Self::Error> {
        match value {
            meta_table_alerts::Operator::EqualTo => Ok(Self::EqualTo),
            meta_table_alerts::Operator::NotEqualTo => Ok(Self::NotEqualTo),
            meta_table_alerts::Operator::GreaterThan => Ok(Self::GreaterThan),
            meta_table_alerts::Operator::GreaterThanEquals => Ok(Self::GreaterThanEquals),
            meta_table_alerts::Operator::LessThan => Ok(Self::LessThan),
            meta_table_alerts::Operator::LessThanEquals => Ok(Self::LessThanEquals),
            meta_table_alerts::Operator::Contains => {
                Err("trigger threshold operator cannot be `contains`".to_owned())
            }
            meta_table_alerts::Operator::NotContains => {
                Err("trigger threshold operator cannot be `not_contains`".to_owned())
            }
        }
    }
}

impl From<meta_table_alerts::SearchEventType> for alerts_table_ser::SearchEventType {
    fn from(value: meta_table_alerts::SearchEventType) -> Self {
        match value {
            meta_table_alerts::SearchEventType::UI => Self::Ui,
            meta_table_alerts::SearchEventType::Dashboards => Self::Dashboards,
            meta_table_alerts::SearchEventType::Reports => Self::Reports,
            meta_table_alerts::SearchEventType::Alerts => Self::Alerts,
            meta_table_alerts::SearchEventType::Values => Self::Values,
            meta_table_alerts::SearchEventType::Other => Self::Other,
            meta_table_alerts::SearchEventType::RUM => Self::Rum,
            meta_table_alerts::SearchEventType::DerivedStream => Self::DerivedStream,
        }
    }
}

impl From<meta_table_alerts::StreamType> for alerts_table_ser::StreamType {
    fn from(value: meta_table_alerts::StreamType) -> Self {
        match value {
            meta_table_alerts::StreamType::Logs => Self::Logs,
            meta_table_alerts::StreamType::Metrics => Self::Metrics,
            meta_table_alerts::StreamType::Traces => Self::Traces,
            meta_table_alerts::StreamType::EnrichmentTables => Self::EnrichmentTables,
            meta_table_alerts::StreamType::Filelist => Self::Filelist,
            meta_table_alerts::StreamType::Metadata => Self::Metadata,
            meta_table_alerts::StreamType::Index => Self::Index,
        }
    }
}

#[cfg(test)]
mod tests {
    use collapse::*;

    use super::*;

    #[test]
    fn org_with_alert_postgres() {
        let stmnt = OrgWithAlert::statement();
        assert_eq!(
            &stmnt.to_string(PostgresQueryBuilder),
            r#"SELECT "key1" FROM "meta" WHERE "module" = 'alerts' GROUP BY "key1" ORDER BY "key1" ASC"#
        );
    }

    #[test]
    fn org_with_alert_mysql() {
        let stmnt = OrgWithAlert::statement();
        assert_eq!(
            &stmnt.to_string(MysqlQueryBuilder),
            r#"SELECT `key1` FROM `meta` WHERE `module` = 'alerts' GROUP BY `key1` ORDER BY `key1` ASC"#
        );
    }

    #[test]
    fn org_with_alert_sqlite() {
        let stmnt = OrgWithAlert::statement();
        assert_eq!(
            &stmnt.to_string(SqliteQueryBuilder),
            r#"SELECT "key1" FROM "meta" WHERE "module" = 'alerts' GROUP BY "key1" ORDER BY "key1" ASC"#
        );
    }

    #[test]
    fn meta_with_folder_postgres() {
        let stmnt = MetaAlertWithFolder::statement();
        collapsed_eq!(
            &stmnt.to_string(PostgresQueryBuilder),
            r#"
                SELECT 
                "meta"."value", 
                "folders"."id" 
                FROM "meta" 
                LEFT JOIN "folders" 
                ON "folders"."org" = "meta"."key1" 
                WHERE "meta"."module" = 'alerts' 
                AND "folders"."type" = 1 
                AND "folders"."folder_id" = 'default'
                ORDER BY "meta"."id" ASC
            "#
        );
    }

    #[test]
    fn meta_with_folder_mysql() {
        let stmnt = MetaAlertWithFolder::statement();
        collapsed_eq!(
            &stmnt.to_string(MysqlQueryBuilder),
            r#"
                SELECT 
                `meta`.`value`, 
                `folders`.`id` 
                FROM `meta` 
                LEFT JOIN `folders` 
                ON `folders`.`org` = `meta`.`key1` 
                WHERE `meta`.`module` = 'alerts' 
                AND `folders`.`type` = 1 
                AND `folders`.`folder_id` = 'default'
                ORDER BY `meta`.`id` ASC
            "#
        );
    }

    #[test]
    fn meta_with_folder_sqlite() {
        let stmnt = MetaAlertWithFolder::statement();
        collapsed_eq!(
            &stmnt.to_string(SqliteQueryBuilder),
            r#"
                SELECT
                "meta"."value", 
                "folders"."id" 
                FROM "meta" 
                LEFT JOIN "folders" 
                ON "folders"."org" = "meta"."key1" 
                WHERE "meta"."module" = 'alerts' 
                AND "folders"."type" = 1 
                AND "folders"."folder_id" = 'default'
                ORDER BY "meta"."id" ASC
            "#
        );
    }
}
