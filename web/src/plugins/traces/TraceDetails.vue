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
  <div class="trace-details full-width" :style="backgroundStyle">
    <div
      class="row q-px-sm"
      v-if="traceTree.length && !searchObj.data.traceDetails.loading"
    >
      <div class="full-width flex items-center toolbar flex justify-between">
        <div class="flex items-center">
          <div
            data-test="add-alert-back-btn"
            class="flex justify-center items-center q-mr-sm cursor-pointer"
            style="
              border: 1.5px solid;
              border-radius: 50%;
              width: 22px;
              height: 22px;
            "
            title="Traces List"
            @click="routeToTracesList"
          >
            <q-icon name="arrow_back_ios_new" size="14px" />
          </div>
          <div
            class="text-subtitle1 q-mr-lg ellipsis toolbar-operation-name"
            :title="traceTree[0]['operationName']"
          >
            {{ traceTree[0]["operationName"] }}
          </div>
          <div class="q-mr-lg flex items-center text-body2">
            <div class="flex items-center">
              Trace ID:
              <div
                class="toolbar-trace-id ellipsis q-pl-xs"
                :title="spanList[0]['trace_id']"
              >
                {{ spanList[0]["trace_id"] }}
              </div>
            </div>
            <q-icon
              class="q-ml-xs cursor-pointer trace-copy-icon"
              size="12px"
              name="content_copy"
              title="Copy"
              @click="copyTraceId"
            />
          </div>

          <div class="q-pb-xs q-mr-lg">Spans: {{ spanList.length }}</div>

          <!-- TODO OK: Create component for this usecase multi select with button -->
          <div class="o2-input flex items-center trace-logs-selector">
            <q-select
              data-test="log-search-index-list-select-stream"
              v-model="searchObj.data.traceDetails.selectedLogStreams"
              :label="
                searchObj.data.traceDetails.selectedLogStreams.length
                  ? ''
                  : t('search.selectIndex')
              "
              :options="filteredStreamOptions"
              data-cy="stream-selection"
              input-debounce="0"
              behavior="menu"
              filled
              multiple
              borderless
              dense
              fill-input
              :title="selectedStreamsString"
            >
              <template #no-option>
                <div class="o2-input log-stream-search-input">
                  <q-input
                    data-test="alert-list-search-input"
                    v-model="streamSearchValue"
                    borderless
                    filled
                    debounce="500"
                    autofocus
                    dense
                    size="xs"
                    @update:model-value="filterStreamFn"
                    class="q-ml-auto q-mb-xs no-border q-pa-xs"
                    :placeholder="t('search.searchStream')"
                  >
                    <template #prepend>
                      <q-icon name="search" class="cursor-pointer" />
                    </template>
                  </q-input>
                </div>
                <q-item>
                  <q-item-section> {{ t("search.noResult") }}</q-item-section>
                </q-item>
              </template>
              <template #before-options>
                <div class="o2-input log-stream-search-input">
                  <q-input
                    data-test="alert-list-search-input"
                    v-model="streamSearchValue"
                    borderless
                    debounce="500"
                    filled
                    dense
                    autofocus
                    @update:model-value="filterStreamFn"
                    class="q-ml-auto q-mb-xs no-border q-pa-xs"
                    :placeholder="t('search.searchStream')"
                  >
                    <template #prepend>
                      <q-icon name="search" class="cursor-pointer" />
                    </template>
                  </q-input>
                </div>
              </template>
            </q-select>
            <q-btn
              data-test="trace-view-logs-btn"
              v-close-popup="true"
              class="text-bold traces-view-logs-btn"
              :label="
                searchObj.meta.redirectedFromLogs
                  ? t('traces.backToLogs')
                  : t('traces.viewLogs')
              "
              text-color="light-text"
              padding="sm sm"
              size="sm"
              no-caps
              dense
              icon="search"
              @click="redirectToLogs"
            />
          </div>
        </div>
        <div class="flex items-center">
          <div class="flex justify-center items-center tw-border tw-pl-2 tw-rounded-sm tw-border-gray-300">
              <q-input
              v-model="searchQuery"
              placeholder="Search..."
              @update:model-value="handleSearchQueryChange" 
              dense
              borderless
              clearable
              debounce="500"
              class="q-mr-sm custom-height flex items-center"
            />
            <p class="tw-mr-1" v-if="searchResults"><small><span>{{currentIndex+1}}</span> of <span>{{searchResults}}</span></small></p>
            <q-btn 
              v-if="searchResults" 
              :disable="currentIndex === 0" 
              class="tw-mr-1 download-logs-btn flex" 
              flat 
              round
              title="Previous"
              icon="keyboard_arrow_up"
              @click="prevMatch"
              dense
              :size="`sm`"
              />
            <q-btn 
              v-if="searchResults"
              :disable="currentIndex+1 === searchResults"
              class="tw-mr-1 download-logs-btn flex"
              flat round title= "Next" 
              icon="keyboard_arrow_down" 
              @click="nextMatch" 
              dense
              :size="`sm`"
            />
            </div>
          <q-btn
            data-test="logs-search-bar-share-link-btn"
            class="q-mr-sm download-logs-btn"
            size="sm"
            icon="share"
            round
            flat
            no-outline
            :title="t('search.shareLink')"
            @click="shareLink"
          />
          <q-btn round flat icon="cancel" size="md" @click="router.back()" />
        </div>
      </div>

      <q-separator style="width: 100%" />
      <div class="col-12 flex justify-between items-end q-pr-sm q-pt-sm">
        <div
          class="trace-chart-btn flex items-center no-wrap cursor-pointer q-mb-sm"
          @click="toggleTimeline"
        >
          <q-icon
            name="expand_more"
            :class="!isTimelineExpanded ? 'rotate-270' : ''"
            size="22px"
            class="cursor-pointer text-grey-10"
          />
          <div class="text-subtitle2 text-bold">
            {{
              activeVisual === "timeline"
                ? "Trace Timeline"
                : "Trace Service Map"
            }}
          </div>
        </div>

        <div
          v-if="isTimelineExpanded"
          class="rounded-borders"
          style="border: 1px solid #cacaca; padding: 2px"
        >
          <template v-for="visual in traceVisuals" :key="visual.value">
            <q-btn
              :color="visual.value === activeVisual ? 'primary' : ''"
              :flat="visual.value === activeVisual ? false : true"
              dense
              no-caps
              size="11px"
              class="q-px-sm visual-selection-btn"
              @click="activeVisual = visual.value"
            >
              <q-icon><component :is="visual.icon" /></q-icon>
              {{ visual.label }}</q-btn
            >
          </template>
        </div>
      </div>
      <div
        v-show="isTimelineExpanded"
        class="col-12"
        :key="isTimelineExpanded.toString()"
      >
        <ChartRenderer
          v-if="activeVisual === 'timeline'"
          class="trace-details-chart"
          id="trace_details_gantt_chart"
          :data="ChartData"
          @updated:chart="updateChart"
          style="height: 200px"
        />
        <ChartRenderer v-else :data="traceServiceMap" style="height: 200px" />
      </div>
      <q-separator style="width: 100%" class="q-mb-sm" />
      <div
        class="histogram-spans-container"
        :class="[
          isSidebarOpen ? 'histogram-container' : 'histogram-container-full',
          isTimelineExpanded ? '' : 'full',
        ]"
      >
        <trace-header
          :baseTracePosition="baseTracePosition"
          :splitterWidth="leftWidth"
        />
        <div class="relative-position full-height">
          <div
            class="trace-tree-container"
            :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
          >
            <div class="position-relative">
              <!-- <div
                :style="{
                  width: '1px',
                  left: `${leftWidth}px`,
                  backgroundColor:
                    store.state.theme === 'dark' ? '#3c3c3c' : '#ececec',
                  zIndex: 999,
                  top: '-28px',
                  height: 'calc(100% + 30px) !important',
                  cursor: 'col-resize',
                }"
                class="absolute full-height"
                @mousedown="startResize"
              /> -->
              <trace-tree
                :collapseMapping="collapseMapping"
                :spans="spanPositionList"
                :baseTracePosition="baseTracePosition"
                :spanDimensions="spanDimensions"
                :spanMap="spanMap"
                :leftWidth="leftWidth"
                ref="traceTreeRef" 
                :search-query="searchQuery"
                :spanList="spanList"
                @toggle-collapse="toggleSpanCollapse"
                @select-span="updateSelectedSpan"
                @update-current-index="handleIndexUpdate"
                @search-result="handleSearchResult"
              />
            </div>
          </div>
        </div>
      </div>
      <q-separator vertical />
      <div
        v-if="isSidebarOpen && (selectedSpanId || showTraceDetails)"
        class="histogram-sidebar"
        :class="isTimelineExpanded ? '' : 'full'"
      >
        <trace-details-sidebar
          :span="spanMap[selectedSpanId as string]"
          :baseTracePosition="baseTracePosition"
          @view-logs="redirectToLogs"
          @close="closeSidebar"
          @open-trace="openTraceLink"
        />
      </div>
    </div>
    <div
      v-else-if="searchObj.data.traceDetails.loading"
      class="flex column items-center justify-center"
      :style="{ height: '100%' }"
    >
      <q-spinner-hourglass color="primary" size="3em" :thickness="2" />
      <div class="q-pt-sm">Fetching your trace.</div>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  type Ref,
  onMounted,
  watch,
  defineAsyncComponent,
  onBeforeMount,
} from "vue";
import { cloneDeep } from "lodash-es";
import SpanRenderer from "./SpanRenderer.vue";
import useTraces from "@/composables/useTraces";
import { computed } from "vue";
import TraceDetailsSidebar from "./TraceDetailsSidebar.vue";
import TraceTree from "./TraceTree.vue";
import TraceHeader from "./TraceHeader.vue";
import { useStore } from "vuex";
import {
  formatTimeWithSuffix,
  getImageURL,
  convertTimeFromNsToMs,
} from "@/utils/zincutils";
import TraceTimelineIcon from "@/components/icons/TraceTimelineIcon.vue";
import ServiceMapIcon from "@/components/icons/ServiceMapIcon.vue";
import {
  convertTimelineData,
  convertTraceServiceMapData,
} from "@/utils/traces/convertTraceData";
import { throttle } from "lodash-es";
import { copyToClipboard, useQuasar } from "quasar";
import { useI18n } from "vue-i18n";
import { outlinedInfo } from "@quasar/extras/material-icons-outlined";
import useStreams from "@/composables/useStreams";
import { b64EncodeUnicode } from "@/utils/zincutils";
import { useRouter } from "vue-router";
import searchService from "@/services/search";
import useNotifications from "@/composables/useNotifications";

