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

<!-- eslint-disable vue/v-on-event-hyphenation -->
<!-- eslint-disable vue/attribute-hyphenation -->
<template>
  <div class="col column overflow-hidden full-height">
    <div
      class="search-list full-height"
      ref="searchListContainer"
      style="width: 100%"
    >
      <div class="row">
        <div
          class="col-6 text-left q-pl-lg q-mt-xs bg-warning text-white rounded-borders"
          v-if="searchObj.data.countErrorMsg != ''"
        >
          <SanitizedHtmlRenderer
            data-test="logs-search-total-count-error-message"
            :htmlContent="searchObj.data.countErrorMsg"
          />
        </div>
        <div v-else class="col-6 text-left q-pl-lg q-mt-xs warning flex">
          {{ noOfRecordsTitle }}
          <span v-if="searchObj.loadingCounter" class="q-ml-md">
            <q-spinner-hourglass
            color="primary"
            size="25px"
            style="margin: 0 auto; display: block"
          />
          <q-tooltip
              anchor="center right"
              self="center left"
              max-width="300px"
            >
              <span style="font-size: 14px"
                >Fetching the search events</span
              >
            </q-tooltip>

          </span>
        </div>

        <div class="col-6 text-right q-pr-md q-gutter-xs pagination-block">
          <q-pagination
            v-if="searchObj.meta.resultGrid.showPagination"
            :disable="searchObj.loading == true"
            v-model="pageNumberInput"
            :key="
              searchObj.data.queryResults.total +
              '-' +
              searchObj.data.resultGrid.currentPage
            "
            :max="
              Math.max(
                1,
                (searchObj.communicationMethod === 'ws' ||
                searchObj.meta.jobId != ''
                  ? searchObj.data.queryResults?.pagination?.length
                  : searchObj.data.queryResults?.partitionDetail?.paginations
                      ?.length) || 0,
              )
            "
            :input="false"
            direction-links
            :boundary-numbers="false"
            :max-pages="5"
            :ellipses="false"
            icon-first="skip_previous"
            icon-last="skip_next"
            icon-prev="fast_rewind"
            icon-next="fast_forward"
            class="float-right paginator-section"
            @update:model-value="getPageData('pageChange')"
            rowsPerPageLabel="Rows per page"
            :rows-per-page-options="rowsPerPageOptions"
            :rows-per-page="searchObj.meta.resultGrid.rowsPerPage"
            style="line-height: 30px; max-height: 30px"
            data-test="logs-search-result-pagination"
          />
          <q-select
            v-if="searchObj.meta.resultGrid.showPagination"
            data-test="logs-search-result-records-per-page"
            v-model="searchObj.meta.resultGrid.rowsPerPage"
            :options="rowsPerPageOptions"
            class="float-right select-pagination"
            size="sm"
            dense
            @update:model-value="getPageData('recordsPerPage')"
            style="line-height: 20px"
          ></q-select>
        </div>
      </div>
      <div v-if="searchObj.data?.histogram?.errorMsg == ''">
        <ChartRenderer
          v-if="searchObj.meta.showHistogram"
          data-test="logs-search-result-bar-chart"
          :data="plotChart"
          style="max-height: 100px"
          @updated:dataZoom="onChartUpdate"
        />
        <div
          class="q-pb-lg"
          style="top: 50px; position: absolute; left: 45%"
          v-if="searchObj.loadingHistogram == true"
        >
          <q-spinner-hourglass
            color="primary"
            size="25px"
            style="margin: 0 auto; display: block"
          />
        </div>
      </div>
      <div
        v-else-if="
          searchObj.data.histogram?.errorMsg != '' &&
          searchObj.meta.showHistogram
        "
      >
        <h6
          class="text-center"
          style="margin: 30px 0px"
          v-if="searchObj.data.histogram.errorCode != 0"
        >
          <q-icon name="warning" color="warning" size="30px"></q-icon> Error
          while fetching histogram data.
          <q-btn
            @click="toggleErrorDetails"
            size="sm"
            data-test="logs-page-histogram-error-details-btn"
            >{{ t("search.histogramErrorBtnLabel") }}</q-btn
          ><br />
          <span v-if="disableMoreErrorDetails">
            {{ searchObj.data.histogram?.errorMsg }}
          </span>
        </h6>
        <h6 class="text-center" v-else>
          {{ searchObj.data.histogram?.errorMsg }}
        </h6>
      </div>
      <tenstack-table
        ref="searchTableRef"
        :columns="getColumns || []"
        :rows="searchObj.data.queryResults?.hits || []"
        :wrap="searchObj.meta.toggleSourceWrap"
        :width="getTableWidth"
        :err-msg="searchObj.data.missingStreamMessage"
        :loading="searchObj.loading"
        :functionErrorMsg="searchObj?.data?.functionError"
        :expandedRows="expandedLogs"
        :highlight-timestamp="searchObj.data?.searchAround?.indexTimestamp"
        :default-columns="!searchObj.data.stream.selectedFields.length"
        class="col-12"
        :style="{
          height: !searchObj.meta.showHistogram
            ? 'calc(100% - 40px)'
            : 'calc(100% - 140px)',
        }"
        @update:columnSizes="handleColumnSizesUpdate"
        @update:columnOrder="handleColumnOrderUpdate"
        @copy="copyLogToClipboard"
        @add-field-to-table="addFieldToTable"
        @add-search-term="addSearchTerm"
        @close-column="closeColumn"
        @click:data-row="openLogDetails"
        @expand-row="expandLog"
        @view-trace="redirectToTraces"
      />

      <q-dialog
        data-test="logs-search-result-detail-dialog"
        v-model="searchObj.meta.showDetailTab"
        position="right"
        full-height
        maximized
        @escap.stop="reDrawChart"
        @hide="reDrawChart"
        @before-hide="reDrawChart"
      >
        <DetailTable
          v-if="searchObj.data.queryResults?.hits?.length"
          :key="
            'dialog_' + searchObj.meta.resultGrid.navigation.currentRowIndex
          "
          v-model="
            searchObj.data.queryResults.hits[
              searchObj.meta.resultGrid.navigation.currentRowIndex
            ]
          "
          :stream-type="searchObj.data.stream.streamType"
          style="margin-bottom: 15px"
          :currentIndex="searchObj.meta.resultGrid.navigation.currentRowIndex"
          :totalLength="parseInt(searchObj.data.queryResults.hits.length)"
          @showNextDetail="navigateRowDetail"
          @showPrevDetail="navigateRowDetail"
          @add:searchterm="addSearchTerm"
          @remove:searchterm="removeSearchTerm"
          @search:timeboxed="onTimeBoxed"
          @add:table="addFieldToTable"
          @view-trace="
            redirectToTraces(
              searchObj.data.queryResults.hits[
                searchObj.meta.resultGrid.navigation.currentRowIndex
              ],
            )
          "
        />
      </q-dialog>
    </div>
  </div>
