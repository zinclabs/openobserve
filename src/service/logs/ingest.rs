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

use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, Cursor, Read},
};

use actix_web::http;
use anyhow::Result;
use chrono::{Duration, Utc};
use config::{
    meta::{
        self_reporting::usage::UsageType,
        stream::{StreamParams, StreamType},
    },
    metrics,
    utils::{flatten, json, time::parse_timestamp_micro_from_value},
    ID_COL_NAME, ORIGINAL_DATA_COL_NAME, TIMESTAMP_COL_NAME,
};
use flate2::read::GzDecoder;
use opentelemetry_proto::tonic::{
    collector::metrics::v1::ExportMetricsServiceRequest,
    common::v1::{any_value::Value, AnyValue, KeyValue},
    metrics::v1::metric::Data,
};
use prost::Message;
use serde_json::json;

use super::{bulk::TS_PARSE_FAILED, ingestion_log_enabled, log_failed_record};
use crate::{
    common::meta::ingestion::{
        AWSRecordType, GCPIngestionResponse, IngestionData, IngestionDataIter, IngestionError,
        IngestionRequest, IngestionResponse, IngestionStatus, KinesisFHIngestionResponse,
        StreamStatus,
    },
    service::{
        format_stream_name, get_formatted_stream_name, ingestion::check_ingestion_allowed,
        logs::bulk::TRANSFORM_FAILED, schema::get_upto_discard_error,
    },
};

