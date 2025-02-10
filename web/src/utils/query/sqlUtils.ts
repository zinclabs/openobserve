import { splitQuotedString, escapeSingleQuotes } from "@/utils/zincutils";

let parser: any;
let parserInitialized = false;

const importSqlParser = async () => {
  if (!parserInitialized) {
    const useSqlParser: any = await import("@/composables/useParser");
    const { sqlParser }: any = useSqlParser.default();
    parser = await sqlParser();
    parserInitialized = true;
  }
  return parser;
};

export const addLabelsToSQlQuery = async (originalQuery: any, labels: any) => {
  await importSqlParser();

  let dummyQuery = "select * from 'default'";

  for (let i = 0; i < labels.length; i++) {
    const label = labels[i];
    dummyQuery = await addLabelToSQlQuery(
      dummyQuery,
      label.name,
      label.value,
      label.operator,
    );
  }

  try {
    const astOfOriginalQuery: any = parser.astify(originalQuery);
    const astOfDummy: any = parser.astify(dummyQuery);

    // if ast already has a where clause
    if (astOfOriginalQuery.where) {
      const newWhereClause = {
        type: "binary_expr",
        operator: "AND",
        left: {
          ...astOfOriginalQuery.where,
          parentheses: true,
        },
        right: {
          ...astOfDummy.where,
          parentheses: true,
        },
      };
      const newAst = {
        ...astOfOriginalQuery,
        where: newWhereClause,
      };
      const sql = parser.sqlify(newAst);
      const quotedSql = sql.replace(/`/g, '"');
      return quotedSql;
    } else {
      const newAst = {
        ...astOfOriginalQuery,
        where: astOfDummy.where,
      };
      const sql = parser.sqlify(newAst);
      const quotedSql = sql.replace(/`/g, '"');
      return quotedSql;
    }
  } catch (error: any) {
    console.error("There was an error generating query:", error);
  }
};

export const addLabelToSQlQuery = async (
  originalQuery: any,
  label: any,
  value: any,
  operator: any,
) => {
  await importSqlParser();

  let condition: any;

  switch (operator) {
    case "Contains":
      operator = "LIKE";
      value = "%" + escapeSingleQuotes(value) + "%";
      break;
    case "Not Contains":
      operator = "NOT LIKE";
      value = "%" + escapeSingleQuotes(value) + "%";
      break;
    case "Is Null":
      operator = "IS NULL";
      break;
    case "Is Not Null":
      operator = "IS NOT NULL";
      break;
    case "IN":
      operator = "IN";
      // add brackets if not present in "IN" conditions
      value = splitQuotedString(value).map((it: any) => ({
        type: "single_quote_string",
        value: escapeSingleQuotes(it),
      }));
      break;
    case "=":
    case "<>":
    case "!=":
    case "<":
    case ">":
    case "<=":
    case ">=":
      // If value starts and ends with quote, remove it
      value =
        value &&
        value.length > 1 &&
        value.startsWith("'") &&
        value.endsWith("'")
          ? value.substring(1, value.length - 1)
          : value;
      // escape single quotes by doubling them
      value = escapeSingleQuotes(value);
      break;
  }

  // Construct condition based on operator
  condition =
    operator === "IS NULL" || operator === "IS NOT NULL"
      ? {
          type: "binary_expr",
          operator: operator,
          left: {
            type: "column_ref",
            table: null,
            column: label,
          },
          right: {
            type: "",
          },
        }
      : {
          type: "binary_expr",
          operator: operator,
          left: {
            type: "column_ref",
            table: null,
            column: label,
          },
          right: {
            type: operator === "IN" ? "expr_list" : "string",
            value: value,
          },
        };

  const ast: any = parser.astify(originalQuery);

  let query = "";
  if (!ast.where) {
    // If there is no WHERE clause, create a new one
    const newWhereClause = condition;

    const newAst = {
      ...ast,
      where: newWhereClause,
    };

    const sql = parser.sqlify(newAst);
    const quotedSql = sql.replace(/`/g, '"');
    query = quotedSql;
  } else {
    const newCondition = {
      type: "binary_expr",
      operator: "AND",
      // parentheses: true,
      left: {
        // parentheses: true,
        ...ast.where,
      },
      right: condition,
    };

    const newAst = {
      ...ast,
      where: newCondition,
    };

    const sql = parser.sqlify(newAst);
    const quotedSql = sql.replace(/`/g, '"');

    query = quotedSql;
  }

  return query;
};