</template>

<script lang="ts">
import {
  computed,
  defineComponent,
  ref,
  onMounted,
  onUpdated,
  defineAsyncComponent,
  watch,
} from "vue";
import { copyToClipboard, useQuasar } from "quasar";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";

import HighLight from "../../components/HighLight.vue";
import { byString } from "../../utils/json";
import { getImageURL, useLocalWrapContent } from "../../utils/zincutils";
import useLogs from "../../composables/useLogs";
import { convertLogData } from "@/utils/logs/convertLogData";
import SanitizedHtmlRenderer from "@/components/SanitizedHtmlRenderer.vue";
import { useRouter } from "vue-router";
import TenstackTable from "./TenstackTable.vue";

export default defineComponent({
  name: "SearchResult",
  components: {
    DetailTable: defineAsyncComponent(() => import("./DetailTable.vue")),
    ChartRenderer: defineAsyncComponent(
      () => import("@/components/dashboards/panels/ChartRenderer.vue"),
    ),
    SanitizedHtmlRenderer,
    TenstackTable: defineAsyncComponent(() => import("./TenstackTable.vue")),
  },
  emits: [
    "update:scroll",
    "update:datetime",
    "remove:searchTerm",
    "search:timeboxed",
    "expandlog",
    "update:recordsPerPage",
    "update:columnSizes",
  ],
  props: {
    expandedLogs: {
      type: Array,
      default: () => [],
    },
  },
  methods: {
    handleColumnSizesUpdate(newColSizes: any) {
      const prevColSizes =
        this.searchObj.data.resultGrid?.colSizes[
          this.searchObj.data.stream.selectedStream
        ]?.[0] || {};
      this.searchObj.data.resultGrid.colSizes[
        this.searchObj.data.stream.selectedStream
      ] = [
        {
          ...prevColSizes,
          ...newColSizes,
        },
      ];
    },
    handleColumnOrderUpdate(newColOrder: string[], columns: any[]) {
      // Here we are checking if the columns are default columns ( _timestamp and source)
      // If selected fields are empty, then we are setting colOrder to empty array as we
      // don't change the order of default columns
      // If you store the colOrder it will create issue when you save the view and load it again

      if (!this.searchObj.data.stream.selectedFields.length) {
        this.searchObj.data.resultGrid.colOrder[
          this.searchObj.data.stream.selectedStream
        ] = [];
      } else {
        this.searchObj.data.resultGrid.colOrder[
          this.searchObj.data.stream.selectedStream
        ] = [...newColOrder];
      }
    },

    getPageData(actionType: string) {
      if (actionType == "prev") {
        if (this.searchObj.data.resultGrid.currentPage > 1) {
          this.searchObj.data.resultGrid.currentPage =
            this.searchObj.data.resultGrid.currentPage - 1;
          this.pageNumberInput = this.searchObj.data.resultGrid.currentPage;
          this.$emit("update:scroll");
          this.scrollTableToTop(0);
        }
      } else if (actionType == "next") {
        if (
          this.searchObj.data.resultGrid.currentPage <=
          Math.round(
            this.searchObj.data.queryResults.total /
              this.searchObj.meta.resultGrid.rowsPerPage,
          )
        ) {
          this.searchObj.data.resultGrid.currentPage =
            this.searchObj.data.resultGrid.currentPage + 1;
          this.pageNumberInput = this.searchObj.data.resultGrid.currentPage;
          this.$emit("update:scroll");
          this.scrollTableToTop(0);
        }
      } else if (actionType == "recordsPerPage") {
        this.searchObj.data.resultGrid.currentPage = 1;
        this.pageNumberInput = this.searchObj.data.resultGrid.currentPage;
        if (this.searchObj.communicationMethod === "ws") {
            if (this.searchObj.meta.jobId === "") {
              this.refreshPagination();
            } else {
              this.refreshJobPagination();
            }
          } else {
            if (this.searchObj.meta.jobId !== "") {
              this.refreshJobPagination();
            } else {
              this.refreshPartitionPagination(true);
            }
          }
        this.$emit("update:recordsPerPage");
        this.scrollTableToTop(0);
      } else if (actionType == "pageChange") {
        const maxPages =
          this.searchObj.communicationMethod === "ws" ||
          this.searchObj.meta.jobId != ""
            ? this.searchObj.data.queryResults.pagination.length
            : this.searchObj.data.queryResults.partitionDetail.paginations
                .length;

        if (this.pageNumberInput > Math.ceil(maxPages)) {
          this.$q.notify({
            type: "negative",
            message:
              "Page number is out of range. Please provide valid page number.",
            timeout: 1000,
          });
          this.pageNumberInput = this.searchObj.data.resultGrid.currentPage;
          return false;
        }

        this.searchObj.data.resultGrid.currentPage = this.pageNumberInput;
        this.$emit("update:scroll");
        this.scrollTableToTop(0);
      }
    },
    closeColumn(col: any) {
      let selectedFields = this.reorderSelectedFields();

      const RGIndex = this.searchObj.data.resultGrid.columns.indexOf(col.id);
      this.searchObj.data.resultGrid.columns.splice(RGIndex, 1);

      const SFIndex = selectedFields.indexOf(col.name);

      selectedFields.splice(SFIndex, 1);

      this.searchObj.data.stream.selectedFields = selectedFields;

      this.searchObj.organizationIdentifier =
        this.store.state.selectedOrganization.identifier;
      this.updatedLocalLogFilterField();
    },
    onChartUpdate({ start, end }: { start: any; end: any }) {
      this.searchObj.meta.showDetailTab = false;
      this.$emit("update:datetime", { start, end });
    },
    onTimeBoxed(obj: any) {
      this.searchObj.meta.showDetailTab = false;
      this.searchObj.data.searchAround.indexTimestamp = obj.key;
      // this.$emit("search:timeboxed", obj);
      this.searchAroundData(obj);
    },
    toggleErrorDetails() {
      this.disableMoreErrorDetails = !this.disableMoreErrorDetails;
    },
  },
  setup(props, { emit }) {
    // Accessing nested JavaScript objects and arrays by string path
    // https://stackoverflow.com/questions/6491463/accessing-nested-javascript-objects-and-arrays-by-string-path
    const { t } = useI18n();
    const store = useStore();
    const $q = useQuasar();
    const searchListContainer = ref(null);
    const noOfRecordsTitle = ref("");
    const scrollPosition = ref(0);
    const rowsPerPageOptions = [10, 25, 50, 100, 250, 500];
    const disableMoreErrorDetails = ref(false);
    const router = useRouter();

    const {
      searchObj,
      updatedLocalLogFilterField,
      searchAroundData,
      extractFTSFields,
      refreshPartitionPagination,
      filterHitsColumns,
      reorderSelectedFields,
      getFilterExpressionByFieldType,
      refreshPagination,
      refreshJobPagination,
    } = useLogs();
    const pageNumberInput = ref(1);
    const totalHeight = ref(0);

    const searchTableRef: any = ref(null);

    const plotChart: any = ref({});

    onMounted(() => {
      reDrawChart();
    });

    onUpdated(() => {
      pageNumberInput.value = searchObj.data.resultGrid.currentPage;
    });
    const columnSizes = ref({});

    const reDrawChart = () => {
      if (
        // eslint-disable-next-line no-prototype-builtins
        searchObj.data.histogram.hasOwnProperty("xData") &&
        searchObj.data.histogram.xData.length > 0
        // && plotChart.value?.reDraw
      ) {
        //format data in form of echarts options
        plotChart.value = convertLogData(
          searchObj.data.histogram.xData,
          searchObj.data.histogram.yData,
          searchObj.data.histogram.chartParams,
        );
        // plotChart.value.forceReLayout();
      }
    };

    const changeMaxRecordToReturn = (val: any) => {
      // searchObj.meta.resultGrid.pagination.rowsPerPage = val;
    };

    const openLogDetails = (props: any, index: number) => {
      searchObj.meta.showDetailTab = true;
      searchObj.meta.resultGrid.navigation.currentRowIndex = index;
    };

    const getRowIndex = (next: boolean, prev: boolean, oldIndex: number) => {
      if (next) {
        return oldIndex + 1;
      } else {
        return oldIndex - 1;
      }
    };

    const navigateRowDetail = (isNext: boolean, isPrev: boolean) => {
      const newIndex = getRowIndex(
        isNext,
        isPrev,
        Number(searchObj.meta.resultGrid.navigation.currentRowIndex),
      );
      searchObj.meta.resultGrid.navigation.currentRowIndex = newIndex;
    };

    const addSearchTerm = (
      field: string | number,
      field_value: string | number | boolean,
      action: string,
    ) => {
      const searchExpression = getFilterExpressionByFieldType(
        field,
        field_value,
        action,
      );
      searchObj.data.stream.addToFilter = searchExpression;
    };

    const removeSearchTerm = (term: string) => {
      emit("remove:searchTerm", term);
    };

    const expandLog = async (index: number) => {
      emit("expandlog", index);
    };

    const getWidth = computed(() => {
      console.log("get search width", searchListContainer);
      return "";
    });

    function addFieldToTable(fieldName: string) {
      if (searchObj.data.stream.selectedFields.includes(fieldName)) {
        searchObj.data.stream.selectedFields =
          searchObj.data.stream.selectedFields.filter(
            (v: any) => v !== fieldName,
          );
      } else {
        searchObj.data.stream.selectedFields.push(fieldName);
      }
      searchObj.organizationIdentifier =
        store.state.selectedOrganization.identifier;
      updatedLocalLogFilterField();
      filterHitsColumns();
    }

    const copyLogToClipboard = (log: any, copyAsJson: boolean = true) => {
      const copyData = copyAsJson ? JSON.stringify(log) : log;
      copyToClipboard(copyData).then(() =>
        $q.notify({
          type: "positive",
          message: "Content Copied Successfully!",
          timeout: 1000,
        }),
      );
    };

    const redirectToTraces = (log: any) => {
      // 15 mins +- from the log timestamp
      const from = log[store.state.zoConfig.timestamp_column] - 900000000;
      const to = log[store.state.zoConfig.timestamp_column] + 900000000;
      const refresh = 0;

      const query: any = {
        name: "traceDetails",
        query: {
          stream: searchObj.meta.selectedTraceStream,
          from,
          to,
          refresh,
          org_identifier: store.state.selectedOrganization.identifier,
          trace_id:
            log[
              store.state.organizationData.organizationSettings
                .trace_id_field_name
            ],
          reload: "true",
        },
      };

      query["span_id"] =
        log[
          store.state.organizationData.organizationSettings.span_id_field_name
        ];

      router.push(query);
    };

    const getTableWidth = computed(() => {
      const leftSidebarMenu = 56;
      const fieldList =
        (window.innerWidth - leftSidebarMenu) *
        (searchObj.config.splitterModel / 100);
      return window.innerWidth - (leftSidebarMenu + fieldList) - 5;
    });

    const scrollTableToTop = (value: number) => {
      searchTableRef.value?.parentRef?.scrollTo({ top: value });
    };

    const getColumns = computed(() => {
      return searchObj.data?.resultGrid?.columns?.filter(
        (col: any) => !!col.id,
      );
    });

    const getPartitionPaginations = computed(() => {
      return searchObj.data.queryResults?.partitionDetail?.paginations || [];
    });

    const getSocketPaginations = computed(() => {
      return searchObj.data.queryResults.pagination || [];
    });

    const getPaginations = computed(() => {
      try {
        if (searchObj.communicationMethod === "http") {
          return getPartitionPaginations.value || [];
        } else {
          return getSocketPaginations.value || [];
        }
      } catch (e) {
        return [];
      }
    });

    return {
      t,
      store,
      plotChart,
      searchObj,
      updatedLocalLogFilterField,
      byString,
      searchTableRef,
      searchAroundData,
      addSearchTerm,
      removeSearchTerm,
      openLogDetails,
      changeMaxRecordToReturn,
      navigateRowDetail,
      totalHeight,
      reDrawChart,
      expandLog,
      getImageURL,
      addFieldToTable,
      searchListContainer,
      getWidth,
      copyLogToClipboard,
      extractFTSFields,
      useLocalWrapContent,
      noOfRecordsTitle,
      scrollPosition,
      rowsPerPageOptions,
      pageNumberInput,
      refreshPartitionPagination,
      disableMoreErrorDetails,
      redirectToTraces,
      getTableWidth,
      scrollTableToTop,
      getColumns,
      reorderSelectedFields,
      getPaginations,
      refreshPagination,
      refreshJobPagination,
    };
  },
  computed: {
    toggleWrapFlag() {
      return this.searchObj.meta.toggleSourceWrap;
    },
    findFTSFields() {
      return this.searchObj.data.stream.selectedStreamFields;
    },
    updateTitle() {
      return this.searchObj.data.histogram.chartParams.title;
    },
    reDrawChartData() {
      return this.searchObj.data.histogram;
    },
  },
  watch: {
    toggleWrapFlag() {
      this.useLocalWrapContent(this.searchObj.meta.toggleSourceWrap);
    },
    findFTSFields() {
      this.extractFTSFields();
    },
    updateTitle() {
      console.log( this.searchObj.data.histogram.chartParams.title,'title in index')
      this.noOfRecordsTitle = this.searchObj.data.histogram.chartParams.title;
    },
    reDrawChartData: {
      deep: true,
      handler: function () {
        this.reDrawChart();
      },
    },
  },
});
</script>

