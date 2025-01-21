<template>
  <div
    :class="
      store.state.theme === 'dark'
        ? 'dark-theme-history-page dark-theme'
        : 'light-theme-history-page light-theme'
    "
  >
    <div v-if="!showSearchResults">
      <div class="flex tw-justify-between tw-items-center" >
        <div class="flex items-center q-py-sm q-pl-md">
          <div
          data-test="search-scheduler-back-btn"
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="border: 1.5px solid; border-radius: 50%; width: 22px; height: 22px;"
          title="Go Back"
          @click="closeSearchHistory">
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div class="text-h6" data-test="search-scheduler-title">
          Search Job Scheduler
        </div>
        </div>
        <div class="flex items-center q-py-sm q-pr-md">
            <div>
              <q-btn
                color="secondary"
                label="Get Jobs"
                @click="fetchSearchHistory"
                class="q-ml-md"
                :disable="isLoading"
              />
            </div>
  
        </div>
      </div>

      <div>
        <q-page>
          <q-table
            data-test="search-scheduler-table"
            ref="qTableSchedule"
            dense
            :rows="dataToBeLoaded"
            :columns="columnsToBeRendered"
            :pagination="pagination"
            row-key="trace_id"
            :rows-per-page-options="[]"
            class="custom-table search-job-list-table"
            style="width: 100%"
            :sort-method="sortMethod"

          >
            <template v-slot:body="props">
              <q-tr
                :data-test="`search-scheduler-table-${props.row.trace_id}-row`"
                :props="props"
                style="cursor: pointer"
                @click="triggerExpand(props)"
              >
                <q-td>
                  <q-btn
                   data-test="search-scheduler-expand-btn"
                    dense
                    flat
                    size="xs"
                    :icon="
                      expandedRow != props.row.trace_id
                        ? 'expand_more'
                        : 'expand_less'
                    "
                  />
                </q-td>

                <q-td
                  v-for="(col, index) in columnsToBeRendered.slice(1)"
                  :key="col.name"
                  :props="props"
                >
                  <!-- Render the content for all but the last item -->
                  <template
                    v-if="
                      index < columnsToBeRendered.slice(1).length - 1 &&
                      col.field != 'status'
                    "
                  >
                    {{ props.row[col.field] }}
                  </template>
                  <template v-else-if="col.field === 'status'">
                    <div class="status-cell">
                      <q-icon
                        :name="getStatusIcon(props.row[col.field])"
                        size="xs"
                        class="q-mr-xs"
                        :color="getStatusColor(props.row[col.field])"
                      />
                      {{ getStatusText(props.row[col.field]) }}
                    </div>
                  </template>
                  <template v-else>
                    <q-btn
                      data-test="search-scheduler-cancel-btn"
                      icon="cancel"
                      :title="'Cancel'"
                      class="q-ml-xs"
                      padding="sm"
                      unelevated
                      size="sm"
                      round
                      flat
                      :disable="
                        props.row.status_code !== 0 &&
                        props.row.status_code !== 1
                      "
                      color="gray"
                      @click="confirmCancelJob(props.row)"
                    ></q-btn>

                    <q-btn
                      data-test="search-scheduler-delete-btn"
                      icon="delete"
                      :title="'Delete'"
                      class="q-ml-xs"
                      padding="sm"
                      unelevated
                      size="sm"
                      round
                      color="red"
                      flat
                      @click="confirmDeleteJob(props.row)"
                    ></q-btn>
                    <q-btn
                    data-test="search-scheduler-restart-btn"
                      icon="refresh"
                      :title="'Restart'"
                      class="q-ml-xs"
                      padding="sm"
                      unelevated
                      size="sm"
                      round
                      color="orange"
                      flat
                      :disable="
                        props.row.status_code !== 2 &&
                        props.row.status_code !== 3
                      "
                      @click="retrySearchJob(props.row)"
                    ></q-btn>
                    <q-btn
                      data-test="search-scheduler-explore-btn"
                      icon="search"
                      :title="'Explore'"
                      class="q-ml-xs"
                      padding="sm"
                      unelevated
                      size="sm"
                      round
                      :disable="
                        props.row.status_code == 0 || props.row.status_code == 3
                      "
                      flat
                      color="green"
                      @click="fetchSearchResults(props.row)"
                    ></q-btn>
                  </template>
                </q-td>
              </q-tr>

              <q-tr v-show="expandedRow === props.row.trace_id" :props="props">
                <q-td colspan="100%">
                  <div class="app-tabs-schedule-list report-list-tabs">
                    <app-tabs
                      data-test="expanded-list-tabs"
                      class="q-mr-md"
                      :tabs="tabs"
                      v-model:active-tab="activeTab"
                    />
                  </div>
                  <div v-if="activeTab == 'query'">
                    <div class="text-left tw-px-2 q-mb-sm expanded-content">
                      <div class="tw-flex tw-items-center q-py-sm">
                        <strong
                          >SQL Query :
                          <span>
                            <q-btn
                              @click.stop="
                                copyToClipboard(props.row.sql, 'SQL Query')
                              "
                              data-test="search-scheduler-copy-sql-btn"

                              size="xs"
                              dense
                              flat
                              icon="content_copy"
                              class="copy-btn-sql tw-ml-2 tw-py-2 tw-px-2" /></span
                        ></strong>
                        <q-btn
                          @click.stop="fetchSearchResults(props.row)"
                          data-test="search-scheduler-go-to-logs-btn"
                          size="xs"
                          label="Logs"
                          dense
                          class="copy-btn tw-py-2 tw-mx-2 tw-px-2"
                          icon="search"
                          flat
                          style="
                            color: #f2452f;
                            border: #f2452f 1px solid;
                            font-weight: bolder;
                          "
                          :disable="
                            props.row.status_code == 0 ||
                            props.row.status_code == 3
                          "
                        />
                      </div>
                      <div class="tw-flex tw-items-start tw-justify-center">
                        <div class="scrollable-content expanded-sql">
                          <pre style="text-wrap: wrap">{{
                            props.row?.sql
                          }}</pre>
                        </div>
                      </div>
                    </div>
                    <div
                      v-if="props.row?.function"
                      class="text-left q-mb-sm tw-px-2 expanded-content"
                    >
                      <div class="tw-flex tw-items-center q-py-sm">
                        <strong
                          >Function Definition :
                          <span>
                            <q-btn
                              @click.stop="
                                copyToClipboard(
                                  props.row.function,
                                  'Function Defination',
                                )
                              "
                              size="xs"
                              dense
                              flat
                              icon="content_copy"
                              class="copy-btn-function tw-ml-2 tw-py-2 tw-px-2" /></span
                        ></strong>
                      </div>

                      <div class="tw-flex tw-items-start tw-justify-center">
                        <div class="scrollable-content expanded-function">
                          <pre style="text-wrap: wrap">{{
                            props.row?.function
                          }}</pre>
                        </div>
                      </div>
                    </div>
                  </div>
                  <div class="q-py-md" v-else>
                    <div
                      class="text-left tw-px-2 q-mb-sm expanded-content flex tw-flex-col"
                    >
                      <query-editor
                        style="height: 130px"
                        :key="props.row.trace_id"
                        :ref="`QueryEditorRef${props.row.trace_id}`"
                        :editor-id="`alerts-query-editor${props.row.trace_id}`"
                        class="monaco-editor"
                        :debounceTime="300"
                        v-model:query="query"
                        language="json"
                        read-only
                      />
                    </div>
                  </div>
                </q-td>
              </q-tr>
            </template>
            <template #bottom="scope">
              <div class="tw-ml-auto tw-mr-2">Max Limit : <b>1000</b></div>
              <q-separator
                style="height: 1.5rem; margin: auto 0"
                vertical
                inset
                class="q-mr-md"
              />

              <div class="q-pl-md">
                <QTablePagination
                  :scope="scope"
                  :position="'bottom'"
                  :resultTotal="resultTotal"
                  :perPageOptions="perPageOptions"
                  @update:changeRecordPerPage="changePagination"
                />
              </div>
            </template>
            <template #no-data>
              <div v-if="!isLoading" class="tw-flex tw-mx-auto">
                <NoData />
              </div>
              <div
                v-if="isLoading"
                class="text-center full-width full-height q-mt-lg tw-flex tw-justify-center"
              >
                <q-spinner-hourglass color="primary" size="lg" />
              </div>
            </template>
          </q-table>

        </q-page>
        <ConfirmDialog
          title="Delete Scheduled Search"
          message="Are you sure you want to delete this scheduled search?"
          @update:ok="deleteSearchJob"
          @update:cancel="confirmDelete = false"
          v-model="confirmDelete"
        />
        <ConfirmDialog
          title="Cancel Scheduled Search"
          message="Are you sure you want to cancel this scheduled search?"
          @update:ok="cancelSearchJob"
          @update:cancel="confirmCancel = false"
          v-model="confirmCancel"
        />
      </div>
    </div>
    
  </div>

  <!-- Show NoData component if there's no data to display -->
