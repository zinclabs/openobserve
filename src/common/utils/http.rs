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
    io::{Error, ErrorKind},
    net::{AddrParseError, IpAddr, SocketAddr},
};

use actix_web::{
    http::header::{HeaderMap, HeaderName},
    web::Query,
};
use config::meta::{
    search::{SearchEventContext, SearchEventType},
    stream::StreamType,
};
use opentelemetry::{global, propagation::Extractor, trace::TraceContextExt};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[inline(always)]
pub(crate) fn get_stream_type_from_request(
    query: &Query<HashMap<String, String>>,
) -> Option<StreamType> {
    query.get("type").map(|s| StreamType::from(s.as_str()))
}

#[inline(always)]
pub(crate) fn get_search_type_from_request(
    query: &Query<HashMap<String, String>>,
) -> Result<Option<SearchEventType>, Error> {
    let event_type = match query.get("search_type") {
        Some(s) => match SearchEventType::try_from(s.as_str()) {
            Ok(search_type) => Some(search_type),
            _ => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "'event_type' query param with value 'ui', 'dashboards', 'reports', 'alerts' , 'rum' or 'values' allowed",
                ));
            }
        },
        None => None,
    };

    Ok(event_type)
}

#[inline(always)]
pub(crate) fn get_search_event_context_from_request(
    search_event_type: &SearchEventType,
    query: &Query<HashMap<String, String>>,
) -> Option<SearchEventContext> {
    match search_event_type {
        SearchEventType::Dashboards => Some(SearchEventContext::with_dashboard(
            query.get("dashboard_id").map(String::from),
            query.get("dashboard_name").map(String::from),
            query.get("folder_id").map(String::from),
            query.get("folder_name").map(String::from),
        )),
        SearchEventType::Alerts => Some(SearchEventContext::with_alert(
            query.get("alert_key").map(String::from),
        )),
        SearchEventType::Reports => Some(SearchEventContext::with_report(
            query.get("report_id").map(String::from),
        )),
        _ => None,
    }
}

#[inline(always)]
pub(crate) fn get_use_cache_from_request(query: &Query<HashMap<String, String>>) -> bool {
    let Some(v) = query.get("use_cache") else {
        return true;
    };
    v.to_lowercase().as_str().parse::<bool>().unwrap_or(true)
}

#[inline(always)]
pub(crate) fn get_folder(query: &Query<HashMap<String, String>>) -> String {
    match query.get("folder") {
        Some(s) => s.to_string(),
        None => config::meta::folder::DEFAULT_FOLDER.to_owned(),
    }
}

#[inline(always)]
pub(crate) fn get_or_create_trace_id(headers: &HeaderMap, span: &tracing::Span) -> String {
    let cfg = config::get_config();
    if let Some(traceparent) = headers.get("traceparent") {
        if cfg.common.tracing_enabled || cfg.common.tracing_search_enabled {
            // OpenTelemetry is initialized -> can use propagator to get traceparent
            let ctx = global::get_text_map_propagator(|propagator| {
                propagator.extract(&RequestHeaderExtractor::new(headers))
            });
            let trace_id = ctx.span().span_context().trace_id().to_string();
            if !span.is_none() {
                span.set_parent(ctx);
            }
            trace_id
        } else {
            // manually parse trace_id
            if let Ok(traceparent_str) = traceparent.to_str() {
                let parts: Vec<&str> = traceparent_str.split('-').collect();
                if parts.len() >= 3 {
                    let trace_id = parts[1].to_string();
                    // If the trace-id value is invalid (for example if it contains non-allowed
                    // characters or all zeros), vendors MUST ignore the traceparent.
                    // https://www.w3.org/TR/trace-context/#traceparent-header
                    if trace_id.len() == 32 && !trace_id.chars().all(|c| c == '0') {
                        return trace_id;
                    }
                }
            }
            // If parsing fails or trace_id is invalid, generate a new one
            log::warn!("Failed to parse valid trace_id from received [Traceparent] header");
            config::ider::uuid()
        }
    } else if !span.is_none() {
        span.context().span().span_context().trace_id().to_string()
    } else {
        config::ider::uuid()
    }
}

/// This function can handle IPv4 and IPv6 addresses which may have port numbers appended
pub fn parse_ip_addr(ip_address: &str) -> Result<(IpAddr, Option<u16>), AddrParseError> {
    let mut port: Option<u16> = None;
    let ip = ip_address.parse::<IpAddr>().or_else(|_| {
        ip_address
            .parse::<SocketAddr>()
            .map(|sock_addr| {
                port = Some(sock_addr.port());
                sock_addr.ip()
            })
            .map_err(|e| {
                log::error!("Error parsing IP address: {}, {}", &ip_address, e);
                e
            })
    })?;

    Ok((ip, port))
}

// Extractor for request headers
pub struct RequestHeaderExtractor<'a> {
    headers: &'a HeaderMap,
}

impl<'a> RequestHeaderExtractor<'a> {
    pub fn new(headers: &'a HeaderMap) -> Self {
        RequestHeaderExtractor { headers }
    }
}

impl Extractor for RequestHeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        // Convert the key to a HeaderName, ignoring case
        HeaderName::try_from(key)
            .ok()
            .and_then(|header_name| self.headers.get(header_name))
            .and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(|header| header.as_str()).collect()
    }
}

pub fn get_work_group(work_group_set: Vec<Option<String>>) -> Option<String> {
    let work_groups = work_group_set.into_iter().flatten().collect::<HashSet<_>>();
    if work_groups.contains("long") {
        return Some("long".to_string());
    } else if work_groups.contains("short") {
        return Some("short".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stream_type_from_request() {
        let key = "type".to_string();

        let mut map: HashMap<String, String> = HashMap::default();
        map.insert(key.clone(), key.clone());

        let resp = get_stream_type_from_request(&Query(map.clone()));
        assert_eq!(resp, Some(StreamType::default()));

        map.insert(key.clone(), "LOGS".to_string());
        let resp = get_stream_type_from_request(&Query(map.clone()));
        assert_eq!(resp, Some(StreamType::Logs));

        map.insert(key.clone(), "METRICS".to_string());
        let resp = get_stream_type_from_request(&Query(map.clone()));
        assert_eq!(resp, Some(StreamType::Metrics));

        map.insert(key.clone(), "TRACES".to_string());
        let resp = get_stream_type_from_request(&Query(map.clone()));
        assert_eq!(resp, Some(StreamType::Traces));
    }

    /// Test logic for IP parsing
    #[test]
    fn test_ip_parsing() {
        let valid_addresses = vec![
            "127.0.0.1",
            "127.0.0.1:8080",
            "::1",
            "192.168.0.1:8080",
            "2001:0db8:85a3:0000:0000:8a2e:0370:7334",
            "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8080",
        ];

        let parsed_addresses: Vec<IpAddr> = valid_addresses
            .iter()
            .map(|ip_addr| parse_ip_addr(ip_addr).unwrap().0)
            .collect();

        assert!(parsed_addresses
            .iter()
            .zip(valid_addresses)
            .map(|(parsed, original)| original.contains(parsed.to_string().as_str()))
            .fold(true, |acc, x| { acc | x }));
    }
}