<style lang="scss" scoped>
.max-result {
  width: 170px;
}

.pagination-block {
  .q-field--dense .q-field__control,
  .q-field--dense .q-field__marginal {
    height: 30px !important;
  }

  .select-pagination {
    position: relative;
    top: -5px;
  }
}

.search-list {
  width: 100%;

  .chart {
    width: 100%;
    border-bottom: 1px solid rgba(0, 0, 0, 0.12);
  }

  .my-sticky-header-table {
    .q-table__top,
    .q-table__bottom,
    thead tr:first-child th {
      /* bg color is important for th; just specify one */
      background-color: white;
    }

    thead tr th {
      position: sticky;
      z-index: 1;
    }

    thead tr:first-child th {
      top: 0;
    }

    /* this is when the loading indicator appears */
    &.q-table--loading thead tr:last-child th {
      /* height of all previous header rows */
      top: 48px;
    }
  }

  .q-table__top {
    padding-left: 0;
    padding-top: 0;
  }

  .q-table thead tr,
  .q-table tbody td,
  .q-table th,
  .q-table td {
    height: 25px;
    padding: 0px 5px;
    font-size: 0.75rem;
  }

  .q-table__bottom {
    width: 100%;
  }

  .q-table__bottom {
    min-height: 40px;
    padding-top: 0;
    padding-bottom: 0;
  }

  .q-td {
    overflow: hidden;
    min-width: 100px;

    .expanded {
      margin: 0;
      white-space: pre-wrap;
      word-wrap: break-word;
      word-break: break-all;
    }
  }

  .highlight {
    background-color: rgb(255, 213, 0);
  }

  .table-header {
    // text-transform: capitalize;

    .table-head-chip {
      background-color: #f5f5f5;
      padding: 0px;

      .header-col-title {
        margin-right: 0.5rem;
        font-size: 14px;
        color: $dark;
      }

      .close-icon {
        &:hover {
          opacity: 0.7;
        }
      }

      .q-table th.sortable {
        cursor: pointer;
        text-transform: capitalize;
        font-weight: bold;
      }
    }

    &.isClosable {
      padding-right: 30px;
      position: relative;

      .q-table-col-close {
        transform: translateX(26px);
        position: absolute;
        margin-top: 2px;
        color: #808080;
      }
    }

    .q-table th.sortable {
      cursor: pointer;
      text-transform: capitalize;
      font-weight: bold;
    }
  }
}
.thead-sticky tr > *,
.tfoot-sticky tr > * {
  position: sticky;
  opacity: 1;
  z-index: 1;
  background: #f5f5f5;
}

