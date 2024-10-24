<template>
  <div class="row tw-w-full">
    <div v-for="(field, index) in fields" :key="index">
      <q-select
        v-model="fields[index].FunctionName"
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
            @click.stop.prevent="fields[index].FunctionName = null"
            class="cursor-pointer"
          />
        </template> -->
      </q-select>

      {{ JSON.stringify(fields[index].args) }}
      <!-- Loop through the args for the first n-1 arguments -->
      <div v-for="(arg, argIndex) in fields[index].args" :key="argIndex">
        <div>
          <div>
            <label :for="'arg-' + index + '-' + argIndex"
              >Arguments {{ argIndex + 1 }}:</label
            >
          </div>
          <div>
            <!-- type selector -->
            <q-select
              v-model="fields[index].args[argIndex].type"
              :options="
                getSupportedTypeBasedOnFunctionNameAndIndex(
                  field.FunctionName,
                  argIndex,
                )
              "
              dense
              filled
              label="Select Type"
              data-test="dashboard-y-item-dropdown"
            />
          </div>
          <!-- Render different input types based on validation -->
          <input
            v-if="isFieldType(fields[index].args[argIndex].type)"
            type="text"
            v-model="fields[index].args[argIndex].fieldName"
            placeholder="Enter field name"
            :required="isRequired(field.FunctionName, argIndex)"
          />

          <input
            v-if="isStringType(fields[index].args[argIndex].type)"
            type="text"
            v-model="fields[index].args[argIndex].value"
            placeholder="Enter string"
            :required="isRequired(field.FunctionName, argIndex)"
          />

          <input
            v-if="isNumberType(fields[index].args[argIndex].type)"
            type="number"
            v-model="fields[index].args[argIndex].value"
            placeholder="Enter number"
            :required="isRequired(field.FunctionName, argIndex)"
          />

          <!-- Nested function handling -->
          <!-- <div v-if="isFunctionType(fields[index].args[argIndex].type)">
              <label>Function Argument:</label>
              <select v-model="arg.FunctionName">
                <option
                  v-for="func in availableFunctions"
                  :key="func.functionName"
                  :value="func.functionName"
                >
                  {{ func.functionName }}
                </option>
              </select>
  
              <div v-if="arg.FunctionName">
                <nested-function-input
                :function-name="arg.FunctionName"
                v-model="arg"
                />
              </div>
            </div> -->
          <!-- Recursively render nested function arguments -->

          <!-- Remove argument button for first n-1 arguments -->
          <button
            v-if="canRemoveArgument(field.FunctionName)"
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
          />
        </div> -->

        <!-- Add more arguments if allowed -->
        <button
          v-if="canAddArgument(field.FunctionName)"
          @click="addArgument(index)"
        >
          Add Argument
        </button>
      </div>
    </div>
  </div>
</template>
<script>
import { ref } from "vue";
import functionValidation from "./functionValidation.json";

export default {
  name: "SelectFunction",
  setup() {
    const fields = ref([
      {
        type: "function",
        alias: "x_axis_2",
        label: "_timestamp",
        FunctionName: "histogram",
        sort_by: null,
        args: [
          {
            type: "field",
            fieldName: "_timestamp",
          },
          {
            type: "histogramInterval",
            value: ",",
          },
        ],
      },
    ]);

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
        fields.value[fieldIndex].FunctionName,
      );
      if (canAddArgument(fields.value[fieldIndex].FunctionName)) {
        // Add an argument before the separator
        fields.value[fieldIndex].args.splice(
          fields.value[fieldIndex].args.length - 1,
          0,
          {
            type: funcValidation.args[0].type[0], // Add default type (e.g., field, string, etc.)
            value: "",
          },
        );
      }
    };

    const removeArgument = (fieldIndex, argIndex) => {
      // Ensure we don't remove the separator (last argument)
      if (argIndex < fields.value[fieldIndex].args.length - 1) {
        fields.value[fieldIndex].args.splice(argIndex, 1);
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

    const getSupportedTypeBasedOnFunctionNameAndIndex = (
      functionName,
      argIndex,
    ) => {
      const funcValidation = getValidationForFunction(functionName);
      return funcValidation.args?.[argIndex]?.type || [];
    };

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