export const getStreamFromQuery = async (query: any) => {
  await importSqlParser();

  try {
    const ast: any = parser.astify(query);
    return ast?.from[0]?.table || "";
  } catch (e: any) {
    return "";
  }
};

// returns 'ASC' or 'DESC' if exist
// return null if not exist

export const isGivenFieldInOrderBy = async (
  sqlQuery: string,
  fieldAlias: string,
) => {
  try {
    await importSqlParser();
    const ast: any = parser.astify(sqlQuery);

    if (ast?.orderby) {
      for (const item of ast.orderby) {
        if (item?.expr?.column?.expr?.value === fieldAlias) {
          return item.type; // 'ASC' or 'DESC'
        }
      }
    }

    return null;
  } catch (error) {
    // Handle the error as needed
    return null;
  }
};

// Function to extract field names, aliases, and aggregation functions
export function extractFields(parsedAst: any, timeField: string) {
  let fields = parsedAst.columns.map((column: any) => {
    const field = {
      column: "",
      alias: "",
      aggregationFunction: null,
    };

    if (column.expr.type === "column_ref") {
      field.column = column?.expr?.column?.expr?.value ?? timeField;
    } else if (column.expr.type === "aggr_func") {
      field.column = column?.expr?.args?.expr?.column?.expr?.value ?? timeField;
      field.aggregationFunction = column?.expr?.name?.toLowerCase() ?? "count";
    } else if (column.expr.type === "function") {
      // histogram field
      field.column =
        column?.expr?.args?.value[0]?.column?.expr?.value ?? timeField;
      field.aggregationFunction =
        column?.expr?.name?.name[0]?.value?.toLowerCase() ?? "histogram";
    }

    field.alias = column?.as ?? field?.column ?? timeField;

    return field;
  });

  // Check if all fields are selected and remove the `*` entry
  const allFieldsSelected = parsedAst.columns.some(
    (column: any) => column.expr && column.expr.column === "*",
  );

  if (allFieldsSelected) {
    // empty fields array
    fields = [];
  }

  return fields;
}

