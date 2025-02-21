import { date } from "quasar";

const units: any = {
  bytes: [
    { unit: "B", divisor: 1 },
    { unit: "KB", divisor: 1024 },
    { unit: "MB", divisor: 1024 * 1024 },
    { unit: "GB", divisor: 1024 * 1024 * 1024 },
    { unit: "TB", divisor: 1024 * 1024 * 1024 * 1024 },
    { unit: "PB", divisor: 1024 * 1024 * 1024 * 1024 * 1024 },
  ],
  seconds: [
    { unit: "ns", divisor: 0.000000001 },
    { unit: "μs", divisor: 0.000001 },
    { unit: "ms", divisor: 0.001 },
    { unit: "s", divisor: 1 },
    { unit: "m", divisor: 60 },
    { unit: "h", divisor: 3600 },
    { unit: "D", divisor: 86400 },
    { unit: "M", divisor: 2592000 }, // Assuming 30 days in a month
    { unit: "Y", divisor: 31536000 }, // Assuming 365 days in a year
  ],
  microseconds: [
    { unit: "ns", divisor: 0.001 },
    { unit: "μs", divisor: 1 },
    { unit: "ms", divisor: 1000 },
    { unit: "s", divisor: 1000000 },
    { unit: "m", divisor: 60 * 1000000 },
    { unit: "h", divisor: 3600 * 1000000 },
    { unit: "D", divisor: 86400 * 1000000 },
    { unit: "M", divisor: 2592000 * 1000000 }, // Assuming 30 days in a month
    { unit: "Y", divisor: 31536000 * 1000000 }, // Assuming 365 days in a year
  ],
  milliseconds: [
    { unit: "ns", divisor: 0.000001 },
    { unit: "μs", divisor: 0.001 },
    { unit: "ms", divisor: 1 },
    { unit: "s", divisor: 1000 },
    { unit: "m", divisor: 60 * 1000 },
    { unit: "h", divisor: 3600 * 1000 },
    { unit: "D", divisor: 86400 * 1000 },
    { unit: "M", divisor: 2592000 * 1000 }, // Assuming 30 days in a month
    { unit: "Y", divisor: 31536000 * 1000 }, // Assuming 365 days in a year
  ],
  nanoseconds: [
    { unit: "ns", divisor: 1 },
    { unit: "μs", divisor: 1000 },
    { unit: "ms", divisor: 1000000 },
    { unit: "s", divisor: 1000000000 },
    { unit: "m", divisor: 60 * 1000000000 },
    { unit: "h", divisor: 3600 * 1000000000 },
    { unit: "D", divisor: 86400 * 1000000000 },
    { unit: "M", divisor: 2592000 * 1000000000 }, // Assuming 30 days in a month
    { unit: "Y", divisor: 31536000 * 1000000000 }, // Assuming 365 days in a year
  ],
  bps: [
    { unit: "B/s", divisor: 1 },
    { unit: "KB/s", divisor: 1024 },
    { unit: "MB/s", divisor: 1024 * 1024 },
    { unit: "GB/s", divisor: 1024 * 1024 * 1024 },
    { unit: "TB/s", divisor: 1024 * 1024 * 1024 * 1024 },
    { unit: "PB/s", divisor: 1024 * 1024 * 1024 * 1024 * 1024 },
  ],
  kilobytes: [
    { unit: "B", divisor: 1 / 1024 },
    { unit: "KB", divisor: 1 },
    { unit: "MB", divisor: 1024 },
    { unit: "GB", divisor: 1024 * 1024 },
    { unit: "TB", divisor: 1024 * 1024 * 1024 },
    { unit: "PB", divisor: 1024 * 1024 * 1024 * 1024 },
  ],
  megabytes: [
    { unit: "B", divisor: 1 / (1024 * 1024) },
    { unit: "KB", divisor: 1 / 1024 },
    { unit: "MB", divisor: 1 },
    { unit: "GB", divisor: 1024 },
    { unit: "TB", divisor: 1024 * 1024 },
    { unit: "PB", divisor: 1024 * 1024 * 1024 },
  ],
  numbers: [
    { unit: "", divisor: 1 },
    { unit: "K", divisor: 1e3 },
    { unit: "M", divisor: 1e6 },
    { unit: "B", divisor: 1e9 },
    { unit: "T", divisor: 1e12 },
    { unit: "Q", divisor: 1e15 },
  ],
};

