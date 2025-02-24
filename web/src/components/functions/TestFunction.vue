<template>
  <div
    data-test="test-function-section"
    class="tw-flex tw-items-center tw-flex-wrap q-pb-sm"
  >
    <div
      data-test="test-function-query-section"
      class="test-function-query-container tw-w-[100%] tw-mt-2"
    >
      <q-form ref="querySelectionRef" @submit="getResults">
        <FullViewContainer
          data-test="test-function-query-title-section"
          name="function"
          v-model:is-expanded="expandState.query"
          :label="t('common.query')"
        >
          <template #left>
            <q-icon
              v-if="!!sqlQueryErrorMsg"
              name="info"
              class="tw-text-red-600 tw-mx-1 tw-cursor-pointer"
              size="16px"
            >
              <q-tooltip
                anchor="center right"
                self="center left"
                :offset="[10, 10]"
                class="tw-text-[12px]"
              >
                {{ sqlQueryErrorMsg }}
              </q-tooltip>
            </q-icon>
          </template>
          <template #right>
            <q-btn
              :label="t('search.runQuery')"
              class="test-function-run-query-btn text-bold tw-ml-[12px] no-border"
              padding="sm md"
              icon="search"
              no-caps
              dense
              size="xs"
              color="primary"
              type="submit"
              :disabled="!selectedStream.name || !inputQuery || loading.events"
            />
          </template>
        </FullViewContainer>
        <div
          class="tw-flex tw-items-center tw-flex-wrap q-px-md q-py-sm tw-w-[100%]"
          :class="
            store.state.theme === 'dark' ? 'tw-bg-gray-950' : ' tw-bg-white'
          "
          v-show="expandState.query"
          data-test="test-function-query-editor-section"
        >
          <div class="function-stream-select-input tw-w-[120px] q-pr-md">
            <div
              class="tw-text-[12px]"
              :class="
                store.state.theme === 'dark'
                  ? 'tw-text-gray-200'
                  : 'tw-text-gray-700'
              "
            >
              {{ t("alerts.streamType") + " *" }}
            </div>

            <q-select
              v-model="selectedStream.type"
              :options="streamTypes"
              :popup-content-style="{ textTransform: 'lowercase' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-xs showLabelOnTop no-case"
              emit-value
              stack-label
              outlined
              filled
              dense
              @update:model-value="updateStreams()"
              style="width: 100px"
            />
          </div>
          <div class="function-stream-select-input tw-w-[300px]">
            <div
              class="tw-text-[12px]"
              :class="
                store.state.theme === 'dark'
                  ? 'tw-text-gray-200'
                  : 'tw-text-gray-700'
              "
            >
              {{ t("alerts.stream_name") + " *" }}
            </div>
            <q-select
              v-model="selectedStream.name"
              :options="filteredStreams"
              :popup-content-style="{ textTransform: 'lowercase' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-xs showLabelOnTop no-case"
              stack-label
              outlined
              filled
              dense
              use-input
              hide-selected
              fill-input
              :loading="isFetchingStreams"
              style="min-width: 120px"
              @filter="filterStreams"
              @update:model-value="updateQuery"
              :rules="[(val: any) => !!val || '']"
            />
          </div>
          <div class="functions-duration-input tw-w-[330px]">
            <div
              class="tw-text-[12px]"
              :class="
                store.state.theme === 'dark'
                  ? 'tw-text-gray-200'
                  : 'tw-text-gray-700'
              "
            >
              {{ t("common.duration") + " *" }}
            </div>
            <DateTime label="Start Time" class="q-py-xs tw-w-full" />
          </div>

          <div
            class="tw-text-[12px] tw-w-[100%] q-mt-xs"
            :class="
              store.state.theme === 'dark'
                ? 'tw-text-gray-200'
                : 'tw-text-gray-700'
            "
          >
            {{ t("common.query") + " *" }}
          </div>
          <div
            class="tw-border-[1px] tw-border-gray-200 tw-relative tw-w-[100%]"
          >
            <query-editor
              data-test="vrl-function-test-sql-editor"
              ref="queryEditorRef"
              editor-id="test-function-query-input-editor"
              class="monaco-editor"
              v-model:query="inputQuery"
              language="sql"
            />
            <div
              class="text-negative q-pa-xs invalid-sql-error tw-min-h-[22px]"
            >
              <span v-show="!!sqlQueryErrorMsg" class="tw-text-[13px]">
                Error: {{ sqlQueryErrorMsg }}</span
              >
            </div>
          </div>
        </div>
      </q-form>
    </div>
  </div>
  <div>
    <div>
      <FullViewContainer
        data-test="test-function-input-title-section"
        name="function"
        v-model:is-expanded="expandState.events"
        :label="t('common.events')"
      >
        <template #left>
          <div
            v-if="loading.events"
            class="text-weight-bold tw-flex tw-items-center tw-text-gray-500 tw-ml-2 tw-text-[13px]"
          >
            <q-spinner-hourglass size="18px" />
            <div class="tw-relative tw-top-[2px]">
              {{ t("confirmDialog.loading") }}
            </div>
          </div>
          <q-icon
            v-if="!!eventsErrorMsg"
            name="info"
            class="tw-text-red-600 tw-mx-1 tw-cursor-pointer"
            size="16px"
          >
            <q-tooltip
              anchor="center right"
              self="center left"
              :offset="[10, 10]"
              class="tw-text-[12px]"
            >
              {{ eventsErrorMsg }}
            </q-tooltip>
          </q-icon>
        </template>
      </FullViewContainer>
      <div
        v-show="expandState.events"
        class="tw-border-[1px] tw-border-gray-200 tw-relative"
        data-test="test-function-input-editor-section"
      >
        <query-editor
          data-test="vrl-function-test-events-editor"
          ref="eventsEditorRef"
          editor-id="test-function-events-input-editor"
          class="monaco-editor test-function-input-editor"
          v-model:query="inputEvents"
          language="json"
        />
      </div>
    </div>
    <div class="q-mt-sm">
      <FullViewContainer
        name="function"
        v-model:is-expanded="expandState.output"
        :label="t('common.output')"
        data-test="test-function-output-title-section"
      >
        <template #left>
          <div
            v-if="loading.output"
            class="text-subtitle2 text-weight-bold tw-flex tw-items-center tw-text-gray-500 tw-ml-2 tw-text-[13px]"
          >
            <q-spinner-hourglass size="18px" />
            <div class="tw-relative tw-top-[2px]">
              {{ t("confirmDialog.loading") }}
            </div>
          </div>

          <q-icon
            v-if="!!outputEventsErrorMsg"
            name="info"
            class="tw-text-red-600 tw-mx-1 tw-cursor-pointer"
            size="16px"
          >
            <q-tooltip
              anchor="center right"
              self="center left"
              :offset="[10, 10]"
              class="tw-text-[12px]"
            >
              {{ outputEventsErrorMsg }}
            </q-tooltip>
          </q-icon>
        </template>
      </FullViewContainer>

      <div
        v-show="expandState.output"
        class="tw-border-[1px] tw-border-gray-200 tw-relative"
        data-test="test-function-output-editor-section"
      >
        <div
          v-if="!outputEvents"
          class="tw-absolute tw-z-10 tw-flex tw-flex-col tw-justify-center tw-items-center tw-w-full tw-h-full tw-opacity-90"
        >
          <q-icon
            :name="outlinedLightbulb"
            size="40px"
            class="tw-text-orange-400"
          />
          <div
            class="tw-text-[15px] tw-text-gray-600"
            :class="
              store.state.theme === 'dark'
                ? 'tw-text-gray-200'
                : 'tw-text-gray-600'
            "
          >
            {{ outputMessage }}
          </div>
        </div>
        <query-editor
          data-test="vrl-function-test-events-output-editor"
          ref="outputEventsEditorRef"
          editor-id="test-function-events-output-editor"
          class="monaco-editor test-function-output-editor"
          v-model:query="outputEvents"
          language="json"
          read-only
        />
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import {
  onBeforeMount,
  ref,
  defineProps,
  defineExpose,
  computed,
  nextTick,
  onMounted,
} from "vue";
import { useI18n } from "vue-i18n";
import QueryEditor from "@/components/QueryEditor.vue";
import DateTime from "@/components/DateTime.vue";
import FullViewContainer from "@/components/functions/FullViewContainer.vue";
import useStreams from "@/composables/useStreams";
import { outlinedLightbulb } from "@quasar/extras/material-icons-outlined";
import useQuery from "@/composables/useQuery";
import { b64EncodeUnicode } from "@/utils/zincutils";
import searchService from "@/services/search";
import { useStore } from "vuex";
import { event, useQuasar } from "quasar";
import { getConsumableRelativeTime } from "@/utils/date";
import AppTabs from "@/components/common/AppTabs.vue";
import jstransform from "@/services/jstransform";

