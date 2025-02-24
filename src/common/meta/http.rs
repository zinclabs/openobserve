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

use actix_web::{http::StatusCode, HttpResponse as ActixHttpResponse};
use infra::errors;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// HTTP response
/// code 200 is success
/// code 400 is error
/// code 404 is not found
/// code 500 is internal server error
/// code 503 is service unavailable
/// code >= 1000 is custom error code
/// message is the message or error message
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HttpResponse {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ESResponse {
    pub took: u16,
    pub errors: bool,
    // pub items: Vec<Item>
}

impl HttpResponse {
    pub fn message(code: u16, message: String) -> Self {
        HttpResponse {
            code,
            message,
            error_detail: None,
            trace_id: None,
        }
    }

    pub fn error(code: u16, error: String) -> Self {
        HttpResponse {
            code,
            message: error,
            error_detail: None,
            trace_id: None,
        }
    }

    pub fn error_code(err: errors::ErrorCodes) -> Self {
        HttpResponse {
            code: err.get_code(),
            message: err.get_message(),
            error_detail: Some(err.get_error_detail()),
            trace_id: None,
        }
    }

    pub fn error_code_with_trace_id(err: errors::ErrorCodes, trace_id: Option<String>) -> Self {
        HttpResponse {
            code: err.get_code(),
            message: err.get_message(),
            error_detail: Some(err.get_error_detail()),
            trace_id,
        }
    }

    /// Send a normal response in json format and associate the
    /// provided message as `message` field.
    pub fn ok(msg: impl ToString) -> ActixHttpResponse {
        ActixHttpResponse::Ok().json(Self::message(StatusCode::OK.into(), msg.to_string()))
    }

    /// Send a BadRequest response in json format and associate the
    /// provided error as `error` field.
    pub fn bad_request(error: impl ToString) -> ActixHttpResponse {
        ActixHttpResponse::BadRequest().json(Self::error(
            StatusCode::BAD_REQUEST.into(),
            error.to_string(),
        ))
    }

    /// Send an Unauthorized response in json format and sets the
    /// provided error as the `error` field.
    pub fn unauthorized(error: impl ToString) -> ActixHttpResponse {
        ActixHttpResponse::Forbidden().json(Self::error(
            StatusCode::UNAUTHORIZED.into(),
            error.to_string(),
        ))
    }

    /// Send a Forbidden response in json format and associate the
    /// provided error as `error` field.
    pub fn forbidden(error: impl ToString) -> ActixHttpResponse {
        ActixHttpResponse::Forbidden()
            .json(Self::error(StatusCode::FORBIDDEN.into(), error.to_string()))
    }

    /// Send a Forbidden response in json format and associate the
    /// provided error as `error` field.
    pub fn conflict(error: impl ToString) -> ActixHttpResponse {
        ActixHttpResponse::Conflict()
            .json(Self::error(StatusCode::CONFLICT.into(), error.to_string()))
    }

    /// Send a NotFound response in json format and associate the
    /// provided error as `error` field.
    pub fn not_found(error: impl ToString) -> ActixHttpResponse {
        ActixHttpResponse::NotFound()
            .json(Self::error(StatusCode::NOT_FOUND.into(), error.to_string()))
    }

    /// Send a InternalServerError response in json format and associate the
    /// provided error as `error` field.
    pub fn internal_error(error: impl ToString) -> ActixHttpResponse {
        ActixHttpResponse::InternalServerError().json(Self::error(
            StatusCode::INTERNAL_SERVER_ERROR.into(),
            error.to_string(),
        ))
    }

    /// Send a response in json format, status code is 200.
    /// The payload should be serde-serializable.
    pub fn json(payload: impl Serialize) -> ActixHttpResponse {
        ActixHttpResponse::Ok().json(payload)
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http;

    use super::*;

    #[test]
    fn test_http_response() {
        let msg = "This is an error response";
        let err = HttpResponse::message(http::StatusCode::OK.into(), msg.to_string());
        assert_eq!(err.code, http::StatusCode::OK);
        assert_eq!(err.message, msg);

        let err = HttpResponse::error(
            http::StatusCode::INTERNAL_SERVER_ERROR.into(),
            msg.to_string(),
        );
        assert_eq!(err.code, http::StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.message, msg);

        let errcode = errors::ErrorCodes::ServerInternalError(msg.to_string());
        let err =
            HttpResponse::error_code(errors::ErrorCodes::ServerInternalError(msg.to_string()));
        assert_eq!(err.code, errcode.get_code());
        assert_eq!(err.message, errcode.get_message());
    }
}