function parseCondition(condition: any) {
  try {
    if (condition.type === "binary_expr") {
      if (condition.operator == "OR" || condition.operator == "AND") {
        const left: any = parseCondition(condition.left);
        const right: any = parseCondition(condition.right);

        // set current logical operator to the right side
        if (Array.isArray(right)) {
          right[0].logicalOperator = condition.operator ?? "AND";
        } else {
          right.logicalOperator = condition.operator ?? "AND";
        }

        // conditions array
        const conditions = [];

        // if left is array
        if (Array.isArray(left)) {
          // distructure left array and push
          conditions.push(...left);
        } else {
          // if left is not array, push left object
          conditions.push(left);
        }

        // if right is array
        if (Array.isArray(right)) {
          // distructure right array and push
          conditions.push(...right);
        } else {
          // if right is not array, push right object
          conditions.push(right);
        }

        // if parentheses are true, create new group
        // else return conditions array
        if (condition.parentheses == true) {
          return {
            filterType: "group",
            logicalOperator: "AND",
            conditions: conditions,
          };
        } else {
          return conditions;
        }
      } else if (
        condition.operator == "=" ||
        condition.operator == "<" ||
        condition.operator == ">" ||
        condition.operator == "<=" ||
        condition.operator == ">="
      ) {
        return {
          type: "condition",
          values: [],
          column: condition?.left?.column?.expr?.value,
          operator: condition?.operator,
          value: `'${condition?.right?.value}'`,
          logicalOperator: "AND",
          filterType: "condition",
        };
      } else if (condition.operator == "!=" || condition.operator == "<>") {
        return {
          type: "condition",
          values: [],
          column: condition?.left?.column?.expr?.value,
          operator: "<>",
          value: `'${condition?.right?.value}'`,
          logicalOperator: "AND",
          filterType: "condition",
        };
      } else if (condition.operator == "IN") {
        // create values array based on right side of condition
        const values = condition.right.value.map(
          (value: any) => `${value?.value}`,
        );

        return {
          type: "list",
          values: values,
          column: condition?.left?.column?.expr?.value ?? "",
          operator: null,
          value: null,
          logicalOperator: "AND",
          filterType: "condition",
        };
      } else if (condition.operator == "IS") {
        // consider this as "IS NULL"
        return {
          type: "condition",
          values: [],
          column: condition?.left?.column?.expr?.value,
          operator: "Is Null",
          value: null,
          logicalOperator: "AND",
          filterType: "condition",
        };
      } else if (condition.operator == "IS NOT") {
        // consider this as "IS NOT NULL"
        return {
          type: "condition",
          values: [],
          column: condition?.left?.column?.expr?.value,
          operator: "Is Not Null",
          value: null,
          logicalOperator: "AND",
          filterType: "condition",
        };
      } else if (condition?.operator == "LIKE") {
        return {
          type: "condition",
          values: [],
          column: condition?.left?.column?.expr?.value,
          operator: "Contains",
          value: `'${condition?.right?.value}'`,
          logicalOperator: "AND",
          filterType: "condition",
        };
      } else if (condition?.operator == "NOT LIKE") {
        return {
          type: "condition",
          values: [],
          column: condition?.left?.column?.expr?.value,
          operator: "Not Contains",
          value: `'${condition?.right?.value}'`,
          logicalOperator: "AND",
          filterType: "condition",
        };
      }
    } else if (condition.type === "function") {
      const conditionName = condition?.name?.name?.[0]?.value?.toLowerCase();

      // function with field name and value
      const conditionsWithFieldName = [
        "str_match",
        "str_match_ignore_case",
        "re_match",
        "re_not_match",
      ];

      // function without field name and with value
      const conditionsWithoutFieldName = [
        "match_all",
        "match_all_raw",
        "match_all_raw_ignore_case",
      ];

      if (conditionsWithFieldName.includes(conditionName)) {
        return {
          type: "condition",
          values: [],
          column: condition?.args?.value[0]?.column?.expr?.value ?? "",
          operator: conditionName,
          value: condition?.args?.value[1]?.value ?? "",
          logicalOperator: "AND",
          filterType: "condition",
        };
      } else if (conditionsWithoutFieldName.includes(conditionName)) {
        return {
          type: "condition",
          values: [],
          column: "",
          operator: conditionName,
          value: condition?.args?.value[0]?.value ?? "",
          logicalOperator: "AND",
          filterType: "condition",
        };
      }
    }
  } catch (error) {
    return {
      filterType: "group",
      logicalOperator: "AND",
      conditions: [],
    };
  }
}

function convertWhereToFilter(where: any) {
  try {
    if (!where) {
      return {
        filterType: "group",
        logicalOperator: "AND",
        conditions: [],
      };
    }
    const parsedCondition = parseCondition(where);

    // if parsed condition is an array, it means it's a group
    if (Array.isArray(parsedCondition)) {
      return {
        filterType: "group",
        logicalOperator: "AND",
        conditions: parsedCondition,
      };
    }
    // if parsed condition is an object, it means it's a condition
    return parsedCondition;
  } catch (error) {
    return {
      filterType: "group",
      logicalOperator: "AND",
      conditions: [],
    };
  }
}

function extractFilters(parsedAst: any) {
  try {
    return convertWhereToFilter(parsedAst.where);
  } catch (error) {
    return {
      filterType: "group",
      logicalOperator: "AND",
      conditions: [],
    };
  }
}

// Function to extract the table name
function extractTableName(parsedAst: any) {
  return parsedAst.from[0].table ?? null;
}

