// Copyright 2024 Zinc Labs Inc.
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

use config::CONFIG;
use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_server::TraceService, ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use tonic::{codegen::*, Response, Status};

use crate::service::traces::{
    flusher,
    flusher::{ExportRequest, WriteBufferFlusher},
};

pub struct TraceServer {
    pub flusher: Arc<WriteBufferFlusher>,
}

#[async_trait]
impl TraceService for TraceServer {
    async fn export(
        &self,
        request: tonic::Request<ExportTraceServiceRequest>,
    ) -> Result<tonic::Response<ExportTraceServiceResponse>, tonic::Status> {
        let metadata = request.metadata().clone();
        let msg = format!(
            "Please specify organization id with header key '{}' ",
            &CONFIG.grpc.org_header_key
        );
        if !metadata.contains_key(&CONFIG.grpc.org_header_key) {
            return Err(Status::invalid_argument(msg));
        }

        // let in_req = request.into_inner();
        let org_id = metadata.get(&CONFIG.grpc.org_header_key);
        if org_id.is_none() {
            println!("{:?}", org_id);
            return Err(Status::invalid_argument(msg));
        }

        let request = tonic::Request::new(ExportRequest::ExportTraceServiceRequest(
            request.into_inner(),
        ));
        match self.flusher.write(request).await {
            Ok(resp) => match resp {
                flusher::BufferedWriteResult::Success(_) => {
                    Ok(Response::new(ExportTraceServiceResponse {
                        partial_success: None,
                    }))
                }
                flusher::BufferedWriteResult::Error(e) => Err(Status::internal(e)),
            },
            Err(e) => {
                println!("{}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
}