export default defineComponent({
  name: "TraceDetails",
  props: {
    traceId: {
      type: String,
      default: "",
    },
  },
  components: {
    SpanRenderer,
    TraceDetailsSidebar,
    TraceTree,
    TraceHeader,
    TraceTimelineIcon,
    ServiceMapIcon,
    ChartRenderer: defineAsyncComponent(
      () => import("@/components/dashboards/panels/ChartRenderer.vue")
    ),
  },
  
  emits: ["shareLink","searchQueryUpdated"],
  setup(props, { emit }) {
    const traceTree: any = ref([]);
    const spanMap: any = ref({});
    const { searchObj, copyTracesUrl } = useTraces();
    const baseTracePosition: any = ref({});
    const collapseMapping: any = ref({});
    const traceRootSpan: any = ref(null);
    const spanPositionList: any = ref([]);
    const splitterModel = ref(25);
    const timeRange: any = ref({ start: 0, end: 0 });
    const store = useStore();
    const traceServiceMap: any = ref({});
    const { getStreams } = useStreams();
    const spanDimensions = {
      height: 30,
      barHeight: 8,
      textHeight: 25,
      gap: 15,
      collapseHeight: "14",
      collapseWidth: 14,
      connectorPadding: 2,
      paddingLeft: 8,
      hConnectorWidth: 20,
      dotConnectorWidth: 6,
      dotConnectorHeight: 6,
      colors: ["#b7885e", "#1ab8be", "#ffcb99", "#f89570", "#839ae2"],
    };

    const { showErrorNotification } = useNotifications();

    const logStreams = ref([]);

    const filteredStreamOptions = ref([]);

    const streamSearchValue = ref<string>("");

    const { t } = useI18n();

    const $q = useQuasar();

    const router = useRouter();

    const traceDetails = ref({});

    const traceVisuals = [
      { label: "Timeline", value: "timeline", icon: TraceTimelineIcon },
      { label: "Service Map", value: "service_map", icon: ServiceMapIcon },
    ];

    const activeVisual = ref("timeline");

    const traceChart = ref({
      data: [],
    });

    const ChartData: any = ref({});

    const leftWidth: Ref<number> = ref(250);
    const initialX: Ref<number> = ref(0);
    const initialWidth: Ref<number> = ref(0);

    const throttledResizing = ref<any>(null);

    const serviceColorIndex = ref(0);
    const colors = ref(["#b7885e", "#1ab8be", "#ffcb99", "#f89570", "#839ae2"]);

    const spanList: any = computed(() => {
      return searchObj.data.traceDetails.spanList;
    });

    const isTimelineExpanded = ref(false);

    const selectedStreamsString = computed(() =>
      searchObj.data.traceDetails.selectedLogStreams.join(", ")
    );

    const showTraceDetails = ref(false);
    const currentIndex = ref(0);
    const searchResults = ref(0);
    const searchQuery = ref('');

    const handleSearchQueryChange = (value:any) => {
      searchQuery.value = value; 
      
    };
    const traceTreeRef = ref<InstanceType<typeof TraceTree> | null>(null);
    const nextMatch = () => {
      if (traceTreeRef.value) {
        traceTreeRef.value.nextMatch();
      }
    };
    const prevMatch = () => {
      if (traceTreeRef.value) {
        traceTreeRef.value.prevMatch();
      }
    };
    const handleIndexUpdate = (newIndex:any) => {
      currentIndex.value = newIndex; // Update the parent's state with the child's emitted value
    };
    const handleSearchResult = (newIndex:any) => {
      searchResults.value = newIndex; // Update the parent's state with the child's emitted value
    };
    // Watch for changes in searchQuery
    

    // Disabled for now
    // onActivated(() => {
    //   const params = router.currentRoute.value.query;

    //   // If selected trace is different from the one in the URL, reset the trace details
    //   // If there is no selected trace, then also reset the trace details

    //   if (
    //     (searchObj.data.traceDetails.selectedTrace &&
    //       params.trace_id !==
    //         searchObj.data.traceDetails.selectedTrace?.trace_id) ||
    //     !searchObj.data.traceDetails.selectedTrace
    //   ) {
    //     resetTraceDetails();
    //     console.log("resetTraceDetails >>>>>>>>>>>>>");
    //     setupTraceDetails();
    //   }
    // });

    onBeforeMount(async () => {
      resetTraceDetails();
      setupTraceDetails();
    });

    watch(
      () => router.currentRoute.value.name,
      (curr, prev) => {
        if (prev === "logs" && curr === "traceDetails") {
          searchObj.meta.redirectedFromLogs = true;
        } else {
          searchObj.meta.redirectedFromLogs = false;
        }
      }
    );

    // Disabled for now
    // watch(
    //   () => router.currentRoute.value.query.trace_id,
    //   (_new, _old) => {
    //     // If trace_id changes, reset the trace details
    //     if (
    //       _new &&
    //       _old &&
    //       _new !== _old &&
    //       _new !== searchObj.data.traceDetails.selectedTrace?.trace_id
    //     ) {
    //       resetTraceDetails();
    //       // setupTraceDetails();
    //       const params = router.currentRoute.value.query;
    //       if (params.span_id) {
    //         updateSelectedSpan(params.span_id as string);
    //       }
    //     }
    //   },
    // );

    const backgroundStyle = computed(() => {
      return {
        background: store.state.theme === "dark" ? "#181a1b" : "#ffffff",
      };
    });

    const resetTraceDetails = () => {
      searchObj.data.traceDetails.showSpanDetails = false;
      searchObj.data.traceDetails.selectedSpanId = "";
      searchObj.data.traceDetails.selectedTrace = {
        trace_id: "",
        trace_start_time: 0,
        trace_end_time: 0,
      };
      searchObj.data.traceDetails.spanList = [];
      searchObj.data.traceDetails.loading = true;
    };

    const setupTraceDetails = async () => {
      showTraceDetails.value = false;
      searchObj.data.traceDetails.showSpanDetails = false;
      searchObj.data.traceDetails.selectedSpanId = "";

      await getTraceMeta();
      await getStreams("logs", false)
        .then((res: any) => {
          logStreams.value = res.list.map((option: any) => option.name);
          filteredStreamOptions.value = JSON.parse(
            JSON.stringify(logStreams.value)
          );

          if (!searchObj.data.traceDetails.selectedLogStreams.length)
            searchObj.data.traceDetails.selectedLogStreams.push(
              logStreams.value[0]
            );
        })
        .catch(() => Promise.reject())
        .finally(() => {});
    };

    onMounted(() => {
      const params = router.currentRoute.value.query;
      if (params.span_id) {
        updateSelectedSpan(params.span_id as string);
      }
    });

    // watch(
    //   () => spanList.value.length,
    //   () => {
    //     if (spanList.value.length) {
    //       buildTracesTree();
    //     } else traceTree.value = [];
    //   },
    //   { immediate: true },
    // );

    const isSidebarOpen = computed(() => {
      return searchObj.data.traceDetails.showSpanDetails;
    });

    const selectedSpanId = computed(() => {
      return searchObj.data.traceDetails.selectedSpanId;
    });

    const getTraceMeta = () => {
      searchObj.loading = true;

      let filter = (router.currentRoute.value.query.filter as string) || "";

      if (filter?.length)
        filter += ` and trace_id='${router.currentRoute.value.query.trace_id}'`;
      else filter += `trace_id='${router.currentRoute.value.query.trace_id}'`;

      searchService
        .get_traces({
          org_identifier: router.currentRoute.value.query
            .org_identifier as string,
          start_time: Number(router.currentRoute.value.query.from),
          end_time: Number(router.currentRoute.value.query.to),
          filter: filter || "",
          size: 1,
          from: 0,
          stream_name: router.currentRoute.value.query.stream as string,
        })
        .then(async (res: any) => {
          const trace = getTracesMetaData(res.data.hits)[0];
          if (!trace) {
            showTraceDetailsError();
            return;
          }
          searchObj.data.traceDetails.selectedTrace = trace;
          getTraceDetails();
        })
        .catch(() => {
          showTraceDetailsError();
        })
        .finally(() => {
          searchObj.loading = false;
        });
    };

    const getDefaultRequest = () => {
      return {
        query: {
          sql: `select min(${store.state.zoConfig.timestamp_column}) as zo_sql_timestamp, min(start_time/1000) as trace_start_time, max(end_time/1000) as trace_end_time, min(service_name) as service_name, min(operation_name) as operation_name, count(trace_id) as spans, SUM(CASE WHEN span_status='ERROR' THEN 1 ELSE 0 END) as errors, max(duration) as duration, trace_id [QUERY_FUNCTIONS] from "[INDEX_NAME]" [WHERE_CLAUSE] group by trace_id order by zo_sql_timestamp DESC`,
          start_time: (new Date().getTime() - 900000) * 1000,
          end_time: new Date().getTime() * 1000,
          from: 0,
          size: 0,
        },
        encoding: "base64",
      };
    };

    const buildTraceSearchQuery = (trace: any) => {
      const req = getDefaultRequest();
      req.query.from = 0;
      req.query.size = 1000;
      req.query.start_time =
        Math.ceil(
          Number(searchObj.data.traceDetails.selectedTrace?.trace_start_time)
        ) - 30000000;
      req.query.end_time =
        Math.ceil(
          Number(searchObj.data.traceDetails.selectedTrace?.trace_end_time)
        ) + 30000000;

      req.query.sql = b64EncodeUnicode(
        `SELECT * FROM ${trace.stream} WHERE trace_id = '${trace.trace_id}' ORDER BY start_time`
      ) as string;

      return req;
    };

    const getTraceDetails = async () => {
      searchObj.data.traceDetails.loading = true;
      searchObj.data.traceDetails.spanList = [];
      const req = buildTraceSearchQuery(router.currentRoute.value.query);

      searchService
        .search(
          {
            org_identifier: router.currentRoute.value.query
              ?.org_identifier as string,
            query: req,
            page_type: "traces",
          },
          "UI"
        )
        .then((res: any) => {
          if (!res.data?.hits?.length) {
            showTraceDetailsError();
            return;
          }
          searchObj.data.traceDetails.spanList = res.data?.hits || [];
          console.log(res.data?.hits)
          buildTracesTree();
        })
        .finally(() => {
          searchObj.data.traceDetails.loading = false;
        });
    };

    const getTracesMetaData = (traces: any[]) => {
      if (!traces.length) return [];

      return traces.map((trace) => {
        const _trace = {
          trace_id: trace.trace_id,
          trace_start_time: Math.round(trace.start_time / 1000),
          trace_end_time: Math.round(trace.end_time / 1000),
          service_name: trace.service_name,
          operation_name: trace.operation_name,
          spans: trace.spans[0],
          errors: trace.spans[1],
          duration: trace.duration,
          services: {} as any,
          zo_sql_timestamp: new Date(trace.start_time / 1000).getTime(),
        };
        trace.service_name.forEach((service: any) => {
          if (!searchObj.meta.serviceColors[service.service_name]) {
            if (serviceColorIndex.value >= colors.value.length)
              generateNewColor();

            searchObj.meta.serviceColors[service.service_name] =
              colors.value[serviceColorIndex.value];

            serviceColorIndex.value++;
          }
          _trace.services[service.service_name] = service.count;
        });
        return _trace;
      });
    };

    const showTraceDetailsError = () => {
      showErrorNotification(
        `Trace ${router.currentRoute.value.query.trace_id} not found`
      );
      const query = cloneDeep(router.currentRoute.value.query);
      delete query.trace_id;
      router.push({
        name: "traces",
        query: {
          ...query,
        },
      });
      return;
    };

    function generateNewColor() {
      // Generate a color in HSL format
      const hue = (colors.value.length * 137.508) % 360; // Using golden angle approximation
      const saturation = 70 + (colors.value.length % 2) * 15;
      const lightness = 50;
      colors.value.push(`hsl(${hue}, ${saturation}%, ${lightness}%)`);
      return colors;
    }

    const calculateTracePosition = () => {
      const tics = [];
      baseTracePosition.value["durationMs"] = timeRange.value.end;
      baseTracePosition.value["startTimeMs"] =
        traceTree.value[0].startTimeMs + timeRange.value.start;
      const quarterMs = (timeRange.value.end - timeRange.value.start) / 4;
      let time = timeRange.value.start;
      for (let i = 0; i <= 4; i++) {
        tics.push({
          value: Number(time.toFixed(2)),
          label: `${formatTimeWithSuffix(time * 1000)}`,
          left: i === 0 ? "-1px" : `${25 * i}%`,
        });
        time += quarterMs;
      }
      baseTracePosition.value["tics"] = tics;
    };

    // Find out spans who has reference_parent_span_id as span_id of first span in sampleTrace
    async function buildTracesTree() {
      if (!spanList.value?.length) return;

      spanMap.value = {};
      traceTree.value = [];
      spanPositionList.value = [];
      collapseMapping.value = {};
      let lowestStartTime: number = spanList.value[0].start_time;
      let highestEndTime: number = spanList.value[0].end_time;

      if (!spanList.value?.length) return;

      spanList.value.forEach((spanData: any) => {
        spanMap.value[spanData.span_id] = spanData;
      });

      const formattedSpanMap: any = {};

      spanList.value.forEach((spanData: any) => {
        formattedSpanMap[spanData.span_id] = getFormattedSpan(spanData);
      });

      for (let i = 0; i < spanList.value.length; i++) {
        if (spanList.value[i].start_time < lowestStartTime) {
          lowestStartTime = spanList.value[i].start_time;
        }
        if (spanList.value[i].end_time > highestEndTime) {
          highestEndTime = spanList.value[i].end_time;
        }

        const span = formattedSpanMap[spanList.value[i].span_id];

        span.style.color = searchObj.meta.serviceColors[span.serviceName];

        span.style.backgroundColor = adjustOpacity(span.style.color, 0.2);

        span.index = i;

        collapseMapping.value[span.spanId] = true;

        if (!span.parentId) {
          traceTree.value.push(span);
        } else if (!formattedSpanMap[span.parentId]) {
          traceTree.value.push(span);
        } else if (span.parentId && formattedSpanMap[span.parentId]) {
          const parentSpan = formattedSpanMap[span.parentId];
          if (!parentSpan.spans) parentSpan.spans = [];
          parentSpan.spans.push(span);
        }
      }

      traceTree.value[0].lowestStartTime =
        convertTimeFromNsToMs(lowestStartTime);
      traceTree.value[0].highestEndTime = convertTimeFromNsToMs(highestEndTime);
      traceTree.value[0].style.color =
        searchObj.meta.serviceColors[traceTree.value[0].serviceName];

      traceTree.value.forEach((span: any) => {
        addSpansPositions(span, 0);
      });

      timeRange.value.end = 0;
      timeRange.value.start = 0;

      calculateTracePosition();
      buildTraceChart();
      buildServiceTree();
    }

    let index = 0;
    const addSpansPositions = (span: any, depth: number) => {
      if (!span.index) index = 0;
      span.depth = depth;
      spanPositionList.value.push(
        Object.assign(span, {
          style: {
            color: span.style.color,
            backgroundColor: span.style.backgroundColor,
            top: index * spanDimensions.height + "px",
            left: spanDimensions.gap * depth + "px",
          },
          hasChildSpans: !!span.spans.length,
          currentIndex: index,
        })
      );
      if (collapseMapping.value[span.spanId]) {
        if (span.spans.length) {
          span.spans.forEach((childSpan: any) => {
            index = index + 1;
            childSpan.totalSpans = addSpansPositions(childSpan, depth + 1);
          });
          span.totalSpans = span.spans.reduce(
            (acc: number, span: any) =>
              acc + ((span?.spans?.length || 0) + (span?.totalSpans || 0)),
            0
          );
        }
        return (span?.spans?.length || 0) + (span?.totalSpans || 0);
      } else {
        return 0;
      }
    };

    function adjustOpacity(hexColor: string, opacity: number) {
      // Ensure opacity is between 0 and 1
      opacity = Math.max(0, Math.min(1, opacity));

      // Convert opacity to a hex value
      const opacityHex = Math.round(opacity * 255)
        .toString(16)
        .padStart(2, "0");

      // Append the opacity hex value to the original hex color
      return hexColor + opacityHex;
    }

    const buildServiceTree = () => {
      const serviceTree: any[] = [];
      let maxDepth = 0;
      let maxHeight: number[] = [0];
      const getService = (
        span: any,
        currentColumn: any[],
        serviceName: string,
        depth: number,
        height: number
      ) => {
        maxHeight[depth] =
          maxHeight[depth] === undefined ? 1 : maxHeight[depth] + 1;
        if (serviceName !== span.serviceName) {
          const children: any[] = [];
          currentColumn.push({
            name: `${span.serviceName} \n (${span.durationMs}ms)`,
            parent: serviceName,
            duration: span.durationMs,
            children: children,
            itemStyle: {
              color: searchObj.meta.serviceColors[span.serviceName],
            },
            emphasis: {
              disabled: true,
            },
          });
          if (span.spans && span.spans.length) {
            span.spans.forEach((_span: any) =>
              getService(_span, children, span.serviceName, depth + 1, height)
            );
          } else {
            if (maxDepth < depth) maxDepth = depth;
          }
          return;
        }
        if (span.spans && span.spans.length) {
          span.spans.forEach((span: any) =>
            getService(span, currentColumn, serviceName, depth + 1, height)
          );
        } else {
          if (maxDepth < depth) maxDepth = depth;
        }
      };
      traceTree.value.forEach((span: any) => {
        getService(span, serviceTree, "", 1, 1);
      });
      traceServiceMap.value = convertTraceServiceMapData(
        cloneDeep(serviceTree),
        maxDepth
      );
    };

    // Convert span object to required format
    // Converting ns to ms
    const getFormattedSpan = (span: any) => {
      return {
        [store.state.zoConfig.timestamp_column]:
          span[store.state.zoConfig.timestamp_column],
        startTimeMs: convertTimeFromNsToMs(span.start_time),
        endTimeMs: convertTimeFromNsToMs(span.end_time),
        durationMs: Number((span.duration / 1000).toFixed(4)), // This key is standard, we use for calculating width of span block. This should always be in ms
        durationUs: Number(span.duration.toFixed(4)), // This key is used for displaying duration in span block. We convert this us to ms, s in span block
        idleMs: convertTime(span.idle_ns),
        busyMs: convertTime(span.busy_ns),
        spanId: span.span_id,
        operationName: span.operation_name,
        serviceName: span.service_name,
        spanStatus: span.span_status,
        spanKind: getSpanKind(span.span_kind.toString()),
        parentId: span.reference_parent_span_id,
        spans: [],
        index: 0,
        style: {
          color: "",
        },
        links: JSON.parse(span.links || "[]"),
      };
    };

    const convertTime = (time: number) => {
      return Number((time / 1000000).toFixed(2));
    };

    const convertTimeFromNsToMs = (time: number) => {
      const nanoseconds = time;
      const milliseconds = Math.floor(nanoseconds / 1000000);
      const date = new Date(milliseconds);
      return date.getTime();
    };

    const getSpanKind = (spanKind: string) => {
      const spanKindMapping: { [key: string]: string } = {
        "1": "Client",
        "2": "Server",
        "3": "Producer",
        "4": "Consumer",
        "5": "Internal",
      };
      return spanKindMapping[spanKind];
    };

    const closeSidebar = () => {
      searchObj.data.traceDetails.showSpanDetails = false;
      searchObj.data.traceDetails.selectedSpanId = null;
    };
    const toggleSpanCollapse = (spanId: number | string) => {
      collapseMapping.value[spanId] = !collapseMapping.value[spanId];
      index = 0;
      spanPositionList.value = [];
      traceTree.value.forEach((span: any) => {
        addSpansPositions(span, 0);
      });
    };
    const buildTraceChart = () => {
      const data: any = [];
      for (let i = spanPositionList.value.length - 1; i > -1; i--) {
        const absoluteStartTime =
          spanPositionList.value[i].startTimeMs -
          traceTree.value[0].lowestStartTime;

        data.push({
          x0: absoluteStartTime,
          x1: Number(
            (absoluteStartTime + spanPositionList.value[i].durationMs).toFixed(
              4
            )
          ),
          fillcolor: spanPositionList.value[i].style.color,
        });
      }
      traceChart.value.data = data;
      ChartData.value = convertTimelineData(traceChart);
    };
    const updateChart = (data: any) => {
      timeRange.value.start = data.start || 0;
      timeRange.value.end = data.end || 0;
      calculateTracePosition();
    };

    onMounted(() => {
      throttledResizing.value = throttle(resizing, 50);
    });

    const startResize = (event: any) => {
      initialX.value = event.clientX;
      initialWidth.value = leftWidth.value;

      window.addEventListener("mousemove", throttledResizing.value);
      window.addEventListener("mouseup", stopResize);
      document.body.classList.add("no-select");
    };

    const resizing = (event: any) => {
      const deltaX = event.clientX - initialX.value;
      leftWidth.value = initialWidth.value + deltaX;
    };

    const stopResize = () => {
      window.removeEventListener("mousemove", throttledResizing.value);
      window.removeEventListener("mouseup", stopResize);
      document.body.classList.remove("no-select");
    };

    const toggleTimeline = () => {
      isTimelineExpanded.value = !isTimelineExpanded.value;
    };

    const copyTraceId = () => {
      $q.notify({
        type: "positive",
        message: "Trace ID copied to clipboard",
        timeout: 2000,
      });
      copyToClipboard(spanList.value[0]["trace_id"]);
    };

    const shareLink = () => {
      copyTracesUrl({
        from: router.currentRoute.value.query.from as string,
        to: router.currentRoute.value.query.to as string,
      });
    };

    const redirectToLogs = () => {
      if (!searchObj.data.traceDetails.selectedTrace) {
        return;
      }
      const stream: string =
        searchObj.data.traceDetails.selectedLogStreams.join(",");
      const from =
        searchObj.data.traceDetails.selectedTrace?.trace_start_time - 60000000;
      const to =
        searchObj.data.traceDetails.selectedTrace?.trace_end_time + 60000000;
      const refresh = 0;

      const query = b64EncodeUnicode(
        `${store.state.organizationData?.organizationSettings?.trace_id_field_name}='${spanList.value[0]["trace_id"]}'`
      );

      router.push({
        path: "/logs",
        query: {
          stream_type: "logs",
          stream,
          from,
          to,
          refresh,
          sql_mode: "false",
          query,
          org_identifier: store.state.selectedOrganization.identifier,
          show_histogram: "true",
          type: "trace_explorer",
          quick_mode: "false",
        },
      });
    };

    const filterStreamFn = (val: any = "") => {
      filteredStreamOptions.value = logStreams.value.filter((stream: any) => {
        return stream.toLowerCase().indexOf(val.toLowerCase()) > -1;
      });
    };

    const openTraceDetails = () => {
      searchObj.data.traceDetails.showSpanDetails = true;
      showTraceDetails.value = true;
    };

    const updateSelectedSpan = (spanId: string) => {
      showTraceDetails.value = false;
      searchObj.data.traceDetails.showSpanDetails = true;
      searchObj.data.traceDetails.selectedSpanId = spanId;
    };

    const routeToTracesList = () => {
      const query = cloneDeep(router.currentRoute.value.query);
      delete query.trace_id;

      if (searchObj.data.datetime.type === "relative") {
        query.period = searchObj.data.datetime.relativeTimePeriod;
      } else {
        query.from = searchObj.data.datetime.startTime.toString();
        query.to = searchObj.data.datetime.endTime.toString();
      }

      router.push({
        name: "traces",
        query: {
          ...query,
        },
      });
    };

    const openTraceLink = async () => {
      resetTraceDetails();
      await setupTraceDetails();
    };

    return {
      router,
      t,
      traceTree,
      collapseMapping,
      traceRootSpan,
      baseTracePosition,
      searchObj,
      spanList,
      isSidebarOpen,
      selectedSpanId,
      spanMap,
      closeSidebar,
      toggleSpanCollapse,
      spanPositionList,
      spanDimensions,
      splitterModel,
      ChartData,
      traceChart,
      updateChart,
      traceServiceMap,
      activeVisual,
      traceVisuals,
      getImageURL,
      store,
      leftWidth,
      startResize,
      isTimelineExpanded,
      toggleTimeline,
      copyToClipboard,
      copyTraceId,
      shareLink,
      outlinedInfo,
      redirectToLogs,
      filteredStreamOptions,
      filterStreamFn,
      streamSearchValue,
      selectedStreamsString,
      openTraceDetails,
      showTraceDetails,
      traceDetails,
      updateSelectedSpan,
      backgroundStyle,
      routeToTracesList,
      openTraceLink,
      convertTimeFromNsToMs,
      searchQuery,
      handleSearchQueryChange,
      traceTreeRef,
      nextMatch,
      prevMatch,
      currentIndex,
      handleIndexUpdate,
      handleSearchResult,
      searchResults
    };
  },
});
</script>

