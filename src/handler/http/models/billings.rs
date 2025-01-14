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

//! These models define the schemas of HTTP request and response JSON bodies in
//! billings API endpoints.

use o2_enterprise::enterprise::cloud::{billings as cloud_billings, org_usage as cloud_org_usage};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// for search job pagination
#[derive(Debug, Deserialize)]
pub struct CheckoutSessionDetailRequestQuery {
    pub session_id: String,
    pub status: String,
    pub plan: String,
}

/// HTTP request body for `ListInvoices` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ListInvoicesResponseBody {
    pub invoices: Vec<cloud_billings::StripeInvoice>,
}

/// HTTP request body for `CreateQuotaThreshold` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ListSubscriptionResponseBody {
    pub subscription_type: String,
}

/// HTTP request body for `CreateQuotaThreshold` endpoint.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct GetQuotaThresholdResponseBody {
    pub data: OrgQuotaThreshold,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, ToSchema, Default)]
pub struct OrgQuotaThreshold {
    ingestion: i64,
    query: i64,
    pipeline_process: i64,
    rum_session: i64,
    dashboard: i64,
    metric: i64,
    trace: i64,
}

impl From<cloud_org_usage::OrgUsageRecord> for GetQuotaThresholdResponseBody {
    fn from(value: cloud_org_usage::OrgUsageRecord) -> Self {
        let data = OrgQuotaThreshold {
            ingestion: value.ingestion_size,
            query: value.query_size,
            pipeline_process: value.pipeline_process_size,
            rum_session: value.rum_session_size,
            dashboard: value.dashboard_size,
            metric: value.metric_size,
            trace: value.trace_size,
        };
        Self {
            data,
            message: "Organization monthly quota pulled successfully.".to_string(),
        }
    }
}

impl From<cloud_billings::CustomerBilling> for ListSubscriptionResponseBody {
    fn from(value: cloud_billings::CustomerBilling) -> Self {
        Self {
            subscription_type: value.subscription_type.to_string(),
        }
    }
}
