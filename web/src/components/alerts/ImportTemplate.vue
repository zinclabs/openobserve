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
          <div class="text-h6 q-ml-md">Import Template</div>
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
              v-if="templateErrorsToDisplay.length > 0"
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
                v-if="templateErrorsToDisplay.length > 0"
              >
                <div class="error-list">
                  <!-- Iterate through the outer array -->
                  <div
                    v-for="(errorGroup, index) in templateErrorsToDisplay"
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
                          errorMessage.field == 'template_name'
                        "
                      >
                        {{ errorMessage.message }}
                        <div style="width: 300px">
                          <q-input
                            v-model="userSelectedTemplateNames[index]"
                            :label="'Template Name *'"
                            color="input-border"
                            bg-color="input-bg"
                            class="showLabelOnTop"
                            stack-label
                            outlined
                            filled
                            dense
                            tabindex="0"
                            @update:model-value="
                              updateTemplateName($event, index)
                            "
                          />
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'body'
                        "
                      >
                        {{ errorMessage.message }}
                        <div style="width: 300px">
                          <q-input
                            v-model="userSelectedTemplateBodies[index]"
                            :label="'Template Body *'"
                            color="input-border"
                            bg-color="input-bg"
                            class="showLabelOnTop"
                            stack-label
                            outlined
                            filled
                            dense
                            tabindex="0"
                            @update:model-value="
                              updateTemplateBody($event, index)
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
                            v-model="userSelectedTemplateTypes[index]"
                            :options="destinationTypes"
                            :label="'Template Type *'"
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
                              updateTemplateType($event, index)
                            "
                            behavior="menu"
                          />
                        </div>
                      </span>
                      <span
                        class="text-red"
                        v-else-if="
                          typeof errorMessage === 'object' &&
                          errorMessage.field == 'title' &&
                          jsonArrayOfObj.value[index]?.type === 'email'
                        "
                      >
                        {{ errorMessage.message }}
                        <div style="width: 300px">
                          <q-input
                            v-model="userSelectedTemplateTitles[index]"
                            :label="'Template Title *'"
                            color="input-border"
                            bg-color="input-bg"
                            class="showLabelOnTop"
                            stack-label
                            outlined
                            filled
                            dense
                            tabindex="0"
                            @update:model-value="
                              updateTemplateTitle($event, index)
                            "
                          />
                        </div>
                      </span>
                      <span class="text-red" v-else>{{ errorMessage }}</span>
                    </div>
                  </div>
                </div>
              </div>

              <div class="error-section" v-if="tempalteCreators.length > 0">
                <div class="section-title text-primary">Alert Creation</div>
                <div
                  class="error-list"
                  v-for="(val, index) in tempalteCreators"
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