.q-table--dark .thead-sticky tr > *,
.q-table--dark .tfoot-sticky tr > * {
  background: #565656;
}

.q-table--dark .table-header {
  // text-transform: capitalize;

  .table-head-chip {
    background-color: #565656;
  }
}

.thead-sticky tr:last-child > * {
  top: 0;
}

.tfoot-sticky tr:first-child > * {
  bottom: 0;
}

.field_list,
.table-head-chip {
  padding: 0px;
  margin-bottom: 0.125rem;
  position: relative;
  overflow: visible;
  cursor: default;
  font-size: 12px;
  font-family: monospace;

  .field_overlay {
    width: fit-content;
    position: absolute;
    height: 100%;
    right: 0;
    top: 0;
    background-color: #ffffff;
    border-radius: 6px;
    padding: 0 6px;
    visibility: hidden;
    display: flex;
    align-items: center;
    transition: all 0.3s linear;

    &.field_overlay_dark {
      background-color: #181a1b;
    }

    .q-icon,
    .q-toggle__inner {
      cursor: pointer;
      opacity: 0;
      transition: all 0.3s linear;
      margin: 0 1px;
    }
  }

  &:hover {
    .field_overlay {
      visibility: visible;

      .q-icon,
      .q-toggle__inner {
        opacity: 1;
      }
    }
  }
}

.table-head-chip {
  font-family: "Nunito Sans", sans-serif;
  .field_overlay {
    background-color: #f5f5f5;

    &.field_overlay_dark {
      background-color: #565656;
    }
  }
}
</style>

<style lang="scss">
.search-list {
  .copy-log-btn {
    .q-icon {
      font-size: 12px !important;
    }
  }

  .view-trace-btn {
    .q-icon {
      font-size: 13px !important;
    }
  }

  .q-pagination__content input {
    border: 1px solid lightgrey;
    top: 7px;
    position: relative;
    height: 30px;
  }
}
</style>
