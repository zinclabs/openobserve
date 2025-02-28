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

import { reactive, computed, watch, onBeforeMount } from "vue";
import StreamService from "@/services/stream";
import { useStore } from "vuex";
import useNotifications from "./useNotifications";
import { splitQuotedString, escapeSingleQuotes } from "@/utils/zincutils";
import { extractFields } from "@/utils/query/sqlUtils";
import { validateSQLPanelFields } from "@/utils/dashboard/convertDataIntoUnitValue";
import {
  buildSQLQueryFromInput,
  buildSQLJoinsFromInput,
} from "@/utils/dashboard/convertDataIntoUnitValue";
import { buildSQLQueryWithParser } from "@/utils/query/sqlUtils";
import useStreams from "./useStreams";

const colors = [
  "#5960b2",
  "#c23531",
  "#2f4554",
  "#61a0a8",
  "#d48265",
  "#91c7ae",
  "#749f83",
  "#ca8622",
  "#bda29a",
  "#6e7074",
  "#546570",
  "#c4ccd3",
];
let parser: any;

const getDefaultDashboardPanelData: any = () => ({
  data: {
    version: 5,
    id: "",
    type: "bar",
    title: "",
    description: "",
    config: {
      trellis: {
        layout: null,
        num_of_columns: 1,
      },
      show_legends: true,
      legends_position: null,
      unit: null,
      unit_custom: null,
      decimals: 2,
      line_thickness: 1.5,
      step_value: "0",
      y_axis_min: null,
      y_axis_max: null,
      top_results: null,
      top_results_others: false,
      axis_width: null,
      axis_border_show: false,
      label_option: {
        position: null,
        rotate: 0,
      },
      show_symbol: false,
      line_interpolation: "smooth",
      legend_width: {
        value: null,
        unit: "px",
      },
      base_map: {
        type: "osm",
      },
      map_type: {
        type: "world",
      },
      map_view: {
        zoom: 1,
        lat: 0,
        lng: 0,
      },
      map_symbol_style: {
        size: "by Value",
        size_by_value: {
          min: 1,
          max: 100,
        },
        size_fixed: 2,
      },
      drilldown: [],
      mark_line: [],
      override_config: [],
      connect_nulls: false,
      no_value_replacement: "",
      wrap_table_cells: false,
      table_transpose: false,
      table_dynamic_columns: false,
      color: {
        mode: "palette-classic-by-series",
        fixedColor: ["#53ca53"],
        seriesBy: "last",
      },
    },
    htmlContent: "",
    markdownContent: "",
    customChartContent: `\ // To know more about ECharts , \n// visit: https://echarts.apache.org/examples/en/index.html \n// Example: https://echarts.apache.org/examples/en/editor.html?c=line-simple \n// Define your ECharts 'option' here. \n// 'data' variable is available for use and contains the response data from the search result and it is an array.\noption = {  \n \n};
  `,
    customChartResult: {},
    queryType: "sql",
    queries: [
      {
        query: "",
        vrlFunctionQuery: "",
        customQuery: false,
        joins: [],
        fields: {
          stream: "",
          stream_type: "logs",
          x: [],
          y: [],
          z: [],
          breakdown: [],
          filter: {
            filterType: "group",
            logicalOperator: "AND",
            conditions: [],
          },
          latitude: null,
          longitude: null,
          weight: null,
          name: null,
          value_for_maps: null,
          source: null,
          target: null,
          value: null,
        },
        config: {
          promql_legend: "",
          layer_type: "scatter",
          weight_fixed: 1,
          limit: 0,
          // gauge min and max values
          min: 0,
          max: 100,
          time_shift: [],
        },
      },
    ],
  },
  layout: {
    splitter: 20,
    querySplitter: 41,
    showQueryBar: false,
    isConfigPanelOpen: false,
    currentQueryIndex: 0,
    vrlFunctionToggle: false,
    showFieldList: true,
  },
  meta: {
    parsedQuery: "",
    dragAndDrop: {
      dragging: false,
      dragElement: null,
      dragSource: null,
      dragSourceIndex: null,
      currentDragArea: null,
      targetDragIndex: null,
    },
    errors: {
      queryErrors: [],
    },
    editorValue: "",
    dateTime: { start_time: "", end_time: "" },
    filterValue: <any>[],
    stream: {
      hasUserDefinedSchemas: false,
      interestingFieldList: [],
      userDefinedSchema: [],
      vrlFunctionFieldList: [],
      selectedStreamFields: [],
      useUserDefinedSchemas: "user_defined_schema",
      customQueryFields: [],
      functions: [],
      streamResults: <any>[],
      streamResultsType: "",
      filterField: "",
    },
    streamFields: {
      groupedFields: [],
    },
  },
});

const dashboardPanelDataObj: any = {
  dashboard: reactive({ ...getDefaultDashboardPanelData() }),
};

const getDefaultCustomChartText = () => {
  return `\ // To know more about ECharts , \n// visit: https://echarts.apache.org/examples/en/index.html \n// Example: https://echarts.apache.org/examples/en/editor.html?c=line-simple \n// Define your ECharts 'option' here. \n// 'data' variable is available for use and contains the response data from the search result and it is an array.\noption = {  \n \n};
  `;
};

