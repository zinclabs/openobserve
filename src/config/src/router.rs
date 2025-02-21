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

const QUERIER_ROUTES: [&str; 20] = [
    "/config",
    "/summary",
    "/organizations",
    "/settings",
    "/schema",
    "/streams",
    "/clusters",
    "/query_manager",
    "/ws",
    "/_search",
    "/_around",
    "/_values",
    "/functions?page_num=",
    "/prometheus/api/v1/series",
    "/prometheus/api/v1/query",
    "/prometheus/api/v1/query_range",
    "/prometheus/api/v1/query_exemplars",
    "/prometheus/api/v1/metadata",
    "/prometheus/api/v1/labels",
    "/prometheus/api/v1/label/",
];
const QUERIER_ROUTES_BY_BODY: [&str; 2] = [
    "/prometheus/api/v1/query_range",
    "/prometheus/api/v1/query_exemplars",
];
const FIXED_QUERIER_ROUTES: [&str; 3] = ["/summary", "/schema", "/streams"];

#[inline]
pub fn is_querier_route(path: &str) -> bool {
    QUERIER_ROUTES.iter().any(|x| path.contains(x))
}

#[inline]
pub fn is_querier_route_by_body(path: &str) -> bool {
    QUERIER_ROUTES_BY_BODY.iter().any(|x| path.contains(x))
}

#[inline]
pub fn is_fixed_querier_route(path: &str) -> bool {
    FIXED_QUERIER_ROUTES.iter().any(|x| path.contains(x))
}