const props = defineProps({
  vrlFunction: {
    type: Object,
    required: true,
  },
});

const emit = defineEmits(["function-error"]);

const inputQuery = ref<string>("");
const inputEvents = ref<string>("");
const outputEvents = ref<string>("");

const dummyEvents = {
  data: {
    results: [
      {
        event: {
          _timestamp: 1735128523652186,
          job: "test",
          level: "info",
          log: "test message for openobserve",
        },
      },
      {
        event: {
          log: "test message for openobserve",
        },
        message: "Error in event",
      },
    ],
  },
};

const originalOutputEvents = ref<any>("");

const eventsErrorMsg = ref<string>("");

const queryEditorRef = ref<InstanceType<typeof QueryEditor>>();

const eventsEditorRef = ref<InstanceType<typeof QueryEditor>>();

const outputEventsEditorRef = ref<InstanceType<typeof QueryEditor>>();

const querySelectionRef = ref<any>();

const outputEventsErrorMsg = ref<string>("");

const loading = ref({
  events: false,
  output: false,
});

const dateTime = ref<any>({});

const selectedStream = ref<{
  name: string;
  type: "logs" | "metrics" | "traces";
}>({ name: "", type: "logs" });

const { getStreams } = useStreams();

const { buildQueryPayload } = useQuery();

