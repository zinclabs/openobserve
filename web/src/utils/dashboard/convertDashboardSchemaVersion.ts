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

const convertPanelSchemaVersion = (data: any) => {
  if (!data || (typeof data === "object" && Object.keys(data).length === 0)) {
    return;
  }
  // converting this to new array as z axis is added in the heatmap
  const queryFields = {
    stream_type: data.fields?.stream_type || "logs",
    stream: data.fields.stream || "",
    x: data.fields?.x || [],
    y: data.fields?.y || [],
    z: [], // this is a new field
    filter: data.fields?.filter || [],
  };

  return {
    id: data.id,
    type: data.type,
    title: data.config.title,
    description: data.config.description,
    config: {
      show_legends: data.config.show_legends,
      legends_position: data.config.legends_position,
      unit: data.config.unit,
      unit_custom: data.config.unit_custom,
    },
    queryType: data.queryType,
    queries: [
      {
        query: data.query,
        customQuery: data.customQuery,
        fields: queryFields,
        config: {
          promql_legend: data.config.promql_legend || "",
        },
      },
    ],
  };
};

const migrateV5FieldsToV6 = (fieldItem: any, isCustomQuery: boolean) => {
  // if fieldItem is undefined, do nothing
  if (!fieldItem) return;
  // mirgrate old args
  // previously, args was only used for histogram interval
  // so, add arg type as histogramInverval
  if (!fieldItem.args) {
    fieldItem.args = [];
  } else {
    fieldItem.args.forEach((arg: any) => {
      if (!arg.type) {
        arg.type = "histogramInverval";
      }
    });
  }

  // if customQuery then do nothing
  // else need to shift column name to as first arg
  if (isCustomQuery) {
    fieldItem.type = "custom";
  } else {
    fieldItem.type = "build";
    if (fieldItem.aggregationFunction) {
      // prepend column in args
      fieldItem.args.unshift({
        type: "field",
        value: fieldItem.column,
      });
      delete fieldItem.column;
    }
  }
};

function migrateFields(
  fields: any | any[],
  isCustomQuery: boolean,
  migrateFunction: (field: any, isCustomQuery: boolean) => void,
) {
  if (Array.isArray(fields)) {
    fields.forEach((field: any) => migrateFunction(field, isCustomQuery));
  } else {
    migrateFunction(fields, isCustomQuery);
  }
}