pub async fn ingest(
    thread_id: usize,
    org_id: &str,
    in_stream_name: &str,
    in_req: IngestionRequest<'_>,
    user_email: &str,
    extend_json: Option<&HashMap<String, serde_json::Value>>,
) -> Result<IngestionResponse> {
    let start = std::time::Instant::now();
    let started_at: i64 = Utc::now().timestamp_micros();
    let cfg = config::get_config();
    let mut need_usage_report = true;
    let log_ingestion_errors = ingestion_log_enabled().await;

    // check stream
    let stream_name = if cfg.common.skip_formatting_stream_name {
        get_formatted_stream_name(StreamParams::new(org_id, in_stream_name, StreamType::Logs))
            .await?
    } else {
        format_stream_name(in_stream_name)
    };
    check_ingestion_allowed(org_id, Some(&stream_name))?;

    let min_ts = (Utc::now() - Duration::try_hours(cfg.limit.ingest_allowed_upto).unwrap())
        .timestamp_micros();

    let mut stream_params = vec![StreamParams::new(org_id, &stream_name, StreamType::Logs)];

    // Start retrieve associated pipeline and construct pipeline components
    let executable_pipeline = crate::service::ingestion::get_stream_executable_pipeline(
        org_id,
        &stream_name,
        &StreamType::Logs,
    )
    .await;
    let mut pipeline_inputs = Vec::new();
    let mut original_options = Vec::new();
    // End pipeline params construction

    if let Some(exec_pl) = &executable_pipeline {
        let pl_destinations = exec_pl.get_all_destination_streams();
        stream_params.extend(pl_destinations);
    }

    // Start get user defined schema
    let mut user_defined_schema_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut streams_need_original_set: HashSet<String> = HashSet::new();
    crate::service::ingestion::get_uds_and_original_data_streams(
        &stream_params,
        &mut user_defined_schema_map,
        &mut streams_need_original_set,
    )
    .await;
    // End get user defined schema

    let json_req: Vec<json::Value>; // to hold json request because of borrow checker
    let (endpoint, usage_type, data) = match in_req {
        IngestionRequest::JSON(req) => {
            json_req = json::from_slice(req).unwrap_or({
                let val: json::Value = json::from_slice(req)?;
                vec![val]
            });
            (
                "/api/org/ingest/logs/_json",
                UsageType::Json,
                IngestionData::JSON(&json_req),
            )
        }
        IngestionRequest::Multi(req) => (
            "/api/org/ingest/logs/_multi",
            UsageType::Multi,
            IngestionData::Multi(req),
        ),
        IngestionRequest::GCP(req) => (
            "/api/org/ingest/logs/_gcs",
            UsageType::GCPSubscription,
            IngestionData::GCP(req),
        ),
        IngestionRequest::KinesisFH(req) => (
            "/api/org/ingest/logs/_kinesis",
            UsageType::KinesisFirehose,
            IngestionData::KinesisFH(req),
        ),
        IngestionRequest::RUM(req) => (
            "/api/org/ingest/logs/_rum",
            UsageType::RUM,
            IngestionData::Multi(req),
        ),
        IngestionRequest::Usage(req) => {
            // no need to report usage for usage data
            need_usage_report = false;
            json_req = json::from_slice(req).unwrap_or({
                let val: json::Value = json::from_slice(req)?;
                vec![val]
            });
            (
                "/api/org/ingest/logs/_usage",
                UsageType::Json,
                IngestionData::JSON(&json_req),
            )
        }
    };

    let mut stream_status = StreamStatus::new(&stream_name);
    let mut json_data_by_stream = HashMap::new();
    for ret in data.iter() {
        let mut item = match ret {
            Ok(item) => item,
            Err(e) => {
                log::error!("IngestionError: {:?}", e);
                return Err(anyhow::anyhow!("Failed processing: {:?}", e));
            }
        };

        if let Some(extend) = extend_json.as_ref() {
            for (key, val) in extend.iter() {
                item[key] = val.clone();
            }
        }

        // store a copy of original data before it's being transformed and/or flattened, when
        // 1. original data is an object
        let original_data = if item.is_object() {
            // 2. current stream does not have pipeline
            if executable_pipeline.is_none() {
                // current stream requires original
                streams_need_original_set
                    .contains(&stream_name)
                    .then_some(item.to_string())
            } else {
                // 3. with pipeline, storing original as long as streams_need_original_set is not
                //    empty
                // because not sure the pipeline destinations
                (!streams_need_original_set.is_empty()).then_some(item.to_string())
            }
        } else {
            None // `item` won't be flattened, no need to store original
        };

        if executable_pipeline.is_some() {
            // handle record's timestamp fist in case record is sent to remote destination
            if let Err(e) = handle_timestamp(&mut item, min_ts) {
                stream_status.status.failed += 1;
                stream_status.status.error = e.to_string();
                metrics::INGEST_ERRORS
                    .with_label_values(&[
                        org_id,
                        StreamType::Logs.as_str(),
                        &stream_name,
                        TS_PARSE_FAILED,
                    ])
                    .inc();
                log_failed_record(log_ingestion_errors, &item, &e.to_string());
                continue;
            };
            // buffer the records, timestamp, and originals for pipeline batch processing
            pipeline_inputs.push(item);
            original_options.push(original_data);
        } else {
            // JSON Flattening
            let mut res = flatten::flatten_with_level(item, cfg.limit.ingest_flatten_level)?;

            // handle timestamp
            let timestamp = match handle_timestamp(&mut res, min_ts) {
                Ok(ts) => ts,
                Err(e) => {
                    stream_status.status.failed += 1;
                    stream_status.status.error = e.to_string();
                    metrics::INGEST_ERRORS
                        .with_label_values(&[
                            org_id,
                            StreamType::Logs.as_str(),
                            &stream_name,
                            TS_PARSE_FAILED,
                        ])
                        .inc();
                    log_failed_record(log_ingestion_errors, &res, &e.to_string());
                    continue;
                }
            };

            // get json object
            let mut local_val = match res.take() {
                json::Value::Object(val) => val,
                _ => unreachable!(),
            };

            if let Some(fields) = user_defined_schema_map.get(&stream_name) {
                local_val = crate::service::logs::refactor_map(local_val, fields);
            }

            // add `_original` and '_record_id` if required by StreamSettings
            if streams_need_original_set.contains(&stream_name) && original_data.is_some() {
                local_val.insert(
                    ORIGINAL_DATA_COL_NAME.to_string(),
                    original_data.unwrap().into(),
                );
                let record_id = crate::service::ingestion::generate_record_id(
                    org_id,
                    &stream_name,
                    &StreamType::Logs,
                );
                local_val.insert(
                    ID_COL_NAME.to_string(),
                    json::Value::String(record_id.to_string()),
                );
            } else {
                // remove _original and _record_id if exist
                // when records are results of a scheduled pipeline
                local_val.remove(ORIGINAL_DATA_COL_NAME);
                local_val.remove(ID_COL_NAME);
            }

            let (ts_data, fn_num) = json_data_by_stream
                .entry(stream_name.clone())
                .or_insert_with(|| (Vec::new(), None));
            ts_data.push((timestamp, local_val));
            *fn_num = need_usage_report.then_some(0); // no pl -> no func
        }
    }

    // batch process records through pipeline
    if let Some(exec_pl) = &executable_pipeline {
        let records_count = pipeline_inputs.len();
        match exec_pl.process_batch(org_id, pipeline_inputs).await {
            Err(e) => {
                log::error!(
                    "[Pipeline] for stream {}/{}: Batch execution error: {}.",
                    org_id,
                    stream_name,
                    e
                );
                stream_status.status.failed += records_count as u32;
                stream_status.status.error = format!("Pipeline batch execution error: {}", e);
                metrics::INGEST_ERRORS
                    .with_label_values(&[
                        org_id,
                        StreamType::Logs.as_str(),
                        &stream_name,
                        TRANSFORM_FAILED,
                    ])
                    .inc();
            }
            Ok(pl_results) => {
                let function_no = exec_pl.num_of_func();
                for (stream_params, stream_pl_results) in pl_results {
                    if stream_params.stream_type != StreamType::Logs {
                        continue;
                    }

                    for (idx, mut res) in stream_pl_results {
                        // get json object
                        let mut local_val = match res.take() {
                            json::Value::Object(val) => val,
                            _ => unreachable!(),
                        };

                        if let Some(fields) =
                            user_defined_schema_map.get(stream_params.stream_name.as_str())
                        {
                            local_val = crate::service::logs::refactor_map(local_val, fields);
                        }

                        // add `_original` and '_record_id` if required by StreamSettings
                        if streams_need_original_set.contains(stream_params.stream_name.as_str())
                            && original_options[idx].is_some()
                        {
                            local_val.insert(
                                ORIGINAL_DATA_COL_NAME.to_string(),
                                original_options[idx].clone().unwrap().into(),
                            );
                            let record_id = crate::service::ingestion::generate_record_id(
                                org_id,
                                &stream_params.stream_name,
                                &StreamType::Logs,
                            );
                            local_val.insert(
                                ID_COL_NAME.to_string(),
                                json::Value::String(record_id.to_string()),
                            );
                        } else {
                            // remove _original and _record_id if exist
                            // when records are results of a scheduled pipeline
                            local_val.remove(ORIGINAL_DATA_COL_NAME);
                            local_val.remove(ID_COL_NAME);
                        }

                        let Some(timestamp) =
                            local_val.get(TIMESTAMP_COL_NAME).and_then(|ts| ts.as_i64())
                        else {
                            let err = "record _timestamp inserted before pipeline processing, but missing after pipeline processing";
                            stream_status.status.failed += 1;
                            stream_status.status.error = err.to_string();
                            metrics::INGEST_ERRORS
                                .with_label_values(&[
                                    org_id,
                                    StreamType::Logs.as_str(),
                                    &stream_name,
                                    TS_PARSE_FAILED,
                                ])
                                .inc();
                            log_failed_record(log_ingestion_errors, &local_val, err);
                            continue;
                        };

                        let (ts_data, fn_num) = json_data_by_stream
                            .entry(stream_params.stream_name.to_string())
                            .or_insert_with(|| (Vec::new(), None));
                        ts_data.push((timestamp, local_val));
                        *fn_num = need_usage_report.then_some(function_no);
                    }
                }
            }
        }
    }

    // if no data, fast return
    if json_data_by_stream.is_empty() {
        return Ok(IngestionResponse::new(
            http::StatusCode::OK.into(),
            vec![stream_status],
        ));
    }

    // drop memory-intensive variables
    drop(streams_need_original_set);
    drop(executable_pipeline);
    drop(original_options);
    drop(user_defined_schema_map);

    let (metric_rpt_status_code, response_body) = {
        let mut status = IngestionStatus::Record(stream_status.status);
        let write_result = super::write_logs_by_stream(
            thread_id,
            org_id,
            user_email,
            (started_at, &start),
            usage_type,
            &mut status,
            json_data_by_stream,
        )
        .await;
        stream_status.status = match status {
            IngestionStatus::Record(status) => status,
            IngestionStatus::Bulk(_) => unreachable!(),
        };
        match write_result {
            Ok(()) => ("200", stream_status),
            Err(e) => {
                log::error!("Error while writing logs: {}", e);
                ("500", stream_status)
            }
        }
    };

    // update ingestion metrics
    let took_time = start.elapsed().as_secs_f64();
    metrics::HTTP_RESPONSE_TIME
        .with_label_values(&[
            endpoint,
            metric_rpt_status_code,
            org_id,
            &stream_name,
            StreamType::Logs.as_str(),
        ])
        .observe(took_time);
    metrics::HTTP_INCOMING_REQUESTS
        .with_label_values(&[
            endpoint,
            metric_rpt_status_code,
            org_id,
            &stream_name,
            StreamType::Logs.as_str(),
        ])
        .inc();

    Ok(IngestionResponse::new(
        http::StatusCode::OK.into(),
        vec![response_body],
    ))
}