const streamTypes = [
  { label: "Logs", value: "logs" },
  { label: "Metrics", value: "metrics" },
  { label: "Traces", value: "traces" },
];

const isFetchingStreams = ref(false);

const store = useStore();

let parser: any = null;

const { t } = useI18n();

const expandState = ref({
  stream: true,
  functions: true,
  query: false,
  events: true,
  output: true,
});

const filteredStreams = ref<any[]>([]);

const sqlQueryErrorMsg = ref<string>("");

const q = useQuasar();

const streams = ref([]);

onBeforeMount(async () => {
  await importSqlParser();
  await updateStreams();
});

onMounted(() => {
  setEventsEditor();
});

const setEventsEditor = () => {
  setTimeout(() => {
    inputEvents.value = `[{"_timestamp":1735128523652186,"job":"test","level":"info","log":"test message for openobserve"},{"_timestamp":1735128522644223,"job":"test","level":"info","log":"test message for openobserve"}]`;
    eventsEditorRef?.value?.formatDocument();
  }, 300);
};

const outputMessage = computed(() => {
  if (!outputEvents.value) {
    return "Please click Test Function to see the events";
  }

  return "";
});

const areInputValid = () => {
  if (!inputQuery.value) {
    q.notify({
      type: "negative",
      message: "Please enter a query",
      timeout: 3000,
    });
    sqlQueryErrorMsg.value = "Please enter a query";
    return false;
  }

  return true;
};

const importSqlParser = async () => {
  const useSqlParser: any = await import("@/composables/useParser");
  const { sqlParser }: any = useSqlParser.default();
  parser = await sqlParser();
};

const filterStreams = (val: string, update: any) => {
  filteredStreams.value = filterColumns(streams.value, val, update);
};