export function convertDashboardSchemaVersion(data: any) {
  if (!data) {
    return;
  }
  if (!data.version) data = { ...data, version: 1 };
  switch (data.version) {
    case 1: {
      // Create a object with key as a panel id and value will be its layout.
      const layoutsObjBasedOnPanelId: any = {};
      data?.layouts?.forEach((layout: any) => {
        layoutsObjBasedOnPanelId[layout.panelId] = {
          x: layout.x,
          y: layout.y,
          h: layout.h,
          w: layout.w,
          i: layout.i,
          // only include above fields
        };
      });

      // add layout object in panels array and also migrate panels schema using panelschemaversionconverted function
      data.panels = data?.panels?.map((panelItem: any) => ({
        //
        ...convertPanelSchemaVersion(panelItem),
        layout: layoutsObjBasedOnPanelId[panelItem.id], // Use the layout item from the mapping
      }));

      // update the version
      data.version = 2;

      // remove layouts key from data
      delete data.layouts;
    }
    case 2: {
      // layout width migration from 12 col number to 48 col number
      data.panels.forEach((panelItem: any) => {
        panelItem.layout.w = panelItem.layout.w * 4;
        panelItem.layout.x = panelItem.layout.x * 4;
      });

      // convert array of panels to array of tabs
      data.tabs = [
        {
          panels: data.panels,
          name: "Default",
          tabId: "default",
        },
      ];
      // remove panels key from data
      delete data.panels;

      // update the version
      data.version = 3;
    }
    case 3: {
      data.tabs.forEach((tabItem: any) => {
        tabItem.panels.forEach((panelItem: any) => {
          panelItem.queries.forEach((queryItem: any) => {
            if (!Array.isArray(queryItem.fields.breakdown)) {
              queryItem.fields.breakdown = [];
            }

            // Move excess x-axis fields to breakdown
            if (queryItem.fields.x.length > 1 && panelItem.type != "table") {
              queryItem.fields.breakdown.push(...queryItem.fields.x.slice(1));
              queryItem.fields.x = [queryItem.fields.x[0]];
            }
          });
        });
      });
      // update the version
      data.version = 4;
    }
    case 4: {
      // Migrate the filter property from an array of {type, values, column, operator, value} to
      // an object with filterType: "group", logicalOperator: "AND", and conditions: [...]
      data.tabs.forEach((tabItem: any) => {
        tabItem.panels.forEach((panelItem: any) => {
          panelItem.queries.forEach((queryItem: any) => {
            if (queryItem.fields.filter) {
              if (queryItem.fields.filter.length > 0) {
                // If the filter array is not empty, convert it to the new format
                const newFilter = {
                  filterType: "group",
                  logicalOperator: "AND",
                  conditions: queryItem.fields.filter.map((filter: any) => ({
                    // The type of the filter
                    type: filter.type,
                    // The values of the filter
                    values: filter.values,
                    // The column of the filter
                    column: filter.column,
                    // The operator of the filter
                    operator: filter.operator,
                    // The value of the filter
                    value: filter.value,
                    // The logical operator of the filter
                    logicalOperator: "AND",
                    // The type of the filter
                    filterType: "condition",
                  })),
                };
                queryItem.fields.filter = newFilter;
              } else {
                // Handle the case where filter is an empty array
                queryItem.fields.filter = {
                  filterType: "group",
                  logicalOperator: "AND",
                  conditions: [],
                };
              }
            }
          });
        });
      });
      // update the version
      data.version = 5;
    }

    case 5: {
      // need to traverse all panels
      // for each panel
      //   for each query
      //      for each fields [x, y, z, breakdown, latitude, longitude, weight, source, target, value] Make sure that some of fields is not array
      //          add type: "build"
      //          field.column will go inside args array : {type: "field", value: field.column}
      data.tabs.forEach((tabItem: any) => {
        tabItem.panels.forEach((panelItem: any) => {
          panelItem.queries.forEach((queryItem: any) => {
            const {
              x,
              y,
              z,
              breakdown,
              latitude,
              longitude,
              weight,
              source,
              target,
              value,
            } = queryItem.fields;

            // Migrate all fields
            [
              x,
              y,
              z,
              breakdown,
              latitude,
              longitude,
              weight,
              source,
              target,
              value,
            ].forEach((field: any) => {
              migrateFields(field, queryItem.customQuery, migrateV5FieldsToV6);
            });
          });
        });
      });

      // update the version
      data.version = 6;
    }
  }

  // return converted data
  return data;
}

// const dataV1 = {
//   version: 1,
//   id: "123",
//   type: "bar",
//   fields: {
//     stream: "",
//     stream_type: "logs",
//     x: [],
//     y: [],
//     filter: [],
//   },
//   config: {
//     title: "",
//     description: "",
//     show_legends: true,
//     legends_position: null,
//     promql_legend: "",
//     unit: null,
//     unit_custom: null,
//   },
//   queryType: "sql",
//   query: "",
//   customQuery: false,
// };

// const dataV2 = {
//   version: 2,
//   id: "456",
//   type: "bar",
//   config: {
//     title: "",
//     description: "",
//     show_legends: true,
//     legends_position: null,
//     unit: null,
//     unit_custom: null,
//   },
//   queryType: "sql",
//   queries: [
//     {
//       query: "",
//       customQuery: false,
//       fields: {
//         stream: "",
//         stream_type: "logs",
//         x: [],
//         y: [],
//         filter: [],
//       },
//       config: {
//         promql_legend: "",
//       },
//     },
//   ],
// };