pub fn handle_timestamp(value: &mut json::Value, min_ts: i64) -> Result<i64, anyhow::Error> {
    let local_val = value
        .as_object_mut()
        .ok_or_else(|| anyhow::Error::msg("Value is not an object"))?;
    let timestamp = match local_val.get(TIMESTAMP_COL_NAME) {
        Some(v) => match parse_timestamp_micro_from_value(v) {
            Ok(t) => t,
            Err(_) => return Err(anyhow::Error::msg("Can't parse timestamp")),
        },
        None => Utc::now().timestamp_micros(),
    };
    // check ingestion time
    if timestamp < min_ts {
        return Err(get_upto_discard_error());
    }
    local_val.insert(
        TIMESTAMP_COL_NAME.to_string(),
        json::Value::Number(timestamp.into()),
    );
    Ok(timestamp)
}

impl Iterator for IngestionDataIter<'_> {
    type Item = Result<json::Value, IngestionError>;

    fn next(&mut self) -> Option<Result<json::Value, IngestionError>> {
        match self {
            IngestionDataIter::JSONIter(iter) => iter.next().cloned().map(Ok),
            IngestionDataIter::MultiIter(iter) => loop {
                match iter.next() {
                    Some(Ok(line)) if line.trim().is_empty() => {
                        // If the line is empty, just continue to the next iteration.
                        continue;
                    }
                    Some(Ok(line)) => {
                        // If the line is not empty, attempt to parse it as JSON.
                        return Some(json::from_str(&line).map_err(IngestionError::from));
                    }
                    Some(Err(e)) => {
                        // If there's an error reading the line, return it.
                        return Some(Err(IngestionError::from(e)));
                    }
                    None => {
                        // If there are no more lines, return None.
                        return None;
                    }
                }
            },
            IngestionDataIter::GCP(iter, err) => match err {
                Some(e) => Some(Err(IngestionError::GCPError(e.clone()))),
                None => iter.next().map(Ok),
            },
            IngestionDataIter::KinesisFH(iter, err) => match err {
                Some(e) => Some(Err(IngestionError::AWSError(e.clone()))),
                None => iter.next().map(Ok),
            },
        }
    }
}

