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
            @click="arrowBackFn"
            icon="arrow_back_ios_new"
          />
          <div class="text-h6 q-ml-md">Import Destination</div>
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

    <div class="flex" style="width: 100%">
      <q-splitter
        class="logs-search-splitter"
        no-scroll
        v-model="splitterModel"
        :style="{
          width: '100%',
          height: '100%',
        }"
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
              v-if="destinationErrorsToDisplay.length > 0"
              class="text-center text-h6"
            >
              Error Validations
            </div>
            <div v-else class="text-center text-h6">Output Messages</div>
            <q-separator class="q-mx-md q-mt-md" />
            <div class="error-report-container">
              <!-- Alert Errors Section -->
              <div
                class="error-section"
                v-if="destinationErrorsToDisplay.length > 0"
              >
                <div class="error-list">
                  <!-- Iterate through the outer array -->
                  <div
                    v-for="(errorGroup, index) in destinationErrorsToDisplay"
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
                          errorMessage.field == 'destination_name'
                        "
                      >
                        {{ errorMessage.message }}

                        <div style="width: 300px">
                          <q-input
                            v-model="userSelectedDestinationName[index]"
                            :label="t('destination.name') + ' *'"
                            color="input-border"
                            bg-color="input-bg"
                            class="showLabelOnTop"
                            stack-label
                            outlined
                            filled
                            dense
                            tabindex="0"
                            @update:model-value="
                              updateDestinationName(userSelectedDestinationName[index], index)
                            "
                          />
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'url'
                        "
                      >
                        {{ errorMessage.message }}

                        <div style="width: 300px">
                          <q-input
                            v-model="userSelectedDestinationUrl[index]"
                            :label="'Destination URL *'"
                            color="input-border"
                            bg-color="input-bg"
                            class="showLabelOnTop"
                            stack-label
                            outlined
                            filled
                            dense
                            tabindex="0"
                            @update:model-value="
                              updateDestinationUrl(userSelectedDestinationUrl[index], index)
                            "
                          />
                        </div>
                      </span>
                      <!-- Check if the errorMessage is an object, if so, display the 'message' property -->
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'type'
                        "
                      >
                        {{ errorMessage.message }}
                        <div style="width: 300px">
                          <q-select
                            v-model="userSelectedDestinationType[index]"
                            :options="destinationTypes"
                            :label="'Destination Type *'"
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
                            @update:model-value="
                              updateDestinationType(userSelectedDestinationType[index], index)
                            "
                            behavior="menu"
                          />
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'method'
                        "
                      >
                        {{ errorMessage.message }}
                        <div style="width: 300px">
                          <q-select
                            v-model="userSelectedDestinationMethod[index]"
                            :options="destinationMethods"
                            :label="'Destination Method *'"
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
                            @update:model-value="
                              updateDestinationMethod(userSelectedDestinationMethod[index], index)
                            "
                            behavior="menu"
                          />
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'template_name'
                        "
                      >
                        {{ errorMessage.message }}
                        <div>
                          <q-select
                            v-model="userSelectedTemplates[index]"
                            :options="filteredTemplates"
                            label="Templates *"
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
                            input-debounce="400"
                            behavior="menu"
                            hide-selected
                            fill-input
                            @filter="filterTemplates"
                            :rules="[
                              (val) => !!val || 'Field is required!',
                            ]"
                            style="width: 300px"
                            @update:model-value="updateDestinationTemplate($event, index)"
                          >
                          </q-select>
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'email_input'
                        "
                      >
                        {{ errorMessage.message }}
                        <div style="width: 300px">
                          <q-input
                            v-model="userSelectedEmails[index]"
                            :label="'Emails (comma separated) *'"
                            color="input-border"
                            bg-color="input-bg"
                            class="showLabelOnTop"
                            stack-label
                            outlined
                            filled
                            dense
                            tabindex="0"
                            @update:model-value="
                              updateDestinationEmails(userSelectedEmails[index], index)
                            "
                          />
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'skip_tls_verify'
                        "
                      >
                        {{ errorMessage.message }}
                        <div style="width: 300px">
                          <q-toggle
                            :model-value="userSelectedSkipTlsVerify[index] ?? false"
                            :label="t('alert_destinations.skip_tls_verify')"
                            class="q-mt-sm"
                            @update:model-value="
                              updateSkipTlsVerify($event, index)
                            "
                          />
                        </div>
                      </span>
                      <span class="text-red" v-else>{{ errorMessage }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <div class="error-section" v-if="destinationCreators.length > 0">
                <div class="section-title text-primary">Alert Creation</div>
                <div
                  class="error-list"
                  v-for="(val, index) in destinationCreators"
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
import { useRouter } from "vue-router";

import { useQuasar } from "quasar";

import QueryEditor from "../QueryEditor.vue";

import destinationService from "@/services/alert_destination";

import AppTabs from "../common/AppTabs.vue";
import axios from "axios";

export default defineComponent({
  name: "ImportDestination",
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
    type destinationCreator = {
      message: string;
      success: boolean;
    }[];

    type destinationErrors = (ErrorMessage | string)[][];
    const { t } = useI18n();
    const store = useStore();
    const router = useRouter();

    const jsonStr: any = ref("");
    const q = useQuasar();

    const destinationErrorsToDisplay = ref<destinationErrors>([]);

    const userSelectedTemplates = ref<string[]>([]);
    const destinationTypes = ["http", "email"];
    const destinationMethods = ["post", "get", "put"];

    const destinationCreators = ref<destinationCreator>([]);
    const queryEditorPlaceholderFlag = ref(true);
    const userSelectedDestinationType = ref([]);
    const userSelectedDestinationMethod = ref([]);
    const jsonFiles = ref(null);
    const userSelectedDestinationName = ref([]);
    const userSelectedDestinationUrl = ref([]);
    const url = ref("");
    const jsonArrayOfObj = ref<any[]>([{}]);
    const activeTab = ref("import_json_file");
    const splitterModel = ref(60);
    const filteredTemplates = ref<string[]>([]);

    const userSelectedSkipTlsVerify = ref<boolean[]>([]);

    const getFormattedTemplates = computed(() => {
      return props.templates
        .filter((template: any) => {
          // Get the current destination type
          const currentDestinationType = jsonArrayOfObj.value[destinationErrorsToDisplay.value.length - 1]?.type;
          
          if (currentDestinationType === "email" && template.type === "email") {
            return true;
          } else if (currentDestinationType !== "email") {
            return true;
          }
          return false;
        })
        .map((template: any) => template.name);
    });

    const userSelectedEmails = ref([]);

    const updateDestinationType = (type: any, index: number) => {
      jsonArrayOfObj.value[index].type = type;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };
    const updateDestinationMethod = (method: any, index: number) => {
      jsonArrayOfObj.value[index].method = method;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    const updateDestinationName = (destinationName: string, index: number) => {
      jsonArrayOfObj.value[index].name = destinationName;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };
    const updateDestinationUrl = (url: any, index: number) => {
      jsonArrayOfObj.value[index].url = url;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    const updateDestinationTemplate = (template: string, index: number) => {
      jsonArrayOfObj.value[index].template = template;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    const updateDestinationEmails = (emails: string, index: number) => {
      // Split comma-separated emails and trim whitespace
      const emailArray = emails.split(',').map(email => email.trim());
      jsonArrayOfObj.value[index].emails = emailArray;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    const updateSkipTlsVerify = (value: boolean, index: number) => {
      userSelectedSkipTlsVerify.value[index] = value;
      jsonArrayOfObj.value[index].skip_tls_verify = value;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    const filterTemplates = (val: string, update: Function) => {
      if (val === "") {
        update(() => {
          filteredTemplates.value = getFormattedTemplates.value;
        });
        return;
      }

      update(() => {
        const needle = val.toLowerCase();
        filteredTemplates.value = getFormattedTemplates.value.filter(
          (template: string) => template.toLowerCase().includes(needle)
        );
      });
    };

    watch(jsonFiles, async (newVal: any, oldVal: any) => {
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

    const importJson = async () => {
      destinationErrorsToDisplay.value = [];
      destinationCreators.value = [];

      try {
        if ((!jsonStr.value || jsonStr.value.trim() === "") && !url.value) {
          throw new Error("JSON string is empty");
        } else {
          const parsedJson = JSON.parse(jsonStr.value);
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

      let hasErrors = false;
      let successCount = 0;
      const totalCount = jsonArrayOfObj.value.length;

      for (const [index, jsonObj] of jsonArrayOfObj.value.entries()) {
        const success = await processJsonObject(jsonObj, index + 1);
        if (!success) {
          hasErrors = true;
        } else {
          successCount++;
        }
      }

      // Only redirect and show success message if ALL destinations were imported successfully
      if (successCount === totalCount) {
        q.notify({
          message: `Successfully imported ${successCount} destination${successCount > 1 ? 's' : ''}`,
          color: "positive",
          position: "bottom",
          timeout: 2000,
        });
        router.push({
          name: "alertDestinations",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
    };

    const processJsonObject = async (jsonObj: any, index: number) => {
      try {
        const isValidDestination = await validateDestinationInputs(
          jsonObj,
          0,
          index,
        );
        if (!isValidDestination) {
          return false;
        }

        if (destinationErrorsToDisplay.value.length === 0) {
          const hasCreatedDestination = await createDestination(jsonObj, index);
          return hasCreatedDestination;
        }
        return false;
      } catch (e: any) {
        q.notify({
          message: "Error importing Destination please check the JSON",
          color: "negative",
          position: "bottom",
          timeout: 2000,
        });
        return false;
      }
    };

    const validateDestinationInputs = async (
      input: any,
      destinationIndex: number = 1,
      index: any,
    ) => {
      let destinationErrors: (string | { message: string; field: string })[] =
        [];

      // Check if 'url' is required for webhook and should not exist for email
      if (!input.hasOwnProperty("type") && !input.url) {
        destinationErrors.push(
          `Destination - ${index} 'url' is required for webhook`,
        );
      }

      if (input.type === "email" && input.url) {
        destinationErrors.push(
          `Destination - ${index} 'url' should not be provided for email`,
        );
      }
      if (checkDestinationInList(props.destinations, input.name)) {
        destinationErrors.push({
          message: `Destination - ${index} "${input.name}" already exists`,
          field: "destination_name",
        });
      }
      if (
        input.type == "http" &&
        (typeof input.url != "string" || input.url == "")
      ) {
        destinationErrors.push({
          message: `Destination - ${index} 'URL' should not be empty for type 'http'`,
          field: "url",
        });
      }
      if (
        !input.method ||
        (input.method !== "post" &&
          input.method !== "get" &&
          input.method !== "put")
      ) {
        destinationErrors.push({
          message: `Destination - ${index} 'method' is required and should be either 'post', 'get', or 'put'`,
          field: "method",
        });
      }

      // Update template validation to consider type-specific templates
      const availableTemplates = props.templates
        .filter((template: any) => {
          if (input.type === "email" && template.type === "email") return true;
          else if (input.type !== "email") return true;
          return false;
        })
        .map((template: any) => template.name);

      if (!availableTemplates.includes(input.template)) {
        destinationErrors.push({
          message: `Destination - ${index} template "${input.template}" does not exist for type "${input.type}"`,
          field: "template_name",
        });
      }

      // Check type for email and it should be present only for email
      if (input.type != "email" && input.type != "http") {
        destinationErrors.push({
          message: `Destination - ${index} 'type' should be email or http`,
          field: "type",
        });
      }

      // Check if 'method' is required for both webhook and email

      // Check if 'skip_tls_verify' is required for both webhook and email, and it should be a boolean
      if (
        input.skip_tls_verify === undefined ||
        typeof input.skip_tls_verify !== "boolean"
      ) {
        destinationErrors.push({
          message: `Destination - ${index} 'skip_tls_verify' is required and should be a boolean value`,
          field: "skip_tls_verify",
        });
      }

      // Check if 'headers' is required for webhook but not for email
      if (
        !input.hasOwnProperty("type") &&
        Object.keys(input.headers).length === 0
      ) {
        destinationErrors.push(
          `Destination - ${index} 'headers' is required for webhook`,
        );
      }

      if (input.type === "email" && Object.keys(input.headers).length !== 0) {
        destinationErrors.push(
          `Destination - ${index} 'headers' should not be provided for email`,
        );
      }

      // Check if 'name' is required for both webhook and email
      if (!input.name || typeof input.name !== "string") {
        destinationErrors.push(
          `Destination - ${index} 'name' is required and should be a string`,
        );
      }

      // 'emails' should be required for email type and should be an array of strings
      if (input.type === "email") {
        if (
          !Array.isArray(input.emails) ||
          input.emails.some((email: any) => typeof email !== "string") ||
          input.emails.length === 0
        ) {
          destinationErrors.push({
            message: `Destination - ${index} 'emails' should be an array of strings for email type`,
            field: "email_input",
          });
        }
      }

      // Check if 'template' is required for both webhook and email
      if (!input.template || typeof input.template !== "string") {
        destinationErrors.push(
          `Destination - ${index} 'template' is required and should be a string`,
        );
      }

      // Log all destination errors at the end if any exist
      if (destinationErrors.length > 0) {
        destinationErrorsToDisplay.value.push(destinationErrors);
        return false;
      }

      // If all validations pass
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
    const checkTemplatesInList = (templates: any, templateName: any) => {
      const templatesList = templates.map((template: any) => template.name);
      return templatesList.includes(templateName);
    };

    const createDestination = async (input: any, index: number) => {
      try {
        await destinationService.create({
          org_identifier: store.state.selectedOrganization.identifier,
          destination_name: input.name,
          data: input,
        });

        destinationCreators.value.push({
          message: `Destination - ${index}: "${input.name}" created successfully \nNote: please remove the created desination object ${input.name} from the json file`,
          success: true,
        });

        // Emit update after each successful creation
        emit("update:destinations");
        
        return true;
      } catch (error: any) {
        destinationCreators.value.push({
          message: `Destination - ${index}: "${input.name}" creation failed --> \n Reason: ${error?.response?.data?.message || "Unknown Error"}`,
          success: false,
        });
        return false;
      }
    };
    const arrowBackFn = () => {
      router.push({
        name: "alertDestinations",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    const onSubmit = (e: any) => {
      e.preventDefault();
    };

    return {
      t,
      jsonStr,
      importJson,
      onSubmit,
      router,
      q,
      destinationErrorsToDisplay,
      destinationCreators,
      queryEditorPlaceholderFlag,
      splitterModel,
      tabs,
      activeTab,
      userSelectedTemplates,
      getFormattedTemplates,
      jsonArrayOfObj,
      userSelectedDestinationType,
      userSelectedDestinationMethod,
      updateDestinationType,
      updateDestinationMethod,
      updateDestinationName,
      updateDestinationUrl,
      jsonFiles,
      updateActiveTab,
      arrowBackFn,
      userSelectedDestinationName,
      userSelectedDestinationUrl,
      destinationTypes,
      destinationMethods,
      url,
      updateDestinationTemplate,
      userSelectedEmails,
      updateDestinationEmails,
      filterTemplates,
      filteredTemplates,
      userSelectedSkipTlsVerify,
      updateSkipTlsVerify,
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
  height: calc(70vh - 20px) !important;
}
.editor-container-url {
  .monaco-editor {
    height: calc(66vh - 8px) !important; /* Total editor height */
    overflow: auto; /* Allows scrolling if content overflows */
    resize: none; /* Remove resize behavior */
  }
}
.editor-container-json {
  .monaco-editor {
    height: calc(65vh - 20px) !important; /* Total editor height */
    overflow: auto; /* Allows scrolling if content overflows */
    resize: none; /* Remove resize behavior */
  }
}
.monaco-editor {
  height: calc(60vh - 14px) !important; /* Total editor height */
  overflow: auto; /* Allows scrolling if content overflows */
  resize: none; /* Remove resize behavior */
}
.error-report-container {
  height: calc(70vh - 8px) !important; /* Total editor height */
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
