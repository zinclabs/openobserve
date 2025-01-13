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
import { TestFunctionPayload } from "@/ts/interfaces/function";

const jstransform = {
  list: (
    page_num: number,
    page_size: number,
    sort_by: string,
    desc: boolean,
    name: string,
    org_identifier: string,
  ) => {
    return http().get(
      `/api/${org_identifier}/functions?page_num=${page_num}&page_size=${page_size}&sort_by=${sort_by}&desc=${desc}&name=${name}`,
    );
  },
  create: (org_identifier: string, data: any) => {
    return http().post(`/api/${org_identifier}/functions`, data);
  },
  getAssociatedPipelines: (org_identifier: string, name: string) => {
    return http().get(`/api/${org_identifier}/functions/${name}`);
  },
  update: (org_identifier: string, data: any) => {
    return http().put(`/api/${org_identifier}/functions/${data.name}`, data);
  },
  delete: (org_identifier: string, transform_name: string, force?: boolean) => {
    const url = `/api/${org_identifier}/functions/${transform_name}`;
    const config = force ? { params: { force: true } } : {};
    return http().delete(url, config);
  },
  create_with_index: (
    org_identifier: string,
    stream_name: string,
    stream_type: string,
    data: any,
  ) => {
    return http().put(
      `/api/${org_identifier}/${stream_name}/functions/${data.name}?type=${stream_type}`,
      data,
    );
  },
  delete_stream_function: (
    org_identifier: string,
    stream_name: string,
    stream_type: string,
    transform_name: string,
  ) => {
    return http().delete(
      `/api/${org_identifier}/${stream_name}/functions/${transform_name}?type=${stream_type}`,
    );
  },
  stream_function: (
    org_identifier: string,
    stream_name: string,
    stream_type: string,
  ) => {
    return http().get(
      `/api/${org_identifier}/streams/${stream_name}/functions?type=${stream_type}`,
    );
  },
  apply_stream_function: (
    org_identifier: string,
    stream_name: string,
    stream_type: string,
    function_name: string,
    data: any,
  ) => {
    return http().put(
      `/api/${org_identifier}/streams/${stream_name}/functions/${function_name}?type=${stream_type}`,
      data,
    );
  },
  remove_stream_function: (
    org_identifier: string,
    stream_name: string,
    stream_type: string,
    function_name: string,
  ) => {
    return http().delete(
      `/api/${org_identifier}/streams/${stream_name}/functions/${function_name}?type=${stream_type}`,
    );
  },
  create_enrichment_table: (
    org_identifier: string,
    table_name: string,
    data: any,
    append: boolean,
    useJson:boolean
  ) => {
    if (useJson) {
      const jsonPayload = {
        file_link: data, // Pass the URL as 'file_link'
      };
  
      // JSON request
      return http().post(
        `/api/${org_identifier}/enrichment_tables/${table_name}?append=${append}`,
        jsonPayload,
      );
    } else {
      // Multipart request
      return http({ headers: { "Content-Type": "multipart/form-data" } }).post(
        `/api/${org_identifier}/enrichment_tables/${table_name}?append=${append}`,
        data,
      );
    }
  },
  test: (org_identifier: string, data: TestFunctionPayload) => {
    return http().post(`/api/${org_identifier}/functions/test`, data);
  },
};

export default jstransform;
