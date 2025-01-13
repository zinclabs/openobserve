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

/* eslint-disable @typescript-eslint/no-explicit-any */
import http from "./http";
import config from "../aws-exports";

const organizations = {
  os_list: (
    page_num: number,
    page_size: number,
    sort_by: string,
    desc: boolean,
    name: string,
    org_identifier: string
  ) => {
    return http().get(
      `/api/organizations?page_num=${page_num}&page_size=${page_size}&sort_by=${sort_by}&desc=${desc}&name=${name}`
    );
  },
  list: (
    page_num: number,
    page_size: number,
    sort_by: string,
    desc: boolean,
    name: string
  ) => {
    return http().get(
      `/api/organizations?page_num=${page_num}&page_size=${page_size}&sort_by=${sort_by}&desc=${desc}&name=${name}`
    );
  },
  create: (data: any) => {
    return http().post("/api/organizations", data);
  },
  add_members: (data: any, orgIdentifier: string) => {
    return http().post(`api/${orgIdentifier}/invites`, data);
  },
  process_subscription: (s: string, action: string, orgIdentifier: string) => {
    return http().put(
      `api/${orgIdentifier}/member_subscription/${s}?action=${action}`, {}
    );
  },
  get_associated_members: (orgIdentifier: string) => {
    return http().get(`api/${orgIdentifier}/organizations/associated_members`);
  },
  update_member_role: (data: any, orgIdentifier: string) => {
    return http().put(`api/${orgIdentifier}/users/${data.email}`, data);
  },
  verify_identifier: (name: string) => {
    return http().get(`api/organizations/verify_identifier/${name}`);
  },
  get_organization_passcode: (orgIdentifier: string) => {
    return http().get(`/api/${orgIdentifier}/passcode`);
  },
  update_organization_passcode: (orgIdentifier: string) => {
    return http().put(`api/${orgIdentifier}/passcode`, {});
  },
  get_organization_summary: (orgIdentifier: string) => {
    return http().get(`/api/${orgIdentifier}/summary`);
  },
  get_organization_settings: (orgIdentifier: string) => {
    return http().get(`/api/${orgIdentifier}/settings`);
  },
  post_organization_settings: (orgIdentifier: string, data: any) => {
    return http().post(`/api/${orgIdentifier}/settings`, data);
  },
};

export default organizations;