export default defineComponent({
  name: "ImportTemplate",
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
    type templateCreator = {
      message: string;
      success: boolean;
    }[];
    type templateErrors = (ErrorMessage | string)[][];
    const { t } = useI18n();
    const store = useStore();
    const router = useRouter();

    const jsonStr = ref<any>("");
    const q = useQuasar();

    const templateErrorsToDisplay = ref<templateErrors>([]);
    const userSelectedTemplates = ref<any[]>([]);
    const destinationTypes = ["http", "email"];
    const destinationMethods = ["post", "get", "put"];

    const tempalteCreators = ref<templateCreator>([]);

    const queryEditorPlaceholderFlag = ref(true);
    const streamList = ref([]);
    const userSelectedTemplateTypes = ref<any[]>([]);
    const jsonFiles = ref(null);
    const userSelectedTemplateNames = ref<any[]>([]);
    const userSelectedTemplateBodies = ref<any[]>([]);
    const userSelectedTemplateTitles = ref<any[]>([]);
    const userSelectedDestinationUrl = ref("");
    const jsonArrayOfObj = ref<any>([{}]);
    const activeTab = ref("import_json_file");
    const splitterModel = ref(60);
    const url = ref("");
    const getFormattedTemplates = computed(() => {
      return props.templates.map((template: any) => {
        return template.name;
      });
    });

    watch(
      () => userSelectedTemplates.value,
      (newVal, oldVal) => {
        if (newVal) {
          jsonArrayOfObj.value.template = newVal;
          jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
        }
      },
    );

    const updateTemplateType = (type: string, index: number) => {
      userSelectedTemplateTypes.value[index] = type;
      jsonArrayOfObj.value[index].type = type;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    const updateTemplateName = (name: any, index: number) => {
      userSelectedTemplateNames.value[index] = name;
      jsonArrayOfObj.value[index].name = name;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };
    const updateTemplateBody = (body: any, index: number) => {
      userSelectedTemplateBodies.value[index] = body;
      jsonArrayOfObj.value[index].body = body;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };
    const updateTemplateTitle = (title: any, index: number) => {
      userSelectedTemplateTitles.value[index] = title;
      jsonArrayOfObj.value[index].title = title;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };
    const updateDestinationUrl = (url: any) => {
      jsonArrayOfObj.value.url = url;
      jsonStr.value = JSON.stringify(jsonArrayOfObj.value, null, 2);
    };

    watch(jsonFiles, async (newVal: any, oldVal: any) => {
      if (newVal) {
        let combinedJson: any[] = [];
        
        for (const file of newVal) {
          try {
            const content = await readFileContent(file);
            const parsedContent = JSON.parse(content);
            
            // Handle both array and single object cases
            if (Array.isArray(parsedContent)) {
              combinedJson = [...combinedJson, ...parsedContent];
            } else {
              combinedJson.push(parsedContent);
            }
          } catch (error) {
            q.notify({
              message: `Error reading file ${file.name}: Invalid JSON format`,
              color: "negative",
              position: "bottom",
              timeout: 2000,
            });
            return;
          }
        }
        
        jsonStr.value = JSON.stringify(combinedJson, null, 2);
      }
    });

    // Add this helper function
    const readFileContent = (file: File): Promise<string> => {
      return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = (e: any) => resolve(e.target.result);
        reader.onerror = (e) => reject(e);
        reader.readAsText(file);
      });
    };

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
      templateErrorsToDisplay.value = [];
      tempalteCreators.value = [];

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

      // Only redirect and show success message if ALL templates were imported successfully
      if (successCount === totalCount) {
        q.notify({
          message: `Successfully imported ${successCount} template${successCount > 1 ? 's' : ''}`,
          color: "positive",
          position: "bottom",
          timeout: 2000,
        });
        router.push({
          name: "alertTemplates",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
    };

    const processJsonObject = async (jsonObj: any, index: number) => {
      try {
        const isValidTemplate = await validateTemplateInputs(jsonObj, index);
        if (!isValidTemplate) {
          return false;
        }

        if (templateErrorsToDisplay.value.length === 0) {
          const hasCreatedTemplate = await createTemplate(jsonObj, index);
          return hasCreatedTemplate;
        }
        return false;
      } catch (e: any) {
        q.notify({
          message: "Error importing Template please check the JSON",
          color: "negative",
          position: "bottom",
          timeout: 2000,
        });
        return false;
      }
    };

    const validateTemplateInputs = async (input: any, index: number) => {
      let templateErrors: (string | { message: string; field: string })[] = [];

      // Validate name using the updated props.templates
      if (!input.name || typeof input.name !== "string" || input.name.trim() === "") {
        templateErrors.push({
          message: `Template - ${index}: The "name" field is required and should be a valid string.`,
          field: "template_name",
        });
      } else if (props.templates.some((template: any) => template.name === input.name)) {
        templateErrors.push({
          message: `Template - ${index}: "${input.name}" already exists`,
          field: "template_name",
        });
      }

      // Validate type
      if (!input.type || (input.type !== "email" && input.type !== "http")) {
        templateErrors.push({
          message: `Template - ${index}: The "type" field must be either "email" or "http"`,
          field: "type",
        });
      }

      // Validate body
      if (!input.body || typeof input.body !== "string" || input.body.trim() === "") {
        templateErrors.push({
          message: `Template - ${index}: The "body" field is required and should be a valid JSON string.`,
          field: "body",
        });
      } else {
        try {
          JSON.parse(input.body);
        } catch (e) {
          templateErrors.push({
            message: `Template - ${index}: The "body" field should contain a valid JSON.`,
            field: "body",
          });
        }
      }

      // Validate title for email type
      if (input.type === "email") {
        if (!input.title || typeof input.title !== "string" || input.title.trim() === "") {
          templateErrors.push({
            message: `Template - ${index}: The "title" field is required for email type templates.`,
            field: "title",
          });
        }
      }

      if (templateErrors.length > 0) {
        templateErrorsToDisplay.value.push(templateErrors);
        return false;
      }

      return true;
    };

    const checkTemplatesInList = (templates: any, templateName: any) => {
      const templatesList = templates.map((template: any) => template.name);
      return templatesList.includes(templateName);
    };

    const createTemplate = async (input: any, index: number) => {
      try {
        await templateService.create({
          org_identifier: store.state.selectedOrganization.identifier,
          template_name: input.name,
          data: {
            name: input.name.trim(),
            body: input.body,
            type: input.type,
            title: input.title,
          },
        });

        tempalteCreators.value.push({
          message: `Template - ${index}: "${input.name}" created successfully`,
          success: true,
        });

        // Emit update after each successful creation
        emit("update:templates");
        
        return true;
      } catch (error: any) {
        tempalteCreators.value.push({
          message: `Template - ${index}: "${input.name}" creation failed --> \n Reason: ${error?.response?.data?.message || "Unknown Error"}`,
          success: false,
        });
        return false;
      }
    };

    const arrowBackFn = () => {
      router.push({
        name: "alertTemplates",
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
      templateErrorsToDisplay,
      tempalteCreators,
      queryEditorPlaceholderFlag,
      splitterModel,
      tabs,
      activeTab,
      userSelectedTemplates,
      getFormattedTemplates,
      jsonArrayOfObj,
      streamList,
      userSelectedTemplateTypes,
      updateTemplateType,
      updateTemplateName,
      updateDestinationUrl,
      jsonFiles,
      updateActiveTab,
      arrowBackFn,
      userSelectedTemplateNames,
      userSelectedTemplateBodies,
      userSelectedTemplateTitles,
      userSelectedDestinationUrl,
      destinationTypes,
      destinationMethods,
      updateTemplateBody,
      updateTemplateTitle,
      url,
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
  height: calc(60vh - 8px) !important; /* Total editor height */
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
