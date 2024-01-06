// Copyright 2023 Zinc Labs Inc.
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

pub const END_POINTS: [&str; 15] = [
    "functions",
    "dashboards",   // dashboard
    "folders",      // dashboard
    "templates",    // alert
    "destinations", // alert
    "alerts",       // alert
    "enrichment_tables",
    "settings",
    "organizations",
    "kv",
    "users",
    "schema",
    "delete_fields",
    "streams",
    "syslog-routes",
];


get /org_id/functions
get /org_id/functions/abcd
post /org_id/functions
put /org_id/functions/abcd
delete /org_id/functions/abcd
