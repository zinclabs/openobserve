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

use std::{cmp::max, fmt::Display};

use chrono::{DateTime, Duration, TimeZone, Utc};
use hashbrown::HashMap;
use proto::cluster_rpc;
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use utoipa::ToSchema;

use super::bitvec::BitVec;
use crate::{
    get_config,
    meta::self_reporting::usage::Stats,
    utils::{
        hash::{gxhash, Sum64},
        json::{self, Map, Value},
    },
};

pub const ALL_STREAM_TYPES: [StreamType; 7] = [
    StreamType::Logs,
    StreamType::Metrics,
    StreamType::Traces,
    StreamType::EnrichmentTables,
    StreamType::Filelist,
    StreamType::Metadata,
    StreamType::Index,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, ToSchema, Hash)]
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

impl StreamType {
    pub fn is_basic_type(&self) -> bool {
        matches!(
            *self,
            StreamType::Logs | StreamType::Metrics | StreamType::Traces
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StreamType::Logs => "logs",
            StreamType::Metrics => "metrics",
            StreamType::Traces => "traces",
            StreamType::EnrichmentTables => "enrichment_tables",
            StreamType::Filelist => "file_list",
            StreamType::Metadata => "metadata",
            StreamType::Index => "index",
        }
    }
}

impl From<&str> for StreamType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "logs" => StreamType::Logs,
            "metrics" => StreamType::Metrics,
            "traces" => StreamType::Traces,
            "enrichment_tables" | "enrich" => StreamType::EnrichmentTables,
            "file_list" => StreamType::Filelist,
            "metadata" => StreamType::Metadata,
            "index" => StreamType::Index,
            _ => StreamType::default(),
        }
    }
}

impl From<String> for StreamType {
    fn from(s: String) -> Self {
        StreamType::from(s.as_str())
    }
}

impl std::fmt::Display for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StreamType::Logs => write!(f, "logs"),
            StreamType::Metrics => write!(f, "metrics"),
            StreamType::Traces => write!(f, "traces"),
            StreamType::EnrichmentTables => write!(f, "enrichment_tables"),
            StreamType::Filelist => write!(f, "file_list"),
            StreamType::Metadata => write!(f, "metadata"),
            StreamType::Index => write!(f, "index"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct StreamParams {
    pub org_id: faststr::FastStr,
    pub stream_name: faststr::FastStr,
    pub stream_type: StreamType,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct RemoteStreamParams {
    pub org_id: faststr::FastStr,
    pub destination_name: faststr::FastStr,
}

impl Default for StreamParams {
    fn default() -> Self {
        Self {
            org_id: String::default().into(),
            stream_name: String::default().into(),
            stream_type: StreamType::default(),
        }
    }
}

impl std::fmt::Display for StreamParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}",
            self.org_id, self.stream_name, self.stream_type
        )
    }
}

impl StreamParams {
    pub fn new(org_id: &str, stream_name: &str, stream_type: StreamType) -> Self {
        Self {
            org_id: org_id.to_string().into(),
            stream_name: stream_name.to_string().into(),
            stream_type,
        }
    }

