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
  <q-page class="q-pa-none" style="min-height: inherit">
    <q-table
      data-test="log-stream-table"
      class="org-streams-table"
      ref="qTable"
      v-model:selected="selected"
      :rows="logStream"
      :columns="columns"
      :row-key="getRowKey"
      :selected-rows-label="getSelectedString"
      selection="multiple"
      :pagination="pagination"
      :filter="filterQuery"
      :filter-method="filterData"
      style="width: 100%"
    >
      <template #no-data>
        <div v-if="!loadingState" class="text-center full-width full-height">
          <NoData />
        </div>
        <div
          v-else
          class="text-center full-width full-height q-mt-lg tw-flex tw-justify-center"
        >
          <q-spinner-hourglass color="primary" size="lg" />
        </div>
      </template>
      <template #header-selection="scope">
        <q-checkbox v-model="scope.selected" size="sm" color="secondary" />
      </template>
      <template #body-selection="scope">
        <q-checkbox v-model="scope.selected" size="sm" color="secondary" />
      </template>
      <template #body-cell-actions="props">
        <q-td :props="props">
          <q-btn
            icon="search"
            :title="t('logStream.explore')"
            class="q-ml-xs"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="exploreStream(props)"
          />
          <q-btn
            icon="list_alt"
            :title="t('logStream.schemaHeader')"
            class="q-ml-xs"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="listSchema(props)"
          />
          <q-btn
            :icon="outlinedDelete"
            :title="t('logStream.delete')"
            class="q-ml-xs"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="confirmDeleteAction(props)"
          />
        </q-td>
      </template>

      <template #top="scope">
        <div class="flex justify-between items-center full-width">
          <div class="q-table__title" data-test="log-stream-title-text">
            {{ t("logStream.header") }}
          </div>
          <div class="flex items-start">
            <div class="flex justify-between items-end q-px-md">
              <div
                style="
                  border: 1px solid #cacaca;
                  padding: 4px;
                  border-radius: 2px;
                "
              >
                <template
                  v-for="visual in streamFilterValues"
                  :key="visual.value"
                >
                  <q-btn
                    :color="
                      visual.value === selectedStreamType ? 'primary' : ''
                    "
                    :flat="visual.value === selectedStreamType ? false : true"
                    dense
                    emit-value
                    no-caps
                    class="visual-selection-btn"
                    style="height: 30px; margin: 0 2px; padding: 4px 12px"
                    @click="onChangeStreamFilter(visual.value)"
                  >
                    {{ visual.label }}</q-btn
                  >
                </template>
              </div>
            </div>
            <div data-test="streams-search-stream-input">
              <q-input
                v-model="filterQuery"
                borderless
                filled
                dense
                class="q-ml-auto q-mb-xs no-border"
                :placeholder="t('logStream.search')"
              >
                <template #prepend>
                  <q-icon name="search" />
                </template>
              </q-input>
            </div>
            <q-btn
              data-test="log-stream-refresh-stats-btn"
              class="q-ml-md q-mb-xs text-bold no-border"
              padding="sm lg"
              color="secondary"
              no-caps
              :label="t(`logStream.refreshStats`)"
              @click="getLogStream(true)"
            />
            <q-btn
              data-test="log-stream-add-stream-btn"
              class="q-ml-md q-mb-xs text-bold no-border"
              padding="sm lg"
              color="secondary"
              no-caps
              :label="t(`logStream.add`)"
              @click="addStream"
            />
          </div>
        </div>
        <QTablePagination
          data-test="log-stream-table-pagination"
          :scope="scope"
          :pageTitle="t('logStream.header')"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          position="top"
          @update:changeRecordPerPage="changePagination"
        />
      </template>

      <template #bottom="scope">
        <div class="bottom-bar">
          <q-btn
            v-if="selected.length > 0"
            class="delete-btn"
            color="red"
            icon="delete"
            :label="isDeleting ? 'Deleting...' : 'Delete'"
            :disable="isDeleting"
            @click="confirmBatchDeleteAction"
          />
          <QTablePagination
            data-test="log-stream-table-pagination"
            :scope="scope"
            :resultTotal="resultTotal"
            :perPageOptions="perPageOptions"
            position="bottom"
            @update:changeRecordPerPage="changePagination"
          />
        </div>
      </template>
    </q-table>
    <q-dialog
      v-model="showIndexSchemaDialog"
      position="right"
      full-height
      maximized
    >
      <SchemaIndex v-model="schemaData" />
    </q-dialog>

    <q-dialog
      v-model="addStreamDialog.show"
      position="right"
      full-height
      maximized
    >
      <AddStream
        :is-in-pipeline="false"
        @close="addStreamDialog.show = false"
        @streamAdded="getLogStream"
      />
    </q-dialog>

    <q-dialog v-model="confirmDelete">
      <q-card style="width: 240px">
        <q-card-section class="confirmBody">
          <div class="head">{{ t("logStream.confirmDeleteHead") }}</div>
          <div class="para">{{ t("logStream.confirmDeleteMsg") }}</div>
        </q-card-section>

        <q-card-actions class="confirmActions">
          <q-btn v-close-popup="true" unelevated no-caps class="q-mr-sm">
            {{ t("logStream.cancel") }}
          </q-btn>
          <q-btn
            v-close-popup="true"
            unelevated
            no-caps
            class="no-border"
            color="primary"
            @click="deleteStream"
          >
            {{ t("logStream.ok") }}
          </q-btn>
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog v-model="confirmBatchDelete">
      <q-card style="width: 240px">
        <q-card-section class="confirmBody">
          <div class="head">{{ t("logStream.confirmBatchDeleteHead") }}</div>
          <div class="para">{{ t("logStream.confirmBatchDeleteMsg") }}</div>
        </q-card-section>

        <q-card-actions class="confirmActions">
          <q-btn v-close-popup="true" unelevated no-caps class="q-mr-sm">
            {{ t("logStream.cancel") }}
          </q-btn>
          <q-btn
            v-close-popup="true"
            unelevated
            no-caps
            class="no-border"
            color="primary"
            @click="deleteBatchStream"
          >
            {{ t("logStream.ok") }}
          </q-btn>
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onActivated,
  onBeforeMount,
  type Ref,
} from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, type QTableProps } from "quasar";
import { useI18n } from "vue-i18n";

