<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <div class="q-mt-md full-width">
    <div class="flex q-mx-md items-center no-wrap">
      <div class="col">
        <div class="flex">
          <q-btn
            no-caps
            padding="xs"
            outline
            @click="router.back()"
            icon="arrow_back_ios_new"
          />
          <div class="text-h6 q-ml-md">Import Alert</div>
        </div>
      </div>
      <div class="flex justify-center">
        <q-btn
          v-close-popup
          class="text-bold q-mr-md"
          :label="t('function.cancel')"
          text-color="light-text"
          padding="sm xl"
          no-caps
          @click="router.back()"
        />
        <q-btn
          class="text-bold no-border"
          :label="t('dashboard.import')"
          color="secondary"
          type="submit"
          padding="sm xl"
          no-caps
          @click="importJson"
        />
      </div>
    </div>

    <q-separator class="q-my-sm q-mx-md" />
  </div>
  <div class="flex">
    <div class="report-list-tabs flex items-center justify-center q-mx-md">
      <app-tabs
        data-test="pipeline-list-tabs"
        class="q-mr-md"
        :tabs="tabs"
        v-model:active-tab="activeTab"
        @update:active-tab="updateActiveTab"
      />
    </div>

    <div class="flex">
      <q-splitter
        class="logs-search-splitter"
        no-scroll
        v-model="splitterModel"
        style="width: calc(95vw - 20px); height: 100%"
      >
        <template #before>
          <div
            v-if="activeTab == 'import_json_url'"
            class="editor-container-url"
          >
            <q-form class="q-mx-md q-mt-md" @submit="onSubmit">
              <div style="width: 100%" class="q-mb-md">
                <q-input
                  v-model="url"
                  :label="t('dashboard.addURL')"
                  color="input-border"
                  bg-color="input-bg"
                  stack-label
                  filled
                  label-slot
                />
              </div>
              <query-editor
                data-test="scheduled-alert-sql-editor"
                ref="queryEditorRef"
                editor-id="alerts-query-editor"
                class="monaco-editor"
                :debounceTime="300"
                v-model:query="jsonStr"
                language="json"
                :class="
                  jsonStr == '' && queryEditorPlaceholderFlag
                    ? 'empty-query'
                    : ''
                "
                @focus="queryEditorPlaceholderFlag = false"
                @blur="queryEditorPlaceholderFlag = true"
              />

              <div></div>
            </q-form>
          </div>
          <div
            v-if="activeTab == 'import_json_file'"
            class="editor-container-json"
          >
            <q-form class="q-mx-md q-mt-md" @submit="onSubmit">
              <div style="width: 100%" class="q-mb-md">
                <q-file
                  v-model="jsonFiles"
                  filled
                  bottom-slots
                  :label="t('dashboard.dropFileMsg')"
                  accept=".json"
                  multiple
                >
                  <template v-slot:prepend>
                    <q-icon name="cloud_upload" @click.stop.prevent />
                  </template>
                  <template v-slot:append>
                    <q-icon
                      name="close"
                      @click.stop.prevent="jsonFiles = null"
                      class="cursor-pointer"
                    />
                  </template>
                  <template v-slot:hint> .json files only </template>
                </q-file>
              </div>
              <query-editor
                data-test="scheduled-alert-sql-editor"
                ref="queryEditorRef"
                editor-id="alerts-query-editor"
                class="monaco-editor"
                :debounceTime="300"
                v-model:query="jsonStr"
                language="json"
                :class="
                  jsonStr == '' && queryEditorPlaceholderFlag
                    ? 'empty-query'
                    : ''
                "
                @focus="queryEditorPlaceholderFlag = false"
                @blur="queryEditorPlaceholderFlag = true"
              />

              <div></div>
            </q-form>
          </div>
        </template>
        <template #after>
          <div
            data-test="logs-vrl-function-editor"
            style="width: 100%; height: 100%"
          >
            <div
              v-if="alertErrorsToDisplay.length > 0"
              class="text-center text-h6"
            >
              Error Validations
            </div>
            <div v-else class="text-center text-h6">Output Messages</div>
            <q-separator class="q-mx-md q-mt-md" />
            <div class="error-report-container">
              <!-- Alert Errors Section -->
              <div class="error-section" v-if="alertErrorsToDisplay.length > 0">
                <div class="error-list">
                  <!-- Iterate through the outer array -->
                  <div
                    v-for="(errorGroup, index) in alertErrorsToDisplay"
                    :key="index"
                  >
                    <!-- Iterate through each inner array (the individual error message) -->
                    <div
                      v-for="(errorMessage, errorIndex) in errorGroup"
                      :key="errorIndex"
                      class="error-item"
                    >
                      <span
                        class="text-red"
                        v-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'alert_name'
                        "
                      >
                        {{ errorMessage.message }}

                        <div style="width: 300px">
                          <q-input
                            v-model="userSelectedAlertName[index]"
                            :label="t('alerts.name') + ' *'"
                            color="input-border"
                            bg-color="input-bg"
                            class="showLabelOnTop"
                            stack-label
                            outlined
                            filled
                            dense
                            tabindex="0"
                            @update:model-value="
                              updateAlertName(userSelectedAlertName[index],index)
                            "
                          />
                        </div>
                      </span>
                      <!-- Check if the errorMessage is an object, if so, display the 'message' property -->
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'stream_name'
                        "
                      >
                        {{ errorMessage.message }}
                        <div style="width: 300px">
                          <q-select
                            v-model="userSelectedStreamName[index]"
                            :options="streamList"
                            :label="t('alerts.stream_name') + ' *'"
                            :popup-content-style="{
                              textTransform: 'lowercase',
                            }"
                            color="input-border"
                            bg-color="input-bg"
                            class="q-py-sm showLabelOnTop no-case"
                            filled
                            stack-label
                            dense
                            use-input
                            hide-selected
                            fill-input
                            :input-debounce="400"
                            @update:model-value="updateStreamFields(userSelectedStreamName[index], index)"
                            behavior="menu"
                          />
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'destination_name'
                        "
                      >
                        {{ errorMessage.message }}
                        <div>
                          <q-select
                            v-model="userSelectedDestinations[index]"
                            :options="filteredDestinations"
                            @filter="filterDestinations"
                            label="Destinations *"
                            :popup-content-style="{
                              textTransform: 'lowercase',
                            }"
                            color="input-border"
                            bg-color="input-bg"
                            class="q-py-sm showLabelOnTop no-case"
                            filled
                            stack-label
                            dense
                            use-input
                            multiple
                            :input-debounce="400"
                            behavior="menu"
                            :rules="[
                              (val: any) => !!val || 'Field is required!',
                            ]"
                            style="width: 300px"
                            @update:model-value="updateUserSelectedDestinations(userSelectedDestinations[index],index)"
                          >
                            <template v-slot:option="scope">
                              <q-item
                                v-bind="scope.itemProps"
                                :data-test="`add-alert-destination-${scope.opt}-select-item`"
                              >
                                <q-item-section side>
                                  <q-checkbox 
                                    :model-value="userSelectedDestinations[index]?.includes(scope.opt) ?? false"
                                    dense
                                    @update:model-value="toggleDestination(scope.opt, index)"
                                  />
                                </q-item-section>
                                <q-item-section>
                                  <q-item-label>{{ scope.opt }}</q-item-label>
                                </q-item-section>
                              </q-item>
                            </template>
                          </q-select>
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'stream_type'
                        "
                      >
                        {{ errorMessage.message }}
                        <div>
                          <q-select
                            v-model="userSelectedStreamType[index]"
                            :options="streamTypes"
                            :label="t('alerts.streamType') + ' *'"
                            :popup-content-style="{ textTransform: 'lowercase' }"
                            color="input-border"
                            bg-color="input-bg"
                            class="q-py-sm showLabelOnTop no-case"
                            stack-label
                            outlined
                            filled
                            dense
                            @update:model-value="updateStreams(userSelectedStreamType[index], index)"
                            :rules="[(val: any) => !!val || 'Field is required!']"
                            style="width: 300px"
                          />
                        </div>
                      </span>

                      <span v-else>{{ errorMessage }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <div class="error-section" v-if="alertCreators.length > 0">
                <div class="section-title text-primary">Alert Creation</div>
                <div
                  class="error-list"
                  v-for="(val, index) in alertCreators"
                  :key="index"
                >
                  <div
                    :class="{
                      'error-item text-bold': true,
                      'text-green ': val.success,
                      'text-red': !val.success,
                    }"
                  >
                    <pre>{{ val.message }}</pre>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>
      </q-splitter>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onMounted,
  reactive,
  computed,
  watch,
} from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter, useRoute } from "vue-router";
import axios from "axios";
import router from "@/router";
import { useQuasar } from "quasar";
import alertsService from "../../services/alerts";