</template>
<script lang="ts">
//@ts-nocheck
import {
  ref,
  watch,
  onMounted,
  nextTick,
  computed,
  onBeforeMount,
  onActivated,
} from "vue";
import {
  timestampToTimezoneDate,
  b64EncodeUnicode,
  b64DecodeUnicode,
  convertDateToTimestamp,
} from "@/utils/zincutils";
import { useRouter, useRoute } from "vue-router";
import { useStore } from "vuex";
import { defineAsyncComponent, defineComponent, reactive } from "vue";
import useLogs from "../../composables/useLogs";
import TenstackTable from "../../plugins/logs/TenstackTable.vue";
import searchService from "@/services/search";
import NoData from "@/components/shared/grid/NoData.vue";
import DateTime from "@/components/DateTime.vue";
import { useI18n } from "vue-i18n";
import { date, qTable, useQuasar } from "quasar";
import type { Ref } from "vue";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import AppTabs from "@/components/common/AppTabs.vue";
import JsonPreview from "./JsonPreview.vue";
import QueryEditor from "@/components/QueryEditor.vue";
import config from "@/aws-exports";


export default defineComponent({
  name: "SearchSchedulersList",
  components: {
    DateTime,
    NoData,
    QTablePagination,
    TenstackTable,
    ConfirmDialog,
    AppTabs,
    JsonPreview,
    QueryEditor,
  },
  props: {
    isClicked: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["closeSearchHistory"],
  methods: {
    closeSearchHistory() {
      this.$emit("closeSearchHistory");
    },
  },
  setup(props, { emit }) {
    const router = useRouter();
    const $q = useQuasar();
    const route = useRoute();
    const store = useStore();
    const { t } = useI18n();
    const confirmDelete = ref(false);
    const toBeDeletedJob = ref({});

    const qTableSchedule: Ref<InstanceType<typeof qTable> | null> = ref(null);
    const qTableRef: Ref<InstanceType<typeof qTableSchedule> | null> = ref(null);
    const searchDateTimeRef = ref(null);
    const { searchObj, extractTimestamps } = useLogs();
    const dataToBeLoaded: any = ref([]);
    const dateTimeToBeSent = ref({
      valueType: "relative",
      relativeTimePeriod: "15m",
      startTime: 0,
      endTime: 0,
    });
    const columnsToBeRendered = ref([]);
    const expandedColumns = ref([]);
    const expandedRow = ref([]); // Array to track expanded rows
    const isLoading = ref(false);
    const isDateTimeChanged = ref(false);
    const showSearchResults = ref(false);
    const toBeCancelled = ref({});
    const confirmCancel = ref(false);
    const activeTab = ref("query");
    const query = ref("");

    const perPageOptions: any = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
    ];
    const tabs = reactive([
      {
        label: "Query / Function",
        value: "query",
      },
      {
        label: "More Details",
        value: "more_details",
      },
    ]);

    // onMounted(async ()=>{
    //   await fetchSearchHistory();
    // })

    const resultTotal = ref<number>(0);

    const pagination = ref({
      page: 1,
      rowsPerPage: 100,
    });
    const selectedPerPage = ref(pagination.value.rowsPerPage);

    const generateColumns = (data: any) => {
      if (data && data.length === 0) return [];

      // Define the desired column order and names
      const desiredColumns = [
        { key: "#", label: "#", align: "center", sortable: false },
        { key: "user_id", label: "User ID" },
        { key: "created_at", label: "Created At" },

        { key: "start_time", label: "Start Time" },
        { key: "duration", label: "Duration" },
        { key: "status", label: "Status" },
        { key: "Actions", label: "Actions" },
      ];

      return desiredColumns.map(({ key, label }) => {
        let columnWidth = 150;

        let align = "center";
        let sortable = true;
        if (key === "user_id") {
          columnWidth = 200;
          align = "center";
          sortable = true;
        }
        if (key === "Actions") {
          columnWidth = 200;
          align = "left";
          sortable = false;
        }
        if (key == "trace_id") {
          columnWidth = 250;
          sortable = false;
        }
        if (key == "duration") {
          align = "left";
          columnWidth = 100;
          sortable = true;
        }
        if (
          key == "start_time" ||
          key == "end_time" ||
          key == "created_at" ||
          key == "started_at" ||
          key == "ended_at"
        ) {
          columnWidth = 200;
          sortable = true;
        }
        if (key == "sql") {
          columnWidth = 300;
          sortable = false;
        }
        if (key == "status") {
          align = "left";
          columnWidth = 200;
          sortable = false;
        }
        if (key == "#") {
          columnWidth = 100;
          sortable = false;
          align = "left";
        }

        // Custom width for each column
        return {
          name: key, // Field name
          label: label, // Column label
          field: key, // Field accessor
          align: align,
          sortable: sortable,
          style: `max-width: ${columnWidth}px; width: ${columnWidth}px;`,
        };
      });
    };
  
    function filterRow(row) {
      const desiredColumns = [
        { key: "trace_id", label: "Trace ID" },
        { key: "start_time", label: "Start Time" },

        { key: "end_time", label: "End Time" },
        { key: "started_at", label: "Job Started At" },
        { key: "ended_at", label: "Job Ended At" },
      ];
      return desiredColumns.reduce((filtered, column) => {
        if (row[column.key] !== undefined) {
          filtered[column.key] = row[column.key];
        }
        return filtered;
      }, {});
    }

    const fetchSearchHistory = async () => {
      if(config.isEnterprise == "false"){
        return;
      }

      try {
        const { org_identifier } = router.currentRoute.value.query;
        // columnsToBeRendered.value = [];
        // dataToBeLoaded.value = [];
        isLoading.value = true;
          let responseToBeFetched = [];
          searchService
            .get_scheduled_search_list({
              org_identifier: store.state.selectedOrganization.identifier,
            })
            .then((res) => {
              responseToBeFetched = res.data;
              resultTotal.value = res.data.length;

              columnsToBeRendered.value = generateColumns(
                responseToBeFetched[0],
              );

              responseToBeFetched.forEach((element) => {
                const {formatted, raw} = calculateDuration(element.start_time, element.end_time);

                element.rawDuration = raw;
                

                element["duration"] = formatted;
                element.toBeStoredStartTime = element.start_time;
                element.toBeStoredEndTime = element.end_time;
                element.toBeCreatedAt = element.created_at;
                element.start_time = convertUnixToQuasarFormat(
                  element.start_time,
                );
                element.end_time = convertUnixToQuasarFormat(element.end_time);
                element.created_at = convertUnixToQuasarFormat(
                  element.created_at,
                );
                element.started_at = convertUnixToQuasarFormat(
                  element.started_at,
                );
                element.ended_at = convertUnixToQuasarFormat(element.ended_at);
                element.status_code = element.status;
                element["sql"] = JSON.parse(element.payload).query.sql;

                if (JSON.parse(element.payload).query.query_fn) {
                  element["function"] = b64DecodeUnicode(
                    JSON.parse(element.payload).query.query_fn,
                  );
                }
              });

              dataToBeLoaded.value = responseToBeFetched;
              isLoading.value = false;
            });
      } catch (error) {
        $q.notify({
          type: "negative",
          message: "Failed to fetch search history. Please try again later.",
          timeout: 5000,
        });
        isLoading.value = false;
      }
    };
    //this method needs to revamped / can be made shorter
    const cancelSearchJob = () => {
      searchService
        .cancel_scheduled_search({
          org_identifier: store.state.selectedOrganization.identifier,
          jobId: toBeCancelled.value.id,
        })
        .then((res) => {
          $q.notify({
            type: "positive",
            message: "Search Job has been cancelled successfully",
            timeout: 2000,
          });
        }).catch((e)=> {
          if(e.response.status.code != 403){
            $q.notify({
            type: "negative",
            message: e.response?.data?.message ||  "Failed to cancel search job",
            timeout: 2000,
          });
          }
          
        }).finally(()=> {
          fetchSearchHistory();
        })
    };
    const retrySearchJob = (row) => {
      searchService
        .retry_scheduled_search({
          org_identifier: store.state.selectedOrganization.identifier,
          jobId: row.id,
        })
        .then((res) => {
          $q.notify({
            type: "positive",
            message: "Search Job has been restarted successfully",
            timeout: 2000,
          });
        }).catch((e)=> {
          if(e.response.status.code != 403){
            $q.notify({
            type: "negative",
            message: e.response?.data?.message ||  "Failed to restart search job",
            timeout: 2000,
          });
          }
          
        }).finally(()=> {
          fetchSearchHistory();

        })
    };
    const confirmDeleteJob = (row) => {
      confirmDelete.value = true;
      toBeDeletedJob.value = row;
    };
    const confirmCancelJob = (row) => {
      confirmCancel.value = true;
      toBeCancelled.value = row;
    };
    const deleteSearchJob = () => {
      searchService
        .delete_scheduled_search({
          org_identifier: store.state.selectedOrganization.identifier,
          jobId: toBeDeletedJob.value.id,
        })
        .then((res) => {
          fetchSearchHistory();
          $q.notify({
            type: "positive",
            message: "Search Job has been deleted successfully",
            timeout: 2000,
          });
        });
    };
    const sortMethod = (rows, sortBy, descending) => {
      const data = [...rows];
      if (sortBy === "user_id") {
        if (descending) {
          return data.sort((a, b) => b.user_id.localeCompare(a.user_id));
        }
        return data.sort((a, b) => a.user_id.localeCompare(b.user_id));
      }
      if (sortBy === "duration") {
        if (descending) {
          return data.sort((a, b) => b.rawDuration - a.rawDuration);
        }
        return data.sort((a, b) => a.rawDuration - b.rawDuration);
      }
      if (sortBy == "start_time") {
        if (descending) {
          return data.sort(
            (a, b) => b.toBeStoredStartTime - a.toBeStoredStartTime,
          );
        }
        return data.sort(
          (a, b) => a.toBeStoredStartTime - b.toBeStoredStartTime,
        );
      }
      if (sortBy == "created_at") {
        if (descending) {
          return data.sort((a, b) => b.toBeCreatedAt - a.toBeCreatedAt);
        }
        return data.sort((a, b) => a.toBeCreatedAt - b.toBeCreatedAt);
      }
      // if(sortBy == "status"){
      //   if (descending) {
      //     return data.sort((a, b) => b.status - a.status);
      //   }
      //   return data.sort((a, b) => a.status - b.status);
      // }

      // return a.rawDuration - b.rawDuration;
    };
    const copyToClipboard = (text, type) => {
      navigator.clipboard
        .writeText(text)
        .then(() => {
          $q.notify({
            type: "positive",
            message: `${type} Copied Successfully!`,
            timeout: 5000,
          });
        })
        .catch(() => {
          $q.notify({
            type: "negative",
            message: "Error while copy content.",
            timeout: 5000,
          });
        });
    };
    const delayMessage = computed(() => {
      const delay = store.state.zoConfig.usage_publish_interval;
      if (delay <= 60) {
        return "60 seconds";
      } else {
        const minutes = Math.floor(delay / 60);
        return `${minutes} minute(s)`;
      }
    });

    const updateDateTime = async (value: any) => {
      const { startTime, endTime } = value;
      dateTimeToBeSent.value = value;
      searchDateTimeRef.value.setAbsoluteTime(value.startTime, value.endTime);
    };
    const formatTime = (took) => {
      return `${took.toFixed(2)} sec`;
    };
    const calculateDuration = (startTime, endTime) => {
      const durationMicroseconds = endTime - startTime;
      const durationSeconds = durationMicroseconds / 1e6;

      // Store the raw duration in a separate property
      const rawDuration = durationSeconds;

      let result = "";

      if (durationSeconds < 60) {
        result = `${durationSeconds.toFixed(2)} seconds`;
      } else if (durationSeconds < 3600) {
        const minutes = Math.floor(durationSeconds / 60);
        const seconds = durationSeconds % 60;
        result = `${minutes} minutes`;
        if (seconds > 0) {
          result += ` and ${seconds.toFixed(2)} seconds`;
        }
      } else if (durationSeconds < 86400) {
        const hours = Math.floor(durationSeconds / 3600);
        const minutes = Math.floor((durationSeconds % 3600) / 60);
        result = `${hours} hours`;
        if (minutes > 0) {
          result += ` and ${minutes} minutes`;
        }
      } else if (durationSeconds < 2592000) {
        const days = Math.floor(durationSeconds / 86400);
        const hours = Math.floor((durationSeconds % 86400) / 3600);
        result = `${days} days`;
        if (hours > 0) {
          result += ` and ${hours} hours`;
        }
      } else if (durationSeconds < 31536000) {
        const months = Math.floor(durationSeconds / 2592000);
        const days = Math.floor((durationSeconds % 2592000) / 86400);
        result = `${months} months`;
        if (days > 0) {
          result += ` and ${days} days`;
        }
      } else {
        const years = Math.floor(durationSeconds / 31536000);
        const months = Math.floor((durationSeconds % 31536000) / 2592000);
        result = `${years} years`;
        if (months > 0) {
          result += ` and ${months} months`;
        }
      }

      return { formatted: result, raw: rawDuration };
    };

    const triggerExpand = (props) => {
      query.value = JSON.stringify(filterRow(props.row), null, 2);
      if (expandedRow.value === props.row.trace_id) {
        expandedRow.value = null;
      } else {
        // Otherwise, expand the clicked row and collapse any other row
        expandedRow.value = props.row.trace_id;
      }
    };
    const goToLogs = (row) => {
      const duration_suffix = row.duration.split(" ")[1];
      const from = row.toBeStoredStartTime;
      const to = row.toBeStoredEndTime;
      const refresh = 0;

      const query = b64EncodeUnicode(row.sql);
      const rawStreamNames = JSON.parse(row.stream_names)
      const stream_name =
        rawStreamNames.length > 1
          ? rawStreamNames.join(",")
          : rawStreamNames[0];
      const queryObject = {
        stream_type: "logs",
        stream: stream_name,
        from: from,
        to: to,
        refresh,
        sql_mode: "true",
        query,
        defined_schemas: "user_defined_schema",
        org_identifier: row.org_id,
        quick_mode: "false",
        show_histogram: "false",
        type: "search_scheduler",
      };

      if (row.hasOwnProperty("function") && row.function) {
        const functionContent = b64EncodeUnicode(row.function);
        queryObject["functionContent"] = functionContent;
      }

      $q.notify({
        type: "positive",
        message: "Search Job have been applied successfully",
        timeout: 2000,
      });

      router.push({
        path: "/logs",
        query: queryObject,
      });
    };
    const changePagination = (val: { label: string; value: any }) => {
      if (val.label == "All") {
        val.value = dataToBeLoaded.value.length;
        val.label = "All";
      }
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTableSchedule.value.setPagination(pagination.value);

      // pagination.value.page = 1;
    };

    watch(
      () => props.isClicked,
      (value) => {
        if (value == true && !isLoading.value) {
          fetchSearchHistory();
        }
      },
    );
    const getStatusText = (status) => {
      switch (status) {
        case 0:
          return "Pending";
        case 1:
          return "Running";
        case 2:
          return "Finished";
        case 3:
          return "Cancelled";
        default:
          return "Unknown";
      }
    };

    const getStatusIcon = (status) => {
      switch (status) {
        case 0:
          return "hourglass_empty"; // Icon for pending
        case 1:
          return "pause_circle"; // Icon for running
        case 2:
          return "check_circle"; // Icon for finished
        case 3:
          return "cancel"; // Icon for cancelled
        default:
          return "help"; // Icon for unknown
      }
    };
    const getStatusColor = (status) => {
      switch (status) {
        case 0:
          return "orange"; // Pending color
        case 1:
          return "blue"; // Running color
        case 2:
          return "green"; // Finished color
        case 3:
          return "gray"; // Cancelled color
        default:
          return "gray"; // Unknown color
      }
    };

    function convertUnixToQuasarFormat(unixMicroseconds: any) {
      if (!unixMicroseconds) return "";
      const unixSeconds = unixMicroseconds / 1e6;
      const dateToFormat = new Date(unixSeconds * 1000);
      const formattedDate = dateToFormat.toISOString();
      return date.formatDate(formattedDate, "YYYY-MM-DDTHH:mm:ssZ");
    }
    const fetchSearchResults = (row) => {
      searchObj.meta.jobId = row.id;
      goToLogs(row);
    };
    return {
      searchObj,
      store,
      generateColumns,
      fetchSearchHistory,
      dataToBeLoaded,
      columnsToBeRendered,
      expandedColumns,
      config,
      t,
      route,
      isLoading,
      qTableSchedule,
      updateDateTime,
      pagination,
      searchDateTimeRef,
      expandedRow,
      goToLogs,
      triggerExpand,
      copyToClipboard,
      formatTime,
      delayMessage,
      sortMethod,
      resultTotal,
      perPageOptions,
      changePagination,
      selectedPerPage,
      getStatusText,
      getStatusIcon,
      getStatusColor,
      showSearchResults,
      fetchSearchResults,
      qTableRef,
      cancelSearchJob,
      retrySearchJob,
      deleteSearchJob,
      toBeDeletedJob,
      confirmDelete,
      confirmDeleteJob,
      tabs,
      activeTab,
      filterRow,
      query,
      confirmCancelJob,
      toBeCancelled,
      confirmCancel,

    };
    // Watch the searchObj for changes
  },
});
</script>
<style lang="scss" scoped>
.expanded-content {
  padding: 0 1rem;
  min-width: calc(90vw - 20px);
  max-height: 100vh; /* Set a fixed height for the container */
  overflow: hidden; /* Hide overflow by default */
}