const useDashboardPanelData = (pageKey: string = "dashboard") => {
  const store = useStore();
  const { showErrorNotification } = useNotifications();
  const { getStreams, getStream } = useStreams();

  // Initialize the state for this page key if it doesn't already exist
  if (!dashboardPanelDataObj[pageKey]) {
    dashboardPanelDataObj[pageKey] = reactive({
      ...getDefaultDashboardPanelData(),
    });
  }

  const dashboardPanelData = reactive(dashboardPanelDataObj[pageKey]);
  const cleanupDraggingFields = () => {
    dashboardPanelData.meta.dragAndDrop.currentDragArea = null;
    dashboardPanelData.meta.dragAndDrop.targetDragIndex = -1;
    dashboardPanelData.meta.dragAndDrop.dragging = false;
    dashboardPanelData.meta.dragAndDrop.dragElement = null;
    dashboardPanelData.meta.dragAndDrop.dragSource = null;
    dashboardPanelData.meta.dragAndDrop.dragSourceIndex = null;
  };

  // get default queries
  const getDefaultQueries = () => {
    return getDefaultDashboardPanelData().data.queries;
  };

  const addQuery = () => {
    const queryType =
      dashboardPanelData.data.queryType === "sql" ? "sql" : "promql";
    const newQuery: any = {
      query: "",
      vrlFunctionQuery: "",
      customQuery: queryType === "promql",
      fields: {
        stream:
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream,
        stream_type:
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream_type,
        x: [],
        y: [],
        z: [],
        breakdown: [],
        filter: {
          filterType: "group",
          logicalOperator: "AND",
          conditions: [],
        },
        latitude: null,
        longitude: null,
        weight: null,
        name: null,
        value_for_maps: null,
        source: null,
        target: null,
        value: null,
      },
      config: {
        promql_legend: "",
        layer_type: "scatter",
        weight_fixed: 1,
      },
    };
    dashboardPanelData.data.queries.push(newQuery);
  };

  const removeQuery = (index: number) => {
    dashboardPanelData.data.queries.splice(index, 1);
  };

  const resetDashboardPanelData = () => {
    Object.assign(dashboardPanelData, getDefaultDashboardPanelData());
  };

  const resetDashboardPanelDataAndAddTimeField = () => {
    resetDashboardPanelData();

    // add _timestamp field in x axis as default
    addXAxisItem({
      name: store.state.zoConfig.timestamp_column ?? "_timestamp",
    });
  };

  const generateLabelFromName = (name: string) => {
    return name
      .replace(/[\_\-\s\.]/g, " ")
      .split(" ")
      .map((string) => string.charAt(0).toUpperCase() + string.slice(1))
      .filter((it) => it)
      .join(" ");
  };

  const promqlMode = computed(
    () => dashboardPanelData.data.queryType == "promql",
  );

  const selectedStreamFieldsBasedOnUserDefinedSchema = computed(() => {
    if (
      store.state.zoConfig.user_defined_schemas_enabled &&
      dashboardPanelData.meta.stream.userDefinedSchema.length > 0 &&
      dashboardPanelData.meta.stream.useUserDefinedSchemas ==
        "user_defined_schema"
    ) {
      return dashboardPanelData.meta.stream.userDefinedSchema ?? [];
    }

    return dashboardPanelData.meta.stream.selectedStreamFields ?? [];
  });

  async function loadStreamFields(streamName: string) {
    try {
      if (!streamName) return;

      // Create a new request and store it in the cache
      return await getStream(
        streamName,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream_type ?? "logs",
        true,
      );
    } catch (e: any) {
      console.error("Error while loading stream fields", e);
    }
  }

  // Helper function to update grouped fields
  const updateGroupedFields = async () => {
    const currentStream =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.stream;
    if (!currentStream) return;

    // Collect streams (main + joins)
    const joinsStreams = [
      { stream: currentStream },
      ...(dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].joins?.filter((stream: any) => stream?.stream) ?? []),
    ];

    // Fetch stream fields
    const groupedFields = await Promise.all(
      joinsStreams.map(async (stream: any) => {
        return {
          ...(await loadStreamFields(stream?.stream)),
          stream_alias: stream?.streamAlias,
        };
      }),
    );

    dashboardPanelData.meta.streamFields.groupedFields = groupedFields;
  };

  const isAddXAxisNotAllowed = computed((e: any) => {
    switch (dashboardPanelData.data.type) {
      case "pie":
      case "donut":
      case "heatmap":
      case "gauge":
      case "area":
      case "bar":
      case "h-bar":
      case "line":
      case "scatter":
      case "area-stacked":
      case "stacked":
      case "h-stacked":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.x.length >= 1
        );
      case "metric":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.x.length >= 0
        );
      case "table":
        return false;
      default:
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.x.length >= 2
        );
    }
  });

  const isAddBreakdownNotAllowed = computed((e: any) => {
    switch (dashboardPanelData.data.type) {
      case "area":
      case "bar":
      case "h-bar":
      case "line":
      case "scatter":
      case "area-stacked":
      case "stacked":
      case "h-stacked":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.breakdown?.length >= 1
        );
    }
  });

  const isAddYAxisNotAllowed = computed((e: any) => {
    switch (dashboardPanelData.data.type) {
      case "pie":
      case "donut":
      case "gauge":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.y.length >= 1
        );
      case "metric":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.y.length >= 1
        );
      case "area-stacked":
      case "stacked":
      case "heatmap":
      case "h-stacked":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.y.length >= 1
        );
      default:
        return false;
    }
  });

  const isAddZAxisNotAllowed = computed((e: any) => {
    switch (dashboardPanelData.data.type) {
      case "heatmap":
        return (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.z.length >= 1
        );
      default:
        return false;
    }
  });

  const checkIsDerivedField = (fieldName: string) => {
    // if given fieldName is from vrlFunctionFields, then it is a derived field
    return !!dashboardPanelData.meta.stream.vrlFunctionFieldList.find(
      (vrlField: any) => vrlField.name == fieldName,
    );
  };

  const addXAxisItem = (row: { name: string; streamAlias?: string }) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x = [];
    }

    if (isAddXAxisNotAllowed.value) {
      return;
    }

    const isDerived = checkIsDerivedField(row.name) ?? false;

    // check for existing field
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.x.find((it: any) => it.column == row.name)
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.x.push({
      label: !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery
        ? generateLabelFromName(row.name)
        : row.name,
      alias:
        !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery && !isDerived
          ? "x_axis_" +
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.x.length +
              1)
          : row.name,
      // column: row.name,
      color: null,
      type: "build",
      functionName:
        row.name == store.state.zoConfig.timestamp_column && !isDerived
          ? "histogram"
          : null,
      args:
        row.name == store.state.zoConfig.timestamp_column && !isDerived
          ? [
              {
                type: "field",
                value: {
                  field: row.name,
                  streamAlias: row.streamAlias,
                },
              },
              {
                type: "histogramInterval",
                value: null,
              },
            ]
          : [
              {
                type: "field",
                value: {
                  field: row.name,
                  streamAlias: row.streamAlias,
                },
              },
            ],
      sortBy:
        row.name == store.state.zoConfig.timestamp_column
          ? dashboardPanelData.data.type == "table"
            ? "DESC"
            : "ASC"
          : null,
      isDerived,
    });

    // }

    updateArrayAlias();
  };

  const addBreakDownAxisItem = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.breakdown
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.breakdown = [];
    }

    if (isAddBreakdownNotAllowed.value) {
      return;
    }

    const isDerived = checkIsDerivedField(row.name) ?? false;

    // check for existing field
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.breakdown.find((it: any) => it.column == row.name)
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.breakdown.push({
      label: !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery
        ? generateLabelFromName(row.name)
        : row.name,
      alias:
        !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery && !isDerived
          ? "breakdown_" +
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.breakdown.length +
              1)
          : row.name,
      // column: row.name,
      color: null,
      type: "build",
      functionName:
        row.name == store.state.zoConfig.timestamp_column && !isDerived
          ? "histogram"
          : null,
      args:
        row.name == store.state.zoConfig.timestamp_column && !isDerived
          ? [
              {
                type: "field",
                value: {
                  field: row.name,
                  streamAlias: row.streamAlias,
                },
              },
              {
                type: "histogramInterval",
                value: null,
              },
            ]
          : [
              {
                type: "field",
                value: {
                  field: row.name,
                  streamAlias: row.streamAlias,
                },
              },
            ],
      sortBy:
        row.name == store.state.zoConfig.timestamp_column
          ? dashboardPanelData.data.type == "table"
            ? "DESC"
            : "ASC"
          : null,
      isDerived,
    });
    // }

    updateArrayAlias();
  };

  const addYAxisItem = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y = [];
    }

    if (isAddYAxisNotAllowed.value) {
      return;
    }

    const isDerived = checkIsDerivedField(row.name) ?? false;

    // HERE NEED CHANGES
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.y.find((it: any) => it?.column == row.name)
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.y.push({
      label: !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery
        ? generateLabelFromName(row.name)
        : row.name,
      alias:
        !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery && !isDerived
          ? "y_axis_" +
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.y.length +
              1)
          : row.name,
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName:
        dashboardPanelData.data.type == "heatmap" || isDerived ? null : "count",
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
      isDerived,
    });
    // }
    updateArrayAlias();
  };

  const addZAxisItem = (row: any) => {
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z = [];
    }

    if (isAddZAxisNotAllowed.value) {
      return;
    }

    const isDerived = checkIsDerivedField(row.name) ?? false;

    // HERE NEED CHANGES
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.z.find((it: any) => it.column == row.name)
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.z.push({
      label: !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery
        ? generateLabelFromName(row.name)
        : row.name,
      alias:
        !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery && !isDerived
          ? "z_axis_" +
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.z.length +
              1)
          : row.name,
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: isDerived ? null : "count",
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
      isDerived,
    });
    // }
    updateArrayAlias();
  };

  const addLatitude = (row: any) => {
    const isDerived = checkIsDerivedField(row.name) ?? false;
    // HERE NEED CHANGES
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.latitude
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.latitude = {
      label: generateLabelFromName(row.name),
      alias: isDerived ? row.name : "latitude",
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: null, // You can set the appropriate aggregation function here
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
      isDerived,
    };
    // }
  };

  const addLongitude = (row: any) => {
    const isDerived = checkIsDerivedField(row.name) ?? false;
    // HERE NEED CHANGES
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.longitude
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.longitude = {
      label: generateLabelFromName(row.name),
      alias: isDerived ? row.name : "longitude",
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: null, // You can set the appropriate aggregation function here
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
      isDerived,
    };
    // }
  };

  const addWeight = (row: any) => {
    const isDerived = checkIsDerivedField(row.name) ?? false;
    // HERE NEED CHANGES
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.weight
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.weight = {
      label: generateLabelFromName(row.name),
      alias: isDerived ? row.name : "weight",
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: isDerived ? null : "count", // You can set the appropriate aggregation function here
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
      isDerived,
    };
    // }
  };

  const addMapName = (row: any) => {
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.name
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.name = {
      label: generateLabelFromName(row.name),
      alias: "name",
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: null, // You can set the appropriate aggregation function here
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
    };
    // }
  };

  const addMapValue = (row: any) => {
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.value_for_maps
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.value_for_maps = {
      label: generateLabelFromName(row.name),
      alias: "value_for_maps",
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: "count", // You can set the appropriate aggregation function here
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
    };
    // }
  };

  const addSource = (row: any) => {
    const isDerived = checkIsDerivedField(row.name) ?? false;
    // HERE NEED CHANGES
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.source
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.source = {
      label: generateLabelFromName(row.name),
      alias: isDerived ? row.name : "source",
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: null, // You can set the appropriate aggregation function here
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
      isDerived,
    };
    // }
  };

  const addTarget = (row: any) => {
    const isDerived = checkIsDerivedField(row.name) ?? false;
    // HERE NEED CHANGES
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.target
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.target = {
      label: generateLabelFromName(row.name),
      alias: isDerived ? row.name : "target",
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: null, // You can set the appropriate aggregation function here
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
      isDerived,
    };
    // }
  };

  const addValue = (row: any) => {
    const isDerived = checkIsDerivedField(row.name) ?? false;
    // HERE NEED CHANGES
    // if (
    //   !dashboardPanelData.data.queries[
    //     dashboardPanelData.layout.currentQueryIndex
    //   ].fields.value
    // ) {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.value = {
      label: generateLabelFromName(row.name),
      alias: isDerived ? row.name : "value",
      // column: row.name,
      color: getNewColorValue(),
      type: "build",
      functionName: isDerived ? null : "sum", // You can set the appropriate aggregation function here
      args: [
        {
          type: "field",
          value: {
            field: row.name,
            streamAlias: row.streamAlias,
          },
        },
      ],
      isDerived,
    };
    // }
  };

  // get new color value based on existing color from the chart
  const getNewColorValue = () => {
    const YAxisColor = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.y.map((it: any) => it?.color);
    let newColor = colors.filter((el: any) => !YAxisColor.includes(el));
    if (!newColor.length) {
      newColor = colors;
    }
    return newColor[0];
  };

  const resetAggregationFunction = () => {
    switch (dashboardPanelData.data.type) {
      case "heatmap":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y.forEach((itemY: any) => {
          itemY.functionName = null;
          // take first arg
          itemY.args = itemY.args.length ? [itemY?.args?.[0]] : [];
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.breakdown = [];
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.latitude = null;
          query.fields.longitude = null;
          query.fields.weight = null;
          query.fields.name = null;
          query.fields.value_for_maps = null;
          query.fields.source = null;
          query.fields.target = null;
          query.fields.value = null;
        });
        if (dashboardPanelData.data.queryType === "sql") {
          dashboardPanelData.layout.currentQueryIndex = 0;
          dashboardPanelData.data.queries =
            dashboardPanelData.data.queries.slice(0, 1);
        }
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.customChartContent =
          getDefaultCustomChartText();
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift = [];
        break;

      case "area":
      case "area-stacked":
      case "bar":
      case "line":
      case "scatter":
      case "h-bar":
      case "stacked":
      case "h-stacked":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y.forEach((itemY: any) => {
          if (itemY.functionName === null && !itemY.isDerived) {
            itemY.functionName = "count";
            // take first arg
            itemY.args = itemY.args.length ? [itemY?.args?.[0]] : [];
          }
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z = [];
        // we have multiple queries for geomap, so if we are moving away, we need to reset
        // the values of lat, lng and weight in all the queries
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.latitude = null;
          query.fields.longitude = null;
          query.fields.weight = null;
          query.fields.name = null;
          query.fields.value_for_maps = null;
          query.fields.source = null;
          query.fields.target = null;
          query.fields.value = null;
        });
        if (dashboardPanelData.data.queryType === "sql") {
          dashboardPanelData.layout.currentQueryIndex = 0;
          dashboardPanelData.data.queries =
            dashboardPanelData.data.queries.slice(0, 1);
        }
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.customChartContent =
          getDefaultCustomChartText();
        break;
      case "table":
      case "pie":
      case "donut":
      case "metric":
      case "gauge":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y.forEach((itemY: any) => {
          if (itemY.functionName === null && !itemY.isDerived) {
            itemY.functionName = "count";
            // take first arg
            itemY.args = itemY.args.length ? [itemY?.args?.[0]] : [];
          }
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.breakdown = [];
        // we have multiple queries for geomap, so if we are moving away, we need to reset
        // the values of lat, lng and weight in all the queries
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.latitude = null;
          query.fields.longitude = null;
          query.fields.weight = null;
          query.fields.name = null;
          query.fields.value_for_maps = null;
          query.fields.source = null;
          query.fields.target = null;
          query.fields.value = null;
        });
        if (dashboardPanelData.data.queryType === "sql") {
          dashboardPanelData.layout.currentQueryIndex = 0;
          dashboardPanelData.data.queries =
            dashboardPanelData.data.queries.slice(0, 1);
        }
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.customChartContent =
          getDefaultCustomChartText();

        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift = [];
        break;
      case "geomap":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z = [];
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.name = null;
          query.fields.value_for_maps = null;
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.breakdown = [];
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.customChartContent =
          getDefaultCustomChartText();

        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.source = null;
          query.fields.target = null;
          query.fields.value = null;
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.limit = 0;
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift = [];
        break;
      case "html":
        dashboardPanelData.data.queries = getDefaultQueries();
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.customChartContent =
          getDefaultCustomChartText();
        dashboardPanelData.data.queryType = "";
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift = [];
        break;
      case "markdown":
        dashboardPanelData.data.queries = getDefaultQueries();
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.customChartContent =
          getDefaultCustomChartText();

        dashboardPanelData.data.queryType = "";
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift = [];
        break;
      case "custom_chart":
        dashboardPanelData.data.queries = getDefaultQueries();
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.queryType = "";
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift = [];
        break;
      case "maps":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.breakdown = [];
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.latitude = null;
          query.fields.longitude = null;
          query.fields.weight = null;
          query.fields.source = null;
          query.fields.target = null;
          query.fields.value = null;
        });
        break;
      case "sankey":
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.breakdown = [];
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.filter = {
          filterType: "group",
          logicalOperator: "AND",
          conditions: [],
        };
        dashboardPanelData.data.htmlContent = "";
        dashboardPanelData.data.markdownContent = "";
        dashboardPanelData.data.customChartContent =
          getDefaultCustomChartText();
        dashboardPanelData.data.queries?.forEach((query: any) => {
          query.fields.latitude = null;
          query.fields.longitude = null;
          query.fields.weight = null;
          query.fields.name = null;
          query.fields.value_for_maps = null;
        });
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.limit = 0;
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift = [];
      default:
        break;
    }

    // aggregation null then count
  };

  // update X or Y axis aliases when new value pushes into the X and Y axes arrays
  const updateArrayAlias = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields?.x?.forEach(
      (it: any, index: any) =>
        (it.alias =
          !dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery && !it.isDerived
            ? "x_axis_" + (index + 1)
            : it?.column),
    );
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields?.y?.forEach(
      (it: any, index: any) =>
        (it.alias =
          !dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery && !it.isDerived
            ? "y_axis_" + (index + 1)
            : it?.column),
    );
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields?.z?.forEach(
      (it: any, index: any) =>
        (it.alias =
          !dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery && !it.isDerived
            ? "z_axis_" + (index + 1)
            : it?.column),
    );
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields?.breakdown?.forEach(
      (it: any, index: any) =>
        (it.alias =
          !dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery && !it.isDerived
            ? "breakdown_" + (index + 1)
            : it?.column),
    );
  };

  const removeXAxisItem = (name: string) => {
    // HERE NEED CHANGES
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.x.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x.splice(index, 1);
    }
  };

  const removeBreakdownItem = (name: string) => {
    // HERE NEED CHANGES
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.breakdown.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.breakdown.splice(index, 1);
    }
  };

  const removeYAxisItem = (name: string) => {
    // HERE NEED CHANGES
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.y.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y.splice(index, 1);
    }
  };

  const removeZAxisItem = (name: string) => {
    // HERE NEED CHANGES
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.z.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z.splice(index, 1);
    }
  };

  const removeFilterItem = (name: string) => {
    const index = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.filter.conditions.findIndex((it: any) => it.column == name);
    if (index >= 0) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter.conditions.splice(index, 1);
    }
  };

  const removeLatitude = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.latitude = null;
  };

  const removeLongitude = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.longitude = null;
  };

  const removeWeight = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.weight = null;
  };

  const removeMapName = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.name = null;
  };

  const removeMapValue = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.value_for_maps = null;
  };

  const removeSource = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.source = null;
  };

  const removeTarget = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.target = null;
  };

  const removeValue = () => {
    dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.value = null;
  };

  const addFilteredItem = async (name: string) => {
    const currentQuery =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ];

    // Ensure the filter array is initialized
    if (!currentQuery.fields.filter) {
      currentQuery.fields.filter = {
        filterType: "group",
        logicalOperator: "AND",
        conditions: [],
      };
    }

    // Add the new filter item
    currentQuery.fields.filter.conditions.push({
      type: "list",
      values: [],
      column: name,
      operator: null,
      value: null,
      logicalOperator: "AND",
      filterType: "condition",
    });

    // Ensure the filterValue array is initialized
    if (!dashboardPanelData.meta.filterValue) {
      dashboardPanelData.meta.filterValue = [];
    }

    try {
      const res = await StreamService.fieldValues({
        org_identifier: store.state.selectedOrganization.identifier,
        stream_name: currentQuery.fields.stream,
        start_time: new Date(
          dashboardPanelData.meta.dateTime["start_time"].toISOString(),
        ).getTime(),
        end_time: new Date(
          dashboardPanelData.meta.dateTime["end_time"].toISOString(),
        ).getTime(),
        fields: [name],
        size: 100,
        type: currentQuery.fields.stream_type,
        no_count: true,
      });

      dashboardPanelData.meta.filterValue.push({
        column: name,
        value: res?.data?.hits?.[0]?.values
          .map((it: any) => it.zo_sql_key)
          .filter((it: any) => it),
      });
    } catch (error: any) {
      const errorDetailValue =
        error.response?.data.error_detail ||
        error.response?.data.message ||
        "Something went wrong!";
      const trimmedErrorMessage =
        errorDetailValue.length > 300
          ? errorDetailValue.slice(0, 300) + " ..."
          : errorDetailValue;

      showErrorNotification(trimmedErrorMessage);
    }
  };

  const loadFilterItem = (name: any) => {
    StreamService.fieldValues({
      org_identifier: store.state.selectedOrganization.identifier,
      stream_name:
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream,
      start_time: new Date(
        dashboardPanelData?.meta?.dateTime?.["start_time"]?.toISOString(),
      ).getTime(),
      end_time: new Date(
        dashboardPanelData?.meta?.dateTime?.["end_time"]?.toISOString(),
      ).getTime(),
      fields: [name],
      size: 100,
      type: dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.stream_type,
      no_count: true,
    })
      .then((res: any) => {
        const find = dashboardPanelData.meta.filterValue.findIndex(
          (it: any) => it.column == name,
        );
        if (find >= 0) {
          dashboardPanelData.meta.filterValue.splice(find, 1);
        }
        dashboardPanelData.meta.filterValue.push({
          column: name,
          value: res?.data?.hits?.[0]?.values
            .map((it: any) => it.zo_sql_key)
            .filter((it: any) => it)
            .map((it: any) => String(it)),
        });
      })
      .catch((error: any) => {
        const errorDetailValue =
          error.response?.data.error_detail ||
          error.response?.data.message ||
          "Something went wrong!";
        const trimmedErrorMessage =
          errorDetailValue.length > 300
            ? errorDetailValue.slice(0, 300) + " ..."
            : errorDetailValue;

        showErrorNotification(trimmedErrorMessage);
      });
  };

  const removeXYFilters = () => {
    if (
      promqlMode.value ||
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery == false
    ) {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x.length,
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y.length,
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z.length,
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields?.breakdown?.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields?.breakdown?.length,
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter.conditions.splice(
        0,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.filter.conditions.length,
      );
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.latitude = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.longitude = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.weight = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.name = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.value_for_maps = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.source = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.target = null;
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.value = null;
    }
  };

  // This function updates the x and y fields of a custom query in the dashboard panel data
  const updateXYFieldsForCustomQueryMode = () => {
    // Check if the custom query is enabled and PromQL mode is disabled
    if (
      !promqlMode.value &&
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery == true
    ) {
      // first, remove all derived fields from x,y,z,latitude,longitude,weight,source,target,value
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x = dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields?.x?.filter((it: any) => !it.isDerived);

      // remove from y axis
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y = dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields?.y?.filter((it: any) => !it.isDerived);

      // remove from z axis
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z = dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields?.z?.filter((it: any) => !it.isDerived);

      // remove from breakdown
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.breakdown = dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields?.breakdown?.filter((it: any) => !it.isDerived);

      // remove from latitude
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.latitude?.alias &&
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.latitude?.isDerived
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.latitude = null;
      }

      // remove from longitude
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.longitude?.alias &&
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.longitude?.isDerived
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.longitude = null;
      }

      // remove from weight
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.weight?.alias &&
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.weight?.isDerived
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.weight = null;
      }

      // remove from source
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.source?.alias &&
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.source?.isDerived
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.source = null;
      }

      // remove from target
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.target?.alias &&
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.target?.isDerived
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.target = null;
      }

      // remove from value
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.value?.alias &&
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.value?.isDerived
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.value = null;
      }

      // remove from name
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.name?.alias &&
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.name?.isDerived
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.name = null;
      }

      // remove from value_for_maps
      if (
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.value_for_maps?.alias &&
        dashboardPanelData?.data?.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.value_for_maps?.isDerived
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.value_for_maps = null;
      }

      // Loop through each custom query field in the dashboard panel data's stream meta
      dashboardPanelData.meta.stream.customQueryFields.forEach(
        (it: any, index: number) => {
          // Get the name of the current custom query field
          const { name } = it;

          // Determine the current field type based on the name
          let field;
          if (name === "latitude") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.latitude;
          } else if (name === "longitude") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.longitude;
          } else if (name === "weight") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.weight;
          } else if (name === "name") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.name;
          } else if (name === "value_for_maps") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.value_for_maps;
          } else if (name === "source") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.source;
          } else if (name === "target") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.target;
          } else if (name === "value") {
            field =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.value;
          } else {
            // For other field types (x, y, z), determine the type and index as before
            let currentFieldType;

            if (
              index <
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.x.length
            ) {
              currentFieldType = "x";
            } else if (
              index <
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.x.length +
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.y.length
            ) {
              currentFieldType = "y";
            } else if (
              index <
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.x.length +
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.y.length +
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.breakdown.length
            ) {
              currentFieldType = "breakdown";
            } else {
              currentFieldType = "z";
            }

            if (currentFieldType === "x") {
              field =
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.x[index];
            } else if (currentFieldType === "y") {
              field =
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.y[
                  index -
                    dashboardPanelData.data.queries[
                      dashboardPanelData.layout.currentQueryIndex
                    ].fields.x.length
                ];
            } else if (currentFieldType === "breakdown") {
              field =
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.breakdown[
                  index -
                    dashboardPanelData.data.queries[
                      dashboardPanelData.layout.currentQueryIndex
                    ].fields.x.length -
                    dashboardPanelData.data.queries[
                      dashboardPanelData.layout.currentQueryIndex
                    ].fields.y.length
                ];
            } else {
              field =
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.z[
                  index -
                    dashboardPanelData.data.queries[
                      dashboardPanelData.layout.currentQueryIndex
                    ].fields.x.length -
                    dashboardPanelData.data.queries[
                      dashboardPanelData.layout.currentQueryIndex
                    ].fields.y.length
                ];
            }
            // If the current field is a y or z field, set the aggregation function to "count"
            if (
              (currentFieldType === "y" || currentFieldType === "z") &&
              !field.isDerived
            ) {
              field.functionName = "count";
              // take first arg
              field.args = field.args.length ? [field?.args?.[0]] : [];
            }
          }

          // Update the properties of the current field
          field.alias = name; // Set the alias to the name of the custom query field
          field.column = name; // Set the column to the name of the custom query field
          field.color = null; // Reset the color to null
        },
      );
    }
  };

  // this updates the fields when you switch from the auto to custom
  const updateXYFieldsOnCustomQueryChange = (oldCustomQueryFields: any) => {
    // Create a copy of the old custom query fields array
    const oldArray = oldCustomQueryFields;
    // Create a deep copy of the new custom query fields array
    const newArray = JSON.parse(
      JSON.stringify(dashboardPanelData.meta.stream.customQueryFields),
    );

    // Check if the length of the old and new arrays are the same
    if (oldArray.length == newArray.length) {
      // Create an array to store the indexes of changed fields
      const changedIndex: any = [];
      // Iterate through the new array
      newArray.forEach((obj: any, index: any) => {
        const { name } = obj;
        // Check if the name of the field at the same index in the old array is different
        if (oldArray[index].name != name) {
          changedIndex.push(index);
        }
      });
      // Check if there is only one changed field
      if (changedIndex.length == 1) {
        const oldName = oldArray[changedIndex[0]]?.name;

        let fieldIndex = dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x?.findIndex((it: any) => it?.alias == oldName);
        // Check if the field is in the x fields array
        if (fieldIndex >= 0) {
          const newName = newArray[changedIndex[0]]?.name;
          const field =
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.x[fieldIndex];

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }
        // Check if the field is in the breakdown fields array
        fieldIndex = dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.breakdown?.findIndex((it: any) => it?.alias == oldName);
        if (fieldIndex >= 0) {
          const newName = newArray[changedIndex[0]]?.name;
          const field =
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.breakdown[fieldIndex];

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }
        // Check if the field is in the y fields array
        fieldIndex = dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y?.findIndex((it: any) => it?.alias == oldName);
        if (fieldIndex >= 0) {
          const newName = newArray[changedIndex[0]]?.name;
          const field =
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.y[fieldIndex];

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }
        // Check if the field is in the z fields array
        fieldIndex = dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.z?.findIndex((it: any) => it?.alias == oldName);
        if (fieldIndex >= 0) {
          const newName = newArray[changedIndex[0]]?.name;
          const field =
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.z[fieldIndex];

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the latitude fields
        let field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.latitude;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the longitude fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.longitude;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the weight fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.weight;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the name fields
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.name;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the value fields
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.value_for_maps;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the source fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.source;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the target fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.target;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          field.alias = newName;
          field.column = newName;
        }

        //Check if the field is in the value fields array
        field =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.value;

        if (field && field.alias == oldName) {
          const newName = newArray[changedIndex[0]]?.name;

          // Update the field alias and column to the new name
          // HERE NEED CHANGES
          field.alias = newName;
          field.column = newName;
        }
      }
    }
  };

  onBeforeMount(async () => {
    await importSqlParser();
  });

  const importSqlParser = async () => {
    const useSqlParser: any = await import("@/composables/useParser");
    const { sqlParser }: any = useSqlParser.default();
    parser = await sqlParser();

    updateQueryValue();
  };

  /**
   * Format a value to be used in a SQL query.
   * @param value - the value to format
   * @returns the formatted value
   */
  const formatValue = (value: any): string | null => {
    if (value == null) {
      // if value is null or undefined, return it as is
      return value;
    }

    // if value is a string, remove any single quotes and add double quotes
    let tempValue = value;
    if (value?.length > 1 && value.startsWith("'") && value.endsWith("'")) {
      tempValue = value.substring(1, value.length - 1);
    }
    // escape any single quotes in the value
    tempValue = escapeSingleQuotes(tempValue);
    // add double quotes around the value
    tempValue = `'${tempValue}'`;
    return tempValue;
  };

  /**
   * Format a value for an IN clause in a SQL query.
   * If the value contains a variable, e.g. $variable, it will be returned as is.
   * Otherwise, if the value is a string, it will be split into individual values
   * using the `splitQuotedString` util function. Each value will be escaped and
   * enclosed in single quotes, and the resulting array of strings will be joined
   * with commas.
   * @param value - the value to format
   * @returns the formatted value
   */
  const formatINValue = (value: any) => {
    // if variable is present, don't want to use splitQuotedString
    if (value?.includes("$")) {
      if (value.startsWith("(") && value.endsWith(")")) {
        return value.substring(1, value.length - 1);
      }
      return value;
    } else {
      return splitQuotedString(value)
        ?.map((it: any) => {
          return `'${escapeSingleQuotes(it)}'`;
        })
        .join(", ");
    }
  };

  /**
   * Build a WHERE clause from the given filter data.
   * @param {array} filterData - an array of filter objects, each with properties
   *   for column, operator, value, and logicalOperator.
   * @returns {string} - the WHERE clause as a string.
   */
  const buildWhereClause = (filterData: any) => {
    /**
     * Build a single condition from the given condition object.
     * @param {object} condition - a filter object with properties for column,
     *   operator, value, and logicalOperator.
     * @returns {string} - the condition as a string.
     */
    const buildCondition = (condition: any) => {
      if (condition.filterType === "group") {
        const groupConditions = condition.conditions
          .map(buildCondition)
          .filter(Boolean);
        const logicalOperators = condition.conditions
          .map((c: any) => c.logicalOperator)
          .filter(Boolean);

        let groupQuery = "";
        groupConditions.forEach((cond: any, index: any) => {
          if (index > 0) {
            groupQuery += ` ${logicalOperators[index]} `;
          }
          groupQuery += cond;
        });

        return groupConditions.length ? `(${groupQuery})` : "";
      } else if (condition.type === "list" && condition.values?.length > 0) {
        return `${condition.column} IN (${condition.values
          .map((value: any) => formatValue(value))
          .join(", ")})`;
      } else if (condition.type === "condition" && condition.operator != null) {
        let selectFilter = "";
        if (["Is Null", "Is Not Null"].includes(condition.operator)) {
          selectFilter += `${condition.column} `;
          switch (condition.operator) {
            case "Is Null":
              selectFilter += `IS NULL`;
              break;
            case "Is Not Null":
              selectFilter += `IS NOT NULL`;
              break;
          }
        } else if (condition.operator === "IN") {
          selectFilter += `${condition.column} IN (${formatINValue(
            condition.value,
          )})`;
        } else if (condition.operator === "match_all") {
          selectFilter += `match_all(${formatValue(condition.value)})`;
        } else if (condition.operator === "match_all_raw") {
          selectFilter += `match_all_raw(${formatValue(condition.value)})`;
        } else if (condition.operator === "match_all_raw_ignore_case") {
          selectFilter += `match_all_raw_ignore_case(${formatValue(
            condition.value,
          )})`;
        } else if (condition.operator === "str_match") {
          selectFilter += `str_match(${condition.column}, ${formatValue(
            condition.value,
          )})`;
        } else if (condition.operator === "str_match_ignore_case") {
          selectFilter += `str_match_ignore_case(${
            condition.column
          }, ${formatValue(condition.value)})`;
        } else if (condition.operator === "re_match") {
          selectFilter += `re_match(${condition.column}, ${formatValue(
            condition.value,
          )})`;
        } else if (condition.operator === "re_not_match") {
          selectFilter += `re_not_match(${condition.column}, ${formatValue(
            condition.value,
          )})`;
        } else if (condition.value != null && condition.value !== "") {
          selectFilter += `${condition.column} `;
          switch (condition.operator) {
            case "=":
            case "<>":
            case "<":
            case ">":
            case "<=":
            case ">=":
              selectFilter += `${condition.operator} ${formatValue(
                condition.value,
              )}`;
              break;
            case "Contains":
              selectFilter += `LIKE '%${condition.value}%'`;
              break;
            case "Not Contains":
              selectFilter += `NOT LIKE '%${condition.value}%'`;
              break;
            default:
              selectFilter += `${condition.operator} ${formatValue(
                condition.value,
              )}`;
              break;
          }
        }
        return selectFilter;
      }
      return "";
    };

    const whereConditions = filterData.map(buildCondition).filter(Boolean);

    const logicalOperators = filterData.map((it: any) => it.logicalOperator);

    if (whereConditions.length > 0) {
      return ` WHERE ${whereConditions
        .map((cond: any, index: any) => {
          const logicalOperator =
            index < logicalOperators.length && logicalOperators[index + 1]
              ? logicalOperators[index + 1]
              : "";

          return index < logicalOperators.length
            ? `${cond} ${logicalOperator}`
            : cond;
        })
        .join(" ")}`;
    }

    return "";
  };

  const sqlchart = async () => {
    // STEP 1: first check if there is at least 1 field selected
    if (
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x.length == 0 &&
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y.length == 0 &&
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.z.length == 0 &&
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.breakdown?.length == 0
    ) {
      return "";
    }

    // STEP 2: Now, continue if we have at least 1 field selected

    // To build the SQL query
    // 1. SELECT
    // 2. FROM & Join query
    // 3. Filter
    // 4. Group by
    // 5. Order by
    // 6. Limit

    let query = "SELECT ";
    // 1. SELECT
    // merge the fields list
    const fields = [
      ...dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.x,
      ...dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.y,
      ...(dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields?.breakdown
        ? [
            ...dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.breakdown,
          ]
        : []),
      ...(dashboardPanelData.data?.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields?.z
        ? [
            ...dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.z,
          ]
        : []),
    ]
      .flat()
      .filter((fieldObj: any) => !fieldObj.isDerived);

    const array = fields.map((field, i) => {
      let selector = "";

      selector += buildSQLQueryFromInput(
        field,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields?.stream,
      );
      // } else {
      //   selector += `${field?.column}`;
      // }

      selector += ` as "${field?.alias}"${i == fields.length - 1 ? " " : ", "}`;
      return selector;
    });
    query += array?.join("");

    // 2. Stream, Join query
    // now add from stream name
    query += ` FROM "${
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields?.stream
    }" ${buildSQLJoinsFromInput(
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].joins,
    )}`;

    // 3. Filter
    // Add the AND/OR condition logic
    const filterData =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter.conditions;

    const whereClause = buildWhereClause(filterData);
    query += whereClause;

    // 4. Group by
    // add group by statement
    const xAxisAlias = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.x
      .filter((it: any) => !it?.isDerived)
      .map((it: any) => it?.alias);

    const yAxisAlias = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields.y
      .filter((it: any) => !it?.isDerived)
      .map((it: any) => it?.alias);

    const bAxisAlias = dashboardPanelData.data.queries[
      dashboardPanelData.layout.currentQueryIndex
    ].fields?.breakdown
      ?.filter((it: any) => !it?.isDerived)
      ?.map((it: any) => it?.alias);

    const tableTypeWithXFieldOnly =
      dashboardPanelData.data.type === "table" &&
      xAxisAlias.length > 0 &&
      yAxisAlias.length === 0 &&
      !bAxisAlias?.length;

    if (!tableTypeWithXFieldOnly) {
      if (dashboardPanelData.data.type == "heatmap") {
        query +=
          xAxisAlias.length && yAxisAlias.length
            ? " GROUP BY " +
              xAxisAlias.join(", ") +
              ", " +
              yAxisAlias.join(", ")
            : "";
      } else if (bAxisAlias?.length) {
        query +=
          xAxisAlias.length && bAxisAlias.length
            ? " GROUP BY " +
              xAxisAlias.join(", ") +
              ", " +
              bAxisAlias.join(", ")
            : "";
      } else {
        query += xAxisAlias.length ? " GROUP BY " + xAxisAlias.join(", ") : "";
      }
    }

    // 5. Order by
    // array of sorting fields with followed by asc or desc
    const orderByArr: string[] = [];

    fields.forEach((it: any) => {
      // ignore if None is selected or sortBy is not there
      if (it?.sortBy) {
        orderByArr.push(`${it?.alias} ${it?.sortBy}`);
      }
    });

    // append with query by joining array with comma
    query += orderByArr.length ? " ORDER BY " + orderByArr.join(", ") : "";

    // 6. Limit
    // append limit
    // if limit is less than or equal to 0 then don't add
    const queryLimit =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].config.limit ?? 0;
    query += queryLimit > 0 ? " LIMIT " + queryLimit : "";

    return query;
  };

  const mapChart = () => {
    const { name, value_for_maps } =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields;

    // Validate required fields
    if (!name?.column) {
      console.warn("Map name field is required but not provided");
      return "";
    }
    if (!value_for_maps?.column) {
      console.warn("Map value field is required but not provided");
      return "";
    }
    let query = "";

    if (name && value_for_maps) {
      // query = `SELECT ${name.column} as "${name.alias}", `;
      query = `SELECT ${buildSQLQueryFromInput(
        name,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields?.stream,
      )} as "${name.alias}",  `;

      if (value_for_maps?.functionName) {
        // switch (value_for_maps.functionName) {
        //   case "p50":
        //     query += `approx_percentile_cont(${value_for_maps.column}, 0.5) as ${value_for_maps.alias}`;
        //     break;
        //   case "p90":
        //     query += `approx_percentile_cont(${value_for_maps.column}, 0.9) as ${value_for_maps.alias}`;
        //     break;
        //   case "p95":
        //     query += `approx_percentile_cont(${value_for_maps.column}, 0.95) as ${value_for_maps.alias}`;
        //     break;
        //   case "p99":
        //     query += `approx_percentile_cont(${value_for_maps.column}, 0.99) as ${value_for_maps.alias}`;
        //     break;
        //   case "count-distinct":
        //     query += `count(distinct(${value_for_maps.column})) as "${value_for_maps.alias}"`;
        //     break;
        //   default:
        //     query += `${value_for_maps.functionName}(${value_for_maps.column}) as "${value_for_maps.alias}"`;
        //     break;
        // }

        query += `${buildSQLQueryFromInput(
          value_for_maps,
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields?.stream,
        )} as "${value_for_maps.alias}"`;
      } else {
        // query += `${value_for_maps.column} as "${value_for_maps.alias}"`;
        query += `${buildSQLQueryFromInput(
          value_for_maps,
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields?.stream,
        )} as "${value_for_maps.alias}"`;
      }

      query += ` FROM "${
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream
      }"`;

      // Add WHERE clause based on applied filters
      const filterData =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.filter.conditions;

      const whereClause = buildWhereClause(filterData);
      query += whereClause;

      // Group By clause
      if (name) {
        query += ` GROUP BY ${name.alias}`;
      }
    }

    // array of sorting fields with followed by asc or desc
    const orderByArr: string[] = [];

    [name, value_for_maps]
      .filter((it: any) => !it?.isDerived)
      .forEach((it: any) => {
        // ignore if None is selected or sortBy is not there
        if (it?.sortBy) {
          orderByArr.push(`${it.alias} ${it.sortBy}`);
        }
      });

    // append with query by joining array with comma
    query += orderByArr.length ? " ORDER BY " + orderByArr.join(", ") : "";

    // append limit
    // if limit is less than or equal to 0 then don't add
    const queryLimit =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].config.limit ?? 0;
    query += queryLimit > 0 ? " LIMIT " + queryLimit : "";

    return query;
  };

  const geoMapChart = () => {
    let query = "";

    const { latitude, longitude, weight } =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields;

    if (latitude && !latitude.isDerived && longitude && !longitude.isDerived) {
      query += `SELECT ${buildSQLQueryFromInput(
        latitude,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields?.stream,
      )} as ${latitude.alias}, ${buildSQLQueryFromInput(
        longitude,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields?.stream,
      )} as ${longitude.alias}`;
      // query += `SELECT ${latitude.column} as ${latitude.alias}, ${longitude.column} as ${longitude.alias}`;
    } else if (latitude && !latitude.isDerived) {
      query += `SELECT ${buildSQLQueryFromInput(
        latitude,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields?.stream,
      )} as ${latitude.alias}`;
      // query += `SELECT ${latitude.column} as ${latitude.alias}`;
    } else if (longitude && !longitude.isDerived) {
      query += `SELECT ${buildSQLQueryFromInput(
        longitude,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields?.stream,
      )} as ${longitude.alias}`;
      // query += `SELECT ${longitude.column} as ${longitude.alias}`;
    }

    if (query) {
      if (weight && !weight.isDerived) {
        // switch (weight?.functionName) {
        //   case "p50":
        //     query += `, approx_percentile_cont(${weight.column}, 0.5) as ${weight.alias}`;
        //     break;
        //   case "p90":
        //     query += `, approx_percentile_cont(${weight.column}, 0.9) as ${weight.alias}`;
        //     break;
        //   case "p95":
        //     query += `, approx_percentile_cont(${weight.column}, 0.95) as ${weight.alias}`;
        //     break;
        //   case "p99":
        //     query += `, approx_percentile_cont(${weight.column}, 0.99) as ${weight.alias}`;
        //     break;
        //   case "count-distinct":
        //     query += `, count(distinct(${weight.column})) as ${weight.alias}`;
        //     break;
        //   default:
        //     query += `, ${weight.functionName}(${weight.column}) as ${weight.alias}`;
        //     break;
        // }
        query += `, ${buildSQLQueryFromInput(
          weight,
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields?.stream,
        )} as ${weight.alias}`;
      }
      query += ` FROM "${
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream
      }" `;
    }

    // Add WHERE clause based on applied filters
    const filterData =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].fields.filter.conditions;

    const whereClause = buildWhereClause(filterData);
    query += whereClause;

    // Group By clause
    let aliases: any = [];

    if (latitude && !latitude.isDerived) {
      aliases.push(latitude?.alias);
    }
    if (longitude && !longitude.isDerived) {
      aliases.push(longitude?.alias);
    }
    if (aliases.length) {
      aliases = aliases.filter(Boolean).join(", ");
      query += `GROUP BY ${aliases}`;
    }

    // array of sorting fields with followed by asc or desc
    const orderByArr: string[] = [];

    [latitude, longitude, weight]
      .filter((it: any) => !it?.isDerived)
      .forEach((it: any) => {
        // ignore if None is selected or sortBy is not there
        if (it?.sortBy) {
          orderByArr.push(`${it.alias} ${it.sortBy}`);
        }
      });

    // append with query by joining array with comma
    query += orderByArr.length ? " ORDER BY " + orderByArr.join(", ") : "";

    // append limit
    // if limit is less than or equal to 0 then don't add
    const queryLimit =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].config.limit ?? 0;
    query += queryLimit > 0 ? " LIMIT " + queryLimit : "";

    return query;
  };

  const sankeyChartQuery = () => {
    const queryData =
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ];
    const { source, target, value } = queryData.fields;
    const stream = queryData.fields.stream;

    if (!source && !target && !value) {
      return "";
    }

    let query = "SELECT ";
    const selectFields = [];

    if (source && !source.isDerived) {
      // selectFields.push(`${source.column} as ${source.alias}`);
      selectFields.push(
        `${buildSQLQueryFromInput(
          source,
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields?.stream,
        )} as ${source.alias}`,
      );
    }

    if (target && !target.isDerived) {
      // selectFields.push(`${target.column} as ${target.alias}`);
      selectFields.push(
        `${buildSQLQueryFromInput(
          target,
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields?.stream,
        )} as ${target.alias}`,
      );
    }

    if (value && !value.isDerived) {
      // switch (value?.functionName) {
      //   case "p50":
      //     selectFields.push(
      //       `approx_percentile_cont(${value?.column}, 0.5) as ${value.alias}`,
      //     );
      //     break;
      //   case "p90":
      //     selectFields.push(
      //       `approx_percentile_cont(${value?.column}, 0.9) as ${value.alias}`,
      //     );
      //     break;
      //   case "p95":
      //     selectFields.push(
      //       `approx_percentile_cont(${value?.column}, 0.95) as ${value.alias}`,
      //     );
      //     break;
      //   case "p99":
      //     selectFields.push(
      //       `approx_percentile_cont(${value?.column}, 0.99) as ${value.alias}`,
      //     );
      //     break;
      //   default:
      //     selectFields.push(
      //       `${value.functionName}(${value.column}) as ${value.alias}`,
      //     );
      //     break;
      // }
      selectFields.push(
        `${buildSQLQueryFromInput(
          value,
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields?.stream,
        )} as ${value.alias}`,
      );
    }

    // Adding the selected fields to the query
    query += selectFields.join(", ");

    query += ` FROM "${stream}"`;

    // Adding filter conditions
    const filterData = queryData.fields.filter.conditions;

    const whereClause = buildWhereClause(filterData);
    query += whereClause;

    // Group By clause
    let aliases: any = [];

    if (source && !source.isDerived) {
      aliases.push(source?.alias);
    }
    if (target && !target.isDerived) {
      aliases.push(target?.alias);
    }
    if (aliases.length) {
      aliases = aliases.filter(Boolean).join(", ");
      query += `GROUP BY ${aliases}`;
    }

    // Adding sorting
    const orderByArr: string[] = [];
    [source, target, value]
      .filter((it: any) => !it?.isDerived)
      .forEach((field) => {
        if (field && field.sortBy) {
          orderByArr.push(`${field.alias} ${field.sortBy}`);
        }
      });

    if (orderByArr.length > 0) {
      query += ` ORDER BY ${orderByArr.join(", ")}`;
    }

    // Adding limit
    const queryLimit = queryData.config.limit ?? 0;
    if (queryLimit > 0) {
      query += ` LIMIT ${queryLimit}`;
    }

    return query;
  };

  // based on chart type it will create auto sql query
  const makeAutoSQLQuery = async () => {
    // only continue if current mode is auto query generation
    if (
      !dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery
    ) {
      let query = "";
      if (dashboardPanelData.data.type == "geomap") {
        query = geoMapChart();
      } else if (dashboardPanelData.data.type == "sankey") {
        query = sankeyChartQuery();
      } else if (dashboardPanelData.data.type == "maps") {
        query = mapChart();
      } else {
        query = await sqlchart();
      }
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].query = query;
      return query;
    }
  };

  // so, it is not above common state
  // expect 2nd arg for x, y and breakdown field validation
  const validatePanel = (
    errors: string[],
    isFieldsValidationRequired: boolean = true,
  ) => {
    //check each query is empty or not for promql
    if (dashboardPanelData?.data?.queryType == "promql") {
      dashboardPanelData.data.queries.map((q: any, index: number) => {
        if (q && q.query == "") {
          errors.push(`Query-${index + 1} is empty`);
        }
      });
    }

    //check each query is empty or not for geomap
    if (dashboardPanelData.data.type == "geomap") {
      dashboardPanelData.data.queries.map((q: any, index: number) => {
        if (q && q.query == "") {
          errors.push(`Query-${index + 1} is empty`);
        }
      });
    }

    //check content should be empty for html
    if (dashboardPanelData.data.type == "html") {
      if (dashboardPanelData.data.htmlContent.trim() == "") {
        errors.push("Please enter your HTML code");
      }
    }

    //check content should be empty for html
    if (dashboardPanelData.data.type == "markdown") {
      if (dashboardPanelData.data.markdownContent.trim() == "") {
        errors.push("Please enter your markdown code");
      }
    }
    if (dashboardPanelData.data.type == "custom_chart") {
      if (dashboardPanelData.data.queries[0].query.trim() == "") {
        errors.push("Please enter query for custom chart");
      }
    }

    if (promqlMode.value) {
      // 1. chart type: only line chart is supported
      const allowedChartTypes = [
        "area",
        "line",
        "bar",
        "scatter",
        "area-stacked",
        "metric",
        "gauge",
        "html",
        "markdown",
      ];
      if (!allowedChartTypes.includes(dashboardPanelData.data.type)) {
        errors.push(
          "Selected chart type is not supported for PromQL. Only line chart is supported.",
        );
      }

      // 2. x axis, y axis, filters should be blank
      if (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.x.length > 0
      ) {
        errors.push(
          "X-Axis is not supported for PromQL. Remove anything added to the X-Axis.",
        );
      }

      if (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.y.length > 0
      ) {
        errors.push(
          "Y-Axis is not supported for PromQL. Remove anything added to the Y-Axis.",
        );
      }

      if (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.filter.conditions.length > 0
      ) {
        errors.push(
          "Filters are not supported for PromQL. Remove anything added to the Filters.",
        );
      }

      // if(!dashboardPanelData.data.query) {
      //   errors.push("Query should not be empty")
      // }
    } else {
      validateSQLPanelFields(
        dashboardPanelData.data,
        dashboardPanelData.layout.currentQueryIndex,
        currentXLabel.value,
        currentYLabel.value,
        errors,
        isFieldsValidationRequired,
      );

      if (isFieldsValidationRequired) {
        // check if field selection is from the custom query fields when the custom query mode is ON
        if (
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery
        ) {
          // const customQueryXFieldError = dashboardPanelData.data.queries[
          //   dashboardPanelData.layout.currentQueryIndex
          // ].fields.x.filter(
          //   (it: any) =>
          //     ![
          //       ...dashboardPanelData.meta.stream.customQueryFields,
          //       ...dashboardPanelData.meta.stream.vrlFunctionFieldList,
          //     ].find((i: any) => i.name == it.column),
          // );
          //  HERE NEED CHANGES
          // Now, we can not check field name is there on stream or not
          // if (customQueryXFieldError.length) {
          //   errors.push(
          //     ...customQueryXFieldError.map(
          //       (it: any) =>
          //         `Please update X-Axis Selection. Current X-Axis field ${it.column} is invalid`,
          //     ),
          //   );
          // }

          const customQueryYFieldError = dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.y.filter(
            (it: any) =>
              ![
                ...dashboardPanelData.meta.stream.customQueryFields,
                ...dashboardPanelData.meta.stream.vrlFunctionFieldList,
              ].find((i: any) => i.name == it.column),
          );
          if (customQueryYFieldError.length) {
            errors.push(
              ...customQueryYFieldError.map(
                (it: any) =>
                  `Please update Y-Axis Selection. Current Y-Axis field ${it.column} is invalid`,
              ),
            );
          }
        } else {
          // check if field selection is from the selected stream fields when the custom query mode is OFF
          // const customQueryXFieldError = dashboardPanelData.data.queries[
          //   dashboardPanelData.layout.currentQueryIndex
          // ].fields.x.filter(
          //   (it: any) =>
          //     ![
          //       ...selectedStreamFieldsBasedOnUserDefinedSchema.value,
          //       ...dashboardPanelData.meta.stream.vrlFunctionFieldList,
          //     ].find((i: any) => i.name == it.column),
          // );
          // if (customQueryXFieldError.length) {
          //   errors.push(
          //     ...customQueryXFieldError.map(
          //       (it: any) =>
          //         `Please update X-Axis Selection. Current X-Axis field ${it.column} is invalid for selected stream`,
          //     ),
          //   );
          // }
          // const customQueryYFieldError = dashboardPanelData.data.queries[
          //   dashboardPanelData.layout.currentQueryIndex
          // ].fields.y.filter(
          //   (it: any) =>
          //     ![
          //       ...selectedStreamFieldsBasedOnUserDefinedSchema.value,
          //       ...dashboardPanelData.meta.stream.vrlFunctionFieldList,
          //     ].find((i: any) => i.name == it.column),
          // );
          // if (customQueryYFieldError.length) {
          //   errors.push(
          //     ...customQueryYFieldError.map(
          //       (it: any) =>
          //         `Please update Y-Axis Selection. Current Y-Axis field ${it.column} is invalid for selected stream`,
          //     ),
          //   );
          // }

          // Add join schema validation
            const currentQuery =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ];

            // Validate joins if present
            if (currentQuery.joins && currentQuery.joins.length > 0) {
              currentQuery.joins.forEach((join: any, index: number) => {
                // Validate required join fields
                if (!join.stream) {
                  errors.push(`Join #${index + 1}: Stream name is required`);
                }
                if (!join.joinType) {
                  errors.push(`Join #${index + 1}: Join type is required`);
                }

                // Validate join conditions
                if (!join.conditions || join.conditions.length === 0) {
                  errors.push(
                    `Join #${index + 1}: At least one join condition is required`,
                  );
                } else {
                  join.conditions.forEach(
                    (condition: any, condIndex: number) => {
                      // Validate left field
                      if (!condition.leftField?.field) {
                        errors.push(
                          `Join #${index + 1}, Condition #${condIndex + 1}: Left field is required`,
                        );
                      }

                      // Validate right field
                      if (!condition.rightField?.field) {
                        errors.push(
                          `Join #${index + 1}, Condition #${condIndex + 1}: Right field is required`,
                        );
                      }

                      // Validate operation
                      if (!condition.operation) {
                        errors.push(
                          `Join #${index + 1}, Condition #${condIndex + 1}: Operation is required`,
                        );
                      }
                    },
                  );
                }
              });
            }
        }
      }
    }
  };

  const VARIABLE_PLACEHOLDER = "substituteValue";

  const validateQuery = (query: any, variables: any) => {
    // Helper to test one replacement (string or number)
    const testReplacement = (q: any, varName: any, replacement: any) => {
      const regex = new RegExp(`\\$${varName}(?!\\w)`, "g"); // Match $VAR_NAME only
      return q.replace(regex, replacement);
    };

    // Recursive validation function
    const validateRecursive: any = (currentQuery: any, remainingVars: any) => {
      if (!remainingVars.length) {
        try {
          // Try parsing the current query
          parser.astify(currentQuery);
          return currentQuery; // Return valid query
        } catch (error) {
          return null; // Invalid query
        }
      }

      // Process next variable
      const [varName, ...restVars] = remainingVars;

      // Try as string
      const stringQuery = testReplacement(
        currentQuery,
        varName,
        "VARIABLE_PLACEHOLDER",
      );
      const resultAsString: any = validateRecursive(stringQuery, restVars);
      if (resultAsString) return resultAsString; // Found valid query

      // Try as number
      const numericQuery = testReplacement(currentQuery, varName, "10");
      const resultAsNumber = validateRecursive(numericQuery, restVars);
      if (resultAsNumber) return resultAsNumber; // Found valid query

      // If neither works, return null
      throw new Error("Invalid query");
    };

    return validateRecursive(query, variables);
  };

  // Extract variables from the query
  const extractVariables = (query: any) => {
    const matches = query.match(/\$(\w+|\{\w+\})/g);
    return matches
      ? [...new Set(matches.map((v: any) => v.replace(/^\$|\{|\}/g, "")))]
      : [];
  };

  // now check if the correct stream is selected
  function isDummyStreamName(tableName: any) {
    return tableName?.includes("VARIABLE_PLACEHOLDER");
  }

  // This function parses the custom query and generates the errors and custom fields
  const updateQueryValue = () => {
    // store the query in the dashboard panel data
    // dashboardPanelData.meta.editorValue = value;
    // dashboardPanelData.data.query = value;

    if (
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery &&
      dashboardPanelData.data.queryType != "promql" &&
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].query
    ) {
      // empty the errors
      dashboardPanelData.meta.errors.queryErrors = [];

      // Get the parsed query
      try {
        let currentQuery =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].query;

        // replace variables with dummy values to verify query is correct or not
        if (/\${[a-zA-Z0-9_-]+:csv}/.test(currentQuery)) {
          currentQuery = currentQuery.replaceAll(
            /\${[a-zA-Z0-9_-]+:csv}/g,
            "1,2",
          );
        }
        if (/\${[a-zA-Z0-9_-]+:singlequote}/.test(currentQuery)) {
          currentQuery = currentQuery.replaceAll(
            /\${[a-zA-Z0-9_-]+:singlequote}/g,
            "'1','2'",
          );
        }
        if (/\${[a-zA-Z0-9_-]+:doublequote}/.test(currentQuery)) {
          currentQuery = currentQuery.replaceAll(
            /\${[a-zA-Z0-9_-]+:doublequote}/g,
            '"1","2"',
          );
        }
        if (/\${[a-zA-Z0-9_-]+:pipe}/.test(currentQuery)) {
          currentQuery = currentQuery.replaceAll(
            /\${[a-zA-Z0-9_-]+:pipe}/g,
            "1|2",
          );
        }

        const variables = extractVariables(currentQuery); // Extract all unique variables
        const validatedQuery = validateQuery(currentQuery, variables);

        if (validatedQuery) {
          dashboardPanelData.meta.parsedQuery = parser.astify(validatedQuery);
        } else {
          dashboardPanelData.meta.parsedQuery = null;
        }
      } catch (e) {
        // exit if not able to parse query
        return null;
      }
      if (!dashboardPanelData.meta.parsedQuery) {
        return;
      }

      // We have the parsed query, now get the columns and tables
      // get the columns first
      if (
        Array.isArray(dashboardPanelData.meta.parsedQuery?.columns) &&
        dashboardPanelData.meta.parsedQuery?.columns?.length > 0
      ) {
        const oldCustomQueryFields = JSON.parse(
          JSON.stringify(dashboardPanelData.meta.stream.customQueryFields),
        );
        dashboardPanelData.meta.stream.customQueryFields = [];

        const fields = extractFields(
          dashboardPanelData.meta.parsedQuery,
          store.state.zoConfig.timestamp_column ?? "_timestamp",
        );

        if (Array.isArray(fields)) {
          fields.forEach((field: any) => {
            const fieldAlias = field.alias ?? field.column;
            if (
              !dashboardPanelData.meta.stream.customQueryFields.find(
                (it: any) => it.name == fieldAlias,
              )
            ) {
              dashboardPanelData.meta.stream.customQueryFields.push({
                name: fieldAlias,
                type: "",
              });
            }
          });
        }

        // update the existing x and y axis fields
        updateXYFieldsOnCustomQueryChange(oldCustomQueryFields);
      } else {
        dashboardPanelData.meta.errors.queryErrors.push("Invalid Columns");
      }

      if (dashboardPanelData.meta.parsedQuery.from?.length > 0) {
        const streamFound = dashboardPanelData.meta.stream.streamResults.find(
          (it: any) =>
            it.name == dashboardPanelData.meta.parsedQuery.from[0].table,
        );

        const currentQuery =
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ];

        const tableName = dashboardPanelData.meta.parsedQuery.from?.[0]?.table;

        if (streamFound) {
          if (currentQuery.fields.stream != streamFound.name) {
            currentQuery.fields.stream = streamFound.name;
          }
        } else if (isDummyStreamName(tableName)) {
          // nothing to do as the stream is dummy
        } else {
          let parsedQuery;
          try {
            parsedQuery = parser.astify(currentQuery?.query);
          } catch (e) {
            // exit if not able to parse query
            return;
          }
        }
      }
    }
  };

  watch(
    () => [
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].query,
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].customQuery, // Only watch for custom query mode changes
      selectedStreamFieldsBasedOnUserDefinedSchema.value,
    ],
    (newVal, oldVal) => {
      // Check if customQuery mode has changed
      const customQueryChanged = newVal[1] !== oldVal[1];

      // Only continue if the current mode is "show custom query"
      if (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery &&
        dashboardPanelData.data.queryType == "sql"
      ) {
        // Call the updateQueryValue function
        if (parser) updateQueryValue();
      } else if (customQueryChanged) {
        // Only clear lists when switching modes
        // auto query mode selected
        // remove the custom fields from the list
        dashboardPanelData.meta.stream.customQueryFields = [];
        dashboardPanelData.meta.stream.vrlFunctionFieldList = []; // Clear VRL function field list
      }
      // if (dashboardPanelData.data.queryType == "promql") {
      //     updatePromQLQuery()
      // }
    },
    { deep: true },
  );

  const currentXLabel = computed(() => {
    return dashboardPanelData.data.type == "table"
      ? "First Column"
      : dashboardPanelData.data.type == "h-bar"
        ? "Y-Axis"
        : "X-Axis";
  });

  const currentYLabel = computed(() => {
    return dashboardPanelData.data.type == "table"
      ? "Other Columns"
      : dashboardPanelData.data.type == "h-bar"
        ? "X-Axis"
        : "Y-Axis";
  });

  return {
    dashboardPanelData,
    resetDashboardPanelData,
    resetDashboardPanelDataAndAddTimeField,
    updateArrayAlias,
    addXAxisItem,
    addYAxisItem,
    addZAxisItem,
    addBreakDownAxisItem,
    addLatitude,
    addLongitude,
    addWeight,
    addMapName,
    addMapValue,
    addSource,
    addTarget,
    addValue,
    removeXAxisItem,
    removeYAxisItem,
    removeZAxisItem,
    removeBreakdownItem,
    removeFilterItem,
    removeLatitude,
    removeLongitude,
    removeWeight,
    removeMapName,
    removeMapValue,
    removeSource,
    removeTarget,
    removeValue,
    addFilteredItem,
    loadFilterItem,
    removeXYFilters,
    updateXYFieldsForCustomQueryMode,
    updateXYFieldsOnCustomQueryChange,
    isAddXAxisNotAllowed,
    isAddBreakdownNotAllowed,
    isAddYAxisNotAllowed,
    isAddZAxisNotAllowed,
    promqlMode,
    addQuery,
    removeQuery,
    resetAggregationFunction,
    cleanupDraggingFields,
    getDefaultQueries,
    validatePanel,
    makeAutoSQLQuery,
    currentXLabel,
    currentYLabel,
    generateLabelFromName,
    selectedStreamFieldsBasedOnUserDefinedSchema,
    updateGroupedFields,
  };
};
export default useDashboardPanelData;
