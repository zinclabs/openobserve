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

pub mod search;
pub mod session;
pub mod sort;
pub mod utils;

use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use config::get_config;
use session::WsSession;
use utils::sessions_cache_utils;

#[get("{org_id}/ws/{request_id}")]
pub async fn websocket(
    path_params: web::Path<(String, String)>,
    req: HttpRequest,
    stream: web::Payload,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let cfg = get_config();

    if !cfg.websocket.enabled {
        log::info!(
            "[WS_HANDLER]: Node Role: {} Websocket is disabled",
            cfg.common.node_role
        );
        return Ok(HttpResponse::NotFound().body("WebSocket is disabled"));
    }

    let (org_id, request_id) = path_params.into_inner();

    let prefix = format!("{}/api/", get_config().common.base_uri);
    let path = req.path().strip_prefix(&prefix).unwrap().to_string();

    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    let ws_session = WsSession::new(session);
    sessions_cache_utils::insert_session(&request_id, ws_session);
    log::info!(
        "[WS_HANDLER]: Node Role: {} Got websocket request for request_id: {}",
        cfg.common.node_role,
        request_id,
    );

    // Spawn the handler
    actix_web::rt::spawn(session::run(msg_stream, user_id, request_id, org_id, path));

    Ok(res)
}

/// Initialize the job init for websocket
pub async fn init() -> Result<(), anyhow::Error> {
    // Run the garbage collector for websocket sessions
    sessions_cache_utils::run_gc_ws_sessions().await;
    Ok(())
}