    pub fn is_valid(&self) -> bool {
        !(self.org_id.is_empty() || self.stream_name.is_empty())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ListStreamParams {
    pub list: Vec<StreamParams>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileKey {
    pub key: String,
    pub meta: FileMeta,
    pub deleted: bool,
    pub segment_ids: Option<BitVec>,
}

impl FileKey {
    pub fn new(key: String, meta: FileMeta, deleted: bool) -> Self {
        Self {
            key,
            meta,
            deleted,
            segment_ids: None,
        }
    }

    pub fn from_file_name(file: &str) -> Self {
        Self {
            key: file.to_string(),
            meta: FileMeta::default(),
            deleted: false,
            segment_ids: None,
        }
    }

    pub fn with_segment_ids(&mut self, segment_ids: BitVec) {
        self.segment_ids = Some(segment_ids);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileMeta {
    pub min_ts: i64, // microseconds
    pub max_ts: i64, // microseconds
    pub records: i64,
    pub original_size: i64,
    pub compressed_size: i64,
    pub index_size: i64,
    pub flattened: bool,
}

impl FileMeta {
    pub fn is_empty(&self) -> bool {
        self.records == 0 && self.original_size == 0
    }
}

impl From<&[parquet::file::metadata::KeyValue]> for FileMeta {
    fn from(values: &[parquet::file::metadata::KeyValue]) -> Self {
        let mut meta = FileMeta::default();
        for kv in values {
            match kv.key.as_str() {
                "min_ts" => meta.min_ts = kv.value.as_ref().unwrap().parse().unwrap(),
                "max_ts" => meta.max_ts = kv.value.as_ref().unwrap().parse().unwrap(),
                "records" => meta.records = kv.value.as_ref().unwrap().parse().unwrap(),
                "original_size" => meta.original_size = kv.value.as_ref().unwrap().parse().unwrap(),
                "compressed_size" => {
                    meta.compressed_size = kv.value.as_ref().unwrap().parse().unwrap()
                }
                _ => {}
            }
        }
        meta
    }
}

#[derive(Clone, Debug, Default)]
pub struct FileListDeleted {
    pub file: String,
    pub index_file: bool,
    pub flattened: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryPartitionStrategy {
    FileNum,
    FileSize,
    FileHash,
}

impl From<&String> for QueryPartitionStrategy {
    fn from(s: &String) -> Self {
        match s.to_lowercase().as_str() {
            "file_num" => QueryPartitionStrategy::FileNum,
            "file_size" => QueryPartitionStrategy::FileSize,
            "file_hash" => QueryPartitionStrategy::FileHash,
            _ => QueryPartitionStrategy::FileNum,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    FileSize,
    FileTime,
}

impl From<&String> for MergeStrategy {
    fn from(s: &String) -> Self {
        match s.to_lowercase().as_str() {
            "file_size" => MergeStrategy::FileSize,
            "file_time" => MergeStrategy::FileTime,
            _ => MergeStrategy::FileSize,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StreamStats {
    pub created_at: i64,
    pub doc_time_min: i64,
    pub doc_time_max: i64,
    pub doc_num: i64,
    pub file_num: i64,
    pub storage_size: f64,
    pub compressed_size: f64,
    pub index_size: f64,
}

impl StreamStats {
    /// Returns true iff [start, end] time range intersects with the stream's
    /// time range.
    pub fn time_range_intersects(&self, start: i64, end: i64) -> bool {
        assert!(start <= end);
        let (min, max) = self.time_range();
        // [min, max] does *not* intersect with [start, end] if either
        //
        // max < start
        // |--------------------|         |--------------------|
        // min                  max       start                end
        //
        // or min >= end
        // |--------------------|         |--------------------|
        // start                end       min                  max
        //
        // The time ranges intersect iff !(max < start || min >= end)
        max >= start && min < end
    }

    fn time_range(&self) -> (i64, i64) {
        assert!(self.doc_time_min <= self.doc_time_max);
        let file_push_interval = Duration::try_seconds(get_config().limit.file_push_interval as _)
            .unwrap()
            .num_microseconds()
            .unwrap();
        (self.doc_time_min, self.doc_time_max + file_push_interval)
    }

    pub fn add_file_meta(&mut self, meta: &FileMeta) {
        self.file_num += 1;
        self.doc_num = max(0, self.doc_num + meta.records);
        self.doc_time_min = self.doc_time_min.min(meta.min_ts);
        self.doc_time_max = self.doc_time_max.max(meta.max_ts);
        self.storage_size += meta.original_size as f64;
        self.compressed_size += meta.compressed_size as f64;
        self.index_size += meta.index_size as f64;
        if self.doc_time_min == 0 {
            self.doc_time_min = meta.min_ts;
        }
        if self.storage_size < 0.0 {
            self.storage_size = 0.0;
        }
        if self.compressed_size < 0.0 {
            self.compressed_size = 0.0;
        }
        if self.index_size < 0.0 {
            self.index_size = 0.0;
        }
    }

    pub fn format_by(&mut self, stats: &StreamStats) {
        self.file_num = stats.file_num;
        self.doc_num = stats.doc_num;
        self.storage_size = stats.storage_size;
        self.compressed_size = stats.compressed_size;
        self.index_size = stats.index_size;
        self.doc_time_min = self.doc_time_min.min(stats.doc_time_min);
        self.doc_time_max = self.doc_time_max.max(stats.doc_time_max);
        if self.doc_time_min == 0 {
            self.doc_time_min = stats.doc_time_min;
        }
    }

    pub fn merge(&mut self, other: &StreamStats) {
        self.created_at = self.created_at.min(other.created_at);
        self.doc_time_min = self.doc_time_min.min(other.doc_time_min);
        self.doc_time_max = self.doc_time_max.max(other.doc_time_max);
        self.doc_num += other.doc_num;
        self.file_num += other.file_num;
        self.storage_size += other.storage_size;
        self.compressed_size += other.compressed_size;
        self.index_size += other.index_size;
    }
}

impl From<&str> for StreamStats {
    fn from(data: &str) -> Self {
        json::from_str::<StreamStats>(data).unwrap()
    }
}

impl From<StreamStats> for Vec<u8> {
    fn from(value: StreamStats) -> Vec<u8> {
        json::to_vec(&value).unwrap()
    }
}

impl From<StreamStats> for String {
    fn from(data: StreamStats) -> Self {
        json::to_string(&data).unwrap()
    }
}

impl From<Stats> for StreamStats {
    fn from(meta: Stats) -> StreamStats {
        StreamStats {
            created_at: 0,
            doc_time_min: meta.min_ts,
            doc_time_max: meta.max_ts,
            doc_num: meta.records,
            file_num: 0,
            storage_size: meta.original_size,
            compressed_size: meta.compressed_size.unwrap_or_default(),
            index_size: meta.index_size.unwrap_or_default(),
        }
    }
}

impl std::ops::Sub<&StreamStats> for &StreamStats {
    type Output = StreamStats;

    fn sub(self, rhs: &StreamStats) -> Self::Output {
        let mut ret = StreamStats {
            created_at: self.created_at,
            file_num: self.file_num - rhs.file_num,
            doc_num: self.doc_num - rhs.doc_num,
            doc_time_min: self.doc_time_min.min(rhs.doc_time_min),
            doc_time_max: self.doc_time_max.max(rhs.doc_time_max),
            storage_size: self.storage_size - rhs.storage_size,
            compressed_size: self.compressed_size - rhs.compressed_size,
            index_size: self.index_size - rhs.index_size,
        };
        if ret.doc_time_min == 0 {
            ret.doc_time_min = rhs.doc_time_min;
        }
        ret
    }
}

impl From<&FileMeta> for cluster_rpc::FileMeta {
    fn from(req: &FileMeta) -> Self {
        cluster_rpc::FileMeta {
            min_ts: req.min_ts,
            max_ts: req.max_ts,
            records: req.records,
            original_size: req.original_size,
            compressed_size: req.compressed_size,
            index_size: req.index_size,
        }
    }
}

impl From<&cluster_rpc::FileMeta> for FileMeta {
    fn from(req: &cluster_rpc::FileMeta) -> Self {
        FileMeta {
            min_ts: req.min_ts,
            max_ts: req.max_ts,
            records: req.records,
            original_size: req.original_size,
            compressed_size: req.compressed_size,
            flattened: false,
            index_size: req.index_size,
        }
    }
}

impl From<&FileKey> for cluster_rpc::FileKey {
    fn from(req: &FileKey) -> Self {
        cluster_rpc::FileKey {
            key: req.key.clone(),
            meta: Some(cluster_rpc::FileMeta::from(&req.meta)),
            deleted: req.deleted,
            segment_ids: None,
        }
    }
}

impl From<&cluster_rpc::FileKey> for FileKey {
    fn from(req: &cluster_rpc::FileKey) -> Self {
        FileKey {
            key: req.key.clone(),
            meta: FileMeta::from(req.meta.as_ref().unwrap()),
            deleted: req.deleted,
            segment_ids: None,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PartitionTimeLevel {
    #[default]
    Unset,
    Hourly,
    Daily,
}

impl PartitionTimeLevel {
    pub fn duration(self) -> i64 {
        match self {
            PartitionTimeLevel::Unset => 0,
            PartitionTimeLevel::Hourly => 3600, // seconds, 1 hour
            PartitionTimeLevel::Daily => 86400, // seconds, 24 hour
        }
    }
}

impl From<&str> for PartitionTimeLevel {
    fn from(data: &str) -> Self {
        match data.to_lowercase().as_str() {
            "unset" => PartitionTimeLevel::Unset,
            "hourly" => PartitionTimeLevel::Hourly,
            "daily" => PartitionTimeLevel::Daily,
            _ => PartitionTimeLevel::Unset,
        }
    }
}

impl std::fmt::Display for PartitionTimeLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartitionTimeLevel::Unset => write!(f, "unset"),
            PartitionTimeLevel::Hourly => write!(f, "hourly"),
            PartitionTimeLevel::Daily => write!(f, "daily"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
pub struct UpdateSettingsWrapper<D> {
    #[serde(default)]
    pub add: Vec<D>,
    #[serde(default)]
    pub remove: Vec<D>,
}

#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
pub struct UpdateStreamSettings {
    #[serde(skip_serializing_if = "Option::None")]
    pub partition_time_level: Option<PartitionTimeLevel>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub partition_keys: UpdateSettingsWrapper<StreamPartition>,
    #[serde(default)]
    pub full_text_search_keys: UpdateSettingsWrapper<String>,
    #[serde(default)]
    pub index_fields: UpdateSettingsWrapper<String>,
    #[serde(default)]
    pub bloom_filter_fields: UpdateSettingsWrapper<String>,
    #[serde(skip_serializing_if = "Option::None")]
    #[serde(default)]
    pub data_retention: Option<i64>,
    #[serde(skip_serializing_if = "Option::None")]
    #[serde(default)]
    pub flatten_level: Option<i64>,
    #[serde(default)]
    pub defined_schema_fields: UpdateSettingsWrapper<String>,
    #[serde(default)]
    pub distinct_value_fields: UpdateSettingsWrapper<String>,
    #[serde(default)]
    pub max_query_range: Option<i64>,
    #[serde(default)]
    pub store_original_data: Option<bool>,
    #[serde(default)]
    pub approx_partition: Option<bool>,
    #[serde(default)]
    pub extended_retention_days: UpdateSettingsWrapper<TimeRange>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
/// WARNING: this implements Eq trait based only on the name,
/// so the timestamp will not be considered when comparing two entries
pub struct DistinctField {
    pub name: String,
    pub added_ts: i64,
}

impl PartialEq for DistinctField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for DistinctField {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct TimeRange {
    /// Start timestamp in microseconds
    pub start: i64,
    /// End timestamp in microseconds
    pub end: i64,
}

impl Display for TimeRange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let time_range_start: DateTime<Utc> = Utc.timestamp_nanos(self.start * 1000);
        let time_range_end: DateTime<Utc> = Utc.timestamp_nanos(self.end * 1000);
        write!(f, "{} to {}", time_range_start, time_range_end)
    }
}
impl TimeRange {
    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start == 0 && self.end == 0
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.start < other.end && self.end > other.start
    }

    /// Returns the intersection of two time ranges
    /// If the time ranges do not intersect, returns nothing
    /// If the time ranges intersect, returns the inverse of the intersection
    pub fn split_by_range(&self, other: &Self) -> anyhow::Result<(Option<Self>, Option<Self>)> {
        let mut left = None;
        let mut right = None;

        if self == other {
            return Ok((left, right));
        }

        if !self.intersects(other) {
            return Err(anyhow::anyhow!("Time ranges do not intersect"));
        }

        if self.start < other.start {
            left = Some(Self::new(self.start, other.start));
        }
        if self.end > other.end {
            right = Some(Self::new(other.end, self.end));
        }
        Ok((left, right))
    }

    pub fn flatten_overlapping_ranges(ranges: Vec<Self>) -> Vec<Self> {
        if ranges.is_empty() {
            return ranges;
        }
        let mut ranges = ranges;
        ranges.sort_by(|a, b| a.start.cmp(&b.start));
        let mut result = Vec::new();
        let mut current = ranges[0].clone();
        for range in ranges.iter().skip(1) {
            if current.intersects(range) {
                current.end = range.end;
            } else {
                result.push(current.clone());
                current = range.clone();
            }
        }
        result.push(current);
        result
    }
}
#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
pub struct StreamSettings {
    #[serde(skip_serializing_if = "Option::None")]
    pub partition_time_level: Option<PartitionTimeLevel>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub partition_keys: Vec<StreamPartition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub full_text_search_keys: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub index_fields: Vec<String>,
    #[serde(default)]
    pub bloom_filter_fields: Vec<String>,
    #[serde(default)]
    pub data_retention: i64,
    #[serde(skip_serializing_if = "Option::None")]
    pub flatten_level: Option<i64>,
    #[serde(skip_serializing_if = "Option::None")]
    pub defined_schema_fields: Option<Vec<String>>,
    #[serde(default)]
    pub max_query_range: i64, // hours
    #[serde(default)]
    pub store_original_data: bool,
    #[serde(default)]
    pub approx_partition: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub distinct_value_fields: Vec<DistinctField>,
    #[serde(default)]
    pub index_updated_at: i64,
    #[serde(default)]
    pub extended_retention_days: Vec<TimeRange>,
}

impl Serialize for StreamSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("stream_settings", 4)?;
        let mut part_keys = HashMap::new();
        for (index, key) in self.partition_keys.iter().enumerate() {
            part_keys.insert(format!("L{index}"), key);
        }
        state.serialize_field(
            "partition_time_level",
            &self.partition_time_level.unwrap_or_default(),
        )?;
        state.serialize_field("partition_keys", &part_keys)?;
        state.serialize_field("full_text_search_keys", &self.full_text_search_keys)?;
        state.serialize_field("index_fields", &self.index_fields)?;
        state.serialize_field("bloom_filter_fields", &self.bloom_filter_fields)?;
        state.serialize_field("distinct_value_fields", &self.distinct_value_fields)?;
        state.serialize_field("data_retention", &self.data_retention)?;
        state.serialize_field("max_query_range", &self.max_query_range)?;
        state.serialize_field("store_original_data", &self.store_original_data)?;
        state.serialize_field("approx_partition", &self.approx_partition)?;
        state.serialize_field("index_updated_at", &self.index_updated_at)?;
        state.serialize_field("extended_retention_days", &self.extended_retention_days)?;

        match self.defined_schema_fields.as_ref() {
            Some(fields) => {
                if !fields.is_empty() {
                    state.serialize_field("defined_schema_fields", fields)?;
                } else {
                    state.skip_field("defined_schema_fields")?;
                }
            }
            None => {
                state.skip_field("defined_schema_fields")?;
            }
        }
        match self.flatten_level.as_ref() {
            Some(flatten_level) => {
                state.serialize_field("flatten_level", flatten_level)?;
            }
            None => {
                state.skip_field("flatten_level")?;
            }
        }
        state.end()
    }
}

impl From<&str> for StreamSettings {
    fn from(data: &str) -> Self {
        let settings: json::Value = json::from_slice(data.as_bytes()).unwrap();

        let mut partition_keys = Vec::new();
        if let Some(value) = settings.get("partition_keys") {
            let mut v: Vec<_> = value.as_object().unwrap().into_iter().collect();
            v.sort_by(|a, b| a.0.cmp(b.0));
            for (_, value) in v.iter() {
                match value {
                    json::Value::String(v) => {
                        partition_keys.push(StreamPartition::new(v));
                    }
                    json::Value::Object(v) => {
                        let val: StreamPartition =
                            json::from_value(json::Value::Object(v.to_owned())).unwrap();
                        partition_keys.push(val);
                    }
                    _ => {}
                }
            }
        }

        let mut partition_time_level = None;
        if let Some(value) = settings.get("partition_time_level") {
            partition_time_level = Some(PartitionTimeLevel::from(value.as_str().unwrap()));
        }

        let mut full_text_search_keys = Vec::new();
        let fields = settings.get("full_text_search_keys");
        if let Some(value) = fields {
            let v: Vec<_> = value.as_array().unwrap().iter().collect();
            for item in v {
                full_text_search_keys.push(item.as_str().unwrap().to_string())
            }
        }

        let mut index_fields = Vec::new();
        let fields = settings.get("index_fields");
        if let Some(value) = fields {
            let v: Vec<_> = value.as_array().unwrap().iter().collect();
            for item in v {
                index_fields.push(item.as_str().unwrap().to_string())
            }
        }

        let mut bloom_filter_fields = Vec::new();
        let fields = settings.get("bloom_filter_fields");
        if let Some(value) = fields {
            let v: Vec<_> = value.as_array().unwrap().iter().collect();
            for item in v {
                bloom_filter_fields.push(item.as_str().unwrap().to_string())
            }
        }

        let mut data_retention = 0;
        if let Some(v) = settings.get("data_retention") {
            data_retention = v.as_i64().unwrap();
        };

        let mut max_query_range = 0;
        if let Some(v) = settings.get("max_query_range") {
            max_query_range = v.as_i64().unwrap();
        };

        let mut defined_schema_fields: Option<Vec<String>> = None;
        if let Some(value) = settings.get("defined_schema_fields") {
            let fields = value
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item.as_str().unwrap().to_string())
                .collect::<Vec<_>>();
            if !fields.is_empty() {
                defined_schema_fields = Some(fields);
            }
        }

        let flatten_level = settings.get("flatten_level").map(|v| v.as_i64().unwrap());

        let store_original_data = settings
            .get("store_original_data")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let approx_partition = settings
            .get("approx_partition")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut distinct_value_fields = Vec::new();
        let fields = settings.get("distinct_value_fields");
        if let Some(value) = fields {
            let v: Vec<_> = value.as_array().unwrap().iter().collect();
            for item in v {
                distinct_value_fields.push(json::from_value(item.clone()).unwrap())
            }
        }

        let index_updated_at = settings
            .get("index_updated_at")
            .and_then(|v| v.as_i64())
            .unwrap_or_default();

        let mut extended_retention_days = vec![];
        if let Some(values) = settings
            .get("extended_retention_days")
            .and_then(|v| v.as_array())
        {
            for item in values {
                let start = item
                    .get("start")
                    .and_then(|v| v.as_i64())
                    .unwrap_or_default();
                let end = item.get("end").and_then(|v| v.as_i64()).unwrap_or_default();
                extended_retention_days.push(TimeRange::new(start, end));
            }
        }

        Self {
            partition_time_level,
            partition_keys,
            full_text_search_keys,
            index_fields,
            bloom_filter_fields,
            data_retention,
            max_query_range,
            flatten_level,
            defined_schema_fields,
            store_original_data,
            approx_partition,
            distinct_value_fields,
            index_updated_at,
            extended_retention_days,
        }
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StreamPartition {
    pub field: String,
    #[serde(default)]
    pub types: StreamPartitionType,
    #[serde(default)]
    pub disabled: bool,
}

impl StreamPartition {
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            types: StreamPartitionType::Value,
            disabled: false,
        }
    }

    pub fn new_hash(field: &str, buckets: u64) -> Self {
        Self {
            field: field.to_string(),
            types: StreamPartitionType::Hash(std::cmp::max(16, buckets)),
            disabled: false,
        }
    }

    pub fn new_prefix(field: &str) -> Self {
        Self {
            field: field.to_string(),
            types: StreamPartitionType::Prefix,
            disabled: false,
        }
    }

    pub fn get_partition_key(&self, value: &str) -> String {
        format!("{}={}", self.field, self.get_partition_value(value))
    }

    pub fn get_partition_value(&self, value: &str) -> String {
        let val = match &self.types {
            StreamPartitionType::Value => value.to_string(),
            StreamPartitionType::Hash(n) => {
                let h = gxhash::new().sum64(value);
                let bucket = h % n;
                bucket.to_string()
            }
            StreamPartitionType::Prefix => value
                .to_ascii_lowercase()
                .chars()
                .next()
                .unwrap_or('_')
                .to_string(),
        };
        if val.is_ascii() {
            val
        } else {
            urlencoding::encode(&val).into_owned()
        }
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum StreamPartitionType {
    #[default]
    Value, // each value is a partition
    Hash(u64), // partition with fixed bucket size by hash
    Prefix,    // partition by first letter of term
}

impl Display for StreamPartitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamPartitionType::Value => write!(f, "value"),
            StreamPartitionType::Hash(_) => write!(f, "hash"),
            StreamPartitionType::Prefix => write!(f, "prefix"),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub struct PartitioningDetails {
    pub partition_keys: Vec<StreamPartition>,
    pub partition_time_level: Option<PartitionTimeLevel>,
}

// Code Duplicated from alerts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RoutingCondition {
    pub column: String,
    pub operator: Operator,
    #[schema(value_type = Object)]
    pub value: Value,
    #[serde(default)]
    pub ignore_case: bool,
}
// Code Duplicated from alerts
impl RoutingCondition {
    pub fn evaluate(&self, row: &Map<String, Value>) -> bool {
        let val = match row.get(&self.column) {
            Some(val) => val,
            None => {
                // field not found -> dropped
                return false;
            }
        };
        match val {
            Value::String(v) => {
                let val = v.as_str();
                let con_val = self.value.as_str().unwrap_or_default().trim_matches('"'); // "" is interpreted as empty string
                match self.operator {
                    Operator::EqualTo => val == con_val,
                    Operator::NotEqualTo => val != con_val,
                    Operator::GreaterThan => val > con_val,
                    Operator::GreaterThanEquals => val >= con_val,
                    Operator::LessThan => val < con_val,
                    Operator::LessThanEquals => val <= con_val,
                    Operator::Contains => val.contains(con_val),
                    Operator::NotContains => !val.contains(con_val),
                }
            }
            Value::Number(_) => {
                let val = val.as_f64().unwrap_or_default();
                let con_val = if self.value.is_number() {
                    self.value.as_f64().unwrap_or_default()
                } else {
                    self.value
                        .as_str()
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default()
                };
                match self.operator {
                    Operator::EqualTo => val == con_val,
                    Operator::NotEqualTo => val != con_val,
                    Operator::GreaterThan => val > con_val,
                    Operator::GreaterThanEquals => val >= con_val,
                    Operator::LessThan => val < con_val,
                    Operator::LessThanEquals => val <= con_val,
                    _ => false,
                }
            }
            Value::Bool(v) => {
                let val = v.to_owned();
                let con_val = if self.value.is_boolean() {
                    self.value.as_bool().unwrap_or_default()
                } else {
                    self.value
                        .as_str()
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default()
                };
                match self.operator {
                    Operator::EqualTo => val == con_val,
                    Operator::NotEqualTo => val != con_val,
                    _ => false,
                }
            }
            Value::Null => {
                matches!(self.operator, Operator::EqualTo)
                    && matches!(&self.value, Value::String(v) if v == "null")
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::EqualTo => write!(f, "="),
            Operator::NotEqualTo => write!(f, "!="),
            Operator::GreaterThan => write!(f, ">"),
            Operator::GreaterThanEquals => write!(f, ">="),
            Operator::LessThan => write!(f, "<"),
            Operator::LessThanEquals => write!(f, "<="),
            Operator::Contains => write!(f, "contains"),
            Operator::NotContains => write!(f, "not contains"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_file_meta() {
        let file_meta = FileMeta {
            min_ts: 1667978841110,
            max_ts: 1667978845354,
            records: 300,
            original_size: 10,
            compressed_size: 1,
            flattened: false,
            index_size: 0,
        };

        let rpc_meta = cluster_rpc::FileMeta::from(&file_meta);
        let resp = FileMeta::from(&rpc_meta);
        assert_eq!(file_meta, resp);
    }

    #[cfg(feature = "gxhash")]
    #[test]
    fn test_hash_partition() {
        let part = StreamPartition::new("field");
        assert_eq!(
            json::to_string(&part).unwrap(),
            r#"{"field":"field","types":"value","disabled":false}"#
        );
        let part = StreamPartition::new_hash("field", 32);
        assert_eq!(
            json::to_string(&part).unwrap(),
            r#"{"field":"field","types":{"hash":32},"disabled":false}"#
        );

        for key in &[
            "hello", "world", "foo", "bar", "test", "test1", "test2", "test3",
        ] {
            println!("{}: {}", key, part.get_partition_key(key));
        }
        assert_eq!(part.get_partition_key("hello"), "field=20");
        assert_eq!(part.get_partition_key("world"), "field=13");
        assert_eq!(part.get_partition_key("foo"), "field=21");
        assert_eq!(part.get_partition_key("bar"), "field=4");
        assert_eq!(part.get_partition_key("test"), "field=10");
        assert_eq!(part.get_partition_key("test1"), "field=21");
        assert_eq!(part.get_partition_key("test2"), "field=18");
        assert_eq!(part.get_partition_key("test3"), "field=6");
    }

    #[cfg(not(feature = "gxhash"))]
    #[test]
    fn test_hash_partition() {
        let part = StreamPartition::new("field");
        assert_eq!(
            json::to_string(&part).unwrap(),
            r#"{"field":"field","types":"value","disabled":false}"#
        );
        let part = StreamPartition::new_hash("field", 32);
        assert_eq!(
            json::to_string(&part).unwrap(),
            r#"{"field":"field","types":{"hash":32},"disabled":false}"#
        );

        for key in &[
            "hello", "world", "foo", "bar", "test", "test1", "test2", "test3",
        ] {
            println!("{}: {}", key, part.get_partition_key(key));
        }
        assert_eq!(part.get_partition_key("hello"), "field=30");
        assert_eq!(part.get_partition_key("world"), "field=20");
        assert_eq!(part.get_partition_key("foo"), "field=26");
        assert_eq!(part.get_partition_key("bar"), "field=7");
        assert_eq!(part.get_partition_key("test"), "field=13");
        assert_eq!(part.get_partition_key("test1"), "field=25");
        assert_eq!(part.get_partition_key("test2"), "field=4");
        assert_eq!(part.get_partition_key("test3"), "field=2");
    }

    #[test]
    fn test_stream_params() {
        let params = StreamParams::new("org_id", "stream_name", StreamType::Logs);
        let param2 = StreamParams::new("org_id", "stream_name", StreamType::Logs);
        let param3 = StreamParams::new("org_id", "stream_name", StreamType::Index);
        assert_eq!(params.org_id, "org_id");
        assert_eq!(params.stream_name, "stream_name");
        assert_eq!(params.stream_type, StreamType::Logs);
        let mut map = HashMap::new();
        map.insert(params, 1);
        map.insert(param2, 2);
        map.insert(param3, 2);
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_split_ranges() {
        // contains
        let range = TimeRange::new(0, 400);
        let other = TimeRange::new(50, 150);
        let (left, right) = range.split_by_range(&other).unwrap();
        assert!(left.as_ref().and(right.as_ref()).is_some());
        assert_eq!(left.unwrap(), TimeRange::new(0, 50));
        assert_eq!(right.unwrap(), TimeRange::new(150, 400));

        // partial overlap
        let range = TimeRange::new(0, 100);
        let other = TimeRange::new(50, 100);
        let (left, right) = range.split_by_range(&other).unwrap();
        assert!(left.as_ref().is_some());
        assert!(!right.is_some());
        assert_eq!(left.unwrap(), TimeRange::new(0, 50));

        // equals
        let range = TimeRange::new(0, 100);
        let other = TimeRange::new(0, 100);
        let (left, right) = range.split_by_range(&other).unwrap();
        assert!(left.and(right).is_none());

        // does not intersect
        let range = TimeRange::new(0, 100);
        let other = TimeRange::new(200, 300);
        let res = range.split_by_range(&other);
        assert!(res.is_err());
    }

    #[test]
    fn test_flatten_ranges() {
        let ranges = vec![TimeRange::new(0, 150), TimeRange::new(100, 200)];
        let expected_res = TimeRange::new(0, 200);
        let mut ranges = TimeRange::flatten_overlapping_ranges(ranges);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0], expected_res);

        ranges.clear();

        ranges = vec![
            TimeRange::new(0, 150),
            TimeRange::new(200, 300),
            TimeRange::new(100, 199),
        ];
        let expected_res = vec![TimeRange::new(0, 199), TimeRange::new(200, 300)];
        assert_eq!(TimeRange::flatten_overlapping_ranges(ranges), expected_res);
    }
}
