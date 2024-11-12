<template>
  <div class="row" style="width: 500px">
    <q-select
      v-model="fields.functionName"
      label="Select Function"
      :options="filteredFunctions"
      data-test="dashboard-y-item-dropdown"
      input-debounce="0"
      behavior="menu"
      use-input
      filled
      borderless
      dense
      hide-selected
      fill-input
      @filter="filterFunctionsOptions"
      option-label="label"
      option-value="value"
      emit-value
      map-options
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
        <div class="tw-flex tw-gap-x-3">
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
            v-if="fields.args[argIndex]?.type === 'field'"
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
            v-if="fields.args[argIndex]?.type === 'string'"
            type="text"
            v-model="fields.args[argIndex].value"
            placeholder="Enter string"
            :required="isRequired(fields.functionName, argIndex)"
            class="tw-flex-1"
          />

          <q-input
            v-if="fields.args[argIndex]?.type === 'number'"
            type="number"
            v-model="fields.args[argIndex].value"
            placeholder="Enter number"
            :required="isRequired(fields.functionName, argIndex)"
            class="tw-flex-1"
          />

          <SelectFunction
            v-if="fields.args[argIndex]?.type === 'function'"
            class="tw-ml-4"
          />

          <!-- histogram interval for sql queries -->
          <HistogramIntervalDropDown
            v-if="fields.args[argIndex]?.type === 'histogramInterval'"
            :model-value="fields.args[argIndex]"
            @update:modelValue="
              (newValue: any) => {
                fields.args[argIndex].value = newValue.value;
              }
            "
            class="tw-flex-1"
          />

          <!-- Remove argument button -->
          <q-btn
            v-if="canRemoveArgument(fields.functionName, argIndex)"
            icon="close"
            dense
            flat
            round
            @click="removeArgument(argIndex)"
            class="tw-h-10 tw-w-10"
          />
        </div>
      </div>

      <!-- Add more arguments if allowed -->
    </div>
    <q-btn
      v-if="canAddArgument(fields.functionName)"
      @click="addArgument()"
      color="primary"
      label="+ Add"
      padding="5px 14px"
      class="tw-mt-3"
      no-caps
      dense
    />
  </div>
</template>

<script lang="ts">
import { ref, watch, toRef, computed, inject } from "vue";
import functionValidation from "./functionValidation.json";
import useDashboardPanelData from "@/composables/useDashboardPanel";
import { useSelectAutoComplete } from "@/composables/useSelectAutocomplete";
import HistogramIntervalDropDown from "../HistogramIntervalDropDown.vue";

export default {
  name: "SelectFunction",
  components: { HistogramIntervalDropDown },
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

    const filteredFunctions: any = ref([]);

    const filterFunctionsOptions = (val: string, update: any) => {
      update(() => {
        filteredFunctions.value = functionValidation
          .map((v) => ({
            label: v.functionLabel,
            value: v.functionName,
          }))
          .filter((v) => v.label.toLowerCase().indexOf(val.toLowerCase()) > -1);
      });
    };

    const fields = ref({
      functionName: "histogram",
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
      return funcValidation?.allowAddArgAt != undefined;
    };

    const canRemoveArgument = (functionName: string, argIndex: number) => {
      const funcValidation: any = getValidationForFunction(functionName);

      // if add arg not allowd, then do not allow to remove argument
      if (funcValidation?.allowAddArgAt === undefined) return false;

      const argsValidation = funcValidation?.args || [];
      const allowAddArgAt = funcValidation?.allowAddArgAt;

      // Determine the actual index based on allowAddArgAt
      const adjustedIndex = getAdjustedIndex(
        argsValidation,
        argIndex,
        allowAddArgAt,
      );

      const minArg = argsValidation[adjustedIndex]?.min ?? 0;
      const functionTotalArgs = argsValidation.length;
      const addedArgCount = argIndex + 1 - (functionTotalArgs - 1);

      return addedArgCount > minArg;
    };

    const addArgument = () => {
      const funcValidation: any = getValidationForFunction(
        fields.value.functionName,
      );

      const adjustedIndex = getAdjustedIndex(
        funcValidation?.args || [],
        fields.value.args.length - 1,
        funcValidation?.allowAddArgAt,
      );

      if (canAddArgument(fields.value.functionName)) {
        if (funcValidation.allowAddArgAt === "n") {
          fields.value.args.push({
            type: funcValidation?.args?.[adjustedIndex]?.type?.[0],
            value: "",
          });
        } else if (funcValidation.allowAddArgAt === "n-1") {
          // Add an argument before the separator
          fields.value.args.splice(fields.value.args.length - 1, 0, {
            type: funcValidation?.args?.[adjustedIndex]?.type?.[0], // Add default type (e.g., field, string, etc.)
            value: "",
          });
        }
      }
    };

    const removeArgument = (argIndex: number) => {
      fields.value.args.splice(argIndex, 1);
    };

    const isRequired = (functionName: string, argIndex: number) => {
      const funcValidation: any = getValidationForFunction(functionName);

      // NOTE: get relavent arg from validation
      return funcValidation?.args?.[argIndex]?.required ?? false;
    };

    const getNonSeparatorArgs = (field: any) => {
      // Return the first n-1 arguments (non-separator)
      return field.args.slice(0, field.args.length - 1);
    };

    const getSeparatorArg = (field: any) => {
      // Return the last argument (separator)
      return field.args[field.args.length - 1];
    };

    // Helper function to adjust the index based on allowAddArgAt
    const getAdjustedIndex = (
      argsValidation: any,
      argIndex: number,
      allowAddArgAt: any,
    ) => {
      const totalArgs = argsValidation.length;

      // Handle different cases for allowAddArgAt
      if (allowAddArgAt === "n") {
        // 'n' means the argument is added at the end
        return totalArgs - 1;
      } else if (allowAddArgAt === "n-1") {
        // 'n-1' means the argument should be added before the last argument
        // if it is last argument
        if (argIndex === totalArgs - 1) {
          return totalArgs - 1;
        } else {
          return Math.min(argIndex, totalArgs - 2);
        }
      } else if (typeof allowAddArgAt === "number") {
        // If allowAddArgAt is a specific index, return that index
        // NOTE: NEED TO REWORK ON THIS CASE
        return Math.min(allowAddArgAt, totalArgs); // Ensure we don't exceed the total number of args
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
      const allowAddArgAt = funcValidation?.allowAddArgAt;

      // Determine the actual index based on allowAddArgAt
      const adjustedIndex = getAdjustedIndex(
        argsValidation,
        argIndex,
        allowAddArgAt,
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
          // need to add args based on funcValidation.args
          fields.value.args = (funcValidation?.args ?? []).flatMap((arg: any) =>
            // need to consider `min` config for each arg
            Array.from({ length: arg.min ?? 1 }).map(() => ({
              type: arg.type[0],
              value: "",
              fieldName: "",
              function: "",
            })),
          );
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
      getNonSeparatorArgs,
      getSeparatorArg,
      getSupportedTypeBasedOnFunctionNameAndIndex,
      filterStreamFn,
      filteredSchemaOptions,
      filteredFunctions,
      filterFunctionsOptions,
    };
  },
};
</script>
1``
