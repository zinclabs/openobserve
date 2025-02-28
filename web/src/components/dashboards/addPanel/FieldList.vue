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
  <div
    class="column index-menu"
    :class="store.state.theme == 'dark' ? 'theme-dark' : 'theme-light'"
  >
    <div class="col-auto">
      <!-- stream type selection will be hidden for metrics page -->
      <q-select
        v-if="dashboardPanelDataPageKey !== 'metrics'"
        v-model="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream_type
        "
        :label="t('dashboard.selectStreamType')"
        :options="data.streamType"
        data-test="index-dropdown-stream_type"
        input-debounce="0"
        behavior="menu"
        filled
        borderless
        dense
        class="q-mb-xs"
      ></q-select>
      <q-select
        v-model="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream
        "
        :label="t('dashboard.selectIndex')"
        :options="filteredStreams"
        data-test="index-dropdown-stream"
        input-debounce="0"
        behavior="menu"
        use-input
        filled
        borderless
        dense
        hide-selected
        fill-input
        @filter="filterStreamFn"
        :loading="streamDataLoading.isLoading.value"
        option-label="name"
        option-value="name"
        emit-value
        :class="
          selectedMetricTypeIcon &&
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream_type == 'metrics'
            ? 'metric_icon_present'
            : ''
        "
      >
        <template
          v-if="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.stream_type == 'metrics' && selectedMetricTypeIcon
          "
          v-slot:prepend
        >
          <q-icon
            style="margin-top: 14px"
            size="xs"
            :name="metricsIconMapping[selectedMetricTypeIcon || '']"
          />
        </template>

        <template
          v-slot:option="scope"
          v-if="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.stream_type == 'metrics'
          "
        >
          <q-item
            :class="
              store.state.theme === 'dark' &&
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.stream !== scope.opt.value
                ? 'text-white'
                : ''
            "
            v-bind="scope.itemProps"
          >
            <q-item-section avatar class="metric-explore-metric-icon">
              <q-icon
                size="xs"
                :name="
                  metricsIconMapping[scope.opt.metrics_meta.metric_type] || ''
                "
              />
            </q-item-section>
            <q-item-section>
              <q-item-label> {{ scope.opt.name }} </q-item-label>
            </q-item-section>
          </q-item>
        </template>

        <template #no-option>
          <q-item>
            <q-item-section> {{ t("search.noResult") }}</q-item-section>
          </q-item>
        </template>
      </q-select>
    </div>
    <div class="column col index-table q-mt-xs">
      <q-table
        class="col"
        :columns="[
          {
            name: 'name',
            field: 'name',
            align: 'left',
            label: 'Field',
            sortable: true,
          },
        ]"
        :rows="flattenGroupedFields"
        v-model:pagination="pagination"
        row-key="column"
        :filter="dashboardPanelData.meta.stream.filterField"
        :filter-method="filterFieldFn"
        hide-header
        virtual-scroll
        id="fieldList"
        :rows-per-page-options="[]"
        :hide-bottom="
          (!store.state.zoConfig.user_defined_schemas_enabled ||
            dashboardPanelData.meta.stream.userDefinedSchema.length == 0) &&
          dashboardPanelData.meta.stream.selectedStreamFields != undefined &&
          (dashboardPanelData.meta.stream.selectedStreamFields.length <=
            pagination.rowsPerPage ||
            dashboardPanelData.meta.stream.selectedStreamFields.length == 0)
        "
      >
        <template #body-cell-name="props">
          <q-tr :props="props">
            <q-td
              class="field_list"
              :props="props"
              v-mutation="mutationHandler"
              @dragenter="onDragEnter"
              @dragleave="onDragLeave"
              @dragover="onDragOver"
              @drop="onDrop"
              @dragend="onDragEnd"
              :style="
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].customQuery && props.pageIndex == customQueryFieldsLength
                  ? 'border: 1px solid black'
                  : ''
              "
            >
              <div
                v-if="props?.row?.isGroup"
                class="tw-pl-2 tw-py-1 tw-font-semibold tw-bg-gray-200"
              >
                {{ props?.row?.groupName }}
              </div>
              <div
                v-else
                class="field_overlay"
                :title="props.row.name"
                :data-test="`field-list-item-${
                  dashboardPanelData.data.queries[
                    dashboardPanelData.layout.currentQueryIndex
                  ].fields?.stream_type
                }-${
                  dashboardPanelData.data.queries[
                    dashboardPanelData.layout.currentQueryIndex
                  ].fields?.stream
                }-${props.row.name}`"
              >
                <div
                  class="field_label"
                  :draggable="
                    !(
                      promqlMode ||
                      (dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery &&
                        props.pageIndex >= customQueryFieldsLength)
                    )
                  "
                  @dragstart="onDragStart($event, props.row)"
                >
                  <q-icon
                    name="drag_indicator"
                    color="grey-13"
                    :class="[
                      'q-mr-xs',
                      !(
                        promqlMode ||
                        (dashboardPanelData.data.queries[
                          dashboardPanelData.layout.currentQueryIndex
                        ].customQuery &&
                          props.pageIndex >= customQueryFieldsLength)
                      )
                        ? 'drag_indicator'
                        : 'drag_disabled',
                    ]"
                    v-if="!promqlMode"
                    data-test="dashboard-add-data-indicator"
                  />

                  <q-icon
                    :name="
                      props.row.type == 'Utf8'
                        ? 'text_fields'
                        : props.row.type == 'Int64'
                          ? 'tag'
                          : 'toggle_off'
                    "
                    color="grey-6"
                    class="q-mr-xs"
                  />
                  {{ props.row.name }}
                </div>
                <div
                  class="field_icons"
                  v-if="
                    !(
                      promqlMode ||
                      (dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery &&
                        props.pageIndex >= customQueryFieldsLength) ||
                      dashboardPanelData.data.type == 'geomap' ||
                      dashboardPanelData.data.type == 'maps' ||
                      dashboardPanelData.data.type == 'custom_chart'
                    )
                  "
                >
                  <q-btn
                    padding="sm"
                    :disabled="isAddXAxisNotAllowed"
                    @click="addXAxisItem(props.row)"
                    data-test="dashboard-add-x-data"
                  >
                    <div>
                      {{
                        dashboardPanelData.data.type != "h-bar" ? "+X" : "+Y"
                      }}
                    </div>
                  </q-btn>
                  <q-btn
                    padding="sm"
                    :disabled="isAddYAxisNotAllowed"
                    @click="addYAxisItem(props.row)"
                    data-test="dashboard-add-y-data"
                  >
                    <div>
                      {{
                        dashboardPanelData.data.type != "h-bar" ? "+Y" : "+X"
                      }}
                    </div>
                  </q-btn>
                  <q-btn
                    v-if="
                      dashboardPanelData.data.type == 'area' ||
                      dashboardPanelData.data.type == 'bar' ||
                      dashboardPanelData.data.type == 'line' ||
                      dashboardPanelData.data.type == 'h-bar' ||
                      dashboardPanelData.data.type == 'h-stacked' ||
                      dashboardPanelData.data.type == 'scatter' ||
                      dashboardPanelData.data.type == 'area-stacked' ||
                      dashboardPanelData.data.type == 'stacked'
                    "
                    padding="sm"
                    :disabled="isAddBreakdownNotAllowed"
                    @click="addBreakDownAxisItem(props.row)"
                    data-test="dashboard-add-b-data"
                  >
                    <div>
                      {{
                        dashboardPanelData.data.type != "h-bar" ? "+B" : "+B"
                      }}
                    </div>
                  </q-btn>
                  <q-btn
                    v-if="dashboardPanelData.data.type == 'heatmap'"
                    padding="sm"
                    :disabled="isAddZAxisNotAllowed"
                    @click="addZAxisItem(props.row)"
                    data-test="dashboard-add-z-data"
                  >
                    <div>+Z</div>
                  </q-btn>
                  <q-btn
                    v-if="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery == false
                    "
                    :disable="
                      !!dashboardPanelData.meta.stream.vrlFunctionFieldList.find(
                        (vrlField: any) => vrlField.name == props.row.name,
                      )
                    "
                    padding="sm"
                    @click="addFilteredItem(props.row.name)"
                    data-test="dashboard-add-filter-data"
                  >
                    <div>+F</div>
                  </q-btn>
                </div>
                <div
                  class="field_icons"
                  v-if="
                    !(
                      promqlMode ||
                      (dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery &&
                        props.pageIndex >= customQueryFieldsLength)
                    ) && dashboardPanelData.data.type == 'geomap'
                  "
                >
                  <q-btn
                    :disabled="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields?.latitude != null
                    "
                    no-caps
                    padding="sm"
                    @click="addLatitude(props.row)"
                    data-test="dashboard-add-latitude-data"
                  >
                    <div>+Lat</div>
                  </q-btn>
                  <q-btn
                    :disabled="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields?.longitude != null
                    "
                    no-caps
                    padding="sm"
                    @click="addLongitude(props.row)"
                    data-test="dashboard-add-longitude-data"
                  >
                    <div>+Lng</div>
                  </q-btn>
                  <q-btn
                    :disabled="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields?.weight != null
                    "
                    padding="sm"
                    @click="addWeight(props.row)"
                    data-test="dashboard-add-weight-data"
                  >
                    <div>+W</div>
                  </q-btn>
                  <q-btn
                    v-if="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery == false
                    "
                    :disable="
                      !!dashboardPanelData.meta.stream.vrlFunctionFieldList.find(
                        (vrlField: any) => vrlField.name == props.row.name,
                      )
                    "
                    padding="sm"
                    @click="addFilteredItem(props.row.name)"
                    data-test="dashboard-add-filter-geomap-data"
                  >
                    <div>+F</div>
                  </q-btn>
                </div>
                <div
                  class="field_icons"
                  v-if="
                    !(
                      promqlMode ||
                      (dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery &&
                        props.pageIndex >= customQueryFieldsLength)
                    ) && dashboardPanelData.data.type == 'maps'
                  "
                >
                  <q-btn
                    :disabled="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields?.name != null
                    "
                    no-caps
                    padding="sm"
                    @click="addMapName(props.row)"
                    data-test="dashboard-add-x-data"
                  >
                    <div>+N</div>
                  </q-btn>
                  <q-btn
                    :disabled="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields?.value_for_maps != null
                    "
                    no-caps
                    padding="sm"
                    @click="addMapValue(props.row)"
                    data-test="dashboard-add-y-data"
                  >
                    <div>+V</div>
                  </q-btn>
                  <q-btn
                    padding="sm"
                    @click="addFilteredItem(props.row.name)"
                    data-test="dashboard-add-filter-maps-data"
                  >
                    <div>+F</div>
                  </q-btn>
                </div>

                <div
                  class="field_icons"
                  v-if="
                    !(
                      promqlMode ||
                      (dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery &&
                        props.pageIndex >= customQueryFieldsLength)
                    ) && dashboardPanelData.data.type == 'sankey'
                  "
                >
                  <q-btn
                    :disabled="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields?.source != null
                    "
                    no-caps
                    padding="sm"
                    @click="addSource(props.row)"
                    data-test="dashboard-add-source-data"
                  >
                    <div>+S</div>
                  </q-btn>
                  <q-btn
                    :disabled="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields?.target != null
                    "
                    no-caps
                    padding="sm"
                    @click="addTarget(props.row)"
                    data-test="dashboard-add-target-data"
                  >
                    <div>+T</div>
                  </q-btn>
                  <q-btn
                    :disabled="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields?.value != null
                    "
                    padding="sm"
                    @click="addValue(props.row)"
                    data-test="dashboard-add-value-data"
                  >
                    <div>+V</div>
                  </q-btn>
                  <q-btn
                    v-if="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery == false
                    "
                    :disable="
                      !!dashboardPanelData.meta.stream.vrlFunctionFieldList.find(
                        (vrlField: any) => vrlField.name == props.row.name,
                      )
                    "
                    padding="sm"
                    @click="addFilteredItem(props.row.name)"
                    data-test="dashboard-add-filter-sankey-data"
                  >
                    <div>+F</div>
                  </q-btn>
                </div>
              </div>
            </q-td>
          </q-tr>
        </template>
        <template #top-right>
          <q-input
            v-model="dashboardPanelData.meta.stream.filterField"
            data-test="index-field-search-input"
            filled
            borderless
            dense
            clearable
            debounce="1"
            :loading="getStreamFields.isLoading.value"
            :placeholder="t('search.searchField')"
          >
            <template #prepend>
              <q-icon name="search" />
            </template>
          </q-input>
        </template>
        <template v-slot:pagination="scope">
          <div
            v-if="
              store.state.zoConfig.user_defined_schemas_enabled &&
              dashboardPanelData.meta.stream.userDefinedSchema.length > 0
            "
            class="fieldList-pagination"
          >
            <q-btn-toggle
              no-caps
              v-model="dashboardPanelData.meta.stream.useUserDefinedSchemas"
              data-test="dashboard-page-field-list-user-defined-schema-toggle"
              class="schema-field-toggle q-mr-xs"
              toggle-color="primary"
              bordered
              size="8px"
              color="white"
              text-color="primary"
              :options="userDefinedSchemaBtnGroupOption"
              >
              <!-- @update:model-value="toggleSchema" -->
              <template v-slot:user_defined_slot>
                <q-icon name="person"></q-icon>
                <q-icon name="schema"></q-icon>
                <q-tooltip
                  data-test="dashboard-page-fields-list-user-defined-fields-warning-tooltip"
                  anchor="center right"
                  self="center left"
                  max-width="300px"
                  class="text-body2"
                >
                  <span class="text-bold" color="white">{{
                    t("search.userDefinedSchemaLabel")
                  }}</span>
                </q-tooltip>
              </template>
              <template v-slot:all_fields_slot>
                <q-icon name="schema"></q-icon>
                <q-tooltip
                  data-test="dashboard-page-fields-list-all-fields-warning-tooltip"
                  anchor="center right"
                  self="center left"
                  max-width="300px"
                  class="text-body2"
                >
                  <span class="text-bold" color="white">{{
                    t("search.allFieldsLabel")
                  }}</span>
                  <q-separator color="white" class="q-mt-xs q-mb-xs" />
                  {{ t("search.allFieldsWarningMsg") }}
                </q-tooltip>
              </template>
            </q-btn-toggle>
          </div>
          <div class="q-ml-xs text-right col" v-if="scope.pagesNumber > 1">
            <q-tooltip
              data-test="dashboard-page-fields-list-pagination-tooltip"
              anchor="center right"
              self="center left"
              max-width="300px"
              class="text-body2"
            >
              Total Fields:
              {{ dashboardPanelData.meta.stream.selectedStreamFields.length }}
            </q-tooltip>
            <q-btn
              data-test="dashboard-page-fields-list-pagination-firstpage-button"
              v-if="scope.pagesNumber > 2"
              icon="skip_previous"
              color="grey-8"
              round
              dense
              flat
              :disable="scope.isFirstPage"
              @click="scope.firstPage"
            />

            <q-btn
              data-test="dashboard-page-fields-list-pagination-previouspage-button"
              icon="fast_rewind"
              color="grey-8"
              round
              dense
              flat
              :disable="scope.isFirstPage"
              @click="scope.prevPage"
            />

            <q-btn
              round
              data-test="dashboard-page-fields-list-pagination-message-button"
              dense
              flat
              class="text text-caption text-regular"
              >{{ scope.pagination.page }}/{{ scope.pagesNumber }}</q-btn
            >

            <q-btn
              data-test="dashboard-page-fields-list-pagination-nextpage-button"
              icon="fast_forward"
              color="grey-8"
              round
              dense
              flat
              :disable="scope.isLastPage"
              @click="scope.nextPage"
            />

            <q-btn
              data-test="dashboard-page-fields-list-pagination-lastpage-button"
              v-if="scope.pagesNumber > 2"
              icon="skip_next"
              color="grey-8"
              round
              dense
              flat
              :disable="scope.isLastPage"
              @click="scope.lastPage"
            />
          </div>
        </template>
      </q-table>
    </div>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  reactive,
  ref,
  watch,
  onActivated,
  computed,
  onMounted,
} from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import { useLoading } from "@/composables/useLoading";
import useStreams from "@/composables/useStreams";
import { inject } from "vue";
import useNotifications from "@/composables/useNotifications";

export default defineComponent({
  name: "FieldList",
  props: ["editMode"],
  setup(props, { emit }) {
    const dashboardPanelDataPageKey: any = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );

    const userDefinedSchemaBtnGroupOption = [
      {
        label: "",
        value: "user_defined_schema",
        slot: "user_defined_slot",
      },
      {
        label: "",
        value: "all_fields",
        slot: "all_fields_slot",
      },
    ];

    const pagination = ref({
      page: 1,
      rowsPerPage: 250,
    });

    // custom query fields length
    // will be updated when filter is applied
    const customQueryFieldsLength = ref(0);

    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const data = reactive<any>({
      // schemaList: [],
      // indexOptions: [],
      streamType: ["logs", "metrics", "traces"],
      // currentFieldsList: [],
    });
    const filteredStreams = ref([]);
    const {
      dashboardPanelData,
      addXAxisItem,
      addYAxisItem,
      addZAxisItem,
      addBreakDownAxisItem,
      addFilteredItem,
      isAddXAxisNotAllowed,
      isAddBreakdownNotAllowed,
      isAddYAxisNotAllowed,
      isAddZAxisNotAllowed,
      promqlMode,
      addLatitude,
      addLongitude,
      addWeight,
      addMapName,
      addMapValue,
      addSource,
      addTarget,
      addValue,
      cleanupDraggingFields,
      updateGroupedFields,
    } = useDashboardPanelData(dashboardPanelDataPageKey);
    const { getStreams, getStream } = useStreams();
    const { showErrorNotification } = useNotifications();
    const onDragEnd = () => {
      cleanupDraggingFields();
    };

    const metricsIconMapping: any = {
      Summary: "description",
      Gauge: "speed",
      Histogram: "bar_chart",
      Counter: "pin",
    };

    const selectedMetricTypeIcon = computed(() => {
      return dashboardPanelData.meta.stream.streamResults.find(
        (it: any) =>
          it.name ==
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream,
      )?.metrics_meta?.metric_type;
    });

    // get stream list
    const streamDataLoading = useLoading(async (stream_type: any) => {
      await getStreamList(stream_type);
    });

    // get the stream list based on the selected stream type
    const loadStreamsListBasedOnType = async () => {
      streamDataLoading.execute(
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream_type,
      );
    };

    // watch the stream type and load the stream list
    watch(
      () =>
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream_type,
      async () => {
        loadStreamsListBasedOnType();
      },
    );

    onMounted(() => {
      loadStreamsListBasedOnType();
    });

    const getStreamFields = useLoading(
      async (fieldName: string, streamType: string) => {
        return await getStream(fieldName, streamType, true);
      },
    );

    // update the selected stream fields list
    watch(
      () => [
        dashboardPanelData.meta.stream.streamResults,
        dashboardPanelData.meta.stream.streamResultsType,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream_type,
      ],
      async () => {
        // get the selected stream fields based on the selected stream type
        const fields: any = dashboardPanelData.meta.stream.streamResults.find(
          (it: any) =>
            it.name ==
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.stream &&
            it.stream_type ==
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.stream_type,
        );

        // if fields found and stream result is of same type
        if (
          fields &&
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream_type ===
            dashboardPanelData.meta.stream.streamResultsType
        ) {
          try {
            // await extractFields();

            // if promql mode
            // NOTE: For the metrics page, we added one watch that resets the query on stream change.
            // Because of that, the default query overrides the original/saved query on the edit panel.
            // To prevent this, we added the dashboardPanelDataPageKey condition.
            if (promqlMode.value && dashboardPanelDataPageKey === "metrics") {
              // set the query
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].query =
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.stream?.toString() + "{}";
            }
          } catch (error: any) {
            showErrorNotification(
              error?.message ?? "Failed to get stream fields",
            );
          }
        }
      },
    );

    watch(
      () => [
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.stream_type,
        dashboardPanelData.meta.stream.streamResults,
        dashboardPanelData.meta.stream.streamResultsType,
      ],
      () => {
        // if (!props.editMode) {
        //   dashboardPanelData.data.queries[
        //     dashboardPanelData.layout.currentQueryIndex
        //   ].fields.stream = "";
        // }

        // data.indexOptions = data.schemaList.filter(
        //   (data: any) =>
        //     data.stream_type ==
        //     dashboardPanelData.data.queries[
        //       dashboardPanelData.layout.currentQueryIndex
        //     ].fields.stream_type
        // );

        // set the first stream as the selected stream when the api loads the data
        // Here, we need to check if the stream results are same as the selected stream type
        if (
          // !props.editMode &&
          // !dashboardPanelData.data.queries[
          //   dashboardPanelData.layout.currentQueryIndex
          // ].fields.stream &&
          dashboardPanelData.meta.stream.streamResults.length > 0 &&
          dashboardPanelData.meta.stream.streamResultsType ===
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.stream_type
        ) {
          const currentIndex = dashboardPanelData.layout.currentQueryIndex;
          // Check if selected stream for current query exists in index options
          // If not, set the first index option as the selected stream
          if (
            dashboardPanelData.meta.stream.streamResults.find(
              (it: any) =>
                it.name ==
                dashboardPanelData.data.queries[
                  dashboardPanelData.layout.currentQueryIndex
                ].fields.stream,
            )
          ) {
            dashboardPanelData.data.queries[currentIndex].fields.stream =
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields.stream;
          } else {
            dashboardPanelData.data.queries[currentIndex].fields.stream =
              dashboardPanelData.meta.stream.streamResults[0]?.name;
          }
        }
      },
    );

    // update the current list fields if any of the lists changes
    watch(
      () => [
        dashboardPanelData.meta.stream.customQueryFields,
        dashboardPanelData.meta.stream.vrlFunctionFieldList,
      ],
      () => {
        // set the custom query fields length
        customQueryFieldsLength.value =
          dashboardPanelData.meta.stream.customQueryFields.length +
          dashboardPanelData.meta.stream.vrlFunctionFieldList.length;
      },
    );

    const flattenGroupedFields = computed(() => {
      const flattenedFields: any[] = [];
      dashboardPanelData.meta.streamFields.groupedFields.forEach(
        (group: any) => {
          // Add a group header row
          flattenedFields.push({
            isGroup: true,
            groupName: group.name,
          });

          if (
            group.settings.hasOwnProperty("defined_schema_fields") &&
            group.settings.defined_schema_fields.length > 0
          ) {
            // add the user defined fields
            // _timestamp field + user defined fields + all_fields_name

            // add _timestamp field
            flattenedFields.push({
              name: store.state.zoConfig?.timestamp_column,
              type: "Int64",
              stream: group.name,
              streamAlias: group.stream_alias,
              isGroup: false,
            });

            // add user defined fields
            for (const field of group.schema) {
              if (
                store.state.zoConfig.user_defined_schemas_enabled &&
                group.settings.hasOwnProperty("defined_schema_fields") &&
                group.settings.defined_schema_fields.length > 0
              ) {
                if (group.settings.defined_schema_fields.includes(field.name)) {
                  // push as a user defined schema
                  flattenedFields.push({
                    ...field,
                    stream: group.name,
                    streamAlias: group.stream_alias,
                    isGroup: false,
                  });
                }
              }
            }

            // add all_fields_name
            flattenedFields.push({
              name: store.state.zoConfig?.all_fields_name,
              type: "Utf8",
              stream: group.name,
              streamAlias: group.stream_alias,
              isGroup: false,
            });
          } else {
            // use schema of the group
            // Add the fields in the group, including the group name
            group.schema.forEach((field: any) => {
              flattenedFields.push({
                ...field,
                stream: group.name,
                streamAlias: group.stream_alias,
                isGroup: false,
              });
            });
          }
        },
      );

      return flattenedFields;
    });

    watch(
      () => ({
        stream:
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream,
        streamType:
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields.stream_type,
        joins:
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].joins,
      }),
      () => {
        updateGroupedFields();
      },
      { deep: true },
    );

    watch(
      () => dashboardPanelData.meta.stream.filterField,
      () => {
        // set the custom query fields length
        customQueryFieldsLength.value =
          dashboardPanelData.meta.stream.customQueryFields.length;
      },
    );

    // get the stream list by making an API call
    const getStreamList = async (stream_type: any) => {
      await getStreams(stream_type, false).then((res: any) => {
        // below line required for pass by reference
        // if we don't set blank, then same object from cache is being set
        // and that doesn't call the watchers,
        // so it will not be updated when we switch to different chart types
        // which doesn't have field list and coming back to field list
        dashboardPanelData.meta.stream.streamResults = [];

        dashboardPanelData.meta.stream.streamResults = res.list;

        dashboardPanelData.meta.stream.streamResultsType = stream_type;
      });
    };

    const filterFieldFn = (rows: any, terms: any) => {
      if (!terms || terms.trim() === "") {
        return rows;
      }

      const searchTerm = terms.toLowerCase();

      const filteredRows = rows.filter((row: any) => {
        // Always include group headers
        if (row.isGroup) {
          return true;
        }

        // Filter fields based on name
        return row.name.toLowerCase().includes(searchTerm);
      });

      return filteredRows;
    };

    const mutationHandler: any = (mutationRecords: any) => {};

    const onDragEnter = (e: any) => {
      e.preventDefault();
    };

    const onDragStart = (e: any, item: any) => {
      dashboardPanelData.meta.dragAndDrop.dragging = true;
      dashboardPanelData.meta.dragAndDrop.dragElement = item;
      dashboardPanelData.meta.dragAndDrop.dragSource = "fieldList";
      dashboardPanelData.meta.dragAndDrop.dragSourceIndex = null;
    };

    const onDragLeave = (e: any) => {
      e.preventDefault();
      // e.target.classList.remove('drag-enter')
    };

    const onDragOver = (e: any) => {
      e.preventDefault();
    };

    const onDrop = (e: any) => {
      dashboardPanelData.meta.dragAndDrop.dragging = false;
      dashboardPanelData.meta.dragAndDrop.dragElement = null;
      dashboardPanelData.meta.dragAndDrop.dragSource = null;
      dashboardPanelData.meta.dragAndDrop.dragSourceIndex = null;
    };

    const filterStreamFn = (val: string, update: any) => {
      update(() => {
        filteredStreams.value =
          dashboardPanelData.meta.stream.streamResults.filter((stream: any) => {
            return stream.name.toLowerCase().indexOf(val.toLowerCase()) > -1;
          });
      });
    };

    // async function extractFields() {
    //   try {
    //     dashboardPanelData.meta.stream.selectedStreamFields = [];
    //     const schemaFields: any = [];
    //     let userDefineSchemaSettings: any = [];

    //     if (
    //       dashboardPanelData.meta.stream.streamResults.length > 0 &&
    //       dashboardPanelData.meta.stream.streamResultsType ===
    //         dashboardPanelData.data.queries[
    //           dashboardPanelData.layout.currentQueryIndex
    //         ].fields.stream_type
    //     ) {
    //       for (const stream of dashboardPanelData.meta.stream.streamResults) {
    //         if (
    //           dashboardPanelData.data.queries[
    //             dashboardPanelData.layout.currentQueryIndex
    //           ].fields.stream == stream.name
    //         ) {
    //           // check for schema exist in the object or not
    //           // if not pull the schema from server.
    //           if (!stream.hasOwnProperty("schema")) {
    //             const streamData: any = await loadStreamFields(stream.name);
    //             const streamSchema: any = streamData.schema;
    //             if (streamSchema == undefined) {
    //               return;
    //             }
    //             stream.settings = streamData.settings;
    //             stream.schema = streamSchema;
    //           }

    //           // create a schema field mapping based on field name to avoid iteration over object.
    //           // in case of user defined schema consideration, loop will be break once all defined fields are mapped.
    //           for (const field of stream.schema) {
    //             if (
    //               store.state.zoConfig.user_defined_schemas_enabled &&
    //               stream.settings.hasOwnProperty("defined_schema_fields") &&
    //               stream.settings.defined_schema_fields.length > 0
    //             ) {
    //               if (
    //                 stream.settings.defined_schema_fields.includes(field.name)
    //               ) {
    //                 // push as a user defined schema
    //                 userDefineSchemaSettings.push(field);
    //               }
    //               schemaFields.push(field);
    //             } else {
    //               schemaFields.push(field);
    //             }
    //           }

    //           dashboardPanelData.meta.stream.selectedStreamFields =
    //             schemaFields ?? [];

    //           if (
    //             stream.settings.hasOwnProperty("defined_schema_fields") &&
    //             stream.settings.defined_schema_fields.length > 0
    //           ) {
    //             dashboardPanelData.meta.stream.hasUserDefinedSchemas = true;
    //             // set user defined schema
    //             // 1) Timestamp field
    //             // 2) selected user defined schema fields
    //             // 3) all_fields_name fields
    //             dashboardPanelData.meta.stream.userDefinedSchema = [
    //               {
    //                 name: store.state.zoConfig?.timestamp_column,
    //                 type: "Int64",
    //               },
    //               ...(userDefineSchemaSettings ?? []),
    //               {
    //                 name: store.state.zoConfig?.all_fields_name,
    //                 type: "Utf8",
    //               },
    //             ];
    //           } else {
    //             dashboardPanelData.meta.stream.hasUserDefinedSchemas = false;
    //             dashboardPanelData.meta.stream.userDefinedSchema = [];
    //           }
    //         }
    //       }
    //     }
    //   } catch (e: any) {
    //     console.log("Error while extracting fields");
    //   }
    // }

    // const toggleSchema = async () => {
    //   await extractFields();
    // };

    return {
      dashboardPanelDataPageKey,
      t,
      store,
      router,
      mutationHandler,
      onDragEnter,
      onDragLeave,
      onDragOver,
      onDrop,
      onDragStart,
      filterFieldFn,
      addXAxisItem,
      addYAxisItem,
      addZAxisItem,
      addBreakDownAxisItem,
      addLatitude,
      addLongitude,
      addWeight,
      addMapName,
      addMapValue,
      addSource,
      addTarget,
      addValue,
      addFilteredItem,
      data,
      getStreamList,
      getStreamFields,
      dashboardPanelData,
      filterStreamFn,
      filteredStreams,
      isAddXAxisNotAllowed,
      isAddBreakdownNotAllowed,
      isAddYAxisNotAllowed,
      isAddZAxisNotAllowed,
      promqlMode,
      streamDataLoading,
      metricsIconMapping,
      selectedMetricTypeIcon,
      onDragEnd,
      customQueryFieldsLength,
      // toggleSchema,
      userDefinedSchemaBtnGroupOption,
      pagination,
      // groupedFields,
      flattenGroupedFields,
      pagesNumber: computed(() => {
        return Math.ceil(
          dashboardPanelData.meta.stream.selectedStreamFields.length /
            pagination.value.rowsPerPage,
        );
      }),
    };
  },
});
</script>

<style lang="scss" scoped>
.metric-explore-metric-icon {
  min-width: 28px !important;
  padding-right: 8px !important;
}
.q-menu {
  box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
  transform: translateY(0.5rem);
  border-radius: 0px;

  .q-virtual-scroll__content {
    padding: 0.5rem;
  }
}

.index-menu {
  width: 100%;
  height: 100%;

  .q-field {
    &__control {
      height: 35px;
      padding: 0px 5px;
      min-height: auto !important;

      &-container {
        padding-top: 0px !important;
      }
    }

    &__native :first-of-type {
      padding-top: 0.25rem;
    }
  }

  .q-select {
    // text-transform: capitalize;
  }

  .index-table {
    width: 100%;
    height: 100%;
    // border: 1px solid rgba(0, 0, 0, 0.02);
    .q-table {
      display: block;
    }

    tr {
      margin-bottom: 1px;
    }

    tbody,
    tr,
    td {
      width: 100%;
      display: block;
      height: 25px;
    }

    :deep(.q-table__top) {
      padding: 0px !important;
      border-bottom: unset;
    }

    :deep(.q-table__control),
    label.q-field {
      width: 100%;
    }

    .q-table thead tr,
    .q-table tbody td {
      height: auto;
    }
  }

  .field-table {
    width: 100%;
  }

  .field_list {
    padding: 0px;
    margin-bottom: 0.125rem;
    position: relative;
    overflow: visible;

    .field_overlay {
      justify-content: space-between;
      background-color: transparent;
      transition: all 0.3s ease;
      padding: 0px 10px;
      align-items: center;
      position: absolute;
      line-height: 2rem;
      overflow: hidden;
      inset: 0;
      display: flex;
      z-index: 1;
      width: 100%;
      border-radius: 0px;
      height: 25px;

      .field_icons {
        padding: 0 0 0 0.25rem;
        transition: all 0.3s ease;
        // background-color: white;
        position: absolute;
        z-index: 3;
        opacity: 0;
        right: 0;

        .q-icon {
          cursor: pointer;
        }
      }

      .field_label {
        // pointer-events: none;
        font-size: 0.825rem;
        position: relative;
        display: inline;
        z-index: 2;
        left: 0;
        // text-transform: capitalize;

        .drag_indicator {
          cursor: -webkit-grab;
          cursor: grab;
        }

        .drag_disabled {
          cursor: not-allowed;
        }
      }
    }

    &.selected {
      .field_overlay {
        background-color: rgba(89, 96, 178, 0.3);

        .field_icons {
          opacity: 0;
        }
      }

      &:hover {
        .field_overlay {
          box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);
          background-color: white;

          .field_icons {
            background-color: white;
          }
        }
      }
    }

    &:hover {
      .field_overlay {
        .field_icons {
          // background-color: white;
          opacity: 1;
        }
      }
    }
  }
}

.theme-dark {
  .field_overlay {
    &:hover {
      box-shadow: 0px 4px 15px rgb(255, 255, 255, 0.1);

      .field_icons {
        background-color: #202224;
        opacity: 1;
      }
    }
  }
}

.theme-light {
  .field_overlay {
    &:hover {
      box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);

      .field_icons {
        background-color: white;
        opacity: 1;
      }
    }
  }
}

.q-item {
  color: $dark-page;
  min-height: 1.3rem;
  padding: 5px 10px;

  &__label {
    font-size: 0.75rem;
  }

  &.q-manual-focusable--focused > .q-focus-helper {
    background: none !important;
    opacity: 0.3 !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &--active {
    // background-color: $selected-list-bg !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &:hover,
  &--active {
    color: $primary;
  }
}

.q-field--dense .q-field__before,
.q-field--dense .q-field__prepend {
  padding: 0px 0px 0px 0px;
  height: auto;
  line-height: auto;
}

.q-field__native,
.q-field__input {
  padding: 0px 0px 0px 0px;
}

.q-field--dense .q-field__label {
  top: 5px;
}

// if metric icon is present then move the label to the left
:deep(.metric_icon_present .q-field__label) {
  margin-left: -24px;
}

.q-field--dense .q-field__control,
.q-field--dense .q-field__marginal {
  height: 34px;
}

.schema-field-toggle .q-btn {
  padding: 5px !important;
}

.schema-field-toggle {
  border: 1px solid light-grey;
  border-radius: 5px;
  line-height: 10px;
}

.q-table__bottom {
  padding: 0px !important;
}

.pagination-field-count {
  line-height: 32px;
  font-weight: 700;
  font-size: 13px;
}

.q-table__bottom .q-table__control {
  min-height: 80px !important;
}

.fieldList-pagination {
  // min-height: 80px;
  display: flex;
  padding-bottom: 2rem;
}
</style>
