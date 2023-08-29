
/**
 * Converts the panel schema version of the given data object.
 *
 * @param {any} data - The data object to convert.
 * @return {any} The converted data object.
 */
export function PanelSchemaVersionConverted(data: any) {
  if (!data || (typeof data === "object" && Object.keys(data).length === 0)) {
    return;
  }
  if (!data.version) data = { ...data, version: 1 };
  switch (data.version) {
    case 1: {

      // converting this to new array as z axis is added in the heatmap
      const queryFields = {
        stream_type: data.fields?.stream_type || "logs",
        stream: data.fields.stream || "",
        x: data.fields?.x || [],
        y: data.fields?.y || [],
        z: [], // this is a new field
        filter: data.fields?.filter || [],
      };

      data = {
        version: 2,
        id: data.id,
        type: data.type,
        config: {
          title: data.config.title,
          description: data.config.description,
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
              promql_legend: data.config.promql_legend,
            },
          },
        ],
      };
    }
  }

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