import QueryEditor from "../QueryEditor.vue";
import { json } from "stream/consumers";
import useStreams from "@/composables/useStreams";
import templateService from "@/services/alert_templates";
import destinationService from "@/services/alert_destination";

import AppTabs from "../common/AppTabs.vue";
import { error } from "console";

export default defineComponent({
  name: "ImportAlert",
  props: {
    destinations: {
      type: Array,
      default: () => [],
    },
    templates: {
      type: Array,
      default: () => [],
    },
    alerts: {
      type: Array,
      default: () => [],
    },
  },
  emits: ["update:destinations", "update:templates", "update:alerts"],
  setup(props, { emit }) {
    type ErrorMessage = {
      field: string;
      message: string;
    };
    type alertCreator = {
      message: string;
      success: boolean;
    }[];

    type AlertErrors = (ErrorMessage | string)[][];
    const { t } = useI18n();
    const store = useStore();
    const router = useRouter();
    const route = useRoute();

    const jsonStr: any = ref("");
    const q = useQuasar();
    const { getStreams } = useStreams();

    const templateErrorsToDisplay = ref([]);

    const destinationErrorsToDisplay = ref([]);

    const alertErrorsToDisplay = ref<AlertErrors>([]);
    const userSelectedDestinations = ref<string[][]>([]);
    const userSelectedAlertName = ref<string[]>([]);

    const tempalteCreators = ref([]);
    const destinationCreators = ref([]);
    const alertCreators = ref<alertCreator>([]);
    const queryEditorPlaceholderFlag = ref(true);
    const streamList = ref<any>([]);
    const userSelectedStreamName = ref<string[]>([]);
    const userSelectedStreamType = ref<string[]>([]);
    const jsonFiles = ref(null);
    const url = ref("");
    const jsonArrayOfObj: any = ref([{}]);
    const streams = ref<any>({});
    const activeTab = ref("import_json_file");
    const splitterModel = ref(60);
    const filteredDestinations = ref<string[]>([]);
    const streamTypes = ["logs", "metrics", "traces"];
    const getFormattedDestinations: any = computed(() => {
      return props.destinations.map((destination: any) => {
        return destination.name;
      });
    });

    // watch(
    //   () => userSelectedDestinations.value,
    //   (newVal, oldVal) => {
    //     if (newVal) {
    //       jsonArrayOfObj.value.destinations = newVal;
    //       jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    //     }
    //   },
    // );

    const updateUserSelectedDestinations = (destinations: string[], index: number) => {
      jsonArrayOfObj.value[index].destinations = destinations;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    }

    const updateStreamFields = (stream_name: string, index: number) => {
      jsonArrayOfObj.value[index].stream_name = stream_name;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    const updateAlertName = (alertName: string, index: number) => {
      jsonArrayOfObj.value[index].name = alertName;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    watch(jsonFiles, async (newVal:any, oldVal:any) => {
      if (newVal && newVal.length > 0) {
        let combinedJson: any[] = [];
        
        for (const file of newVal) {
          try {
            const result: any = await new Promise((resolve) => {
              const reader = new FileReader();
              reader.onload = (e: any) => {
                try {
                  const parsedJson = JSON.parse(e.target.result);
                  // Convert to array if it's a single object
                  const jsonArray = Array.isArray(parsedJson) ? parsedJson : [parsedJson];
                  resolve(jsonArray);
                } catch (error) {
                  q.notify({
                    message: `Error parsing JSON from file ${file.name}`,
                    color: "negative",
                    position: "bottom",
                    timeout: 2000,
                  });
                  resolve([]);
                }
              };
              reader.readAsText(file);
            });
            
            combinedJson = [...combinedJson, ...result];
          } catch (error) {
            console.error('Error reading file:', error);
          }
        }
        
        // Update the refs with combined JSON data
        jsonArrayOfObj.value = combinedJson;
        jsonStr.value = JSON.stringify(combinedJson, null, 2);
      }
    });
    watch(url, async (newVal, oldVal) => {
      try {
        if (newVal) {
          const response = await axios.get(newVal);

          // Check if the response body is valid JSON
          try {
            if (
              response.headers["content-type"].includes("application/json") ||
              response.headers["content-type"].includes("text/plain")
            ) {
              jsonStr.value = JSON.stringify(response.data, null, 2);
              jsonArrayOfObj.value = response.data;
            } else {
              q.notify({
                message: "Invalid JSON format in the URL",
                color: "negative",
                position: "bottom",
                timeout: 2000,
              });
            }
          } catch (parseError) {
            // If parsing fails, display an error message
            q.notify({
              message: "Invalid JSON format",
              color: "negative",
              position: "bottom",
              timeout: 2000,
            });
          }
        }
      } catch (error) {
        q.notify({
          message: "Error fetching data",
          color: "negative",
          position: "bottom",
          timeout: 2000,
        });
      }
    });

    const tabs = reactive([
      {
        label: "File Upload / JSON",
        value: "import_json_file",
      },
      {
        label: "URL Import",
        value: "import_json_url",
      },
    ]);

    const updateActiveTab = () => {
      jsonStr.value = "";
      jsonFiles.value = null;
      url.value = "";
      jsonArrayOfObj.value = [{}];
    };

    onMounted(() => {});

    const importJson = async () => {
      alertErrorsToDisplay.value = [];
      templateErrorsToDisplay.value = [];
      destinationErrorsToDisplay.value = [];
      destinationCreators.value = [];
      alertCreators.value = [];

      try {
        // Check if jsonStr.value is empty or null
        if ((!jsonStr.value || jsonStr.value.trim() === "") && !url.value) {
          throw new Error("JSON string is empty");
        } else {
          const parsedJson = JSON.parse(jsonStr.value);
          // Convert single object to array if needed
          jsonArrayOfObj.value = Array.isArray(parsedJson) ? parsedJson : [parsedJson];
        }
      } catch (e: any) {
        q.notify({
          message: e.message || "Invalid JSON format",
          color: "negative",
          position: "bottom", 
          timeout: 2000,
        });
        return;
      }

      let allAlertsCreated = true;
      // Now we can always process as an array
      for (const [index, jsonObj] of jsonArrayOfObj.value.entries()) {
        const success = await processJsonObject(jsonObj, index + 1);
        if (!success) {
          allAlertsCreated = false;
        }
      }

      if (allAlertsCreated) {
        emit("update:alerts");
        q.notify({
          message: "Alert(s) imported successfully",
          color: "positive",
          position: "bottom",
          timeout: 2000,
        });
        router.push({
          name: "alertList",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
    };

    const processJsonObject = async (jsonObj: any, index: number) => {
      try {
        const isValidAlert = await validateAlertInputs(jsonObj, index);
        if (!isValidAlert) {
          return false;
        }

        if (alertErrorsToDisplay.value.length === 0 && isValidAlert) {
          return await createAlert(jsonObj, index);
        }
      } catch (e: any) {
        q.notify({
          message: "Error importing Alert(s) please check the JSON",
          color: "negative",
          position: "bottom",
          timeout: 2000,
        });
        return false;
      }
      return false;
    };

    const validateAlertInputs = async (input: any, index: number) => {
      let alertErrors: (string | { message: string; field: string })[] = [];

      // 1. Validate 'name' field
      if (
        !input.name ||
        typeof input.name !== "string" ||
        input.name.trim() === ""
      ) {
        alertErrors.push({
          message: `Alert - ${index}: Name is mandatory and should be a valid string.`,
          field: "alert_name",
        });
      }

      if (checkAlertsInList(props.alerts, input.name)) {
        alertErrors.push({
          message: `Alert - ${index}: "${input.name}" already exists`,
          field: "alert_name",
        });
      }
      const organizationData = store.state.organizations;
      const orgList = organizationData.map((org: any) => org.identifier);

      // 2. Validate 'org_id' field
      if (
        !input.org_id ||
        typeof input.org_id !== "string" ||
        input.org_id.trim() === "" ||
        !orgList.includes(input.org_id)
      ) {
        alertErrors.push(
          `Alert - ${index}: Organization Id is mandatory, should exist in organization list and should be a valid string.`,
        );
      }

      // 3. Validate 'stream_type' field
      const validStreamTypes = ["logs", "metrics", "traces"];
      if (!input.stream_type || !validStreamTypes.includes(input.stream_type)) {
        alertErrors.push(
          {
            message: `Alert - ${index}: Stream Type is mandatory and should be one of: 'logs', 'metrics', 'traces'.`,
            field: "stream_type",
          }
        );
      }

      try {
        const streamResponse: any = await getStreams(input.stream_type, false);
        streamList.value = streamResponse.list.map(
          (stream: any) => stream.name,
        );
      } catch (e) {
        alertErrors.push();
        const err: any = {
          message: `Alert - ${index}: Error fetching stream list. Please try again.`,
          field: "stream_list",
        };
        alertErrorsToDisplay.value.push(err);
      }

      // 4. Validate 'stream_name' field
      if (
        !input.stream_name ||
        typeof input.stream_name !== "string" ||
        !streamList.value.includes(input.stream_name)
      ) {
        alertErrors.push({
          message: `Alert - ${index}: Stream Name is mandatory, should exist in the stream list and should be a valid string.`,
          field: "stream_name",
        });
      }

      // 5. Validate 'is_real_time' field
      if (typeof input.is_real_time !== "boolean") {
        alertErrors.push(
          `Alert - ${index}: Is Real-Time is mandatory and should be a boolean value.`,
        );
      }

      // 6. Validate 'query_condition' field
      if (input.query_condition && input.query_condition.conditions) {
        if (!Array.isArray(input.query_condition.conditions)) {
          alertErrors.push(
            `Alert - ${index}: Query conditions should be an array.`,
          );
        }

        for (let condition of input.query_condition.conditions) {
          if (!condition.column || !condition.operator || !condition.value) {
            alertErrors.push(
              `Alert - ${index}: Each query condition must have 'column', 'operator', and 'value'.`,
            );
          }

          if (
            input.query_condition.type === "custom" &&
            !["=", ">", "<", ">=", "<=", "Contains", "NotContains"].includes(
              condition.operator,
            )
          ) {
            alertErrors.push(
              `Alert - ${index}: Invalid operator in query condition. Allowed operators: '=', '>', '<', '>=', '<=', 'Contains', 'NotContains'.`,
            );
          }
        }
      }
      // 7. Validate 'sql' and 'promql'
      if (
        input.query_condition.type === "sql" &&
        typeof input.query_condition.sql !== "string"
      ) {
        alertErrors.push(
          `Alert - ${index}: SQL should be provided when the type is 'sql'.`,
        );
      }

      if (
        input.query_condition.type === "promql" &&
        typeof input.query_condition.promql !== "string"
      ) {
        alertErrors.push(
          `Alert - ${index}: PromQL should be provided when the type is 'promql'.`,
        );
      }

      // 8. Validate 'vrl_function'
      if (
        input.query_condition.vrl_function &&
        typeof input.query_condition.vrl_function !== "string"
      ) {
        alertErrors.push(
          `Alert - ${index}: VRL function should be a string or null.`,
        );
      }

      // 9. Validate 'multi_time_range'
      if (input.query_condition.type === 'custom' && 
          input.query_condition.multi_time_range !== null && 
          (!Array.isArray(input.query_condition.multi_time_range) || input.query_condition.multi_time_range.length > 0)) {
        alertErrors.push(
          `Alert - ${index}: Multi Time Range should be an empty array or null.`,
        );
      }

      // 10. Validate 'trigger_condition'
      const triggerCondition = input.trigger_condition;
      if (!triggerCondition) {
        alertErrors.push(`Alert - ${index}: Trigger Condition is required.`);
      }
      if (
        isNaN(Number(triggerCondition.period)) ||
        triggerCondition.period < 1 ||
        typeof triggerCondition.period !== "number"
      ) {
        alertErrors.push(
          `Alert - ${index}: Period should be a positive number greater than 0 and should be a number.`,
        );
      }

      const validOperators = [
        "=",
        "!=",
        ">=",
        "<=",
        ">",
        "<",
        "Contains",
        "NotContains",
      ];
      if (!validOperators.includes(triggerCondition.operator)) {
        alertErrors.push(
          `Alert - ${index}: Operator should be one of: '=', '!=', '>=', '<=', '>', '<', 'Contains', 'NotContains'.`,
        );
      }

      if (
        isNaN(Number(triggerCondition.frequency)) ||
        triggerCondition.frequency < 1 ||
        typeof triggerCondition.frequency !== "number"
      ) {
        alertErrors.push(
          `Alert - ${index}: Frequency should be a positive number greater than 0 and should be a number.`,
        );
      }

      if (triggerCondition.cron && typeof triggerCondition.cron !== "string") {
        alertErrors.push(
          `Alert - ${index}: Cron expression should be a string.`,
        );
      }

      if (
        isNaN(Number(triggerCondition.threshold)) ||
        triggerCondition.threshold < 1 ||
        typeof triggerCondition.threshold !== "number"
      ) {
        alertErrors.push(
          `Alert - ${index}: Threshold should be a positive number greater than 0 and should be a number.`,
        );
      }

      if (
        isNaN(Number(triggerCondition.silence)) ||
        triggerCondition.silence < 1 ||
        typeof triggerCondition.silence !== "number"
      ) {
        alertErrors.push(
          `Alert - ${index}: Silence should be a positive number greater than 0 and should be a number.`,
        );
      }

      if (
        triggerCondition.frequency_type !== "minutes" ||
        typeof triggerCondition.frequency_type !== "string"
      ) {
        alertErrors.push(
          `Alert - ${index}: Frequency Type must be 'minutes' and should be a string.`,
        );
      }

      if (
        !input.destinations ||
        !Array.isArray(input.destinations) ||
        input.destinations.length === 0
      ) {
        alertErrors.push({
          message: `Alert - ${index}: Destinations are required and should be an array.`,
          field: "destination_name",
        });
      }

      if (typeof input.enabled !== "boolean") {
        alertErrors.push(`Alert - ${index}: Enabled should be Boolean.`);
      }

      if (
        input.tz_offset &&
        (typeof input.tz_offset !== "number" || input.tz_offset < 0)
      ) {
        alertErrors.push(
          `Alert - ${index}: Timezone offset should be a number.`,
        );
      }

      input.destinations.forEach((destination: any) => {
        if (!checkDestinationInList(props.destinations, destination)) {
          alertErrors.push({
            message: `Alert - ${index}: "${destination}" destination does not exist`,
            field: "destination_name",
          });
        }
      });

      // Log all alert errors at the end
      if (alertErrors.length > 0) {
        alertErrorsToDisplay.value.push(alertErrors);
        return false;
      }

      return true;
    };

    const checkDestinationInList = (
      destinations: any,
      destinationName: any,
    ) => {
      const destinationsList = destinations.map(
        (destination: any) => destination.name,
      );
      return destinationsList.includes(destinationName);
    };

    const checkAlertsInList = (alerts: any, alertName: any) => {
      const alertsList = alerts.map((alert: any) => alert.name);
      return alertsList.includes(alertName);
    };

    const createAlert = async (input: any, index: any) => {
      try {
        await alertsService.create(
          store.state.selectedOrganization.identifier,
          input.stream_name,
          input.stream_type,
          input,
        );

        // Success
        alertCreators.value.push({
          message: `Alert - ${index}: "${input.name}" created successfully \nNote: please remove the created alert object ${input.name} from the json file`,
          success: true,
        });

        // Emit update after each successful creation
        emit("update:alerts");
        
        return true;
      } catch (error: any) {
        // Failure
        alertCreators.value.push({
          message: `Alert - ${index}: "${input.name}" creation failed --> \n Reason: ${error?.response?.data?.message || "Unknown Error"}`,
          success: false,
        });
        return false;
      }
    };

    const onSubmit = (e: any) => {
      e.preventDefault();
    };
    const updateStreams = async (streamType: string, index: number) => {
      jsonArrayOfObj.value[index].stream_type = streamType;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
      
      try {
        const streamResponse: any = await getStreams(streamType, false);
        streamList.value = streamResponse.list.map(
          (stream: any) => stream.name,
        );
      } catch (error) {
        console.error('Error fetching streams:', error);
      }
    };
    const filterDestinations = (val: string, update: Function) => {
        if (val === "") {
          update(() => {
            filteredDestinations.value = getFormattedDestinations.value;
          });
          return;
        }

        update(() => {
          filteredDestinations.value = getFormattedDestinations.value.filter((destination: string) =>
            destination.toLowerCase().includes(val.toLowerCase())
          );
        });
      };

    const toggleDestination = (destination: string, index: number) => {
      if (!userSelectedDestinations.value[index]) {
        userSelectedDestinations.value[index] = [];
      }
      
      const destinations = userSelectedDestinations.value[index];
      const destinationIndex = destinations.indexOf(destination);
      
      if (destinationIndex === -1) {
        destinations.push(destination);
      } else {
        destinations.splice(destinationIndex, 1);
      }
      
      updateUserSelectedDestinations(destinations, index);
    };

    return {
      t,
      jsonStr,
      importJson,
      onSubmit,
      router,
      q,
      templateErrorsToDisplay,
      destinationErrorsToDisplay,
      alertErrorsToDisplay,
      tempalteCreators,
      destinationCreators,
      alertCreators,
      queryEditorPlaceholderFlag,
      splitterModel,
      tabs,
      activeTab,
      userSelectedDestinations,
      getFormattedDestinations,
      jsonArrayOfObj,
      streamList,
      userSelectedStreamName,
      updateStreamFields,
      updateAlertName,
      jsonFiles,
      updateActiveTab,
      url,
      userSelectedAlertName,
      streamTypes,
      userSelectedStreamType,
      updateStreams,
      streams,
      filterDestinations,
      filteredDestinations,
      updateUserSelectedDestinations,
      toggleDestination,
    };
  },
  components: {
    QueryEditor,
    AppTabs,
  },
});
</script>

<style scoped lang="scss">
.empty-query .monaco-editor-background {
  background-image: url("../../assets/images/common/query-editor.png");
  background-repeat: no-repeat;
  background-size: 115px;
}

.empty-function .monaco-editor-background {
  background-image: url("../../assets/images/common/vrl-function.png");
  background-repeat: no-repeat;
  background-size: 170px;
}
.editor-container {
  height: calc(78vh - 20px) !important;
}
.editor-container-url {
  .monaco-editor {
    height: calc(70vh - 16px) !important; /* Total editor height */
    overflow: auto; /* Allows scrolling if content overflows */
    resize: none; /* Remove resize behavior */
  }
}
.editor-container-json {
  .monaco-editor {
    height: calc(68vh - 20px) !important; /* Total editor height */
    overflow: auto; /* Allows scrolling if content overflows */
    resize: none; /* Remove resize behavior */
  }
}
.monaco-editor {
  height: calc(81vh - 14px) !important; /* Total editor height */
  overflow: auto; /* Allows scrolling if content overflows */
  resize: none; /* Remove resize behavior */
}
.error-report-container {
  height: calc(78vh - 24px) !important; /* Total editor height */
  overflow: auto; /* Allows scrolling if content overflows */
  resize: none;
}
.error-container {
  display: flex;
  overflow-y: auto;

  flex-direction: column;
  border: 1px solid #ccc;
  height: calc(100% - 100px) !important; /* Total container height */
}

.error-section {
  padding: 10px;
  margin-bottom: 10px;
}

.section-title {
  font-size: 16px;
  margin-bottom: 10px;
  text-transform: uppercase;
}

.error-list {
}

.error-item {
  padding: 5px 0px;
  font-size: 14px;
}
.report-list-tabs {
  height: fit-content;

  :deep(.rum-tabs) {
    border: 1px solid #464646;
  }

  :deep(.rum-tab) {
    &:hover {
      background: #464646;
    }

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}
.report-list-tabs {
  height: fit-content;

  :deep(.rum-tabs) {
    border: 1px solid #eaeaea;
    height: fit-content;
    border-radius: 4px;
    overflow: hidden;
  }

  :deep(.rum-tab) {
    width: fit-content !important;
    padding: 4px 12px !important;
    border: none !important;

    &:hover {
      background: #eaeaea;
    }

    &.active {
      background: #5960b2;
      color: #ffffff !important;
    }
  }
}
</style>
