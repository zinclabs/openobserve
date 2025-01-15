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

import { generateTraceContext, getWebSocketUrl } from "@/utils/zincutils";
import http from "./http";
import stream from "./stream";

const search = {
  search: (
    {
      org_identifier,
      query,
      page_type = "logs",
      traceparent,
      dashboard_id,
      folder_id,
    }: {
      org_identifier: string;
      query: any;
      page_type: string;
      traceparent?: string;
      dashboard_id?: string;
      folder_id?: string;
    },
    search_type: string = "UI",
  ) => {
    if (!traceparent) traceparent = generateTraceContext()?.traceparent;
    const use_cache: boolean =
      (window as any).use_cache !== undefined
        ? (window as any).use_cache
        : true;
    // const url = `/api/${org_identifier}/_search?type=${page_type}&search_type=${search_type}`;
    let url = `/api/${org_identifier}/_search?type=${page_type}&search_type=${search_type}&use_cache=${use_cache}`;
    if (dashboard_id) url += `&dashboard_id=${dashboard_id}`;
    if (folder_id) url += `&folder_id=${folder_id}`;
    if (typeof query.query.sql != "string") {
      url = `/api/${org_identifier}/_search_multi?type=${page_type}&search_type=${search_type}&use_cache=${use_cache}`;
      if (query.hasOwnProperty("aggs")) {
        return http({ headers: { traceparent } }).post(url, {
          ...query.query,
          aggs: query.aggs,
        });
      } else {
        return http({ headers: { traceparent } }).post(url, query.query);
      }
    }
    return http({ headers: { traceparent } }).post(url, query);
  },

  search_around: ({
    org_identifier,
    index,
    key,
    size,
    query_context,
    query_fn,
    stream_type,
    regions,
    clusters,
    is_multistream,
    traceparent,
  }: {
    org_identifier: string;
    index: string;
    key: string;
    size: string;
    query_context: any;
    query_fn: any;
    stream_type: string;
    regions: string;
    clusters: string;
    is_multistream: boolean;
    traceparent: string;
  }) => {
    // let url = `/api/${org_identifier}/${index}/_around?key=${key}&size=${size}&sql=${query_context}&type=${stream_type}`;
    let url: string = "";
    if (is_multistream) {
      url = `/api/${org_identifier}/${index}/_around_multi?key=${key}&size=${size}&sql=${query_context}&type=${stream_type}`;
    } else {
      url = `/api/${org_identifier}/${index}/_around?key=${key}&size=${size}&sql=${query_context}&type=${stream_type}`;
    }
    if (query_fn.trim() != "") {
      url = url + `&query_fn=${query_fn}`;
    }

    if (regions.trim() != "") {
      url = url + `&regions=${regions}`;
    }

    if (clusters.trim() != "") {
      url = url + `&clusters=${clusters}`;
    }
    return http({ headers: { traceparent } }).get(url);
  },
  metrics_query_range: ({
    org_identifier,
    query,
    start_time,
    end_time,
  }: {
    org_identifier: string;
    query: string;
    start_time: number;
    end_time: number;
  }) => {
    const url = `/api/${org_identifier}/prometheus/api/v1/query_range?start=${start_time}&end=${end_time}&step=0&query=${encodeURIComponent(
      query,
    )}`;
    return http().get(url);
  },
  metrics_query: ({
    org_identifier,
    query,
    start_time,
    end_time,
  }: {
    org_identifier: string;
    query: string;
    start_time: number;
    end_time: number;
  }) => {
    const url = `/api/${org_identifier}/prometheus/api/v1/query?time=${end_time}&query=${query}`;
    return http().get(url);
  },
  get_promql_series: ({
    org_identifier,
    labels,
    start_time,
    end_time,
  }: {
    org_identifier: string;
    labels: string;
    start_time: number;
    end_time: number;
  }) => {
    const url = `/api/${org_identifier}/prometheus/api/v1/series?match[]=${labels}&start=${start_time}&end=${end_time}`;
    return http().get(url);
  },

  get_traces: ({
    org_identifier,
    filter,
    start_time,
    end_time,
    from,
    size,
    stream_name,
  }: {
    org_identifier: string;
    filter: string;
    start_time: number;
    end_time: number;
    from: number;
    size: number;
    stream_name: string;
  }) => {
    const url = `/api/${org_identifier}/${stream_name}/traces/latest?filter=${filter}&start_time=${start_time}&end_time=${end_time}&from=${from}&size=${size}`;
    return http().get(url);
  },
  partition: ({
    org_identifier,
    query,
    page_type = "logs",
    traceparent,
  }: {
    org_identifier: string;
    query: any;
    page_type: string;
    traceparent: string;
  }) => {
    // const url = `/api/${org_identifier}/_search_partition?type=${page_type}`;

    let url = `/api/${org_identifier}/_search_partition?type=${page_type}`;
    if (typeof query.sql != "string") {
      url = `/api/${org_identifier}/_search_partition_multi?type=${page_type}`;
    }

    return http({
      headers: { traceparent },
    }).post(url, query);
  },
  get_running_queries: (org_identifier: string) => {
    const url = `/api/${org_identifier}/query_manager/status`;
    return http().get(url);
  },
  delete_running_queries: (org_identifier: string, traceIDs: string[]) => {
    const url = `/api/${org_identifier}/query_manager/cancel`;
    return http().put(url, traceIDs);
  },
  get_regions: () => {
    const url = `/api/clusters`;
    return http().get(url);
  },
  get_history: (org_identifier: string, startTime = null, endTime = null) => {
    const payload: any = {
      stream_type: "logs",
      org_identifier,
      user_email: null,
    };
    // Add startTime and endTime to the payload if provided
    if (startTime) {
      payload.start_time = startTime;
    }

    if (endTime) {
      payload.end_time = endTime;
    }

    return http().post(
      `/api/${org_identifier}/_search_history`,
      payload, // Send the payload as the request body
    );
  },
  schedule_search: (
    {
      org_identifier,
      query,
      page_type = "logs",
      traceparent,
    }: {
      org_identifier: string;
      query: any;
      page_type: string;
      traceparent?: string;
    },
    search_type: string = "UI",
  ) => {
    if (!traceparent) traceparent = generateTraceContext()?.traceparent;
    const use_cache: boolean =
      (window as any).use_cache !== undefined
        ? (window as any).use_cache
        : true;
    const url = `/api/${org_identifier}/search_jobs?type=${page_type}&search_type=${search_type}&use_cache=${use_cache}`;
    return http({ headers: { traceparent } }).post(url, query);
  },
  cancel_scheduled_search: (
    {
      org_identifier,
      jobId,
      traceparent,
    }: {
      org_identifier: string;
      jobId: string;
      traceparent?: string;
    },
    search_type: string = "UI",
  ) => {
    if (!traceparent) traceparent = generateTraceContext()?.traceparent;
    const use_cache: boolean =
      (window as any).use_cache !== undefined
        ? (window as any).use_cache
        : true;
    const url = `/api/${org_identifier}/search_jobs/${jobId}/cancel`;
    return http({ headers: { traceparent } }).post(url);
  },
  retry_scheduled_search: (
    {
      org_identifier,
      jobId,
      traceparent,
    }: {
      org_identifier: string;
      jobId: string;
      traceparent?: string;
    },
    search_type: string = "UI",
  ) => {
    if (!traceparent) traceparent = generateTraceContext()?.traceparent;
    const use_cache: boolean =
      (window as any).use_cache !== undefined
        ? (window as any).use_cache
        : true;
    const url = `/api/${org_identifier}/search_jobs/${jobId}/retry`;
    return http({ headers: { traceparent } }).post(url);
  },
  delete_scheduled_search: (
    {
      org_identifier,
      jobId,
      traceparent,
    }: {
      org_identifier: string;
      jobId: string;
      traceparent?: string;
    },
    search_type: string = "UI",
  ) => {
    if (!traceparent) traceparent = generateTraceContext()?.traceparent;
    const use_cache: boolean =
      (window as any).use_cache !== undefined
        ? (window as any).use_cache
        : true;
    const url = `/api/${org_identifier}/search_jobs/${jobId}`;
    return http({ headers: { traceparent } }).delete(url);
  },
  get_scheduled_search_list: (
    {
      org_identifier,
      page_type = "logs",
      traceparent,
    }: {
      org_identifier: string;
      page_type: string;
      traceparent?: string;
    },
    search_type: string = "UI",
  ) => {
    if (!traceparent) traceparent = generateTraceContext()?.traceparent;
    const use_cache: boolean =
      (window as any).use_cache !== undefined
        ? (window as any).use_cache
        : true;
    const url = `/api/${org_identifier}/search_jobs?type=${page_type}&search_type=${search_type}&use_cache=${use_cache}`;
    return http({ headers: { traceparent } }).get(url);
  },
  get_scheduled_search_result: (
    {
      org_identifier,
      page_type = "logs",
      jobId,
      traceparent,
      query,
    }: {
      org_identifier: string;
      jobId: string;
      page_type: string;
      traceparent?: string;
      query: any;
    },
    search_type: string = "UI",
  ) => {
    if (!traceparent) traceparent = generateTraceContext()?.traceparent;
    const { size, from } = query.query;
    const use_cache: boolean =
      (window as any).use_cache !== undefined
        ? (window as any).use_cache
        : true;
        let url = `/api/${org_identifier}/search_jobs/${jobId}/result?type=${page_type}&search_type=${search_type}&use_cache=${use_cache}`;
          url += `&size=${size}&from=${from}`;

    return http({ headers: { traceparent } }).get(url);
  },
};

export default search;
