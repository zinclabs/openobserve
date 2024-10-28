<template>
  <div class="row tw-w-full">
    <q-select
      v-model="fields.functionName"
      :options="
        functionValidation.map((v) => ({
          label: v.functionLabel,
          value: v.functionName,
        }))
      "
      dense
      filled
      emit-value
      map-options
      label="Select Function"
      data-test="dashboard-y-item-dropdown"
      class="tw-w-full"
    >
      <!-- <template v-slot:append>
          <q-icon
            name="close"
            size="small"
            @click.stop.prevent="fields.functionName = null"
            class="cursor-pointer"
          />
        </template> -->
    </q-select>

    <!-- {{ JSON.stringify(fields.args) }} -->
    <!-- Loop through the args for the first n-1 arguments -->
    <div
      v-for="(arg, argIndex) in fields.args"
      :key="argIndex + JSON.stringify(arg)"
      class="tw-w-full tw-flex tw-flex-col"
    >
      <div>
        <div>
          <label :for="'arg-' + argIndex">Arguments {{ argIndex + 1 }}:</label>
        </div>
        <div>
          <!-- type selector -->
          <q-select
            v-model="fields.args[argIndex].type"
            :options="
              getSupportedTypeBasedOnFunctionNameAndIndex(
                fields.functionName,
                argIndex,
              )
            "
            dense
            filled
            label="Select Type"
            data-test="dashboard-y-item-dropdown"
            class="tw-w-full"
          />
        </div>
        <!-- Render different input types based on validation -->
        <input
          v-if="isFieldType(fields.args[argIndex].type)"
          type="text"
          v-model="fields.args[argIndex].fieldName"
          placeholder="Enter field name"
          :required="isRequired(field.functionName, argIndex)"
          class="tw-w-full"
        />

        <input
          v-if="isStringType(fields.args[argIndex].type)"
          type="text"
          v-model="fields.args[argIndex].value"
          placeholder="Enter string"
          :required="isRequired(field.functionName, argIndex)"
          class="tw-w-full"
        />

        <input
          v-if="isNumberType(fields.args[argIndex].type)"
          type="number"
          v-model="fields.args[argIndex].value"
          placeholder="Enter number"
          :required="isRequired(field.functionName, argIndex)"
          class="tw-w-full"
        />

        <!-- Nested function handling -->
        <!-- <div v-if="isFunctionType(fields.args[argIndex].type)">
              <label>Function Argument:</label>
              <select v-model="arg.functionName">
                <option
                  v-for="func in availableFunctions"
                  :key="func.functionName"
                  :value="func.functionName"
                >
                  {{ func.functionName }}
                </option>
              </select>
  
              <div v-if="arg.functionName">
                <nested-function-input
                :function-name="arg.functionName"
                v-model="arg"
                />
              </div>
            </div> -->
        <!-- Recursively render nested function arguments -->

        <!-- Remove argument button for first n-1 arguments -->
        <button
          v-if="canRemoveArgument(fields.functionName)"
          @click="removeArgument(index, argIndex)"
        >
          Remove Argument
        </button>
      </div>

      <!-- Separator input (last argument) -->
      <!-- <div v-if="getSeparatorArg(field)">
          <label>Separator:</label>
          <input
            type="text"
            v-model="getSeparatorArg(field).value"
            placeholder="Enter separator"
            required
            class="tw-w-full"
          />
        </div> -->

      <!-- Add more arguments if allowed -->
      <button
        v-if="canAddArgument(fields.functionName)"
        @click="addArgument(index)"
      >
        Add Argument
      </button>
    </div>
  </div>
</template>
<script>
import { ref, watch } from "vue";
import functionValidation from "./functionValidation.json";

