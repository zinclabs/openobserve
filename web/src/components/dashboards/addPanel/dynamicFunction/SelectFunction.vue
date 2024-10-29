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
      :key="argIndex"
      class="tw-w-full tw-flex tw-flex-col"
    >
      <div>
        <div>
          <label :for="'arg-' + argIndex">Parameters {{ argIndex + 1 }}</label>
        </div>
        <div class="tw-flex tw-gap-x-6">
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
            class="tw-flex-1"
          />

          <!-- Render different input types based on validation -->
          <q-select
            v-if="isFieldType(fields.args[argIndex])"
            v-model="fields.args[argIndex].fieldName"
            :options="filteredSchemaOptions"
            label="Select Field"
            input-debounce="0"
            behavior="menu"
            dense
            filled
            use-input
            borderless
            hide-selected
            fill-input
            emit-value
            @filter="filterStreamFn"
            :required="isRequired(fields.functionName, argIndex)"
            class="tw-flex-1"
          />

          <q-input
            v-if="isStringType(fields.args[argIndex])"
            type="text"
            v-model="fields.args[argIndex].value"
            placeholder="Enter string"
            :required="isRequired(fields.functionName, argIndex)"
            class="tw-flex-1"
          />

          <q-input
            v-if="isNumberType(fields.args[argIndex])"
            type="number"
            v-model="fields.args[argIndex].value"
            placeholder="Enter number"
            :required="isRequired(fields.functionName, argIndex)"
            class="tw-flex-1"
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
            @click="removeArgument(argIndex)"
          >
            Remove Argument
          </button>
        </div>
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
      <button v-if="canAddArgument(fields.functionName)" @click="addArgument()">
        Add Argument
      </button>
    </div>
  </div>
</template>

<script lang="ts">
import { ref, watch, toRef, computed, inject } from "vue";
import functionValidation from "./functionValidation.json";
import useDashboardPanelData from "@/composables/useDashboardPanel";
import { useSelectAutoComplete } from "@/composables/useSelectAutocomplete";

export default {
  name: "SelectFunction",
  setup() {
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );
    const { selectedStreamFieldsBasedOnUserDefinedSchema } =
      useDashboardPanelData(dashboardPanelDataPageKey);

    const schemaOptions = computed(() =>
      selectedStreamFieldsBasedOnUserDefinedSchema?.value?.map(
        (field: any) => ({
          label: field.name,
          value: field.name,
        }),
      ),
    );

    const { filterFn: filterStreamFn, filteredOptions: filteredSchemaOptions } =
      useSelectAutoComplete(toRef(schemaOptions), "label");

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

    const getValidationForFunction = (functionName: string) => {
      return (
        functionValidation.find((v) => v.functionName === functionName) ?? {}
      );
    };

    const canAddArgument = (functionName: string) => {
      const funcValidation: any = getValidationForFunction(functionName);
      return funcValidation?.args?.[0]?.allowAddArg ?? false;
    };

    const canRemoveArgument = (functionName: string) => {
      return functionName === "arrzip"; // You can customize this logic for other functions if needed
    };

    const addArgument = () => {
      const funcValidation: any = getValidationForFunction(
        fields.value.functionName,
      );
      if (canAddArgument(fields.value.functionName)) {
        // Add an argument before the separator
        fields.value.args.splice(fields.value.args.length - 1, 0, {
          type: funcValidation?.args?.[0]?.type?.[0], // Add default type (e.g., field, string, etc.)
          value: "",
        });
      }
    };

    const removeArgument = (argIndex: number) => {
      // Ensure we don't remove the separator (last argument)
      if (argIndex < fields.value.args.length - 1) {
        fields.value.args.splice(argIndex, 1);
      }
    };

    const isRequired = (functionName: string, argIndex: number) => {
      const funcValidation: any = getValidationForFunction(functionName);
      return funcValidation?.args?.[argIndex]?.required ?? false;
    };

    const isFieldType = (arg: any) => arg.type === "field";
    const isStringType = (arg: any) => arg.type === "string";
    const isNumberType = (arg: any) => arg.type === "number";
    const isFunctionType = (arg: any) => arg.type === "function";

    const getNonSeparatorArgs = (field: any) => {
      // Return the first n-1 arguments (non-separator)
      return field.args.slice(0, field.args.length - 1);
    };

    const getSeparatorArg = (field: any) => {
      // Return the last argument (separator)
      return field.args[field.args.length - 1];
    };

    // Helper function to adjust the index based on addArgPosition
    const getAdjustedIndex = (
      argsValidation: any,
      argIndex: number,
      addArgPosition: any,
    ) => {
      const totalArgs = argsValidation.length;

      // Handle different cases for addArgPosition
      if (addArgPosition === "n") {
        // 'n' means the argument is added at the end
        return argIndex;
      } else if (addArgPosition === "n-1") {
        // 'n-1' means the argument should be added before the last argument
        return Math.max(argIndex - 1, 0); // Ensure we don't go below 0
      } else if (typeof addArgPosition === "number") {
        // If addArgPosition is a specific index, return that index
        // NOTE: NEED TO REWORK ON THIS CASE
        return Math.min(addArgPosition, totalArgs); // Ensure we don't exceed the total number of args
      } else {
        // Default case: return the provided argIndex without adjustment
        return argIndex;
      }
    };

    const getSupportedTypeBasedOnFunctionNameAndIndex = (
      functionName: string,
      argIndex: number,
    ) => {
      const funcValidation: any = getValidationForFunction(functionName);

      if (!funcValidation) {
        return [];
      }

      const argsValidation = funcValidation?.args || [];
      const addArgPosition = funcValidation?.addArgPosition || "n"; // Default to 'n' (end) if not specified

      // Determine the actual index based on addArgPosition
      const adjustedIndex = getAdjustedIndex(
        argsValidation,
        argIndex,
        addArgPosition,
      );

      // Return the type for the adjusted index, or an empty array if the index is out of bounds
      return argsValidation[adjustedIndex]?.type || [];
    };

    // watcher on functionName
    watch(
      () => fields.value.functionName,
      (newVal) => {
        // Reset the args array
        // get the validation for the selected function
        const funcValidation: any = getValidationForFunction(
          fields.value.functionName,
        );

        // rebuild fields.value.args based on funcValidation.args
        if (funcValidation) {
          fields.value.args = (funcValidation?.args ?? []).map((arg: any) => ({
            type: arg.type[0],
            value: "",
            fieldName: "",
            function: "",
          }));
        }
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
      filterStreamFn,
      filteredSchemaOptions,
    };
  },
};
</script>
1``
