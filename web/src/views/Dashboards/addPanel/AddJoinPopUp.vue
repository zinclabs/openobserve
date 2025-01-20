<!-- Copyright 2023 OpenObserve Inc.

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

     http:www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License. 
-->

<!-- eslint-disable vue/no-unused-components -->
<template>
  <div
    style="padding: 0px 10px; min-width: 60%"
    class="scroll o2-input"
    data-test="dashboard-drilldown-popup"
  >
    <div
      class="flex justify-between items-center q-pa-md"
      style="border-bottom: 2px solid gray; margin-bottom: 5px"
    >
      <div class="flex items-center q-table__title q-mr-md">
        <span data-test="dashboard-drilldown-title" v-if="isEditMode"
          >Edit Join
        </span>
        <span data-test="dashboard-drilldown-title" v-else>Add Join</span>
      </div>
    </div>
    <div>
      <div
        class="tw-flex tw-flex-row tw-w-full tw-gap-10 items-center q-table__title q-mr-md"
      >
        <q-select
          outlined
          filled
          v-model="mainStream"
          :options="[]"
          :disable="true"
          label="Joining Stream"
          class="q-py-md showLabelOnTop tw-w-1/3"
          stack-label
          data-test="dashboard-config-panel-join-from"
        />

        <q-select
          outlined
          filled
          v-model="modelValue.joinType"
          :options="joinOptions"
          label="With Join Type"
          class="q-py-md showLabelOnTop tw-w-1/3"
          stack-label
          data-test="dashboard-config-panel-join-type"
        />

        <q-select
          outlined
          filled
          v-model="modelValue.stream"
          :options="streamOptions"
          label="On Stream"
          class="q-py-md showLabelOnTop tw-w-1/3"
          stack-label
          data-test="dashboard-config-panel-join-to"
        />
      </div>

      <q-separator />

      <div>
        <span class="tw-w-full tw-text-center tw-mt-5 tw-text-lg">On</span>
        <div
          v-for="(arg, argIndex) in modelValue.conditions"
          :key="argIndex"
          class="tw-w-full tw-flex tw-flex-col"
        >
          <div>
            <div>
              <label :for="'arg-' + argIndex"
                >condition {{ argIndex + 1 }}</label
              >
            </div>
            <div class="tw-flex tw-gap-x-3">
              <!-- field1 selector -->
              <!-- @filter="filterFunctionsOptions" -->
              <q-select
                input-debounce="0"
                behavior="menu"
                use-input
                borderless
                hide-selected
                fill-input
                v-model="modelValue.conditions[argIndex].field1"
                :options="getFieldList()"
                dense
                filled
                label="Select Field"
                :data-test="`dashboard-join-condition-field1-${argIndex}`"
                class="tw-w-1/3"
              />

              <!-- operator selector -->
              <q-select
                behavior="menu"
                borderless
                hide-selected
                v-model="modelValue.conditions[argIndex].operation"
                :options="operationOptions"
                dense
                filled
                label="Select Operation"
                :data-test="`dashboard-join-condition-operation-${argIndex}`"
                class="tw-w-1/3"
              />

              <q-select
                input-debounce="0"
                behavior="menu"
                use-input
                borderless
                hide-selected
                fill-input
                v-model="modelValue.conditions[argIndex].field2"
                :options="getFieldList()"
                dense
                filled
                label="Select Field"
                :data-test="`dashboard-join-condition-field2-${argIndex}`"
                class="tw-w-1/3"
              />

              <!-- Remove argument button -->
              <!-- only allow if more than 1 -->
              <q-btn
                v-if="modelValue.conditions.length > 1"
                :data-test="`dashboard-join-condition-remove-${argIndex}`"
                icon="close"
                dense
                flat
                round
                @click="removeCondition(argIndex)"
                class="tw-h-10 tw-w-10"
              />
            </div>
          </div>
        </div>
        <q-btn
          @click="addCondition()"
          color="primary"
          label="+ Add Condition"
          padding="5px 14px"
          class="tw-mt-3"
          no-caps
          dense
        />
      </div>
    </div>

    <q-card-actions class="confirmActions">
      <q-btn
        unelevated
        no-caps
        class="q-mr-sm"
        @click="$emit('close')"
        data-test="cancel-button"
      >
        {{ t("confirmDialog.cancel") }}
      </q-btn>
      <q-btn
        unelevated
        no-caps
        class="no-border"
        color="primary"
        style="min-width: 60px"
        data-test="confirm-button"
        :label="isEditMode ? 'Update' : 'Add'"
      />
      <!-- :disable="isFormValid" -->
      <!-- @click="saveDrilldown" -->
    </q-card-actions>
  </div>
</template>

<script lang="ts">
import { inject } from "vue";
import { defineComponent } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
export default defineComponent({
  name: "AddJoinPopUp",
  components: {},
  props: {
    isEditMode: {
      type: Boolean,
      default: false,
    },
    mainStream: {
      type: String,
      required: true,
    },
    modelValue: {
      type: Object,
      default: () => {
        return {
          stream: "",
          joinType: "inner",
          conditions: [
            {
              field1: "",
              field2: "",
              logicalOperator: "and",
              operation: "=",
            },
          ],
        };
      },
    },
  },
  emits: ["close"],
  setup(props, { emit }) {
    const { t } = useI18n();
    const store = useStore();

    const streamOptions = ["default1", "default2", "default3"];
    const joinOptions = ["inner", "left", "right"];

    const getFieldList = () => {
      return ["field1", "field2", "field3"];
    };

    const removeCondition = (argIndex: number) => {
      props.modelValue.conditions.splice(argIndex, 1);
    };

    const addCondition = () => {
      props.modelValue.conditions.push({
        field1: "",
        field2: "",
        logicalOperator: "and",
        operation: "=",
      });
    };

    const operationOptions = ["=", "!=", ">", "<", ">=", "<="];

    return {
      t,
      streamOptions,
      joinOptions,
      getFieldList,
      removeCondition,
      addCondition,
      operationOptions,
    };
  },
});
</script>

<style lang="scss" scoped>
.selected {
  background-color: var(--q-primary) !important;
  font-weight: bold;
  color: white;
}
.dropdownDiv {
  display: flex;
  align-items: center;
  margin: 10px 0px;
  width: 100%;
}

.dropdownLabel {
  width: 150px;
}

.dropdown {
  min-width: 100%;
}

:deep(.no-case .q-field__native > :first-child) {
  text-transform: none !important;
}
</style>