/**
 * Converts a value to a specific unit of measurement.
 *
 * @param {any} value - The value to be converted.
 * @param {string} unit - The unit of measurement to convert to.
 * @param {string} customUnit - (optional) A custom unit of measurement.
 * @return {object} An object containing the converted value and unit.
 */
export const getUnitValue = (
  value: any,
  unit: string,
  customUnit: string,
  decimals: number = 2,
) => {
  // console.time("getUnitValue:");
  let formattedValue;
  if (
    [
      "currency-dollar",
      "currency-euro",
      "currency-pound",
      "currency-yen",
      "currency-rupee",
    ].includes(unit)
  ) {
    const numericValue = parseFloat(value) || 0;
    const formattedNumber = numericValue.toFixed(decimals);

    const localeMap: any = {
      "currency-dollar": "en-US", // US Dollar
      "currency-euro": "de-DE", // Euro
      "currency-pound": "en-GB", // British Pound
      "currency-yen": "ja-JP", // Japanese Yen
      "currency-rupee": "en-IN", // Indian Rupee
    };

    formattedValue = new Intl.NumberFormat(localeMap[unit], {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(Number(formattedNumber));
  }

  // value sign: positive = 1, negative = -1
  const sign = Math.sign(value);
  // abs value
  const absValue = Math.abs(value);

  if (Number.isNaN(absValue) || Number.isNaN(sign)) {
    return { value: value, unit: "" };
  }

  // if value is missing use - as a placeholder
  if (isNaN(value) || value === "") {
    return { value: value === "" ? "-" : value, unit: "" };
  }

  switch (unit) {
    case "numbers":
    case "bytes":
    case "seconds":
    case "microseconds":
    case "milliseconds":
    case "nanoseconds":
    case "kilobytes":
    case "bps":
    case "megabytes": {
      // start with last index
      let unitIndex = units[unit].length - 1;
      // while the value is smaller than the divisor
      while (unitIndex > 0 && absValue < units[unit][unitIndex].divisor) {
        unitIndex--;
      }

      // calculate the final value: sign * absValue / divisor
      const finalValue = (
        (sign * absValue) /
        units[unit][unitIndex].divisor
      ).toFixed(decimals);
      const finalUnit = units[unit][unitIndex].unit;

      // console.timeEnd("getUnitValue:");
      return { value: finalValue, unit: finalUnit };
    }
    case "custom": {
      // console.timeEnd("getUnitValue:");
      return {
        value: `${parseFloat(value)?.toFixed(decimals) ?? 0}`,
        unit: `${customUnit ?? ""}`,
      };
    }
    case "percent-1": {
      // console.timeEnd("getUnitValue:");
      return {
        value: `${(parseFloat(value) * 100)?.toFixed(decimals) ?? 0}`,
        unit: "%",
      };
    }
    case "percent": {
      // console.timeEnd("getUnitValue:");
      return {
        value: `${parseFloat(value)?.toFixed(decimals) ?? 0}`,
        unit: "%",
      };
    }
    case "currency-pound": {
      return {
        value: `${formattedValue}`,
        unit: "£",
      };
    }
    case "currency-dollar": {
      return {
        value: `${formattedValue}`,
        unit: "$",
      };
    }
    case "currency-euro": {
      return {
        value: `${formattedValue}`,
        unit: "€",
      };
    }
    case "currency-yen": {
      return {
        value: `${formattedValue}`,
        unit: "¥",
      };
    }
    case "currency-rupee": {
      return {
        value: `${formattedValue}`,
        unit: "₹",
      };
    }
    case "default":
    default: {
      return {
        value: isNaN(value)
          ? value
          : value === ""
            ? "-"
            : ((+value)?.toFixed(decimals) ?? 0),
        unit: "",
      };
    }
  }
};

/**
 * Formats a unit value.
 *
 * @param {any} obj - The object containing the value and unit.
 * @return {string} The formatted unit value.
 */
export const formatUnitValue = (obj: any) => {
  const { unit } = obj;

  if (["$", "€", "£", "¥", "₹"].includes(unit)) {
    return `${obj.unit}${obj.value}`;
  }
  return `${obj.value}${obj.unit}`;
};

/**
 * Formats the given date into a string in the format "YY-MM-DD HH:MM:SS".
 *
 * @param {any} date - The date to be formatted.
 * @return {string} The formatted date string.
 */
export const formatDate = (date: any) => {
  const year = String(date.getFullYear());
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  const seconds = String(date.getSeconds()).padStart(2, "0");

  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
};

// Check if the sample is time series
export const isTimeSeries = (sample: any) => {
  const iso8601Pattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$/;
  return sample.every((value: any) => {
    return iso8601Pattern.test(value);
  });
};

//Check if the sample is timestamp
export const isTimeStamp = (sample: any) => {
  const microsecondsPattern = /^\d{16}$/;
  return sample.every((value: any) =>
    microsecondsPattern.test(value?.toString()),
  );
};

export function convertOffsetToSeconds(
  offset: string,
  endISOTimestamp: number,
) {
  try {
    const periodValue = parseInt(offset.slice(0, -1)); // Extract the numeric part
    const period = offset.slice(-1); // Extract the last character (unit)

    if (isNaN(periodValue)) {
      return {
        seconds: 0,
        periodAsStr: "",
      };
    }

    const subtractObject: any = {};

    let periodAsStr = periodValue.toString();

    switch (period) {
      case "s": // Seconds
        subtractObject.seconds = periodValue;
        periodAsStr += " Seconds ago";
        break;
      case "m": // Minutes
        subtractObject.minutes = periodValue;
        periodAsStr += " Minutes ago";
        break;
      case "h": // Hours
        subtractObject.hours = periodValue;
        periodAsStr += " Hours ago";
        break;
      case "d": // Days
        subtractObject.days = periodValue;
        periodAsStr += " Days ago";
        break;
      case "w": // Weeks
        subtractObject.days = periodValue * 7;
        periodAsStr += " Weeks ago";
        break;
      case "M": // Months (approximate, using 30 days per month)
        subtractObject.months = periodValue;
        periodAsStr += " Months ago";
        break;
      default:
        return {
          seconds: 0,
          periodAsStr: "",
        };
    }

    // subtract period from endISOTimestamp
    const startTimeStamp = date.subtractFromDate(
      endISOTimestamp,
      subtractObject,
    );

    // return difference of seconds between endISOTimestamp and startTimeStamp
    return {
      seconds: endISOTimestamp - startTimeStamp.getTime(),
      periodAsStr: periodAsStr,
    };
  } catch (error) {
    console.log(error);
    return {
      seconds: 0,
      periodAsStr: "",
    };
  }
}

// will find first valid mapped value based on given fieldToCheck
export const findFirstValidMappedValue = (
  value: any,
  mappings: any[],
  fieldToCheck: string,
) => {
  return mappings?.find((v: any) => {
    let isMatch = false;

    // Check based on type
    if (v?.type == "value") {
      isMatch = v?.value == value;
    } else if (v?.type == "range") {
      if (
        v?.from &&
        v?.to &&
        !Number.isNaN(+v?.from) &&
        !Number.isNaN(+v?.to)
      ) {
        isMatch = +v?.from <= +value && +v?.to >= +value;
      }
    } else if (v?.type == "regex") {
      isMatch = new RegExp(v?.pattern ?? "").test(value);
    }

    // If a match is found, check if the required field (color or text) is valid
    if (isMatch && v[fieldToCheck]) {
      return true;
    }

    return false;
  });
};

/**
 * Calculates the width of a given text.
 * Useful to calculate nameGap for the left axis
 *
 * @param {string} text - The text to calculate the width of.
 * @param {string} fontSize - The font size of the text.
 * @return {number} The width of the text in pixels.
 */
export const calculateWidthText = (
  text: string,
  fontSize: string = "12px",
): number => {
  if (!text) return 0;

  const span = document.createElement("span");
  document.body.appendChild(span);

  span.style.font = "sans-serif";
  span.style.fontSize = fontSize || "12px";
  span.style.height = "auto";
  span.style.width = "auto";
  span.style.top = "0px";
  span.style.position = "absolute";
  span.style.whiteSpace = "no-wrap";
  span.innerHTML = text;

  const width = Math.ceil(span.clientWidth);
  span.remove();
  return width;
};

/**
 * Calculates the optimal font size for a given text that fits the canvas width.
 * @param text - The text to calculate the font size for.
 * @param canvasWidth - canvas width in pixels
 * @returns {number} - The optimal font size in pixels.
 */
import functionValidation from "../../components/dashboards/addPanel/dynamicFunction/functionValidation.json";
export const calculateOptimalFontSize = (text: string, canvasWidth: number) => {
  let minFontSize = 1; // Start with the smallest font size
  let maxFontSize = 90; // Set a maximum possible font size
  let optimalFontSize = minFontSize;

  while (minFontSize <= maxFontSize) {
    const midFontSize = Math.floor((minFontSize + maxFontSize) / 2);
    const textWidth = calculateWidthText(text, `${midFontSize}px`);

    if (textWidth > canvasWidth) {
      maxFontSize = midFontSize - 1; // Text is too wide, reduce font size
    } else {
      optimalFontSize = midFontSize; // Text fits, but we try larger
      minFontSize = midFontSize + 1;
    }
  }

  return optimalFontSize; // Return the largest font size that fits
};

/**
 * Validate the filters in the panel
 * @param conditions the conditions array
 * @param errors the array to push the errors to
 */
function validateConditions(conditions: any, errors: any) {
  conditions.forEach((it: any) => {
    if (it.filterType === "condition") {
      // If the condition is a list, check if at least 1 item is selected
      if (it.type == "list" && !it.values?.length) {
        errors.push(
          `Filter: ${it.column}: Select at least 1 item from the list`,
        );
      }

      if (it.type == "condition") {
        // Check if condition operator is selected
        if (it.operator == null) {
          errors.push(`Filter: ${it.column}: Operator selection required`);
        }

        // Check if condition value is required based on the operator
        if (
          !["Is Null", "Is Not Null"].includes(it.operator) &&
          (it.value == null || it.value == "")
        ) {
          errors.push(`Filter: ${it.column}: Condition value required`);
        }
      }
    } else if (it.filterType === "group") {
      // Recursively validate the conditions in the group
      validateConditions(it.conditions, errors);
    }
  });
}

export const validateSQLPanelFields = (
  panelData: any,
  queryIndex: number,
  currentXLabel: any,
  currentYLabel: any,
  errors: string[],
  isFieldsValidationRequired: boolean = true,
) => {
  // check if fields validation is required
  if (isFieldsValidationRequired === false) {
    return;
  }

  switch (panelData?.type) {
    case "donut":
    case "pie": {
      if (
        panelData?.queries[queryIndex].fields.y.length > 1 ||
        panelData?.queries[queryIndex].fields.y.length == 0
      ) {
        errors.push("Add one value field for donut and pie charts");
      }

      if (
        panelData?.queries[queryIndex].fields.x.length > 1 ||
        panelData?.queries[queryIndex].fields.x.length == 0
      ) {
        errors.push("Add one label field for donut and pie charts");
      }

      break;
    }
    case "metric": {
      if (
        panelData.queries[queryIndex].fields.y.length > 1 ||
        panelData.queries[queryIndex].fields.y.length == 0
      ) {
        errors.push("Add one value field for metric charts");
      }

      if (panelData.queries[queryIndex].fields.x.length) {
        errors.push(`${currentXLabel} field is not allowed for Metric chart`);
      }

      break;
    }
    case "gauge": {
      if (panelData.queries[queryIndex].fields.y.length != 1) {
        errors.push("Add one value field for gauge chart");
      }
      // gauge can have zero or one label
      if (
        panelData.queries[queryIndex].fields.x.length != 1 &&
        panelData.queries[queryIndex].fields.x.length != 0
      ) {
        errors.push(`Add one label field for gauge chart`);
      }

      break;
    }
    case "h-bar":
    case "area":
    case "line":
    case "scatter":
    case "bar": {
      if (panelData.queries[queryIndex].fields.y.length < 1) {
        errors.push("Add at least one field for the Y-Axis");
      }

      if (
        panelData.queries[queryIndex].fields.x.length > 1 ||
        panelData.queries[queryIndex].fields.x.length == 0
      ) {
        errors.push(`Add one fields for the X-Axis`);
      }

      break;
    }
    case "table": {
      if (
        panelData.queries[queryIndex].fields.y.length == 0 &&
        panelData.queries[queryIndex].fields.x.length == 0
      ) {
        errors.push("Add at least one field on X-Axis or Y-Axis");
      }

      break;
    }
    case "heatmap": {
      if (panelData.queries[queryIndex].fields.y.length == 0) {
        errors.push("Add at least one field for the Y-Axis");
      }

      if (panelData.queries[queryIndex].fields.x.length == 0) {
        errors.push(`Add one field for the X-Axis`);
      }

      if (panelData.queries[queryIndex].fields.z.length == 0) {
        errors.push(`Add one field for the Z-Axis`);
      }

      break;
    }
    case "area-stacked":
    case "stacked":
    case "h-stacked": {
      if (
        panelData.queries[queryIndex].fields.y.length > 1 ||
        panelData.queries[queryIndex].fields.y.length == 0
      ) {
        errors.push(
          "Add exactly one field on Y-Axis for stacked and h-stacked charts",
        );
      }
      if (
        panelData.queries[queryIndex].fields.x.length != 1 ||
        panelData.queries[queryIndex].fields.breakdown.length != 1
      ) {
        errors.push(
          `Add exactly one fields on the X-Axis and breakdown for stacked, area-stacked and h-stacked charts`,
        );
      }

      break;
    }
    case "geomap": {
      if (panelData.queries[queryIndex].fields.latitude == null) {
        errors.push("Add one field for the latitude");
      }
      if (panelData.queries[queryIndex].fields.longitude == null) {
        errors.push("Add one field for the longitude");
      }
      break;
    }

    case "sankey": {
      if (panelData.queries[queryIndex].fields.source == null) {
        errors.push("Add one field for the source");
      }
      if (panelData.queries[queryIndex].fields.target == null) {
        errors.push("Add one field for the target");
      }
      if (panelData.queries[queryIndex].fields.value == null) {
        errors.push("Add one field for the value");
      }
      break;
    }
    case "maps": {
      if (panelData.queries[queryIndex].fields.name == null) {
        errors.push("Add one field for the name");
      }
      if (panelData.queries[queryIndex].fields.value_for_maps == null) {
        errors.push("Add one field for the value");
      }
      break;
    }
    default:
      break;
  }

  // check if aggregation function is selected or not
  if (panelData?.type && !(panelData?.type == "heatmap")) {
    const aggregationFunctionError = panelData.queries[
      queryIndex
    ].fields.y.filter(
      (it: any) =>
        !it.isDerived &&
        it.type == "build" &&
        (it.functionName == null || it.functionName == ""),
    );
    if (
      panelData.queries[queryIndex].fields.y.length &&
      aggregationFunctionError.length
    ) {
      errors.push(
        ...aggregationFunctionError.map(
          (it: any) =>
            `${currentYLabel}: ${it.column}: Aggregation function required`,
        ),
      );
    }
  }

  // check if labels are there for y axis items
  const labelError = panelData?.queries?.[queryIndex]?.fields?.y?.filter(
    (it: any) => it?.label == null || it?.label == "",
  );
  if (
    panelData?.queries?.[queryIndex]?.fields?.y?.length &&
    labelError?.length
  ) {
    errors.push(
      ...labelError.map(
        (it: any) => `${currentYLabel}: ${it.column}: Label required`,
      ),
    );
  }

  if (panelData?.queries?.[queryIndex]?.fields?.filter?.conditions?.length) {
    // Validate the top-level conditions
    validateConditions(
      panelData?.queries?.[queryIndex]?.fields?.filter?.conditions,
      errors,
    );
  }
};

export function buildSQLQueryFromInput(
  fields: any,
  defaultStream: any,
): string {
  // if fields type is raw, return rawQuery
  if (fields.type === "raw") {
    return `${fields?.rawQuery ?? ""}`;
  }

  // Extract functionName and args from the input
  const { functionName, args } = fields;

  // Find the function definition based on the functionName
  const selectedFunction = functionValidation.find(
    (fn: any) => fn.functionName === functionName,
  );

  // If the function is not found, throw an error
  if (!selectedFunction) {
    throw new Error(`Function "${functionName}" is not supported.`);
  }

  // Validate the provided args against the function's argument definitions
  const argsDefinition = selectedFunction.args;

  if (!argsDefinition || argsDefinition.length === 0) {
    return `${functionName}()`; // If no args are required, return the function call
  }

  const sqlArgs = [];
  for (let i = 0; i < args.length; i++) {
    const argValue = args[i]?.value;
    const argType = args[i]?.type;

    if (argValue === undefined || argValue === null) {
      continue;
    }

    // Add the argument to the SQL query
    if (argType === "field") {
      // If the argument type is "field", do not wrap with quotes
      sqlArgs.push(
        argValue.streamAlias
          ? argValue.streamAlias + "." + argValue.field
          : defaultStream + "." + argValue.field,
      );
    } else if (argType === "string" || argType === "histogramInterval") {
      // Wrap strings in quotes if they are not already wrapped
      if (
        typeof argValue === "string" &&
        !argValue.startsWith("'") &&
        !argValue.endsWith("'")
      ) {
        sqlArgs.push(`'${argValue}'`);
      } else {
        sqlArgs.push(argValue);
      }
    } else if (argType === "number") {
      // Add numbers as-is
      sqlArgs.push(argValue);
    } else if (argType === "function") {
      // Recursively build the SQL query for the nested function
      const nestedFunctionQuery = buildSQLQueryFromInput(
        argValue,
        defaultStream,
      );
      sqlArgs.push(nestedFunctionQuery);
    } else {
      throw new Error(
        `Unsupported argument type "${argType}" for argument at position ${i + 1}.`,
      );
    }
  }

  // TODO: add aggregator
  switch (functionName) {
    case "count-distinct":
      return `count(distinct(${sqlArgs.join(", ")}))`;
    case "p50":
      return `approx_percentile_cont(${sqlArgs.join(", ")}, 0.5)`;
    case "p90":
      return `approx_percentile_cont(${sqlArgs.join(", ")}, 0.9)`;
    case "p95":
      return `approx_percentile_cont(${sqlArgs.join(", ")}, 0.95)`;
    case "p99":
      return `approx_percentile_cont(${sqlArgs.join(", ")}, 0.99)`;
    case "p50":
      return `approx_percentile_cont(${sqlArgs.join(", ")}, 0.5)`;
  }

  // Construct the SQL query string
  // if the function is not null, return the function call statement
  // else return the first argument(if function is null, always only one argument will be there)
  return functionName
    ? `${functionName}(${sqlArgs.join(", ")})`
    : `${sqlArgs[0]}`;
}

export function buildSQLJoinsFromInput(joins: any[]): string {
  if (!joins || joins.length === 0) {
    return ""; // No joins, return empty string
  }

  let joinClauses: string[] = [];

  for (const join of joins) {
    const { stream, streamAlias, joinType, conditions } = join;

    if (
      !stream ||
      !streamAlias ||
      !joinType ||
      !conditions ||
      conditions.length === 0
    ) {
      // Invalid join, return empty string
      return "";
    }

    let joinConditionStrings: string[] = [];

    for (const condition of conditions) {
      const { leftField, rightField, operation, logicalOperator } = condition;

      if (!leftField || !rightField || !operation) {
        // Invalid condition, return empty string
        return "";
      }

      const leftFieldStr = leftField.streamAlias
        ? `${leftField.streamAlias}.${leftField.field}`
        : leftField.field;

      const rightFieldStr = rightField.streamAlias
        ? `${rightField.streamAlias}.${rightField.field}`
        : rightField.field;

      joinConditionStrings.push(
        `${leftFieldStr} ${operation} ${rightFieldStr}`,
      );
    }

    // Combine conditions with logical operators (e.g., AND, OR)
    const joinConditionsSQL = joinConditionStrings.join(" AND ");

    // Construct the JOIN SQL statement
    joinClauses.push(
      `${joinType.toUpperCase()} JOIN "${stream}" AS ${streamAlias} ON ${joinConditionsSQL}`,
    );
  }

  return joinClauses.join(" ");
}

export function addMissingArgs(fields: any): any {
  const { functionName, args } = fields;

  // Find the function definition in functionValidation
  const functionDef = functionValidation.find(
    (fn: any) => fn.functionName === functionName,
  );

  if (!functionDef) {
    return fields;
  }

  const updatedArgs = [...args]; // Clone the existing args array

  // Iterate through the function definition's arguments
  functionDef.args.forEach((argDef: any) => {
    const isArgProvided = updatedArgs.some((arg: any) => {
      // Check if the argument's type matches any of the required types
      return argDef.type.includes(arg.type);
    });

    if (!isArgProvided) {
      // If the argument is missing, add it
      const argType = argDef.type[0]; // Always take the first type
      const defaultValue =
        argDef.defaultValue !== undefined ? argDef.defaultValue : "";

      updatedArgs.push({
        type: argType,
        value: defaultValue,
      });
    }
  });

  return {
    ...fields,
    args: updatedArgs,
  };
}
