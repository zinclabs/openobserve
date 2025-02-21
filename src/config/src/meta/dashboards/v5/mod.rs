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

use std::hash::{Hash, Hasher};

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{datetime_now, OrdF64};
use crate::meta::stream::StreamType;

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    version: i32,
    #[serde(default)]
    pub dashboard_id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default = "datetime_now")]
    #[schema(value_type = String, format = DateTime)]
    pub created: DateTime<FixedOffset>,
    #[serde(default)]
    pub tabs: Vec<Tab>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Variables>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_datetime_duration: Option<DateTimeOptions>,
    #[serde(default, skip_serializing)]
    pub updated_at: i64,
}

impl From<Dashboard> for super::Dashboard {
    fn from(value: Dashboard) -> Self {
        let version: i32 = 5;

        let mut hasher = std::hash::DefaultHasher::new();
        hasher.write_i32(version);
        value.hash(&mut hasher);
        let hash = hasher.finish().to_string();
        let updated_at = value.updated_at;

        Self {
            v1: None,
            v2: None,
            v3: None,
            v4: None,
            v5: Some(value),
            version,
            hash,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub i: i64,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub tab_id: String,
    pub name: String,
    #[serde(default)]
    pub panels: Vec<Panel>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub id: String,
    #[serde(rename = "type")]
    pub typ: String,
    pub title: String,
    pub description: String,
    pub config: PanelConfig,
    #[serde(default)]
    pub query_type: String,
    pub queries: Vec<Query>,
    pub layout: Layout,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_chart_content: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub query: Option<String>,
    pub vrl_function_query: Option<String>,
    pub custom_query: bool,
    pub fields: PanelFields,
    pub config: QueryConfig,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct PanelFields {
    pub stream: String,
    pub stream_type: StreamType,
    pub x: Vec<AxisItem>,
    pub y: Vec<AxisItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z: Option<Vec<AxisItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakdown: Option<Vec<AxisItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<AxisItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<AxisItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<AxisItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<AxisItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_for_maps: Option<AxisItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<AxisItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<AxisItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<AxisItem>,
    pub filter: PanelFilter,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AxisItem {
    pub label: String,
    pub alias: String,
    pub column: String,
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregation_function: Option<AggregationFunc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<AxisArg>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_derived: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct AxisArg {
    value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AggregationFunc {
    Count,
    CountDistinct,
    Histogram,
    Sum,
    Min,
    Max,
    Avg,
    Median,
    P50,
    P90,
    P95,
    P99,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(untagged, rename_all = "camelCase")]
pub enum PanelFilter {
    #[serde(rename = "condition")]
    Condition(FilterCondition),
    #[serde(rename = "group")]
    Group(GroupType),
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupType {
    pub filter_type: String,
    pub logical_operator: String,
    pub conditions: Vec<PanelFilter>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FilterCondition {
    #[serde(rename = "type")]
    pub typ: String,
    pub values: Vec<String>,
    pub column: String,
    pub operator: Option<String>,
    pub value: Option<String>,
    pub logical_operator: String,
    pub filter_type: String,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct PanelConfig {
    show_legends: bool,
    legends_position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit_custom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decimals: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_thickness: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    step_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_results: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y_axis_min: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    y_axis_max: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_results_others: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    axis_width: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    axis_border_show: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label_option: Option<LabelOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    show_symbol: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_interpolation: Option<LineInterpolation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    legend_width: Option<LegendWidth>,
    base_map: Option<BaseMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    map_type: Option<MapType>,
    map_view: Option<MapView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    map_symbol_style: Option<MapSymbolStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drilldown: Option<Vec<DrillDown>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mark_line: Option<Vec<MarkLine>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    override_config: Option<Vec<OverrideConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connect_nulls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_value_replacement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wrap_table_cells: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    table_transpose: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    table_dynamic_columns: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mappings: Option<Vec<Mapping>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<ColorCfg>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trellis: Option<Trellis>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ColorCfg {
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed_color: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    series_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    typee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "match")]
    matchh: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DrillDown {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    type_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_blank: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    find_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<DrillDownData>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarkLine {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    typee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverrideConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<Field>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<Vec<Config>>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    match_by: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "type")]
    typee: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Value {
    unit: String,
    custom_unit: String,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DrillDownData {
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dashboard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tab: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pass_all_variables: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<Vec<DrillDownVariables>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logs_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logs_query: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct DrillDownVariables {
    name: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct QueryConfig {
    promql_legend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    layer_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight_fixed: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_shift: Option<Vec<TimeShift>>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimeShift {
    #[serde(skip_serializing_if = "Option::is_none")]
    off_set: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variables {
    pub list: Vec<VariableList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_dynamic_filters: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeOptions {
    #[serde(rename = "type")]
    pub typee: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative_time_period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableList {
    #[serde(rename = "type")]
    pub type_field: String,
    pub name: String,
    pub label: String,
    #[serde(rename = "query_data")]
    pub query_data: Option<QueryData>,
    pub value: Option<String>,
    pub options: Option<Vec<CustomFieldsOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_select: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_on_dashboard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select_all_value_for_multi_select: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_multi_select_value: Option<Vec<String>>,
}

#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct QueryData {
    pub stream_type: StreamType,
    pub stream: String,
    pub field: String,
    pub max_record_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Vec<Filters>>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Filters {
    pub name: Option<String>,
    pub operator: Option<String>,
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldsOption {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct BaseMap {
    #[serde(rename = "type")]
    pub type_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct MapType {
    #[serde(rename = "type")]
    pub type_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct MapView {
    pub zoom: OrdF64,
    pub lat: OrdF64,
    pub lng: OrdF64,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct Trellis {
    pub layout: Option<String>,
    pub num_of_columns: i64,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct MapSymbolStyle {
    pub size: String,
    pub size_by_value: Option<SizeByValue>,
    pub size_fixed: OrdF64,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct SizeByValue {
    pub min: OrdF64,
    pub max: OrdF64,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct LegendWidth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<OrdF64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct LabelOption {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<LabelPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotate: Option<OrdF64>,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LineInterpolation {
    Smooth,
    Linear,
    StepStart,
    StepEnd,
    StepMiddle,
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum LabelPosition {
    Top,
    Inside,
    InsideTop,
    InsideBottom,
}
