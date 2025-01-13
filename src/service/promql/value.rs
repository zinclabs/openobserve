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

use std::{cmp::Ordering, fmt, sync::Arc, time::Duration};

use config::{meta::promql::NAME_LABEL, utils::json, FxIndexMap};
use hashbrown::HashSet;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{
    de::{Deserializer, SeqAccess, Visitor},
    ser::{SerializeSeq, SerializeStruct, Serializer},
    Deserialize, Serialize,
};

// https://prometheus.io/docs/concepts/data_model/#metric-names-and-labels
static RE_VALID_LABEL_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap());

pub type Labels = Vec<Arc<Label>>;

/// Added functionalities on Labels
pub trait LabelsExt {
    /// Remove the metric name i.e. __name__ from the given label
    ///
    /// ```json
    /// {"__name__": "my-metric", "job": "k8s"} -> {"job": "k8s"}
    /// ```
    fn without_metric_name(&self) -> Labels;

    /// Return the value of the label associated with this name of the label.
    fn get_value(&self, name: &str) -> String;

    /// Set a new label -> value to this label
    fn set(&mut self, name: &str, value: &str);

    /// Get the values
    fn values(&self) -> Vec<String>;

    /// Get the keys
    fn keys(&self) -> Vec<String>;

    /// Without label, drops the given label name from the output.
    fn without_label(&self, name: &str) -> Labels;

    /// Signature for the given set of labels
    fn signature(&self) -> Signature;

    /// keep the labels as described by the `labels` vector
    /// and delete everything else
    fn keep(&self, labels: &[String]) -> Labels;

    /// delete the labels as described by the `labels` vector
    /// and keep the remaining
    fn delete(&self, labels: &[String]) -> Labels;

    /// Sort the labels by name
    fn sort(&mut self);
}

impl LabelsExt for Labels {
    fn without_label(&self, name: &str) -> Labels {
        self.iter()
            .filter(|label| label.name != name)
            .cloned()
            .collect()
    }

    fn without_metric_name(&self) -> Labels {
        self.without_label(NAME_LABEL)
    }

    fn get_value(&self, name: &str) -> String {
        self.iter()
            .filter(|l| l.name == name)
            .map(|l| l.value.to_string())
            .take(1)
            .collect()
    }

    fn set(&mut self, name: &str, value: &str) {
        self.push(Arc::new(Label {
            name: name.to_string(),
            value: value.to_string(),
        }))
    }

    fn signature(&self) -> Signature {
        signature(self)
    }

