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

use std::io::Error;

use actix_web::{get, post, web, HttpResponse};
use config::meta::short_url::ShortenUrlResponse;

use crate::service::short_url::ShortUrl;

/// Shorten a URL
#[utoipa::path(
    post,
    context_path = "/short",
    request_body(
        content = ShortenUrlRequest,
        description = "The original URL to shorten",
        content_type = "application/json",
        example = json!({
            "original_url": "https://example.com/some/long/url"
        })
    ),
    responses(
        (
            status = 200,
            description = "Shortened URL",
            body = ShortenUrlResponse,
            content_type = "application/json",
            example = json!({
                "short_url": "http://localhost:5080/short/ddbffcea3ad44292"
            })
        ),
        (status = 400, description = "Invalid request", content_type = "application/json")
    ),
    tag = "Short Url"
)]
#[post("")]
pub async fn shorten(body: web::Bytes) -> Result<HttpResponse, Error> {
    let req: config::meta::short_url::ShortenUrlRequest = serde_json::from_slice(&body)?;
    let short_url_service = ShortUrl::new(None);
    let short_url = short_url_service.shorten(&req.original_url).await;
    let response = ShortenUrlResponse {
        short_url: short_url.clone(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Retrieve the original URL from a short_id
#[utoipa::path(
    get,
    context_path = "/short",
    params(
        ("short_id" = String, Path, description = "The short ID to retrieve the original URL")
    ),
    responses(
        (status = 302, description = "Redirect to the original URL", headers(
            ("Location" = String, description = "The original URL to which the client is redirected")
        )),
        (status = 404, description = "Short URL not found", content_type = "text/plain")
    ),
    tag = "Short Url"
)]
#[get("/{short_id}")]
pub async fn retrieve(short_id: web::Path<String>) -> Result<HttpResponse, Error> {
    let short_id = short_id.into_inner();
    let short_url_service = ShortUrl::new(None);
    let original_url = short_url_service.retrieve(&short_id).await;

    if let Some(url) = original_url {
        // Use MovedPermanently (301) or Found (302)
        let response = HttpResponse::Found()
            .append_header(("Location", url.clone()))
            .finish();
        Ok(response)
    } else {
        Ok(HttpResponse::NotFound().body("Short URL not found"))
    }
}