export const getFieldsFromQuery = async (
  query: any,
  timeField: string = "_timestamp",
) => {
  try {
    await importSqlParser();

    const ast: any = parser.astify(query);

    const streamName = extractTableName(ast) ?? null;
    let fields = extractFields(ast, timeField);
    let filters: any = extractFilters(ast);

    // remove wrong fields and filters
    fields = fields.filter((field: any) => field.column);

    // if type is condition
    if (filters?.filterType === "condition") {
      filters = {
        filterType: "group",
        logicalOperator: "AND",
        conditions: [filters],
      };
    }

    return {
      fields,
      filters,
      streamName,
    };
  } catch (error) {
    return {
      fields: [
        {
          column: timeField,
          alias: "x_axis_1",
          aggregationFunction: "histogram",
        },
        {
          column: timeField,
          alias: "y_axis_1",
          aggregationFunction: "count",
        },
      ],
      filters: {
        filterType: "group",
        logicalOperator: "AND",
        conditions: [],
      },
      streamName: null,
    };
  }
};

export const buildSqlQuery = (
  tableName: string,
  fields: any,
  whereClause: string,
) => {
  let query = "SELECT ";

  // If the fields array is empty, use *, otherwise join the fields with commas
  if (fields.length === 0) {
    query += "*";
  } else {
    query += fields.join(", ");
  }

  // Add the table name
  query += ` FROM "${tableName}"`;

  // If the whereClause is not empty, add it
  if (whereClause.trim() !== "") {
    query += " WHERE " + whereClause;
  }

  // Return the constructed query
  return query;
};
export const changeHistogramInterval = async (
  query: any,
  histogramInterval: any,
) => {
  // if histogramInterval is null or query is null or query is empty, return query
  if (histogramInterval === null || query === null || query === "") {
    return query;
  }

  await importSqlParser();
  const ast: any = parser.astify(query);

  // Iterate over the columns to check if the column is histogram
  ast.columns.forEach((column: any) => {
    // check if the column is histogram
    if (
      column.expr.type === "function" &&
      column?.expr?.name?.name?.[0]?.value === "histogram"
    ) {
      const histogramExpr = column.expr;
      if (histogramExpr.args && histogramExpr.args.type === "expr_list") {
        // if selected histogramInterval is null then remove interval argument
        if (!histogramInterval) {
          histogramExpr.args.value = histogramExpr.args.value.slice(0, 1);
        }
        // else update interval argument
        else {
          // check if there is existing interval value
          // if have then do not do anything
          // else insert new arg with given histogramInterval
          if (histogramExpr.args.value[1]) {
            // Update existing interval value
            // histogramExpr.args.value[1] = {
            //   type: "single_quote_string",
            //   value: `${histogramInterval}`,
            // };
          } else {
            // create new arg for interval
            histogramExpr.args.value.push({
              type: "single_quote_string",
              value: `${histogramInterval}`,
            });
          }
        }
      }
    }
  });

  const sql = parser.sqlify(ast);
  return sql.replace(/`/g, '"');
};

// // List of known aggregation functions
// const aggregationFunctions = new Set([
//   "count",
//   "count-distinct",
//   "sum",
//   "avg",
//   "min",
//   "max",
//   "p50",
//   "p90",
//   "p95",
//   "p99",
// ]);

// // Helper function to process function arguments
// const processFunctionArgs = (args: any[]) => {
//   return {
//     type: "expr_list",
//     value: args.map((arg) => {
//       if (!arg || !arg.type)
//         return { type: "default", value: "unknown_column" };

//       switch (arg.type) {
//         case "field":
//           return {
//             type: "column_ref",
//             table: null,
//             column: arg.value?.field || "unknown_column",
//           };

//         case "number":
//           return {
//             type: "number",
//             value: arg.value ?? 0,
//           };

//         case "string":
//           return {
//             type: "string",
//             value: arg.value || "",
//           };

//         case "function":
//           return processField(arg.value); // Recursively process nested functions

//         default:
//           return { type: "default", value: "unknown_column" };
//       }
//     }),
//   };
// };

// // Helper function to process fields for SELECT clause
// const processField: any = (field: any) => {
//   if (!field || !field.alias) return null; // Ignore invalid fields

//   if (field.functionName) {
//     const functionNameLower = field.functionName.toLowerCase();
//     const isAggregation = aggregationFunctions.has(functionNameLower);

//     console.log(isAggregation, "isAggregation", field);

//     return {
//       type: "expr",
//       expr: {
//         type: isAggregation ? "aggr_func" : "function",
//         name: isAggregation
//           ? field.functionName.toUpperCase()
//           : { name: [{ type: "default", value: field.functionName }] },
//         args: processFunctionArgs(field.args || []),
//         over: null,
//       },
//       as: field.alias,
//     };
//   } else {
//     return {
//       type: "expr",
//       expr: {
//         type: "column_ref",
//         table: null,
//         column: field.column || "unknown_column",
//       },
//       as: field.alias,
//     };
//   }
// };

// // Main function to build SQL query using AST
// export async function buildSQLQueryWithParser(
//   fields: any,
//   joins: any[],
// ): Promise<string> {
//   // import parser
//   await importSqlParser();

//   console.log(
//     parser.astify(
//       "select histogram(_timestamp) as x_axis_1, count(_timestamp) as y_axis_1 from default group by x_axis_1 order by x_axis_1",
//     ),
//     "parser.astify",
//   );

//   const ast: any = {
//     type: "select",
//     columns: [],
//     from: [{ db: null, table: fields?.stream || "unknown_table", as: null }],
//     where: null,
//     groupby: null,
//     orderby: null,
//     joins: [],
//   };

//   // Process X-Axis Fields
//   if (Array.isArray(fields?.x)) {
//     fields.x.forEach((xField: any) => {
//       const processedField = processField(xField);
//       if (processedField) ast.columns.push(processedField);
//     });
//   }

//   // Process Y-Axis Fields
//   if (Array.isArray(fields?.y)) {
//     fields.y.forEach((yField: any) => {
//       const processedField = processField(yField);
//       if (processedField) ast.columns.push(processedField);
//     });
//   }

//   console.log(ast, "AST");

//   const sql = parser.sqlify(ast);
//   return sql.replace(/`/g, '"');
// }

// List of known aggregation functions
const aggregationFunctions = new Set([
  "count",
  "count-distinct",
  "sum",
  "avg",
  "min",
  "max",
  "p50",
  "p90",
  "p95",
  "p99",
]);

// Helper function to process function arguments
const processFunctionArgs = (args: any[], isAggregation: boolean) => {
  if (isAggregation) {
    // Aggregation functions need a single `expr` argument
    return {
      distinct: null,
      expr: {
        type: "column_ref",
        table: null,
        column: args[0]?.value?.field || args[0]?.value || "unknown_column",
      },
      orderby: null,
      separator: null,
    };
  } else {
    // Regular functions need an `expr_list`
    return {
      type: "expr_list",
      value: args.map((arg) => ({
        type: "column_ref",
        table: null,
        column: arg.value?.field || arg.value || "unknown_column",
      })),
    };
  }
};

// Helper function to process fields for SELECT clause
const processField = (field: any) => {
  if (!field || !field.alias) return null; // Ignore invalid fields

  if (field.functionName) {
    const functionNameLower = field.functionName.toLowerCase();
    const isAggregation = aggregationFunctions.has(functionNameLower);

    return {
      type: "expr",
      expr: {
        type: isAggregation ? "aggr_func" : "function",
        name: isAggregation
          ? field.functionName.toUpperCase()
          : { name: [{ type: "default", value: field.functionName }] },
        args: processFunctionArgs(field.args || [], isAggregation),
        over: null,
      },
      as: field.alias,
    };
  } else {
    return {
      type: "expr",
      expr: {
        type: "column_ref",
        table: null,
        column: field.column || "unknown_column",
      },
      as: field.alias,
    };
  }
};

function buildJoinConditions(conditions: any[]) {
  if (!conditions || conditions.length === 0) return null;

  if (conditions.length === 1) {
    return createBinaryExpr(conditions[0]);
  }

  let conditionTree = createBinaryExpr(conditions[0]);

  for (let i = 1; i < conditions.length; i++) {
    conditionTree = {
      type: "binary_expr",
      operator: "AND",
      left: conditionTree,
      right: createBinaryExpr(conditions[i]),
    };
  }

  return conditionTree;
}

function createBinaryExpr(condition: any) {
  return {
    type: "binary_expr",
    operator: condition.operation,
    left: {
      type: "column_ref",
      table: condition.leftField.streamAlias || null,
      column: { expr: { type: "default", value: condition.leftField.field } },
    },
    right: {
      type: "column_ref",
      table: condition.rightField.streamAlias || null,
      column: { expr: { type: "default", value: condition.rightField.field } },
    },
  };
}

// Main function to build SQL query using AST
export async function buildSQLQueryWithParser(
  fields: any,
  joins: any[],
): Promise<string> {
  // Import parser
  await importSqlParser();

  const ast: any = {
    with: null,
    type: "select",
    options: null,
    distinct: { type: null },
    columns: [],
    from: [],
    where: null,
    groupby: { columns: [] },
    having: null,
    orderby: [],
    limit: { separator: "", value: [] },
    window: null,
  };

  const groupByFields: any[] = [];

  // Main table reference
  if (fields?.stream) {
    ast.from.push({
      db: null,
      table: fields.stream,
      as: null,
    });
  }

  // Process X-Axis Fields (Included in GROUP BY & ORDER BY if applicable)
  if (Array.isArray(fields?.x)) {
    fields.x.forEach((xField: any) => {
      const processedField = processField(xField);
      if (processedField) {
        ast.columns.push(processedField);
        groupByFields.push({
          type: "column_ref",
          table: null,
          column: xField.alias || xField.column || "unknown_column",
        });

        // Handle ORDER BY for X-axis
        if (xField.sortBy) {
          ast.orderby.push({
            expr: {
              type: "column_ref",
              table: null,
              column: xField.alias || xField.column || "unknown_column",
            },
            type: xField.sortBy.toLowerCase() === "desc" ? "DESC" : "ASC",
          });
        }
      }
    });
  }

  // Process Breakdown Fields (Included in GROUP BY & ORDER BY if applicable)
  if (Array.isArray(fields?.breakdown)) {
    fields.breakdown.forEach((breakdownField: any) => {
      const processedField = processField(breakdownField);
      if (processedField) {
        ast.columns.push(processedField);
        groupByFields.push({
          type: "column_ref",
          table: null,
          column:
            breakdownField.alias || breakdownField.column || "unknown_column",
        });

        // Handle ORDER BY for Breakdown
        if (breakdownField.sortBy) {
          ast.orderby.push({
            expr: {
              type: "column_ref",
              table: null,
              column:
                breakdownField.alias ||
                breakdownField.column ||
                "unknown_column",
            },
            type:
              breakdownField.sortBy.toLowerCase() === "desc" ? "DESC" : "ASC",
          });
        }
      }
    });
  }

  // Process Y-Axis Fields (These may have ORDER BY)
  if (Array.isArray(fields?.y)) {
    fields.y.forEach((yField: any) => {
      const processedField = processField(yField);
      if (processedField) {
        ast.columns.push(processedField);
        if (yField.sortBy) {
          ast.orderby.push({
            expr: {
              type: "column_ref",
              table: null,
              column: yField.alias || yField.column || "unknown_column",
            },
            type: yField.sortBy.toLowerCase() === "desc" ? "DESC" : "ASC",
          });
        }
      }
    });
  }

  // Assign GROUP BY fields if there are any
  if (groupByFields.length > 0) {
    ast.groupby = groupByFields;
  }

  // Process Joins
  if (Array.isArray(joins)) {
    joins.forEach((join: any) => {
      ast.from.push({
        db: null,
        table: join.stream,
        as: join.streamAlias,
        join: join.joinType.toUpperCase() + " JOIN",
        on: buildJoinConditions(join.conditions),
      });
    });
  }

  console.log(
    "Abhay: ast",
    parser.astify(
      `SELECT histogram(default._timestamp) as "x_axis_1", count(stream_0.kubernetes_host) as "y_axis_1"  FROM "default" join e2e_automate as stream_0 on default.k8s_namespace_name = stream_0.k8s_namespace_name AND default.abc != stream_0.bcd  GROUP BY x_axis_1 ORDER BY x_axis_1 ASC`,
    ),
  );

  // Convert AST to SQL
  const sql = parser.sqlify(ast);
  return sql.replace(/`/g, '"'); // Replace backticks with double quotes for consistency
}