<style scoped lang="scss">
$sidebarWidth: 60%;
$separatorWidth: 2px;
$toolbarHeight: 50px;
$traceHeaderHeight: 30px;
$traceChartHeight: 210px;
$appNavbarHeight: 57px;

$traceChartCollapseHeight: 42px;

.toolbar {
  height: $toolbarHeight;
}
.trace-details {
  overflow: auto;
}
.histogram-container-full {
  width: 100%;
}
.histogram-container {
  width: calc(100% - $sidebarWidth - $separatorWidth);
}

.histogram-sidebar {
  width: $sidebarWidth;
  height: calc(
    100vh - $toolbarHeight - $traceChartHeight - 44px - $appNavbarHeight
  );
  overflow-y: auto;
  overflow-x: hidden;

  &.full {
    height: calc(100vh - $toolbarHeight - 8px - 44px - $appNavbarHeight);
  }
}

.histogram-spans-container {
  height: calc(
    100vh - $toolbarHeight - $traceChartHeight - 44px - $appNavbarHeight
  );
  overflow-y: auto;
  position: relative;
  overflow-x: hidden;

  &.full {
    height: calc(100vh - $toolbarHeight - 8px - 44px - $appNavbarHeight);
  }
}

.trace-tree-container {
  overflow: auto;
}

