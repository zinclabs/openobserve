// Copyright 2023 OpenObserve Inc.
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

const config = {
  aws_mobile_analytics_app_id: "ab7e9321f83c45a8967ff3b9bd90e83a",
  aws_mobile_analytics_app_region: "us-west-2",
  openReplayKey: import.meta.env.VITE_OPENREPLAY_KEY,
  sentryDSN: import.meta.env.VITE_SENTRY_DSN,
  zincQuotaThreshold: import.meta.env.VITE_ZINC_QUOTA_THRESHOLD,
  enableAnalytics: import.meta.env.VITE_ZINC_ANALYTICS_ENABLED,
  isCloud: import.meta.env.VITE_OPENOBSERVE_CLOUD
    ? import.meta.env.VITE_OPENOBSERVE_CLOUD
    : "false",
  isEnterprise: import.meta.env.VITE_OPENOBSERVE_ENTERPRISE
    ? import.meta.env.VITE_OPENOBSERVE_ENTERPRISE
    : "false",
  freePlan: "Free-Plan-USD-Monthly",
  paidPlan: "professional-USD-Monthly",
  ooApplicationID: import.meta.env.VITE_OO_APP_ID,
  ooClientToken: import.meta.env.VITE_OO_CLIENT_TOKEN,
  ooSite: import.meta.env.VITE_OO_SITE,
  ooService: import.meta.env.VITE_OO_SERVICE,
  ooOrgIdentifier: import.meta.env.VITE_OO_ORG_IDENTIFIER,
  environment: import.meta.env.VITE_ENVIRONMENT,
  ddAPPID: import.meta.env.VITE_DD_APP_ID,
  ddClientToken: import.meta.env.VITE_DD_CLIENT_TOKEN,
  ddSite: import.meta.env.VITE_DD_SITE,
};

export default config;
