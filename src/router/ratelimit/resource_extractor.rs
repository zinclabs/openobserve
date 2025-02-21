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

use actix_web::dev::ServiceRequest;
use futures_util::future::BoxFuture;

use crate::{
    common::utils::auth::extract_auth_str,
    handler::http::auth::validator::get_user_email_from_auth_str,
};

pub fn default_extractor(req: &ServiceRequest) -> BoxFuture<'_, String> {
    let auth_str = extract_auth_str(req.request());
    let local_path = req.path().to_string();
    let path = match local_path
        .strip_prefix(format!("{}/api/", config::get_config().common.base_uri).as_str())
    {
        Some(path) => path,
        None => &local_path,
    };

    let path_columns = path.split('/').collect::<Vec<&str>>();
    let (path, org_id) = (path.to_string(), path_columns[0].to_string());

    Box::pin(async move {
        let user_email = get_user_email_from_auth_str(&auth_str)
            .await
            .unwrap_or_default();
        format!("{}:{}:{}", org_id, user_email, path)
    })
}