impl<'a> IngestionData<'a> {
    pub fn iter(&'a self) -> IngestionDataIter<'a> {
        match self {
            IngestionData::JSON(vec) => IngestionDataIter::JSONIter(vec.iter()),
            IngestionData::Multi(data) => {
                IngestionDataIter::MultiIter(std::io::BufReader::new(*data).lines())
            }
            IngestionData::GCP(request) => {
                let data = &request.message.data;
                let request_id = &request.message.message_id;
                let req_timestamp = &request.message.publish_time;
                match decode_and_decompress_to_string(data) {
                    Ok(decompressed_data) => {
                        let value: json::Value = json::from_str(&decompressed_data).unwrap();
                        IngestionDataIter::GCP(vec![value].into_iter(), None)
                    }
                    Err(e) => IngestionDataIter::GCP(
                        vec![].into_iter(),
                        Some(GCPIngestionResponse {
                            request_id: request_id.to_string(),
                            error_message: Some(e.to_string()),
                            timestamp: req_timestamp.to_string(),
                        }),
                    ),
                }
            }
            IngestionData::KinesisFH(request) => {
                let mut events = Vec::new();
                let request_id = &request.request_id;
                let req_timestamp = request.timestamp.unwrap_or(Utc::now().timestamp_micros());

                for record in &request.records {
                    match decode_and_decompress_to_vec(&record.data) {
                        Err(err) => {
                            return IngestionDataIter::KinesisFH(
                                events.into_iter(),
                                Some(KinesisFHIngestionResponse {
                                    request_id: request_id.to_string(),
                                    error_message: Some(err.to_string()),
                                    timestamp: req_timestamp,
                                }),
                            );
                        }
                        Ok(decompressed_data) => {
                            match deserialize_aws_record_from_vec(decompressed_data, request_id) {
                                Ok(parsed_events) => events.extend(parsed_events),
                                Err(err) => {
                                    return IngestionDataIter::KinesisFH(
                                        events.into_iter(),
                                        Some(KinesisFHIngestionResponse {
                                            request_id: request_id.to_string(),
                                            error_message: Some(err.to_string()),
                                            timestamp: req_timestamp,
                                        }),
                                    );
                                }
                            }
                        }
                    }
                }
                IngestionDataIter::KinesisFH(events.into_iter(), None)
            }
        }
    }
}

// Protobufs are not valid UTF-8 strings, so we need to maintain them as byte arrays
pub fn decode_and_decompress_to_vec(
    encoded_data: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let decoded_data = config::utils::base64::decode_raw(encoded_data)?;
    let mut gz = GzDecoder::new(decoded_data.as_slice());
    let mut vec = Vec::new();
    match gz.read_to_end(&mut vec) {
        Ok(_) => Ok(vec),
        Err(_) => Ok(decoded_data),
    }
}

// Use this function when we know the data is JSON since it will be valid UTF-8
pub fn decode_and_decompress_to_string(
    encoded_data: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let decoded_data = config::utils::base64::decode_raw(encoded_data)?;
    let mut gz = GzDecoder::new(decoded_data.as_slice());
    let mut decompressed_data = String::new();
    match gz.read_to_string(&mut decompressed_data) {
        Ok(_) => Ok(decompressed_data),
        Err(_) => Ok(String::from_utf8(decoded_data)?),
    }
}

/// Calculate size of VarInt header from byte array
///
/// See https://protobuf.dev/programming-guides/encoding/#varints for more info
pub fn get_size_of_var_int_header(bytes: &[u8]) -> Option<usize> {
    for (i, &b) in bytes.iter().enumerate() {
        // if most significant bit is 0
        if b & 0x80 == 0 {
            return Some(i + 1);
        }
    }

    None
}

fn deserialize_aws_record_from_vec(data: Vec<u8>, request_id: &str) -> Result<Vec<json::Value>> {
    // If it's a protobuf, process it as an OpenTelemetry 1.0 metric
    if let Some(header) = get_size_of_var_int_header(&data) {
        if let Ok(a) = ExportMetricsServiceRequest::decode(&mut Cursor::new(&data[header..])) {
            return construct_values_from_open_telemetry_v1_metric(a);
        }
    }

    let mut events = vec![];
    let mut value;
    let data = String::from_utf8(data)?;

    // It's likely newline-delimited JSON objects
    for line in data.lines() {
        match json::from_str(line) {
            Ok(AWSRecordType::KinesisFHLogs(kfh_log_data)) => {
                for event in kfh_log_data.log_events.iter() {
                    value = json::to_value(event)?;
                    let local_val = value
                        .as_object_mut()
                        .ok_or(anyhow::anyhow!("Error to convert Value to object"))?;

                    local_val.insert("requestId".to_owned(), request_id.into());
                    local_val.insert(
                        "messageType".to_owned(),
                        kfh_log_data.message_type.clone().into(),
                    );
                    local_val.insert("owner".to_owned(), kfh_log_data.owner.clone().into());
                    local_val.insert("logGroup".to_owned(), kfh_log_data.log_group.clone().into());
                    local_val.insert(
                        "logStream".to_owned(),
                        kfh_log_data.log_stream.clone().into(),
                    );
                    local_val.insert(
                        "subscriptionFilters".to_owned(),
                        kfh_log_data.subscription_filters.clone().into(),
                    );

                    let local_msg = event.message.as_str().unwrap();

                    if local_msg.starts_with('{') && local_msg.ends_with('}') {
                        let result: Result<json::Value, json::Error> = json::from_str(local_msg);

                        match result {
                            Err(_e) => {
                                local_val.insert("message".to_owned(), event.message.clone());
                            }
                            Ok(message_val) => {
                                local_val.insert("message".to_owned(), message_val.clone());
                            }
                        }
                    } else {
                        local_val.insert("message".to_owned(), local_msg.into());
                    }

                    local_val.insert(TIMESTAMP_COL_NAME.to_string(), event.timestamp.into());

                    value = local_val.clone().into();
                    events.push(value);
                }
            }
            Ok(AWSRecordType::KinesisFHMetrics(kfh_metric_data)) => {
                // Parse "dimensions" and "values" fields from KinesisFHMetricData
                let values = json::to_value(kfh_metric_data.value.clone())?;
                let dimensions = kfh_metric_data.dimensions.clone();
                let timestamp = kfh_metric_data.timestamp;

                let mut parsed_metric_value = json::to_value(kfh_metric_data)?;
                let local_parsed_metric_value = parsed_metric_value.as_object_mut().ok_or(
                    anyhow::anyhow!("CloudWatch metrics failed to parse Metric Object"),
                )?;

                for (value_name, value_val) in values.as_object().ok_or(anyhow::anyhow!(
                    "CloudWatch metrics failed to Metric Value Object"
                ))? {
                    local_parsed_metric_value.insert(value_name.to_owned(), value_val.to_owned());
                }
                local_parsed_metric_value.remove("value");

                let metric_dimensions = dimensions
                    .as_object()
                    .ok_or(anyhow::anyhow!(
                        "CloudWatch metrics dimensions parsing failed"
                    ))?
                    .iter()
                    .map(|(k, v)| format!("{}=[{}]", k, v))
                    .collect::<Vec<_>>()
                    .join(", ");

                local_parsed_metric_value
                    .insert("metric_dimensions".to_owned(), metric_dimensions.into());
                local_parsed_metric_value.remove("dimensions");

                local_parsed_metric_value.insert(TIMESTAMP_COL_NAME.to_string(), timestamp.into());
                local_parsed_metric_value.remove("timestamp");

                value = local_parsed_metric_value.clone().into();
                events.push(value);
            }
            _ => {
                value = json::from_str(line)?;
                events.push(value);
            }
        }
    }
    Ok(events)
}

/// Extract a resource ID from an Amazon Resource Number string
///
/// See https://docs.aws.amazon.com/IAM/latest/UserGuide/reference-arns.html for more information
/// on ARNs
fn extract_resource_id_from_amazon_resource_number(arn: &str) -> &str {
    // skip the "arn" through the "account-id"
    let mut iter = arn.split(':').skip(5);
    // store directly into static array to avoid allocating Vec since we know what we want
    let split = [iter.next(), iter.next()];

    // If ARN looks like "arn:partition:service:region:account-id:resource-type:resource-id"
    if let Some(resource_id) = split[1] {
        return resource_id;
    }

    // If ARN looks like "arn:partition:service:region:account-id:resource-type/resource-id"
    if let Some((_, resource_id)) = split[0].unwrap().split_once('/') {
        return resource_id;
    }

    // ARN looks like "arn:partition:service:region:account-id:resource-id"
    split[0].unwrap()
}

/// Get the StringValue pair from the nested open telemetry KeyValue struct, else return None if it
/// isn't a StringValue
fn get_tuple_from_open_telemetry_key_value(kv: KeyValue) -> Option<(String, String)> {
    if let Some(AnyValue {
        value: Some(Value::StringValue(s)),
    }) = kv.value
    {
        Some((kv.key, s))
    } else {
        None
    }
}

/// Convert an OpenTelemetry v1.0 formatted request into a vector of json values.
///
/// The values are formatted to look the same as the ones extracted from AWS JSON telemetry format
fn construct_values_from_open_telemetry_v1_metric(
    data: ExportMetricsServiceRequest,
) -> Result<Vec<json::Value>> {
    let mut events = Vec::new();

    for resource_metric in data.resource_metrics {
        if resource_metric.resource.is_none() {
            continue;
        }

        // Collect all resource key value attributes e.g. cloud account ID and region
        let resource_attributes: HashMap<_, _> = resource_metric
            .resource
            .unwrap()
            .attributes
            .into_iter()
            .filter_map(get_tuple_from_open_telemetry_key_value)
            .collect();

        for sm in resource_metric.scope_metrics {
            for m in sm.metrics {
                let summary = match m.data {
                    Some(Data::Summary(summary)) => summary,
                    _ => continue, // AWS docs state that type should always be Summary
                };

                for i_sum in summary.data_points {
                    let dimensions = i_sum
                        .attributes
                        .iter()
                        .find(|kv| kv.key == "Dimensions")
                        .cloned();

                    let summary_attributes: HashMap<_, _> = i_sum
                        .attributes
                        .into_iter()
                        .filter_map(get_tuple_from_open_telemetry_key_value)
                        .collect();

                    let resource_id = extract_resource_id_from_amazon_resource_number(
                        resource_attributes.get("aws.exporter.arn").unwrap(),
                    );

                    let mut mv = json!({
                        "metric_stream_name": resource_id,
                        "account_id": resource_attributes.get("cloud.account.id").unwrap(),
                        "region": resource_attributes.get("cloud.region").unwrap(),
                        "namespace": summary_attributes.get("Namespace").unwrap(),
                        "metric_name": summary_attributes.get("MetricName").unwrap(),
                        TIMESTAMP_COL_NAME: std::time::Duration::from_nanos(i_sum.time_unix_nano).as_millis(),
                        "unit": m.unit,
                        "count": i_sum.count,
                        "sum": i_sum.sum,
                    });
                    let metric_value = mv.as_object_mut().unwrap();

                    if let Some(dimensions) = dimensions {
                        let string = match dimensions.value {
                            Some(AnyValue {
                                value: Some(Value::KvlistValue(kv_list)),
                            }) => kv_list.values,
                            _ => Vec::new(),
                        }
                        .into_iter()
                        .filter_map(get_tuple_from_open_telemetry_key_value)
                        .map(|(k, v)| format!("{}=[\"{}\"]", k, v))
                        .collect::<Vec<_>>()
                        .join(", ");
                        metric_value.insert("metric_dimensions".to_string(), string.into());
                    }

                    for q in i_sum.quantile_values {
                        match q.quantile {
                            // Min and max values are the observed values for 0.0 and 1.0 quantiles
                            0.0 => metric_value.insert("min".to_string(), q.value.into()),
                            1.0 => metric_value.insert("max".to_string(), q.value.into()),
                            // Insert the rest of the quantiles in a format similar to p99.9
                            _ => metric_value
                                .insert(format!("p{:.1}", q.quantile * 100.0), q.value.into()),
                        };
                    }

                    events.push(mv);
                }
            }
        }
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_and_decompress_to_string, decode_and_decompress_to_vec,
        deserialize_aws_record_from_vec, extract_resource_id_from_amazon_resource_number,
        get_size_of_var_int_header,
    };

    #[test]
    fn test_decode_and_decompress_success_string() {
        let encoded_data = "H4sIAAAAAAAAADWO0QqCMBiFX2XsOkKJZHkXot5YQgpdhMTSPzfSTbaZhPjuzbTLj3M45xtxC1rTGvJPB9jHQXrOL2lyP4VZdoxDvMFyEKDmpJF9NVBTskTW2gaNrGMl+85mC2VGAW0X1P1Dl4p3hksR8caA0ti/Fb9e+AZhZhwxr5a64VbD0NaOuR5xPLJzycEh+81fbxa4JmjVQ6uejwIG5YuLGjGgjWFIPlFll7ig8zOKuAImNWzxVExfL8ipzewAAAA=";
        let expected = "{\"messageType\":\"CONTROL_MESSAGE\",\"owner\":\"CloudwatchLogs\",\"logGroup\":\"\",\"logStream\":\"\",\"subscriptionFilters\":[],\"logEvents\":[{\"id\":\"\",\"timestamp\":1680683189085,\"message\":\"CWL CONTROL MESSAGE: Checking health of destination Firehose.\"}]}";
        let result = decode_and_decompress_to_string(encoded_data)
            .expect("Failed to decode and decompress data");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_and_decompress_success_vec() {
        let encoded_data = "H4sIAAAAAAAAADWO0QqCMBiFX2XsOkKJZHkXot5YQgpdhMTSPzfSTbaZhPjuzbTLj3M45xtxC1rTGvJPB9jHQXrOL2lyP4VZdoxDvMFyEKDmpJF9NVBTskTW2gaNrGMl+85mC2VGAW0X1P1Dl4p3hksR8caA0ti/Fb9e+AZhZhwxr5a64VbD0NaOuR5xPLJzycEh+81fbxa4JmjVQ6uejwIG5YuLGjGgjWFIPlFll7ig8zOKuAImNWzxVExfL8ipzewAAAA=";
        let expected = vec![
            123, 34, 109, 101, 115, 115, 97, 103, 101, 84, 121, 112, 101, 34, 58, 34, 67, 79, 78,
            84, 82, 79, 76, 95, 77, 69, 83, 83, 65, 71, 69, 34, 44, 34, 111, 119, 110, 101, 114,
            34, 58, 34, 67, 108, 111, 117, 100, 119, 97, 116, 99, 104, 76, 111, 103, 115, 34, 44,
            34, 108, 111, 103, 71, 114, 111, 117, 112, 34, 58, 34, 34, 44, 34, 108, 111, 103, 83,
            116, 114, 101, 97, 109, 34, 58, 34, 34, 44, 34, 115, 117, 98, 115, 99, 114, 105, 112,
            116, 105, 111, 110, 70, 105, 108, 116, 101, 114, 115, 34, 58, 91, 93, 44, 34, 108, 111,
            103, 69, 118, 101, 110, 116, 115, 34, 58, 91, 123, 34, 105, 100, 34, 58, 34, 34, 44,
            34, 116, 105, 109, 101, 115, 116, 97, 109, 112, 34, 58, 49, 54, 56, 48, 54, 56, 51, 49,
            56, 57, 48, 56, 53, 44, 34, 109, 101, 115, 115, 97, 103, 101, 34, 58, 34, 67, 87, 76,
            32, 67, 79, 78, 84, 82, 79, 76, 32, 77, 69, 83, 83, 65, 71, 69, 58, 32, 67, 104, 101,
            99, 107, 105, 110, 103, 32, 104, 101, 97, 108, 116, 104, 32, 111, 102, 32, 100, 101,
            115, 116, 105, 110, 97, 116, 105, 111, 110, 32, 70, 105, 114, 101, 104, 111, 115, 101,
            46, 34, 125, 93, 125,
        ];
        let result = decode_and_decompress_to_vec(encoded_data)
            .expect("Failed to decode and decompress data");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_success_string() {
        let encoded_data = "eyJtZXNzYWdlIjoiMiAwNTg2OTQ4NTY0NzYgZW5pLTAzYzBmNWJhNzlhNjZlZjE3IDEwLjMuMTY2LjcxIDEwLjMuMTQxLjIwOSA0NDMgMzg2MzQgNiAxMDMgNDI5MjYgMTY4MDgzODU1NiAxNjgwODM4NTc4IEFDQ0VQVCBPSyJ9Cg==";
        let expected = "{\"message\":\"2 058694856476 eni-03c0f5ba79a66ef17 10.3.166.71 10.3.141.209 443 38634 6 103 42926 1680838556 1680838578 ACCEPT OK\"}\n";
        let result = decode_and_decompress_to_string(encoded_data).expect("Failed to decode data");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_success_vec() {
        let encoded_data = "eyJtZXNzYWdlIjoiMiAwNTg2OTQ4NTY0NzYgZW5pLTAzYzBmNWJhNzlhNjZlZjE3IDEwLjMuMTY2LjcxIDEwLjMuMTQxLjIwOSA0NDMgMzg2MzQgNiAxMDMgNDI5MjYgMTY4MDgzODU1NiAxNjgwODM4NTc4IEFDQ0VQVCBPSyJ9Cg==";
        let expected = vec![
            123, 34, 109, 101, 115, 115, 97, 103, 101, 34, 58, 34, 50, 32, 48, 53, 56, 54, 57, 52,
            56, 53, 54, 52, 55, 54, 32, 101, 110, 105, 45, 48, 51, 99, 48, 102, 53, 98, 97, 55, 57,
            97, 54, 54, 101, 102, 49, 55, 32, 49, 48, 46, 51, 46, 49, 54, 54, 46, 55, 49, 32, 49,
            48, 46, 51, 46, 49, 52, 49, 46, 50, 48, 57, 32, 52, 52, 51, 32, 51, 56, 54, 51, 52, 32,
            54, 32, 49, 48, 51, 32, 52, 50, 57, 50, 54, 32, 49, 54, 56, 48, 56, 51, 56, 53, 53, 54,
            32, 49, 54, 56, 48, 56, 51, 56, 53, 55, 56, 32, 65, 67, 67, 69, 80, 84, 32, 79, 75, 34,
            125, 10,
        ];
        let result = decode_and_decompress_to_vec(encoded_data).expect("Failed to decode data");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_and_decompress_invalid_base64_string() {
        let encoded_data = "H4sIAAAAAAAC/ytJLS4BAAxGw7gNAAA&"; // Invalid base64 string
        let result = decode_and_decompress_to_string(encoded_data);
        assert!(
            result.is_err(),
            "Expected an error due to invalid base64 input"
        );
    }

    #[test]
    fn test_decode_and_decompress_invalid_base64_vec() {
        let encoded_data = "H4sIAAAAAAAC/ytJLS4BAAxGw7gNAAA&"; // Invalid base64 string
        let result = decode_and_decompress_to_vec(encoded_data);
        assert!(
            result.is_err(),
            "Expected an error due to invalid base64 input"
        );
    }

    #[test]
    fn test_deserialize_from_str_metrics() {
        let encoded_data = "eyJtZXRyaWNfc3RyZWFtX25hbWUiOiJDdXN0b21QYXJ0aWFsLUJDbjVjQSIsImFjY291bnRfaWQiOiI3MzkxNDcyMjI5ODkiLCJyZWdpb24iOiJ1cy1lYXN0LTIiLCJuYW1lc3BhY2UiOiJBV1MvVXNhZ2UiLCJtZXRyaWNfbmFtZSI6IkNhbGxDb3VudCIsImRpbWVuc2lvbnMiOnsiQ2xhc3MiOiJOb25lIiwiUmVzb3VyY2UiOiJHZXRNZXRyaWNEYXRhIiwiU2VydmljZSI6IkNsb3VkV2F0Y2giLCJUeXBlIjoiQVBJIn0sInRpbWVzdGFtcCI6MTcxMzkwMjcwMDAwMCwidmFsdWUiOnsibWF4IjoxLjAsIm1pbiI6MS4wLCJzdW0iOjMuMCwiY291bnQiOjMuMH0sInVuaXQiOiJOb25lIn0KeyJtZXRyaWNfc3RyZWFtX25hbWUiOiJDdXN0b21QYXJ0aWFsLUJDbjVjQSIsImFjY291bnRfaWQiOiI3MzkxNDcyMjI5ODkiLCJyZWdpb24iOiJ1cy1lYXN0LTIiLCJuYW1lc3BhY2UiOiJBV1MvRmlyZWhvc2UiLCJtZXRyaWNfbmFtZSI6IktNU0tleUludmFsaWRTdGF0ZSIsImRpbWVuc2lvbnMiOnsiRGVsaXZlcnlTdHJlYW1OYW1lIjoiUFVULUhUUC1SZFFXOCJ9LCJ0aW1lc3RhbXAiOjE3MTM5MDI2NDAwMDAsInZhbHVlIjp7Im1heCI6MC4wLCJtaW4iOjAuMCwic3VtIjowLjAsImNvdW50Ijo2MC4wfSwidW5pdCI6IkNvdW50In0KeyJtZXRyaWNfc3RyZWFtX25hbWUiOiJDdXN0b21QYXJ0aWFsLUJDbjVjQSIsImFjY291bnRfaWQiOiI3MzkxNDcyMjI5ODkiLCJyZWdpb24iOiJ1cy1lYXN0LTIiLCJuYW1lc3BhY2UiOiJBV1MvRmlyZWhvc2UiLCJtZXRyaWNfbmFtZSI6IktNU0tleU5vdEZvdW5kIiwiZGltZW5zaW9ucyI6eyJEZWxpdmVyeVN0cmVhbU5hbWUiOiJQVVQtSFRQLVJkUVc4In0sInRpbWVzdGFtcCI6MTcxMzkwMjY0MDAwMCwidmFsdWUiOnsibWF4IjowLjAsIm1pbiI6MC4wLCJzdW0iOjAuMCwiY291bnQiOjYwLjB9LCJ1bml0IjoiQ291bnQifQo=";
        let decoded = decode_and_decompress_to_vec(encoded_data);
        assert!(decoded.is_ok());
        let decoded = decoded.unwrap();
        let request_id = "test_id".to_string();
        let result = deserialize_aws_record_from_vec(decoded, &request_id);
        assert!(result.is_ok());
        let value = result.unwrap();
        for val in value {
            assert_eq!(val.get("account_id").unwrap(), "739147222989");
        }
    }

    #[test]
    fn test_deserialize_from_str_logs() {
        let encoded_data = "eyJtZXNzYWdlVHlwZSI6IkRBVEFfTUVTU0FHRSIsIm93bmVyIjoiMTIzNDU2Nzg5MDEyIiwibG9nR3JvdXAiOiJsb2dfZ3JvdXBfbmFtZSIsImxvZ1N0cmVhbSI6ImxvZ19zdHJlYW1fbmFtZSIsInN1YnNjcmlwdGlvbkZpbHRlcnMiOlsic3Vic2NyaXB0aW9uX2ZpbHRlcl9uYW1lIl0sImxvZ0V2ZW50cyI6W3siaWQiOiIwMTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDEyMzQ1IiwidGltZXN0YW1wIjoxNzEzOTgzNDQ2LCJtZXNzYWdlIjoibG9nbWVzc2FnZTEifSx7ImlkIjoiMDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NSIsInRpbWVzdGFtcCI6IDE3MTM5ODM0NDYsIm1lc3NhZ2UiOiJsb2dtZXNzYWdlMiJ9XX0=";
        let decoded = decode_and_decompress_to_vec(encoded_data);
        assert!(decoded.is_ok());
        let decoded = decoded.unwrap();
        let request_id = "test_id".to_string();
        let result = deserialize_aws_record_from_vec(decoded, &request_id);
        assert!(result.is_ok());
        let result = result.unwrap();
        for val in result {
            assert_eq!(val.get("owner").unwrap(), "123456789012");
        }
    }

    #[test]
    fn test_var_int_header_empty_array() {
        let bytes = [];
        assert_eq!(get_size_of_var_int_header(&bytes), None);
    }

    #[test]
    fn test_var_int_header_no_valid_bytes() {
        let bytes = [0xFF; 100];
        assert_eq!(get_size_of_var_int_header(&bytes), None);
    }

    #[test]
    fn test_var_int_header() {
        let bytes: Vec<_> = (0..=u8::MAX).rev().collect();
        assert_eq!(get_size_of_var_int_header(&bytes), Some(129));
    }

    #[test]
    fn extract_resource_id_with_colon() {
        let arn = "arn:partition:service:region:account-id:resource-type:resource-id";
        assert_eq!(
            extract_resource_id_from_amazon_resource_number(arn),
            "resource-id"
        );
    }

    #[test]
    fn extract_resource_id_with_slash() {
        let arn = "arn:partition:service:region:account-id:resource-type/resource-id";
        assert_eq!(
            extract_resource_id_from_amazon_resource_number(arn),
            "resource-id"
        );
    }

    #[test]
    fn extract_resource_id_without_resource_type() {
        let arn = "arn:partition:service:region:account-id:resource-id";
        assert_eq!(
            extract_resource_id_from_amazon_resource_number(arn),
            "resource-id"
        );
    }
}