export default {
  name: "SelectFunction",
  setup() {
    const fields = ref({
      type: "function",
      alias: "x_axis_2",
      label: "_timestamp",
      functionName: "histogram",
      sort_by: null,
      args: [
        {
          type: "field",
          fieldName: "_timestamp",
        },
        {
          type: "histogramInterval",
          value: "5 min",
        },
      ],
    });

    const availableFunctions = ref(["arrzip", "concat", "count", "sum"]);

    const getValidationForFunction = (functionName) => {
      return (
        functionValidation.find((v) => v.functionName === functionName) || {}
      );
    };

    const canAddArgument = (functionName) => {
      const funcValidation = getValidationForFunction(functionName);
      return funcValidation.args?.[0]?.allowAddArg || false;
    };

    const canRemoveArgument = (functionName) => {
      return functionName === "arrzip"; // You can customize this logic for other functions if needed
    };

    const addArgument = (fieldIndex) => {
      const funcValidation = getValidationForFunction(
        fields.value.functionName,
      );
      if (canAddArgument(fields.value.functionName)) {
        // Add an argument before the separator
        fields.value.args.splice(fields.value.args.length - 1, 0, {
          type: funcValidation.args[0].type[0], // Add default type (e.g., field, string, etc.)
          value: "",
        });
      }
    };

    const removeArgument = (fieldIndex, argIndex) => {
      // Ensure we don't remove the separator (last argument)
      if (argIndex < fields.value.args.length - 1) {
        fields.value.args.splice(argIndex, 1);
      }
    };

    const isRequired = (functionName, argIndex) => {
      const funcValidation = getValidationForFunction(functionName);
      return funcValidation.args?.[argIndex]?.required || false;
    };

    const isFieldType = (arg) => arg.type === "field";
    const isStringType = (arg) => arg.type === "string";
    const isNumberType = (arg) => arg.type === "number";
    const isFunctionType = (arg) => arg.type === "function";

    const getNonSeparatorArgs = (field) => {
      // Return the first n-1 arguments (non-separator)
      return field.args.slice(0, field.args.length - 1);
    };

    const getSeparatorArg = (field) => {
      // Return the last argument (separator)
      return field.args[field.args.length - 1];
    };

    // Helper function to adjust the index based on addArgPosition
    const getAdjustedIndex = (argsValidation, argIndex, addArgPosition) => {
      const totalArgs = argsValidation.length;

      // Handle different cases for addArgPosition
      if (addArgPosition === "n") {
        // 'n' means the argument is added at the end
        return totalArgs; // Add at the end, so return the current length (next available index)
      } else if (addArgPosition === "n-1") {
        // 'n-1' means the argument should be added before the last argument
        return Math.max(totalArgs - 1, 0); // Ensure we don't go below 0
      } else if (typeof addArgPosition === "number") {
        // If addArgPosition is a specific index, return that index
        return Math.min(addArgPosition, totalArgs); // Ensure we don't exceed the total number of args
      } else {
        // Default case: return the provided argIndex without adjustment
        return argIndex;
      }
    };

    const getSupportedTypeBasedOnFunctionNameAndIndex = (
      functionName,
      argIndex,
    ) => {
      const funcValidation = getValidationForFunction(functionName);

      if (!funcValidation) {
        return [];
      }

      const argsValidation = funcValidation.args || [];
      const addArgPosition = funcValidation.addArgPosition || "n"; // Default to 'n' (end) if not specified

      // Determine the actual index based on addArgPosition
      const adjustedIndex = getAdjustedIndex(
        argsValidation,
        argIndex,
        addArgPosition,
      );

      console.log("adjustedIndex", adjustedIndex, argsValidation);

      // Return the type for the adjusted index, or an empty array if the index is out of bounds
      return argsValidation[adjustedIndex]?.type || [];
    };

    // watcher on functionName
    watch(
      () => fields.value.functionName,
      (newVal) => {
        // Reset the args array
        // get the validation for the selected function
        const funcValidation = getValidationForFunction(
          fields.value.functionName,
        );

        console.log("funcValidation", funcValidation);

        fields.value.args = funcValidation.args;
      },
    );

    return {
      fields,
      functionValidation,
      availableFunctions,
      getValidationForFunction,
      canAddArgument,
      canRemoveArgument,
      addArgument,
      removeArgument,
      isRequired,
      isFieldType,
      isStringType,
      isNumberType,
      isFunctionType,
      getNonSeparatorArgs,
      getSeparatorArg,
      getSupportedTypeBasedOnFunctionNameAndIndex,
    };
  },
};
</script>
