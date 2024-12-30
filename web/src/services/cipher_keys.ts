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

import http from "./http";

const cipherKeys = {
  // create: ({ org_identifier, template_name, data }: any) => {
  //   return http().post(`/api/${org_identifier}/alerts/templates`, data);
  // },
  // update: ({ org_identifier, template_name, data }: any) => {
  //   return http().put(
  //     `/api/${org_identifier}/alerts/templates/${encodeURIComponent(
  //       template_name
  //     )}`,
  //     data
  //   );
  // },
  list: (org_identifier: string) => {
    return http().get(`/api/${org_identifier}/cipher_keys/`);
  },
  // get_by_name: ({ org_identifier, template_name }: any) => {
  //   return http().get(
  //     `/api/${org_identifier}/alerts/templates/${encodeURIComponent(
  //       template_name
  //     )}`
  //   );
  // },
  // delete: ({ org_identifier, template_name }: any) => {
  //   return http().delete(
  //     `/api/${org_identifier}/alerts/templates/${encodeURIComponent(
  //       template_name
  //     )}`
  //   );
  // },
};

export default cipherKeys;
