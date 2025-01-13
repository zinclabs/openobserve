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

use actix_web::HttpResponse;
use config::meta::{stream::StreamType, user::User};
use o2_enterprise::enterprise::openfga::meta::mapping::OFGA_MODELS;

use crate::{
    common::{
        meta::http::HttpResponse as MetaHttpResponse,
        utils::auth::{is_root_user, AuthExtractor},
    },
    service::users::get_user,
};

// Check permissions on stream
pub async fn check_stream_permissions(
    stream_name: &str,
    org_id: &str,
    user_id: &str,
    stream_type: &StreamType,
) -> Option<HttpResponse> {
    if !is_root_user(user_id) {
        let user: User = get_user(Some(org_id), user_id).await.unwrap();
        let stream_type_str = stream_type.to_string();

        if !crate::handler::http::auth::validator::check_permissions(
            user_id,
            AuthExtractor {
                auth: "".to_string(),
                method: "GET".to_string(),
                o2_type: format!(
                    "{}:{}",
                    OFGA_MODELS
                        .get(stream_type_str.as_str())
                        .map_or(stream_type_str.as_str(), |model| model.key),
                    stream_name
                ),
                org_id: org_id.to_string(),
                bypass_check: false,
                parent_id: "".to_string(),
            },
            user.role,
            user.is_external,
        )
        .await
        {
            return Some(MetaHttpResponse::forbidden("Unauthorized Access"));
        }
    }
    None
}