.scrollable-content {
  width: 100%; /* Use the full width of the parent */
  overflow-y: auto; /* Enable vertical scrolling for long content */
  padding: 10px; /* Optional: padding for aesthetics */
  border: 1px solid #ddd; /* Optional: border for visibility */
  height: 100%;
  max-height: 200px;
  /* Use the full height of the parent */
  text-wrap: normal;
  background-color: #e8e8e8;
  color: black;
}

.q-td {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.custom-table .q-tr > .q-td:nth-child(2) {
  text-align: left;
}

.copy-btn-sql {
  border: #7a54a2 1px solid;
  color: #7a54a2;
}

.copy-btn-function {
  border: #0a7ebc 1px solid;
  color: #0a7ebc;
}

.warning-text {
  color: #f5a623;
  border: 1px solid #f5a623;
  border-radius: 2px;
}
.expanded-sql {
  border-left: #7a54a2 3px solid;
}
.expanded-function {
  border-left: #0a7ebc 3px solid;
}

.search-job-list-table {
  th:last-child,
  td:last-child {
    position: sticky;
    right: 0;
    z-index: 1;
    background: #ffffff;
    box-shadow: -4px 0px 4px 0 rgba(0, 0, 0, 0.1);
  }
}
td:nth-child(2) {
  text-align: center !important;
}
th:first-child,
td:first-child {
  width: 50px;
}

.dark-theme {
  th:last-child,
  td:last-child {
    background: var(--q-dark);
    box-shadow: -4px 0px 4px 0 rgba(144, 144, 144, 0.1);
  }
}

.light-theme {
  th:last-child,
  td:last-child {
    background: #ffffff;
  }
}
.search-job-exapnded-table {
}

.report-list-tabs {
  padding: 0 1rem;
  height: fit-content;
  width: fit-content;

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