.trace-chart-btn {
  cursor: pointer;
  padding-right: 8px;
  border-radius: 2px;
  padding-top: 2px;
  padding-bottom: 2px;

  &:hover {
    background-color: rgba($primary, 0.9);
    color: #ffffff;

    .q-icon {
      color: #ffffff !important;
    }
  }
}

.log-stream-search-input {
  width: 226px;

  .q-field .q-field__control {
    padding: 0px 8px;
  }
}

.toolbar-trace-id {
  max-width: 150px;
}

.toolbar-operation-name {
  max-width: 225px;
}
</style>
<style lang="scss">
.trace-details {
  .q-splitter__before,
  .q-splitter__after {
    overflow: revert !important;
  }

  .q-splitter__before {
    z-index: 999 !important;
  }

  .trace-details-chart {
    .rangeslider-slidebox {
      fill: #7076be !important;
    }
    .rangeslider-mask-max,
    .rangeslider-mask-min {
      fill: #d2d2d2 !important;
      fill-opacity: 1 !important;
    }
  }

  .visual-selection-btn {
    .q-icon {
      padding-right: 5px;
      font-size: 15px;
    }
  }
}

.no-select {
  user-select: none !important;
  -moz-user-select: none !important;
  -webkit-user-select: none !important;
  -ms-user-select: none !important;
}

.trace-copy-icon {
  &:hover {
    &.q-icon {
      text-shadow: 0px 2px 8px rgba(0, 0, 0, 0.5);
    }
  }
}

.trace-logs-selector {
  .q-field {
    span {
      display: inline-block;
      width: 180px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      text-align: left;
    }
  }
}

.log-stream-search-input {
  .q-field .q-field__control {
    padding: 0px 4px;
  }
}

.traces-view-logs-btn {
  height: 36px;
  margin-left: -1px;
  border-top-left-radius: 0px;
  border-bottom-left-radius: 0px;

  .q-btn__content {
    span {
      font-size: 12px;
    }
  }
}
.custom-height {
  height: 34px;
}

.custom-height .q-field__control,.custom-height .q-field__append {
  height: 100%; /* Ensures the input control fills the container height */
  line-height: 36px; /* Vertically centers the text inside */
}

</style>
