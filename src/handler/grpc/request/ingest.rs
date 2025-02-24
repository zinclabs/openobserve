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

use actix_web::http::StatusCode;
use config::{
    meta::{otlp::OtlpRequestType, stream::StreamType},
    metrics,
    utils::json,
};
use proto::cluster_rpc::{
    ingest_server::Ingest, IngestionRequest, IngestionResponse, IngestionType,
};
use tonic::{Request, Response, Status};

use crate::service::ingestion::create_log_ingestion_req;

#[derive(Default)]
pub struct Ingester;

#[tonic::async_trait]
impl Ingest for Ingester {
    async fn ingest(
        &self,
        request: Request<IngestionRequest>,
    ) -> Result<Response<IngestionResponse>, Status> {
        let start = std::time::Instant::now();
        let req = request.into_inner();
        let org_id = req.org_id;
        let stream_type: StreamType = req.stream_type.into();
        let stream_name = req.stream_name;
        let in_data = req.data.unwrap_or_default();

        let resp = match stream_type {
            StreamType::Logs => {
                let log_ingestion_type = req.ingestion_type.unwrap_or_default();
                let data = bytes::Bytes::from(in_data.data);
                match create_log_ingestion_req(log_ingestion_type, &data) {
                    Err(e) => Err(e),
                    Ok(ingestion_req) => crate::service::logs::ingest::ingest(
                        0,
                        &org_id,
                        &stream_name,
                        ingestion_req,
                        "",
                        None,
                    )
                    .await
                    .map_or_else(Err, |_| Ok(())),
                }
            }
            StreamType::Metrics => {
                let log_ingestion_type: IngestionType = req
                    .ingestion_type
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or(IngestionType::Multi); // multi is just place holder
                if log_ingestion_type != IngestionType::Json {
                    Err(anyhow::anyhow!(
                        "Internal gPRC metric ingestion only supports json type data, got {:?}",
                        log_ingestion_type
                    ))
                } else {
                    let data = bytes::Bytes::from(in_data.data);
                    crate::service::metrics::json::ingest(&org_id, data)
                        .await
                        .map(|_| ()) // we don't care about success response
                        .map_err(|e| anyhow::anyhow!("error in ingesting metrics {}", e))
                }
            }
            StreamType::Traces => {
                let log_ingestion_type: IngestionType = req
                    .ingestion_type
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or(IngestionType::Multi); // multi is just place holder
                if log_ingestion_type != IngestionType::Json {
                    Err(anyhow::anyhow!(
                        "Internal gRPC trace ingestion only supports json type data, got {:?}",
                        log_ingestion_type
                    ))
                } else {
                    let data = bytes::Bytes::from(in_data.data);
                    crate::service::traces::ingest_json(&org_id, data, OtlpRequestType::Grpc, &stream_name)
                        .await
                        .map(|_| ()) // we don't care about success response
                        .map_err(|e| anyhow::anyhow!("error in ingesting traces {}", e))
                }
            }
            StreamType::EnrichmentTables => {
                let json_records: Vec<json::Map<String, json::Value>> =
                    json::from_slice(&in_data.data).unwrap_or({
                        let vec_value: Vec<json::Value> = json::from_slice(&in_data.data).unwrap();
                        vec_value
                            .into_iter()
                            .filter_map(|v| match v {
                                json::Value::Object(map) => Some(map),
                                _ => None,
                            })
                            .collect()
                    });
                let append_data = match req.metadata {
                    Some(metadata) => metadata
                        .data
                        .get("append_data")
                        .and_then(|v| v.parse::<bool>().ok())
                        .unwrap_or(true),
                    None => true,
                };
                match crate::service::enrichment_table::save_enrichment_data(
                    &org_id,
                    &stream_name,
                    json_records,
                    append_data,
                )
                .await
                {
                    Err(e) => Err(anyhow::anyhow!(
                        "Internal gPRC ingestion service errors saving enrichment data: {}",
                        e.to_string()
                    )),
                    Ok(res) => {
                        if res.status() != StatusCode::OK {
                            let status: StatusCode = res.status();
                            log::error!(
                                "Internal gPRC ingestion service errors saving enrichment data: code: {}, body: {:?}",
                                status,
                                res.into_body()
                            );
                            Err(anyhow::anyhow!(
                                "Internal gPRC ingestion service errors saving enrichment data: http code {}",
                                status
                            ))
                        } else {
                            Ok(())
                        }
                    }
                }
            }
            _ => Err(anyhow::anyhow!(
                "Internal gPRC ingestion service currently only supports Logs and EnrichmentTables",
            )),
        };

        let reply = match resp {
            Ok(_) => IngestionResponse {
                status_code: 200,
                message: "OK".to_string(),
            },
            Err(err) => IngestionResponse {
                status_code: 500,
                message: err.to_string(),
            },
        };

        // metrics
        let time = start.elapsed().as_secs_f64();
        metrics::GRPC_RESPONSE_TIME
            .with_label_values(&["/ingest/inner", "200", "", "", ""])
            .observe(time);
        metrics::GRPC_INCOMING_REQUESTS
            .with_label_values(&["/ingest/inner", "200", "", "", ""])
            .inc();

        Ok(Response::new(reply))
    }
}
