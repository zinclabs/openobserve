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
      ref="qTable"
      v-model:selected="selected"
      :rows="logStream"
      :columns="columns"
      row-key="name"
      :pagination="pagination"
      :filter="filterQuery"
      :filter-method="filterData"
      style="width: 100%"
    >
      <template #no-data>
        <NoData />
      </template>
      <template v-slot:header="props">
        <q-tr :props="props">
          <q-th auto-width />
          <q-th v-for="col in props.cols" :key="col.name" :props="props">
            {{ col.label }}
          </q-th>
        </q-tr>
      </template>
      <template v-slot:body="props">
        <q-tr
          :data-test="`stream-association-table-${props.row.name}-row`"
          :props="props"
          style="cursor: pointer"
          @click="toggleStreamRow(props)"
        >
          <q-td auto-width>
            <q-btn
              dense
              flat
              size="xs"
              :icon="
                expandedRow.name != props.row.name
                  ? 'expand_more'
                  : 'expand_less'
              "
            />
          </q-td>
          <q-td v-for="col in props.cols" :key="col.name" :props="props">
            {{ col.value }}
          </q-td>
        </q-tr>
        <q-tr
          v-show="
            expandedRow.name == props.row.name &&
            expandedRow.stream_type === props.row.stream_type
          "
          :props="props"
          no-hover
          style="
            height: min-content;
            /* background-color: white; */
            border: 1px solid black;
          "
        >
          <q-td colspan="100%">
            <div
              v-show="loadingFunctions"
              class="q-pl-md q-py-xs"
              style="height: 60px"
            >
              <q-inner-loading
                size="sm"
                :showing="loadingFunctions"
                label="Fetching functions..."
                label-style="font-size: 1.1em"
              />
            </div>
            <div v-show="!loadingFunctions">
              <q-table
                :data-test="`associated-functions-${props.row.name}-table`"
                class="border"
                hide-bottom
                bordered
                :rows="functionsList"
                :columns="functionsColumns"
                :title="t('function.associatedFunctionHeader')"
              >
                <template v-slot:body="props">
                  <q-tr
                    :data-test="`associated-function-table-${props.row.name}-row`"
                    :props="props"
                  >
                    <q-td key="#" :props="props">
                      {{ props.pageIndex + 1 }}
                    </q-td>
                    <q-td key="name" :props="props">
                      {{ props.row.name }}
                    </q-td>
                    <q-td key="order" :props="props">
                      {{ props.row.order }}
                    </q-td>
                    <q-td
                      v-if="expandedRow.stream_type === 'logs'"
                      key="applyBeforeFlattening"
                      :props="props"
                    >
                      <q-toggle
                        data-test="stream-association-applyBeforeFlattening-toggle"
                        class="q-mt-sm"
                        v-model="props.row.applyBeforeFlattening"
                        @update:model-value="
                          updateAssociatedFunctions(props.row)
                        "
                      />
                    </q-td>
                    <q-td key="actions" :props="props">
                      <q-btn
                        data-test="stream-association-delete-function-btn"
                        :icon="outlinedDelete"
                        :title="t('function.deleteAssociatedFunction')"
                        class="q-ml-xs"
                        padding="sm"
                        unelevated
                        size="sm"
                        round
                        flat
                        @click.stop="deleteFunctionFromStream(props.row.name)"
                      ></q-btn>
                    </q-td>
                  </q-tr>
                </template>
                <template v-slot:bottom-row>
                  <q-tr v-if="addFunctionInProgress">
                    <q-td></q-td>
                    <q-td data-test="stream-association-functions-select-input">
                      <q-select
                        v-model="selectedFunction"
                        option-value="name"
                        option-label="name"
                        :label="t('function.selectFunction')"
                        :options="filterFunctions"
                        :loading="addFunctionInProgressLoading"
                        :disable="addFunctionInProgressLoading"
                        filled
                        borderless
                        dense
                        use-input
                        hide-selected
                        fill-input
                        @filter="filterFn"
                      ></q-select>
                    </q-td>
                    <q-td></q-td>
                    <q-td> </q-td>
                    <q-td></q-td>
                  </q-tr>
                </template>
                <template v-slot:top>
                  <div
                    style="
                      display: flex;
                      flex-direction: row;
                      width: 100%;
                      justify-content: space-between;
                    "
                  >
                    <div
                      class="q-table__title row items-center"
                      data-test="log-stream-title-text"
                    >
                      {{ t("function.associatedFunctionHeader") }}
                    </div>
                    <q-btn
                      data-test="stream-association-associate-function-btn"
                      color="secondary"
                      class="q-ml-md q-mb-xs text-bold no-border"
                      @click="addFunctionInProgress = true"
                      no-caps
                      >Associate Function</q-btn
                    >
                  </div>
                </template>
                <template v-slot:no-data>
                  <div
                    style="width: 100%; display: flex; flex-direction: column"
                  >
                    <div
                      v-if="!functionsList.length && !addFunctionInProgress"
                      style="width: 100%; text-align: center"
                    >
                      No functions found
                    </div>
                    <!-- <div>
                    <q-btn @click="addFunctionInProgress = true" no-caps>Associate Function</q-btn>
                  </div> -->
                  </div>
                </template>
              </q-table>
            </div>
          </q-td>
        </q-tr>
      </template>

      <template #top="scope">
        <div class="q-table__title" data-test="log-stream-title-text">
          {{ t("logStream.header") }}
        </div>
        <div class="q-ml-auto" data-test="stream-association-search-input">
          <q-input
            v-model="filterQuery"
            borderless
            filled
            dense
            class="q-mb-xs no-border"
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
          @click="getLogStream"
        />

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
        <QTablePagination
          data-test="log-stream-table-pagination"
          :scope="scope"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          position="bottom"
          @update:changeRecordPerPage="changePagination"
        />
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
  </q-page>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onActivated,
  onMounted,
  watch,
  computed,
} from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, type QTableProps } from "quasar";
import { useI18n } from "vue-i18n";
import jsTransformService from "../../services/jstransform";