const filterColumns = (options: any[], val: String, update: Function) => {
  let filteredOptions: any[] = [];
  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
    return filteredOptions;
  }
  update(() => {
    const value = val.toLowerCase();
    filteredOptions = options.filter(
      (column: any) => column.toLowerCase().indexOf(value) > -1,
    );
  });
  return filteredOptions;
};

const updateQuery = () => {
  inputQuery.value = `SELECT * FROM "${selectedStream.value.name}"`;
  expandState.value.query = true;
};

const updateStreams = async (resetStream = true) => {
  if (resetStream) selectedStream.value.name = "";

  if (!selectedStream.value.type) return Promise.resolve();

  isFetchingStreams.value = true;
  return getStreams(selectedStream.value.type, false)
    .then((res: any) => {
      streams.value = res.list.map((data: any) => {
        return data.name;
      });

      return Promise.resolve();
    })
    .catch(() => Promise.reject())
    .finally(() => (isFetchingStreams.value = false));
};

const getResults = async () => {
  loading.value.events = true;

  const timestamps: any =
    dateTime.value.type === "relative"
      ? getConsumableRelativeTime(dateTime.value.relativeTimePeriod)
      : dateTime.value;

  const query = buildQueryPayload({
    sqlMode: true,
    streamName: selectedStream.value.name,
    timestamps,
  });

  delete query.aggs;

  // We get 15 minutes time range for the query, so reducing it by 13 minutes to get 2 minute data
  query.query.start_time = query.query.start_time + 780000000;

  // TODO: Handle the edge case when user enters limit in the query
  query.query.sql = inputQuery.value;

  query.query.from = 0;
  query.query.size = 10;

  searchService
    .search({
      org_identifier: store.state.selectedOrganization.identifier,
      query,
      page_type: "logs",
    })
    .then((res: any) => {
      expandState.value.stream = false;
      expandState.value.query = false;
      expandState.value.events = true;
      inputEvents.value = JSON.stringify(res.data.hits, null, 2);
      sqlQueryErrorMsg.value = "";
    })
    .catch((err: any) => {
      sqlQueryErrorMsg.value = err.response?.data?.message
        ? err.response?.data?.message
        : "Invalid SQL Query";

      // Show error only if it is not real time alert
      // This case happens when user enters invalid query and then switches to real time alert
      q.notify({
        type: "negative",
        message: "Invalid SQL Query : " + err.response?.data?.message,
        timeout: 3000,
      });
    })
    .finally(() => {
      loading.value.events = false;
    });
};
const isInputValid = () => {
  try {
    JSON.parse(inputEvents.value);
    return true;
  } catch (e: any) {
    eventsErrorMsg.value = `Invalid events: ${e?.message}`;
    q.notify({
      type: "negative",
      message: eventsErrorMsg.value,
      timeout: 3000,
    });
    return false;
  }
};

const processTestResults = async (results: any) => {
  expandState.value.query = false;
  expandState.value.output = true;
  originalOutputEvents.value = JSON.stringify(results?.data?.results);

  const processedEvents =
    results?.data?.results.map((event: any) => event.event || event.events) ||
    [];

  outputEvents.value = JSON.stringify(processedEvents);
  await Promise.all([
    eventsEditorRef.value?.formatDocument(),
    outputEventsEditorRef.value?.formatDocument(),
  ]);

  await nextTick();
  setTimeout(() => {
    highlightSpecificEvent();
  }, 1000);
};

const handleTestError = (err: any) => {
  console.error("Error in testing function:", err);
  const errMsg = err.response?.data?.message || "Error in testing function";
  outputEventsErrorMsg.value = "Error while transforming results";

  q.notify({
    type: "negative",
    message: errMsg,
    timeout: 3000,
  });

  outputEvents.value = errMsg;
};

const testFunction = async () => {
  loading.value.output = true;
  eventsErrorMsg.value = "";
  outputEventsErrorMsg.value = "";

  if (!isInputValid()) {
    return;
  }
  const payload = {
    function: props.vrlFunction.function,
    events: JSON.parse(inputEvents.value),
  };
  jstransform
    .test(store.state.selectedOrganization.identifier, payload)
    .then((res: any) => {
      processTestResults(res);
    })
    .catch((err: any) => {
      handleTestError(err);
    })
    .finally(() => {
      loading.value.output = false;
    });
};