import QTablePagination from "../components/shared/grid/Pagination.vue";
import streamService from "../services/stream";
import SchemaIndex from "../components/logstream/schema.vue";
import NoData from "../components/shared/grid/NoData.vue";
import segment from "../services/segment_analytics";
import {
  getImageURL,
  verifyOrganizationStatus,
  formatSizeFromMB,
} from "../utils/zincutils";
import config from "@/aws-exports";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import { cloneDeep } from "lodash-es";
import useStreams from "@/composables/useStreams";
import AddStream from "@/components/logstream/AddStream.vue";
import { watch } from "vue";

export default defineComponent({
  name: "PageLogStream",
  components: { QTablePagination, SchemaIndex, NoData, AddStream },
  emits: ["update:changeRecordPerPage", "update:maxRecordToReturn"],
  setup(props, { emit }) {
    const store = useStore();
    const { t } = useI18n();
    const $q = useQuasar();
    const router = useRouter();
    const logStream: Ref<any[]> = ref([]);
    const showIndexSchemaDialog = ref(false);
    const isDeleting = ref(false);
    const confirmDelete = ref<boolean>(false);
    const confirmBatchDelete = ref<boolean>(false);
    const schemaData = ref({ name: "", schema: [Object], stream_type: "" });
    const resultTotal = ref<number>(0);
    const selected = ref<any>([]);
    const orgData: any = ref(store.state.selectedOrganization);
    const qTable: any = ref(null);
    const previousOrgIdentifier = ref("");
    const filterQuery = ref("");
    const duplicateStreamList: Ref<any[]> = ref([]);
    const selectedStreamType = ref("all");
    const loadingState = ref(true);

    const streamFilterValues = [
      { label: t("logStream.labelAll"), value: "all" },
      { label: t("logStream.labelLogs"), value: "logs" },
      { label: t("logStream.labelMetrics"), value: "metrics" },
      { label: t("logStream.labelTraces"), value: "traces" },
      { label: t("logStream.labelMetadata"), value: "metadata" },
      { label: t("logStream.labelIndex"), value: "index" },
    ];
    const { getStreams, resetStreams, removeStream, getStream } = useStreams();
    const columns = ref<QTableProps["columns"]>([
      {
        name: "#",
        label: "#",
        field: "#",
        align: "left",
      },
      {
        name: "name",
        field: "name",
        label: t("logStream.name"),
        align: "left",
        sortable: true,
      },
      {
        name: "stream_type",
        field: "stream_type",
        label: t("logStream.type"),
        align: "left",
        sortable: true,
      },
      {
        name: "doc_num",
        field: (row: any) => row.doc_num.toLocaleString(),
        label: t("logStream.docNum"),
        align: "left",
        sortable: true,
        sort: (a, b, rowA, rowB) => {
          return parseInt(rowA.doc_num) - parseInt(rowB.doc_num);
        },
      },
      {
        name: "storage_size",
        label: t("logStream.storageSize"),
        field: (row: any) => formatSizeFromMB(row.storage_size),
        align: "left",
        sortable: true,
        sort: (a, b, rowA, rowB) => {
          return parseInt(rowA.storage_size) - parseInt(rowB.storage_size);
        },
      },
      {
        name: "compressed_size",
        field: (row: any) => formatSizeFromMB(row.compressed_size),
        label: t("logStream.compressedSize"),
        align: "left",
        sortable: true,
        sort: (a, b, rowA, rowB) =>
          parseInt(rowA.compressed_size) - parseInt(rowB.compressed_size),
      },
      {
        name: "index_size",
        field: (row: any) => formatSizeFromMB(row.index_size),
        label: t("logStream.indexSize"),
        align: "left",
        sortable: true,
        sort: (a, b, rowA, rowB) =>
          parseInt(rowA.index_size) - parseInt(rowB.index_size),
      },
      {
        name: "actions",
        field: "actions",
        label: t("user.actions"),
        align: "center",
      },
    ]);

    if (config.isCloud == "true") {
      columns.value?.splice(5, 1);
    }

    const addStreamDialog = ref({
      show: false,
      data: {
        name: "",
        stream_type: "",
        index_type: "",
        retention_period: 14,
      },
    });

    let deleteStreamName = "";
    let deleteStreamType = "";

    onBeforeMount(() => {
      if (columns.value && !store.state.zoConfig.show_stream_stats_doc_num) {
        columns.value = columns.value.filter(
          (column) => column.name !== "doc_num",
        );
      }

      if (router.currentRoute.value.name === "streamExplorer") {
        router.push({
          name: "streamExplorer",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
        return;
      }
    });

    // As filter data don't gets called when search input is cleared.
    // So calling onChangeStreamFilter to filter again
    watch(
      () => filterQuery.value,
      (value) => {
        if (!value) {
          onChangeStreamFilter(selectedStreamType.value);
        }
      },
    );

    const getLogStream = (refresh: boolean = false) => {
      if (store.state.selectedOrganization != null) {
        loadingState.value = true;
        previousOrgIdentifier.value =
          store.state.selectedOrganization.identifier;
        const dismiss = $q.notify({
          spinner: true,
          message: "Please wait while loading streams...",
        });
        logStream.value = [];

        if (refresh) resetStreams();

        let counter = 1;
        getStreams("", false, false)
          .then((res: any) => {
            logStream.value = [];
            let doc_num = "";
            let storage_size = "";
            let compressed_size = "";
            let index_size = "";
            resultTotal.value += res.list.length;
            logStream.value.push(
              ...res.list.map((data: any) => {
                doc_num = "--";
                storage_size = "--";
                if (data.stats) {
                  doc_num = data.stats.doc_num;
                  storage_size = data.stats.storage_size + " MB";
                  compressed_size = data.stats.compressed_size + " MB";
                  index_size = data.stats.index_size + " MB";
                }
                return {
                  "#": counter <= 9 ? `0${counter++}` : counter++,
                  name: data.name,
                  doc_num: doc_num,
                  storage_size: storage_size,
                  compressed_size: compressed_size,
                  index_size: index_size,
                  storage_type: data.storage_type,
                  actions: "action buttons",
                  schema: data.schema ? data.schema : [],
                  stream_type: data.stream_type,
                };
              }),
            );
            duplicateStreamList.value = [...logStream.value];

            logStream.value.forEach((element: any) => {
              if (element.name == router.currentRoute.value.query.dialog) {
                listSchema({ row: element });
              }
            });

            onChangeStreamFilter(selectedStreamType.value);
            loadingState.value = false;
            dismiss();
          })
          .catch((err) => {
            if(err.response.status != 403){
              $q.notify({
              type: "negative",
              message: err.response?.data?.message || "Error while fetching streams.",
              timeout: 2000,
            });
            }
            loadingState.value = false;
            dismiss();  
          });
      }

      segment.track("Button Click", {
        button: "Refresh Streams",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        page: "Streams",
      });
    };

    getLogStream();

    const listSchema = (props: any) => {
      schemaData.value.name = props.row.name;
      schemaData.value.schema = props.row.schema;
      schemaData.value.stream_type = props.row.stream_type;
      showIndexSchemaDialog.value = true;

      segment.track("Button Click", {
        button: "Actions",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        stream_name: props.row.name,
        page: "Streams",
      });
    };

    const perPageOptions: any = [
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "250", value: 250 },
      { label: "500", value: 500 },
    ];
    const maxRecordToReturn = ref<number>(100);
    const selectedPerPage = ref<number>(20);
    const pagination: any = ref({
      rowsPerPage: 20,
    });
    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };
    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
    };

    const confirmDeleteAction = (props: any) => {
      confirmDelete.value = true;
      deleteStreamName = props.row.name;
      deleteStreamType = props.row.stream_type;
    };
    const confirmBatchDeleteAction = () => {
      confirmBatchDelete.value = true;
      console.log(selected, "selected items");
    };

    const deleteStream = () => {
      streamService
        .delete(
          store.state.selectedOrganization.identifier,
          deleteStreamName,
          deleteStreamType,
        )
        .then((res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              color: "positive",
              message: "Stream deleted successfully.",
            });
            removeStream(deleteStreamName, deleteStreamType);
            selected.value = [];
            getLogStream();
          }
        })
        .catch((err: any) => {
         if(err.response.status != 403){
          $q.notify({
            color: "negative",
            message: "Error while deleting stream.",
          });
         }
        });
    };
    const deleteBatchStream = () => {
      isDeleting.value = true;
      const selectedItems = selected.value;
      const promises: Promise<any>[] = [];

      selectedItems.forEach((stream: any) => {
        promises.push(
          streamService.delete(
            store.state.selectedOrganization.identifier,
            stream.name,
            stream.stream_type,
          ),
        );
      });

      Promise.all(promises)
        .then((responses) => {
          const successfulDeletions = responses.filter(
            (res) => res.data.code === 200,
          );
          const failedDeletions = responses.filter(
            (res) => res.data.code !== 200,
          );

          if (successfulDeletions.length > 0) {
            $q.notify({
              color: "positive",
              message: `Deleted ${successfulDeletions.length} streams successfully.`,
            });
          }

          if (failedDeletions.length > 0) {
            $q.notify({
              color: "negative",
              message: `Failed to delete ${failedDeletions.length} streams.`,
            });
          }

          // Remove deleted streams from the list
          console.log(selectedItems, "after deleting streams");
          selectedItems.forEach((stream: any) => {
            removeStream(stream.name, stream.stream_type);
            selected.value = selected.value.filter(
              (item: any) =>
                item.name !== stream.name &&
                item.stream_type !== stream.stream_type,
            );
          });

          getLogStream();
        })
        .catch((error) => {
          if(error.response.status != 403){
            $q.notify({
            color: "negative",
            message: error.response?.data?.message || "Error while deleting streams.",
          });
          }
        })
        .finally(() => {
          isDeleting.value = false;
        });
    };

    onActivated(() => {
      if (logStream.value.length > 0) {
        logStream.value.forEach((element: any) => {
          if (element.name == router.currentRoute.value.query.dialog) {
            listSchema({ row: element });
          }
        });
      }

      if (
        previousOrgIdentifier.value !=
        store.state.selectedOrganization.identifier
      ) {
        getLogStream();
      }
    });
    const getSelectedString = () => {
      return selected.value.length === 0
        ? ""
        : `${selected.value.length} record${
            selected.value.length > 1 ? "s" : ""
          } selected`;
    };

    /**
     * Get time range for stream explorer, for enrichment tables it will get the time range from the stream data min and max time
     * @param stream: Stream object
     */
    const getTimeRange = async (stream: any) => {
      const dateTime: { period?: string; from?: number; to?: number } = {};

      if (stream.stream_type === "enrichment_tables") {
        const dismiss = $q.notify({
          spinner: true,
          message: "Redirecting to explorer...",
          color: "secondary",
        });

        await getStream(stream.name, stream.stream_type, true)
          .then((streamResponse) => {
            dateTime["from"] = streamResponse.stats.doc_time_min - 60000000;
            dateTime["to"] = streamResponse.stats.doc_time_max + 60000000;
          })
          .catch((err) => {
            console.error(err);
            dateTime["period"] = "15m";
          })
          .finally(() => {
            dismiss();
          });
      } else {
        dateTime["period"] = "15m";
      }

      return dateTime;
    };

    const exploreStream = async (props: any) => {
      const dateTime = await getTimeRange(props.row);
      router.push({
        name: "logs",
        query: {
          stream_type: props.row.stream_type,
          stream: props.row.name,
          refresh: "0",
          query: "",
          type: "stream_explorer",
          org_identifier: store.state.selectedOrganization.identifier,
          ...dateTime,
        },
      });
    };

    const filterData = (rows: any, terms: any) => {
      var filtered = [];
      terms = terms.toLowerCase();

      for (var i = 0; i < duplicateStreamList.value.length; i++) {
        if (
          (selectedStreamType.value ===
            duplicateStreamList.value[i]["stream_type"] ||
            selectedStreamType.value === "all") &&
          (duplicateStreamList.value[i]["name"].toLowerCase().includes(terms) ||
            duplicateStreamList.value[i]["stream_type"]
              .toLowerCase()
              .includes(terms))
        ) {
          filtered.push(duplicateStreamList.value[i]);
        }
      }
      return filtered;
    };

    const onChangeStreamFilter = (value: string) => {
      selectedStreamType.value = value;
      logStream.value = filterData(
        duplicateStreamList.value,
        filterQuery.value.toLowerCase(),
      );
      resultTotal.value = logStream.value.length;
    };

    const getSelectedItems = () => {
      console.log(selected.value[0], "selected");
    };
    const addStream = () => {
      addStreamDialog.value.show = true;
      // router.push({
      //   name: "addStream",
      //   query: {
      //     org_identifier: store.state.selectedOrganization.identifier,
      //   },
      // });
    };

    const getRowKey = (row: any) => {
      return `${row.name}-${row.stream_type}`; // Unique key by combining `name` and `stream_type`
    };

    return {
      t,
      qTable,
      router,
      store,
      logStream: logStream,
      columns,
      selected,
      orgData,
      getLogStream: getLogStream,
      pagination,
      resultTotal,
      listSchema,
      deleteStream,
      deleteBatchStream,
      confirmDeleteAction,
      confirmBatchDeleteAction,
      confirmDelete,
      confirmBatchDelete,
      schemaData,
      perPageOptions,
      selectedPerPage,
      changePagination,
      maxRecordToReturn,
      showIndexSchemaDialog,
      changeMaxRecordToReturn,
      outlinedDelete,
      filterQuery,
      filterData,
      getImageURL,
      verifyOrganizationStatus,
      exploreStream,
      selectedStreamType,
      streamFilterValues,
      onChangeStreamFilter,
      addStreamDialog,
      addStream,
      loadingState,
      getSelectedString,
      getSelectedItems,
      isDeleting,
      getRowKey,
    };
  },
});
</script>

<style lang="scss" scoped>
.org-streams-table {
  :deep(.q-table th),
  :deep(.q-table td) {
    padding: 0px 16px;
    height: 32px;
  }
}
</style>

<style lang="scss">
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}
.bottom-bar {
  display: flex;
  width: 100%;
  justify-content: space-between;
  align-items: center;
}
.delete-btn {
  width: 10vw;
}

.confirmBody {
  padding: 11px 1.375rem 0;
  font-size: 0.875rem;
  text-align: center;
  font-weight: 700;

  .head {
    line-height: 2.125rem;
    margin-bottom: 0.5rem;
  }

  .para {
    color: $light-text;
  }
}

.confirmActions {
  justify-content: center;
  padding: 1.25rem 1.375rem 1.625rem;
  display: flex;

  .q-btn {
    font-size: 0.75rem;
    font-weight: 700;
  }
}
</style>