import QTablePagination from "../shared/grid/Pagination.vue";
import streamService from "../../services/stream";
import SchemaIndex from "../logstream/schema.vue";
import NoData from "../shared/grid/NoData.vue";
import segment from "../../services/segment_analytics";
import { getImageURL, verifyOrganizationStatus } from "@/utils/zincutils";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import useStreams from "@/composables/useStreams";

export default defineComponent({
  name: "PageLogStream",
  components: { QTablePagination, SchemaIndex, NoData },
  emits: ["update:changeRecordPerPage", "update:maxRecordToReturn"],
  setup(props, { emit }) {
    const store = useStore();
    const { t } = useI18n();
    const $q = useQuasar();
    const router = useRouter();
    const logStream = ref([]);
    const showIndexSchemaDialog = ref(false);
    const schemaData = ref({ name: "", schema: [Object], stream_type: "" });
    const resultTotal = ref<number>(0);
    const selected = ref<any>([]);
    const orgData: any = ref(store.state.selectedOrganization);
    const qTable: any = ref(null);
    const previousOrgIdentifier = ref("");
    const functionsList = ref<any>([]);
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
        field: "doc_num",
        label: t("logStream.docNum"),
        align: "left",
        sortable: true,
      },
      {
        name: "storage_size",
        field: "storage_size",
        label: t("logStream.storageSize"),
        align: "left",
        sortable: true,
      },
      {
        name: "compressed_size",
        field: "compressed_size",
        label: t("logStream.compressedSize"),
        align: "left",
        sortable: true,
      },
      // {
      //   name: "actions",
      //   field: "actions",
      //   label: t("user.actions"),
      //   align: "center",
      // },
    ]);
    const addFunctionInProgress = ref(false);
    const addFunctionInProgressLoading = ref(false);
    const { getStreams } = useStreams();

    let deleteStreamName = "";
    let deleteStreamType = "";
    const loadingFunctions = ref(false);
    const expandedRow = ref({ name: "", stream_type: "" });
    const allFunctionsList = ref([]);
    const selectedFunction = ref<any | null>(null);
    const filterFunctions = ref([]);

    const functionsColumns = computed(() => {
      return [
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
          name: "order",
          field: "order",
          label: "Order",
          align: "left",
          sortable: true,
        },
        {
          name: "applyBeforeFlattening",
          field: "applyBeforeFlattening",
          label: "Apply Before Flattening",
          align: "left",
          sortable: true,
        },
        {
          name: "actions",
          field: "actions",
          label: t("user.actions"),
          align: "left",
        },
      ].filter((column) => {
        if (expandedRow.value.stream_type !== "logs") {
          return column.name !== "applyBeforeFlattening";
        }
        return true;
      }) as QTableProps["columns"];
    });

    const getLogStream = () => {
      if (store.state.selectedOrganization != null) {
        previousOrgIdentifier.value =
          store.state.selectedOrganization.identifier;
        const dismiss = $q.notify({
          spinner: true,
          message: "Please wait while loading streams...",
        });

        getStreams("", false)
          .then((res: any) => {
            let counter = 1;
            let doc_num = "";
            let storage_size = "";
            let compressed_size = "";
            resultTotal.value = res.list.length;
            logStream.value = res.list
              .filter(
                (stream: any) => stream.stream_type !== "enrichment_tables"
              )
              .map((data: any) => {
                doc_num = "--";
                storage_size = "--";
                if (data.stats) {
                  doc_num = data.stats.doc_num;
                  storage_size = data.stats.storage_size + " MB";
                  compressed_size = data.stats.compressed_size + " MB";
                }
                return {
                  "#": counter <= 9 ? `0${counter++}` : counter++,
                  name: data.name,
                  doc_num: doc_num,
                  storage_size: storage_size,
                  compressed_size: compressed_size,
                  storage_type: data.storage_type,
                  actions: "action buttons",
                  schema: data.schema ? data.schema : [],
                  stream_type: data.stream_type,
                };
              });

            if (logStream.value.length > 0) {
              getAllFunctions();
            }
            // logStream.value.forEach((element: any) => {
            //   if (element.name == router.currentRoute.value.query.dialog) {
            //     listSchema({ row: element });
            //   }
            // });

            dismiss();
          })
          .catch((err) => {
            dismiss();
            $q.notify({
              type: "negative",
              message: "Error while pulling stream.",
              timeout: 2000,
            });
          });
      }

      segment.track("Button Click", {
        button: "Refresh Streams",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        page: "Streams",
      });
    };

    const filterFn = (val: string, update: any) => {
      update(() => {
        const needle = val.toLowerCase();
        filterFunctions.value = allFunctionsList.value
          .filter(
            (item: any) =>
              !functionsList.value.some((obj: any) => obj.name === item.name)
          ) // filter existing applied functions
          .filter((v: any) => v.name.toLowerCase().indexOf(needle) > -1); // filter based on search term
      });
    };

    getLogStream();

    const getAllFunctions = () => {
      jsTransformService
        .list(
          1,
          100000,
          "name",
          false,
          "",
          store.state.selectedOrganization.identifier
        )
        .then((res: any) => {
          res.data.list.forEach((element: any) => {
            element.applyBeforeFlattening =
              element.applyBeforeFlattening || false;
          });
          allFunctionsList.value = res.data?.list || [];
          filterFunctions.value = res.data?.list || [];
        })
        .catch((err) => {
          $q.notify({
            type: "negative",
            message:
              JSON.stringify(err.response.data["error"]) ||
              "Function fetching failed",
            timeout: 2000,
          });
        });
    };

    watch([selectedFunction], async () => {
      if (selectedFunction.value) {
        // save it
        const order =
          functionsList.value.reduce((prev: any, current: any) => {
            return prev == null || prev.order < current.order ? current : prev;
          }, null)?.order || 0;

        const apiData = {
          order: order + 1,
        };
        addFunctionInProgressLoading.value = true;
        await jsTransformService
          .apply_stream_function(
            store.state.selectedOrganization.identifier,
            expandedRow.value.name,
            expandedRow.value.stream_type,
            selectedFunction.value.name,
            apiData
          )
          .then(() => {
            return getStreamFunctions(
              expandedRow.value.name,
              expandedRow.value.stream_type
            );
          })
          .finally(() => {
            addFunctionInProgressLoading.value = false;
            addFunctionInProgress.value = false;
            selectedFunction.value = null;
          });
      }
    });

    const toggleStreamRow = (props: any) => {
      if (expandedRow.value.name == props.row.name) {
        expandedRow.value = { name: "", stream_type: "" };
      } else {
        expandedRow.value.name = props.row.name;
        expandedRow.value.stream_type = props.row.stream_type;
      }
      if (expandedRow.value.name) {
        addFunctionInProgress.value = false;
        getStreamFunctions(props.row.name, props.row.stream_type);
      }
    };

    const getStreamFunctions = async (
      stream_name: any,
      stream_type: string
    ) => {
      loadingFunctions.value = stream_name ? true : false;
      await jsTransformService
        .stream_function(
          store.state.selectedOrganization.identifier,
          stream_name,
          stream_type
        )
        .then((res: any) => {
          functionsList.value = res.data?.list || [];
          functionsList.value.forEach((element: any) => {
            element.applyBeforeFlattening =
              element.applyBeforeFlattening || false;
          });
        })
        .catch((err) => {
          $q.notify({
            type: "negative",
            message:
              JSON.stringify(err.response.data["error"]) ||
              "Function creation failed",
            timeout: 2000,
          });
        })
        .finally(() => {
          loadingFunctions.value = false;
        });
    };

    const deleteFunctionFromStream = async (functionName: string) => {
      await jsTransformService
        .remove_stream_function(
          store.state.selectedOrganization.identifier,
          expandedRow.value.name,
          expandedRow.value.stream_type,
          functionName
        )
        .then(() => {
          return getStreamFunctions(
            expandedRow.value.name,
            expandedRow.value.stream_type
          );
        })
        .finally(() => {
          addFunctionInProgressLoading.value = false;
        });
    };

    // const listSchema = (props: any) => {
    //   schemaData.value.name = props.row.name;
    //   schemaData.value.schema = props.row.schema;
    //   schemaData.value.stream_type = props.row.stream_type;
    //   showIndexSchemaDialog.value = true;

    //   segment.track("Button Click", {
    //     button: "Actions",
    //     user_org: store.state.selectedOrganization.identifier,
    //     user_id: store.state.userInfo.email,
    //     stream_name: props.row.name,
    //     page: "Streams",
    //   });
    // };

    const perPageOptions: any = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
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

    const deleteStream = () => {
      streamService
        .delete(
          store.state.selectedOrganization.identifier,
          deleteStreamName,
          deleteStreamType
        )
        .then((res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              color: "positive",
              message: "Stream deleted successfully.",
            });
            getLogStream();
          }
        })
        .catch((err: any) => {
          $q.notify({
            color: "negative",
            message: "Error while deleting stream.",
          });
        });
    };

    onMounted(() => {
      // getAllFunctions();
    });

    onActivated(() => {
      // if (logStream.value.length > 0) {
      //   logStream.value.forEach((element: any) => {
      //     if (element.name == router.currentRoute.value.query.dialog) {
      //       listSchema({ row: element });
      //     }
      //   });
      // }
      // getAllFunctions();
      if (
        previousOrgIdentifier.value !=
        store.state.selectedOrganization.identifier
      ) {
        getLogStream();
      }
    });

    const updateAssociatedFunctions = (_function: any) => {
      jsTransformService
        .apply_stream_function(
          store.state.selectedOrganization.identifier,
          expandedRow.value.name,
          expandedRow.value.stream_type,
          _function.name,
          _function
        )
        .then((res) => {
          getStreamFunctions(
            expandedRow.value.name,
            expandedRow.value.stream_type
          );
        });
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
      // listSchema,
      deleteStream,
      schemaData,
      perPageOptions,
      selectedPerPage,
      changePagination,
      maxRecordToReturn,
      showIndexSchemaDialog,
      changeMaxRecordToReturn,
      getStreamFunctions,
      functionsList,
      expandedRow,
      filterQuery: ref(""),
      functionsColumns,
      deleteFunctionFromStream,
      addFunctionInProgress,
      allFunctionsList,
      selectedFunction,
      filterFn,
      filterFunctions,
      addFunctionInProgressLoading,
      toggleStreamRow,
      outlinedDelete,
      filterData(rows: any, terms: any) {
        var filtered = [];
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (
            rows[i]["name"].toLowerCase().includes(terms) ||
            rows[i]["stream_type"].toLowerCase().includes(terms)
          ) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
      getImageURL,
      loadingFunctions,
      verifyOrganizationStatus,
      updateAssociatedFunctions,
    };
  },
  computed: {
    selectedOrg() {
      return this.store.state.selectedOrganization.identifier;
    },
  },
  watch: {
    selectedOrg(newVal: any, oldVal: any) {
      this.verifyOrganizationStatus(
        this.store.state.organizations,
        this.router
      );
      this.orgData = newVal;
      if (
        (newVal != oldVal || this.logStream.values == undefined) &&
        this.router.currentRoute.value.name == "streams"
      ) {
        this.logStream = [];
        this.resultTotal = 0;
        this.getLogStream();
      }
    },
  },
});
</script>

<style lang="scss" scoped>
:deep(.q-table__top) {
  border-bottom: 1px solid $border-color;
  justify-content: flex-start !important;
}

:deep(.q-table__title) {
  font-size: 15px;
  font-weight: 600;
}

.confirmBody {
  padding: 11px 1.375rem 0;
  font-size: 0.875rem;
  text-align: center;
  font-weight: 700;

  .head {
    line-height: 2.125rem;
    margin-bottom: 0.5rem;
    color: $dark-page;
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