    fn keep(&self, labels: &[String]) -> Labels {
        self.iter()
            .flat_map(|label| {
                if labels.contains(&label.name) {
                    Some(label.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn delete(&self, labels: &[String]) -> Labels {
        self.iter()
            .flat_map(|label| {
                if !labels.contains(&label.name) {
                    Some(label.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    fn sort(&mut self) {
        self.sort_by(|a, b| a.name.cmp(&b.name))
    }

    fn values(&self) -> Vec<String> {
        self.iter().map(|label| label.value.clone()).collect()
    }

    fn keys(&self) -> Vec<String> {
        self.iter().map(|label| label.name.clone()).collect()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Label {
    pub name: String,
    pub value: String,
}

impl Label {
    /// Create a new instance of Label.
    pub fn new<S: Into<String>>(name: S, value: S) -> Self {
        Label {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Check if the label name is valid.
    pub fn is_valid_label_name(name: &str) -> bool {
        RE_VALID_LABEL_NAME.is_match(name)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Sample {
    /// Time in microseconds
    pub timestamp: i64,
    pub value: f64,
}

impl Serialize for Sample {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&(self.timestamp / 1_000_000))?;
        seq.serialize_element(&self.value.to_string())?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Sample {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SampleVisitor;

        impl<'de> Visitor<'de> for SampleVisitor {
            type Value = Sample;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of [timestamp, value]")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Sample, V::Error>
            where
                V: SeqAccess<'de>,
            {
                // Get timestamp (in seconds)
                let timestamp: f64 = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                // Get value as string
                let value_str: String = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

                // Parse value string to f64
                let value = value_str.parse::<f64>().map_err(serde::de::Error::custom)?;

                // Convert timestamp from seconds to microseconds
                let timestamp = (timestamp * 1_000_000.0) as i64;

                Ok(Sample { timestamp, value })
            }
        }

        deserializer.deserialize_seq(SampleVisitor)
    }
}

impl Sample {
    pub(crate) fn new(timestamp: i64, value: f64) -> Self {
        Self { timestamp, value }
    }

    pub(crate) fn is_nan(&self) -> bool {
        self.value.is_nan()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = json::Map::new();
        buf.insert("_timestamp".to_string(), json::Value::from(self.timestamp));
        buf.insert("value".to_string(), json::Value::from(self.value));
        json::to_vec(&buf).unwrap()
    }

    pub fn from_bytes(data: &[u8]) -> Sample {
        let data: json::Value = json::from_slice(data).unwrap();
        let data = data.as_object().unwrap();
        let timestamp = data.get("_timestamp").unwrap().as_i64().unwrap();
        let value = data.get("value").unwrap().as_f64().unwrap();
        Sample::new(timestamp, value)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Exemplar {
    /// Time in microseconds
    pub timestamp: i64,
    pub value: f64,
    pub labels: Labels,
}

impl Serialize for Exemplar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_struct("exemplars", 3)?;
        let labels_map = self
            .labels
            .iter()
            .map(|l| (l.name.as_str(), l.value.as_str()))
            .collect::<FxIndexMap<_, _>>();
        seq.serialize_field("timestamp", &(self.timestamp / 1_000_000))?;
        seq.serialize_field("value", &self.value.to_string())?;
        seq.serialize_field("labels", &labels_map)?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Exemplar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ExemplarVisitor;

        impl<'de> Visitor<'de> for ExemplarVisitor {
            type Value = Exemplar;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Exemplar")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Exemplar, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut timestamp = None;
                let mut value = None;
                let mut labels = Vec::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "timestamp" => {
                            let ts = map.next_value::<i64>()?;
                            timestamp = Some(ts * 1_000_000); // Convert seconds to microseconds
                        }
                        "value" => {
                            let val_str = map.next_value::<String>()?;
                            value = Some(val_str.parse::<f64>().map_err(serde::de::Error::custom)?);
                        }
                        "labels" => {
                            let label_map = map.next_value::<FxIndexMap<String, String>>()?;
                            labels = label_map
                                .into_iter()
                                .map(|(name, value)| Arc::new(Label::new(name, value)))
                                .collect();
                        }
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                let timestamp =
                    timestamp.ok_or_else(|| serde::de::Error::missing_field("timestamp"))?;
                let value = value.ok_or_else(|| serde::de::Error::missing_field("value"))?;

                Ok(Exemplar {
                    timestamp,
                    value,
                    labels,
                })
            }
        }

        deserializer.deserialize_struct(
            "exemplars",
            &["timestamp", "value", "labels"],
            ExemplarVisitor,
        )
    }
}

impl From<&json::Map<String, json::Value>> for Exemplar {
    fn from(data: &json::Map<String, json::Value>) -> Self {
        let timestamp = data.get("_timestamp").map(|v| v.as_i64().unwrap_or(0));
        let value = data.get("value").map(|v| v.as_f64().unwrap_or(0.0));
        let mut labels = vec![];
        for (k, v) in data.iter() {
            if k == "_timestamp" || k == "value" {
                continue;
            }
            labels.push(Arc::new(Label::new(
                k.to_string(),
                json::get_string_value(v),
            )));
        }
        Self {
            timestamp: timestamp.unwrap_or(0),
            value: value.unwrap_or(0.0),
            labels,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstantValue {
    pub labels: Labels,
    pub sample: Sample,
}

impl Serialize for InstantValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_struct("instant_value", 2)?;
        let labels_map = self
            .labels
            .iter()
            .map(|l| (l.name.as_str(), l.value.as_str()))
            .collect::<FxIndexMap<_, _>>();
        seq.serialize_field("metric", &labels_map)?;
        seq.serialize_field("value", &self.sample)?;
        seq.end()
    }
}

#[derive(Debug, Clone)]
pub struct TimeWindow {
    /// Evaluation timestamp, microseconds.
    pub eval_ts: i64,
    pub range: Duration,
    /// The offset used during the query execution.
    /// We don't use it (yet), so its value is always zero.
    // See https://github.com/prometheus/prometheus/blob/80b7f73d267a812b3689321554aec637b75f468d/promql/parser/ast.go#L192-L198
    pub offset: Duration,
}

impl TimeWindow {
    pub fn new(eval_ts: i64, range: Duration) -> Self {
        assert!(eval_ts > 0);
        Self {
            eval_ts,
            range,
            offset: Duration::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RangeValue {
    pub labels: Labels,
    pub samples: Vec<Sample>,
    pub exemplars: Option<Vec<Exemplar>>,
    pub time_window: Option<TimeWindow>,
}

impl RangeValue {
    /// Returns the values from the `samples` field as an array of f64
    pub fn get_sample_values(&self) -> Vec<f64> {
        self.samples.iter().map(|sample| sample.value).collect()
    }

    pub fn extend(&mut self, other: RangeValue) {
        self.samples.extend(other.samples);
        // check exemplars
        if let Some(exemplars) = &other.exemplars {
            if let Some(self_exemplars) = &mut self.exemplars {
                self_exemplars.extend(exemplars.iter().cloned());
            } else {
                self.exemplars = Some(exemplars.clone());
            }
        }
    }
}

impl Serialize for RangeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.exemplars.is_none() {
            let mut seq = serializer.serialize_struct("range_value", 2)?;
            let labels_map = self
                .labels
                .iter()
                .map(|l| (l.name.as_str(), l.value.as_str()))
                .collect::<FxIndexMap<_, _>>();
            seq.serialize_field("metric", &labels_map)?;
            seq.serialize_field("values", &self.samples)?;
            seq.end()
        } else {
            let mut seq = serializer.serialize_struct("range_value", 2)?;
            let labels_map = self
                .labels
                .iter()
                .map(|l| (l.name.as_str(), l.value.as_str()))
                .collect::<FxIndexMap<_, _>>();
            seq.serialize_field("seriesLabels", &labels_map)?;
            seq.serialize_field("exemplars", &self.exemplars.as_ref().unwrap())?;
            seq.end()
        }
    }
}

impl<'de> Deserialize<'de> for RangeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RangeValueVisitor;

        impl<'de> Visitor<'de> for RangeValueVisitor {
            type Value = RangeValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map with metric/seriesLabels and values/exemplars fields")
            }

            fn visit_map<V>(self, mut map: V) -> Result<RangeValue, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut labels_map: Option<FxIndexMap<String, String>> = None;
                let mut samples: Option<Vec<Sample>> = None;
                let mut exemplars: Option<Vec<Exemplar>> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "metric" | "seriesLabels" => {
                            labels_map = Some(map.next_value()?);
                        }
                        "values" => {
                            samples = Some(map.next_value()?);
                        }
                        "exemplars" => {
                            exemplars = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                let labels_map =
                    labels_map.ok_or_else(|| serde::de::Error::missing_field("metric"))?;
                let labels = labels_map
                    .into_iter()
                    .map(|(name, value)| Arc::new(Label { name, value }))
                    .collect();

                Ok(RangeValue {
                    labels,
                    samples: samples.unwrap_or_default(),
                    exemplars,
                    time_window: None,
                })
            }
        }

        deserializer.deserialize_map(RangeValueVisitor)
    }
}

impl RangeValue {
    pub(crate) fn new<S>(labels: Labels, samples: S) -> Self
    where
        S: IntoIterator<Item = Sample>,
    {
        Self {
            labels,
            samples: Vec::from_iter(samples),
            exemplars: None,
            time_window: None,
        }
    }

    pub(crate) fn new_with_exemplars<S>(labels: Labels, exemplars: S) -> Self
    where
        S: IntoIterator<Item = Exemplar>,
    {
        Self {
            labels,
            samples: vec![],
            exemplars: Some(Vec::from_iter(exemplars)),
            time_window: None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ExtrapolationKind {
    /// Calculate the per-second average rate of increase of the time series.
    /// Adjust for breaks in monotonicity (counter resets).
    /// Should only be used with counters.
    ///
    /// See <https://prometheus.io/docs/prometheus/latest/querying/functions/#rate>
    Rate,

    /// Calculate the increase in the time series. Adjust for counter resets.
    /// Should only be used with counters.
    ///
    /// See <https://prometheus.io/docs/prometheus/latest/querying/functions/#increase>
    Increase,

    /// Calculate the difference between the first and last value. Don't adjust
    /// for counter resets. Should only be used with gauges.
    ///
    /// See <https://prometheus.io/docs/prometheus/latest/querying/functions/#delta>
    Delta,
}

/// `extrapolated_rate` is a utility function for rate/increase/delta.
///
/// Calculates the rate (allowing for counter resets if `kind` is Rate or
/// Increase), extrapolates if the first/last sample is close to the boundary,
/// and returns the result as either per-second (if `kind` is Rate) or overall.
///
/// Returns `None` if there are fewer than two samples.
///
/// See the diagrams at <https://promlabs.com/blog/2021/01/29/how-exactly-does-promql-calculate-rates/#extrapolation-of-data>
///
/// # Panics
///
/// Panics if the samples are not in the range.
// cf. https://github.com/prometheus/prometheus/blob/80b7f73d267a812b3689321554aec637b75f468d/promql/functions.go#L67
pub(crate) fn extrapolated_rate(
    samples: &[Sample],
    eval_ts: i64,
    range: Duration,
    offset: Duration,
    kind: ExtrapolationKind,
) -> Option<f64> {
    if samples.len() < 2 {
        // Not enough samples.
        return None;
    }

    let start = {
        let range_plus_offset = range
            .checked_add(offset)
            .expect("BUG: overflow")
            .as_micros()
            .try_into()
            .expect("BUG: integer conversion failed");
        eval_ts
            .checked_sub(range_plus_offset)
            .expect("BUG: overflow")
    };
    assert!(start > 0);
    let end = eval_ts
        .checked_sub(
            offset
                .as_micros()
                .try_into()
                .expect("BUG: integer conversion failed"),
        )
        .expect("BUG: overflow");
    assert!(end > 0);
    assert!(start <= end);

    let first = &samples[0];
    let last = &samples.last().unwrap();

    // The caller must ensure that the samples are in the range.
    assert!(first.timestamp <= last.timestamp);
    assert!(first.timestamp >= start);
    assert!(last.timestamp <= end);

    let mut result = last.value - first.value;

    let is_counter = matches!(kind, ExtrapolationKind::Rate | ExtrapolationKind::Increase);
    if is_counter {
        // Handle counter resets.
        let mut prev_value = first.value;
        for sample in &samples[1..] {
            if sample.value < prev_value {
                result += prev_value;
            }
            prev_value = sample.value;
        }
    }

    // Duration between first/last samples and boundary of range.
    let mut duration_to_start = (first.timestamp - start) as f64 / 1_000.0;
    let duration_to_end = (end - last.timestamp) as f64 / 1_000.0;

    let sampled_interval = (last.timestamp - first.timestamp) as f64 / 1_000.0;
    let avg_duration_between_samples = sampled_interval / (samples.len() - 1) as f64;

    if is_counter && result > 0.0 && first.value >= 0.0 {
        // Counters cannot be negative. If we have any slope at all
        // (i.e. `result` went up), we can extrapolate the zero point
        // of the counter. If the duration to the zero point is shorter
        // than the `duration_to_start`, we take the zero point as the start
        // of the series, thereby avoiding extrapolation to negative
        // counter values.
        let duration_to_zero = sampled_interval * (first.value / result);
        if duration_to_zero < duration_to_start {
            duration_to_start = duration_to_zero;
        }
    }

    // If the first/last samples are close to the boundaries of the range,
    // extrapolate the result. This is as we expect that another sample
    // will exist given the spacing between samples we've seen thus far,
    // with an allowance for noise.
    let extrapolation_threshold = avg_duration_between_samples * 1.1;
    let mut extrapolate_to_interval = sampled_interval;

    if duration_to_start < extrapolation_threshold {
        extrapolate_to_interval += duration_to_start;
    } else {
        extrapolate_to_interval += avg_duration_between_samples / 2.0;
    }
    if duration_to_end < extrapolation_threshold {
        extrapolate_to_interval += duration_to_end;
    } else {
        extrapolate_to_interval += avg_duration_between_samples / 2.0;
    }
    let factor = extrapolate_to_interval / sampled_interval;
    if matches!(kind, ExtrapolationKind::Rate) {
        result *= factor / range.as_secs_f64();
    } else {
        result *= factor;
    }

    Some(result)
}

pub fn labels_value(labels: &Labels, name: &str) -> Option<String> {
    labels
        .binary_search_by_key(&name, |label| label.name.as_str())
        .ok()
        .map(|index| labels[index].value.clone())
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Value {
    Instant(InstantValue),
    Range(RangeValue),
    Vector(Vec<InstantValue>),
    Matrix(Vec<RangeValue>),
    Sample(Sample), // only used for return literal value
    Float(f64),
    String(String),
    None,
}

impl Value {
    pub(crate) fn get_ref_matrix_values(&self) -> Option<&Vec<RangeValue>> {
        match self {
            Value::Matrix(values) => Some(values),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn get_vector(&self) -> Option<&Vec<InstantValue>> {
        match self {
            Value::Vector(values) => Some(values),
            _ => None,
        }
    }

    pub(crate) fn get_string(&self) -> Option<String> {
        match self {
            Value::String(v) => Some(v.into()),
            _ => None,
        }
    }

    pub(crate) fn get_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn get_type(&self) -> &str {
        match self {
            Value::Instant(_) => "vector",
            Value::Range(_) => "matrix",
            Value::Vector(_) => "vector",
            Value::Matrix(_) => "matrix",
            Value::Sample(_) => "scalar",
            Value::Float(_) => "scalar",
            Value::String(_) => "string",
            Value::None => "scalar",
        }
    }

    pub fn sort(&mut self) {
        match self {
            Value::Vector(v) => {
                v.sort_by(|a, b| {
                    b.sample
                        .value
                        .partial_cmp(&a.sample.value)
                        .unwrap_or(Ordering::Equal)
                });
            }
            Value::Matrix(v) => {
                v.sort_by(|a, b| {
                    let a = a.samples.iter().map(|x| x.value).sum::<f64>();
                    let b = b.samples.iter().map(|x| x.value).sum::<f64>();
                    b.partial_cmp(&a).unwrap_or(Ordering::Equal)
                });
            }
            _ => {}
        }
    }

    /// Checks if the vector or matrix types contain duplicated label set or
    /// not. This is an undefined condition, hence caller should raise an
    /// error in case this evaluates to `true`.
    pub fn contains_same_label_set(&self) -> bool {
        match self {
            Value::Vector(v) => match v.len() {
                0 | 1 => false,
                2 => v[0].labels.signature() == v[1].labels.signature(),
                _ => {
                    let mut signatures = HashSet::new();
                    for instant in v.iter() {
                        // If the set already contained this value, false is returned.
                        let new = signatures.insert(instant.labels.signature());
                        if !new {
                            return true;
                        }
                    }
                    false
                }
            },
            Value::Matrix(v) => match v.len() {
                0 | 1 => false,
                2 => v[0].labels.signature() == v[1].labels.signature(),
                _ => {
                    let mut signatures = HashSet::new();
                    for instant in v.iter() {
                        // If the set already contained this value, false is returned.
                        let new = signatures.insert(instant.labels.signature());
                        if !new {
                            return true;
                        }
                    }
                    false
                }
            },
            _ => false,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Signature([u8; 32]);

impl From<Signature> for String {
    fn from(sig: Signature) -> Self {
        hex::encode(sig.0)
    }
}

// REFACTORME: make this a method of `Metric`
pub fn signature(labels: &Labels) -> Signature {
    signature_without_labels(labels, &[])
}

/// `signature_without_labels` is just as [`signature`], but only for labels not
/// matching `names`.
// REFACTORME: make this a method of `Metric`
pub fn signature_without_labels(labels: &Labels, exclude_names: &[&str]) -> Signature {
    let mut hasher = blake3::Hasher::new();
    labels
        .iter()
        .filter(|item| !exclude_names.contains(&item.name.as_str()))
        .for_each(|item| {
            hasher.update_rayon(item.name.as_bytes());
            hasher.update_rayon(item.value.as_bytes());
        });
    Signature(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use expect_test::expect;
    use float_cmp::approx_eq;

    use super::*;

    fn generate_test_labels() -> Labels {
        let labels: Labels = vec![
            Arc::new(Label {
                name: "a".to_owned(),
                value: "1".to_owned(),
            }),
            Arc::new(Label {
                name: "b".to_owned(),
                value: "2".to_owned(),
            }),
            Arc::new(Label {
                name: "c".to_owned(),
                value: "3".to_owned(),
            }),
            Arc::new(Label {
                name: "d".to_owned(),
                value: "4".to_owned(),
            }),
        ];
        labels
    }
    #[test]
    fn test_signature_without_labels() {
        let labels: Labels = generate_test_labels();

        let sig = signature(&labels);
        expect![[r#"
            "f287fde2994111abd7740b5c7c28b0eeabe3f813ae65397bb6acb684e2ab6b22"
        "#]]
        .assert_debug_eq(&String::from(sig));

        let sig: String = signature_without_labels(&labels, &["a", "c"]).into();
        expect![[r#"
            "ec9c3a0c9c03420d330ab62021551cffe993c07b20189c5ed831dad22f54c0c7"
        "#]]
        .assert_debug_eq(&sig);
        assert_eq!(sig.len(), 64);

        assert_eq!(signature(&labels), signature_without_labels(&labels, &[]));
    }

    #[test]
    fn test_extrapolated_rate() {
        fn extrapolate(samples: &[Sample], kind: ExtrapolationKind) -> f64 {
            extrapolated_rate(
                samples,
                75_000_000,
                Duration::from_secs(60),
                Duration::ZERO,
                kind,
            )
            .unwrap()
        }

        // See the diagrams at
        // https://promlabs.com/blog/2021/01/29/how-exactly-does-promql-calculate-rates/#extrapolation-of-data

        // Diagram 1
        let samples = [
            Sample::new(23_000_000, 1.0),
            Sample::new(38_000_000, 1.0),
            Sample::new(53_000_000, 2.0),
            Sample::new(68_000_000, 2.0),
        ];

        let rate = extrapolate(&samples, ExtrapolationKind::Rate);
        assert!(approx_eq!(f64, rate, 0.0222, epsilon = 0.0001));

        let increase = extrapolate(&samples, ExtrapolationKind::Increase);
        assert!(approx_eq!(f64, increase, 1.3333, epsilon = 0.0001));
        assert!(approx_eq!(f64, increase, rate * 60.0));

        let delta = extrapolate(&samples, ExtrapolationKind::Delta);
        assert_eq!(delta, increase);

        // Diagram 2: the first value is too far from the boundary to extrapolate fully
        let samples_2 = &samples[1..];

        let rate = extrapolate(samples_2, ExtrapolationKind::Rate);
        assert!(approx_eq!(f64, rate, 0.0247, epsilon = 0.0001));

        let increase = extrapolate(samples_2, ExtrapolationKind::Increase);
        assert!(approx_eq!(f64, increase, 1.4833, epsilon = 0.0001));
        assert!(approx_eq!(f64, increase, rate * 60.0));

        let delta = extrapolate(samples_2, ExtrapolationKind::Delta);
        assert_eq!(delta, increase);

        // Diagram 3: dealing with counter resets
        let samples = [
            Sample::new(23_000_000, 6.0),
            Sample::new(38_000_000, 10.0),
            Sample::new(53_000_000, 4.0),
            Sample::new(68_000_000, 9.0),
        ];

        let rate = extrapolate(&samples, ExtrapolationKind::Rate);
        assert!(approx_eq!(f64, rate, 0.2888, epsilon = 0.0001));

        let increase = extrapolate(&samples, ExtrapolationKind::Increase);
        assert!(approx_eq!(f64, increase, 17.3333, epsilon = 0.0001));
        assert!(approx_eq!(f64, increase, rate * 60.0));

        let delta = extrapolate(&samples, ExtrapolationKind::Delta);
        assert!(approx_eq!(f64, delta, 4.0));
    }

    #[test]
    fn test_invalid_label_name() {
        assert!(!Label::is_valid_label_name("~invalid-label-name"));
    }

    #[test]
    fn test_get_value() {
        let labels: Labels = generate_test_labels();

        let value = labels.get_value("a");
        assert!(value == "1");

        let value = labels.get_value("non-existent-label");
        assert!(value.is_empty());
    }

    #[test]
    fn test_keep() {
        let labels: Labels = generate_test_labels();

        let labels_to_include = vec!["a".into(), "b".into()];
        let expected = vec![
            Arc::new(Label::new("a", "1")),
            Arc::new(Label::new("b", "2")),
        ];
        let output = labels.keep(&labels_to_include);

        use std::iter::zip;
        for (expect, got) in zip(expected, output.clone()) {
            assert_eq!(expect.name, got.name, "{:?}", &output);
        }
    }

    #[test]
    fn test_delete() {
        let labels: Labels = generate_test_labels();

        let labels_to_exclude = vec!["a".into(), "b".into()];
        let expected = vec![
            Arc::new(Label::new("c", "3")),
            Arc::new(Label::new("d", "4")),
        ];
        let output = labels.delete(&labels_to_exclude);

        use std::iter::zip;
        for (expect, got) in zip(expected, output.clone()) {
            assert_eq!(expect.name, got.name, "{:?}", &output);
        }
    }

    #[test]
    fn test_keep_delete_empty() {
        let labels: Labels = generate_test_labels();

        let labels_to_exclude = vec![];
        let expected = labels.clone();
        let output_deleted = labels.delete(&labels_to_exclude);
        let output_kept = labels.delete(&labels_to_exclude);

        use std::iter::zip;
        for (expect, got) in zip(expected.clone(), output_deleted.clone()) {
            assert_eq!(expect.name, got.name, "{:?}", &output_deleted);
        }
        for (expect, got) in zip(expected, output_kept.clone()) {
            assert_eq!(expect.name, got.name, "{:?}", &output_kept);
        }
    }
}