function getLineRanges(object: any) {
  try {
    if (!outputEventsEditorRef.value) return;
    const model = outputEventsEditorRef.value.getModel(); // Get Monaco Editor model
    const contentLines = model.getLinesContent(); // Get content as an array of lines
    const ranges = [];

    // Convert object to JSON string for comparison
    const serializedObject = JSON.stringify(object.event, null, 4);
    const serializedLines = serializedObject.split("\n");

    console.log("Serialized Lines:", serializedLines);

    let startLine = -1;

    // Iterate over the editor content lines
    for (let i = 0; i < contentLines.length; i++) {
      const line = contentLines[i];

      // Check if the current line matches the first line of the serialized object
      if (line.trim() === serializedLines[0].trim()) {
        let isMatch = true;

        // Check subsequent lines to ensure the entire object matches
        for (let j = 0; j < serializedLines.length; j++) {
          let editorLine = contentLines[i + j]?.trim();
          const objectLine = serializedLines[j]?.trim();

          if (editorLine === "},") {
            editorLine = "}";
          }

          if (editorLine !== objectLine) {
            isMatch = false;
            break;
          }
        }

        if (isMatch) {
          startLine = i;
          break; // Exit the loop once a match is found
        }
      }
    }

    if (startLine !== -1) {
      const endLine = startLine + serializedLines.length - 1;
      ranges.push({
        startLine: startLine + 1,
        endLine: endLine + 1,
        error: `Error: Failed to apply VRL Function.\n${object.message}`,
      }); // Monaco uses 1-based indexing
    }

    return ranges;
  } catch (e) {
    console.log("Error in getLineRanges", e);
  }
}

function highlightSpecificEvent() {
  try {
    const errorEvents = JSON.parse(originalOutputEvents.value).filter(
      (event: any) => event.message?.trim(),
    );
    const errorEventRanges: any[] = [];

    errorEvents.forEach((event: any) => {
      const ranges = getLineRanges(event);
      if (ranges && ranges.length > 0) {
        errorEventRanges.push(ranges[0]);
      }
    });
    if (errorEventRanges.length) {
      outputEventsErrorMsg.value = "Failed to apply VRL Function on few events";
    }

    if (outputEventsEditorRef.value)
      outputEventsEditorRef.value.addErrorDiagnostics(errorEventRanges);
  } catch (e) {
    console.log("Error in highlightSpecificEvent", e);
  }
}

defineExpose({
  testFunction,
});
</script>

<style lang="scss" scoped>
.monaco-editor {
  width: 100%;
  min-height: 10rem;
  border-radius: 5px;
}

.test-function-input-editor,
.test-function-output-editor {
  height: calc((100vh - 260px) / 2) !important;
}

.test-function-option-tabs {
  :deep(.rum-tab) {
    width: auto !important;
    font-size: 12px;
    padding: 3px 10px;
    border: none !important;
  }

  :deep(.active) {
    background-color: $primary !important;
    color: white !important;
  }
}
.test-function-query-container {
  :deep(.test-function-run-query-btn) {
    padding: 2px 8px !important;
    font-size: 11px !important;
    margin: 1px 2px !important;

    .q-icon {
      margin-right: 4px !important;
      font-size: 13px;
    }
  }
}

.function-stream-select-input {
  :deep(.q-field--auto-height .q-field__control) {
    height: 32px;
    min-height: auto;

    .q-field__control-container {
      height: 32px;

      .q-field__native {
        min-height: 32px !important;
        height: 32px !important;
      }
    }

    .q-field__marginal {
      height: 32px;
      min-height: auto;
    }
  }
}

.functions-duration-input {
  :deep(.date-time-button) {
    width: 100%;

    .q-icon.on-right {
      margin-left: auto;
    }
  }
}
</style>
