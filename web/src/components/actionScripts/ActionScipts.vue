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
  <div
    data-test="action-scripts-list-page"
    class="q-pa-none flex tw-w-full"
    style="height: calc(100vh - 42px)"
    :class="store.state.theme === 'dark' ? 'dark-theme' : 'light-theme'"
  >
    <div
      v-if="!showAddActionScriptDialog"
      class="full-width action-scripts-table"
    >
      <q-table
        data-test="action-scripts-table"
        ref="qTable"
        :rows="actionsScriptRows"
        :columns="columns"
        row-key="id"
        :pagination="pagination"
        :filter="filterQuery"
        :filter-method="filterData"
        style="width: 100%"
      >
        <template #no-data>
          <NoData />
        </template>
        <template v-slot:body-cell-actions="props">
          <q-td :props="props">
            <div
              data-test="action-scripts-loading"
              v-if="alertStateLoadingMap[props.row.uuid]"
              style="display: inline-block; width: 33.14px; height: auto"
              class="flex justify-center items-center q-ml-xs"
              :title="`Turning ${props.row.enabled ? 'Off' : 'On'}`"
            >
              <q-circular-progress
                indeterminate
                rounded
                size="16px"
                :value="1"
                color="secondary"
              />
            </div>
            <q-btn
              :data-test="`alert-list-${props.row.name}-update-alert`"
              icon="edit"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alerts.edit')"
              @click="showAddUpdateFn(props)"
            ></q-btn>
            <q-btn
              :data-test="`alert-list-${props.row.name}-delete-alert`"
              :icon="outlinedDelete"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alerts.delete')"
              @click="showDeleteDialogFn(props)"
            ></q-btn>
          </q-td>
        </template>

        <template v-slot:body-cell-function="props">
          <q-td :props="props">
            <q-tooltip>
              <pre>{{ props.row.sql }}</pre>
            </q-tooltip>
            <pre style="white-space: break-spaces">{{ props.row.sql }}</pre>
          </q-td>
        </template>

        <template #top="scope">
          <div class="q-table__title" data-test="alerts-list-title">
            {{ t("actions.header") }}
          </div>
          <q-input
            data-test="alert-list-search-input"
            v-model="filterQuery"
            borderless
            filled
            dense
            class="q-ml-auto q-mb-xs no-border tw-w-[350px]"
            :placeholder="t('actions.search')"
          >
            <template #prepend>
              <q-icon name="search" class="cursor-pointer" />
            </template>
          </q-input>
          <q-btn
            data-test="alert-list-add-alert-btn"
            class="q-ml-md q-mb-xs text-bold no-border"
            padding="sm lg"
            color="secondary"
            no-caps
            :label="t(`actions.add`)"
            @click="showAddUpdateFn({})"
          />

          <QTablePagination
            :scope="scope"
            :pageTitle="t('actions.header')"
            :position="'top'"
            :resultTotal="resultTotal"
            :perPageOptions="perPageOptions"
            @update:changeRecordPerPage="changePagination"
          />
        </template>

        <template #bottom="scope">
          <QTablePagination
            :scope="scope"
            :position="'bottom'"
            :resultTotal="resultTotal"
            :perPageOptions="perPageOptions"
            @update:changeRecordPerPage="changePagination"
          />
        </template>
      </q-table>
    </div>
    <template v-else>
      <div class="tw-w-full">
        <EditScript
          :isUpdated="isUpdated"
          @update:list="refreshList"
          @cancel:hideform="hideForm"
          @get-action-scripts="getActionScripts"
        />
      </div>
    </template>
    <ConfirmDialog
      title="Delete Action"
      message="Are you sure you want to delete Action?"
      @update:ok="deleteAlert"
      @update:cancel="confirmDelete = false"
      v-model="confirmDelete"
    />
    <template>
      <q-dialog class="q-pa-md" v-model="showForm" persistent>
        <q-card class="clone-alert-popup">
          <div class="row items-center no-wrap q-mx-md q-my-sm">
            <div class="flex items-center">
              <div
                data-test="add-alert-back-btn"
                class="flex justify-center items-center q-mr-md cursor-pointer"
                style="
                  border: 1.5px solid;
                  border-radius: 50%;
                  width: 22px;
                  height: 22px;
                "
                title="Go Back"
                @click="showForm = false"
              >
                <q-icon name="arrow_back_ios_new" size="14px" />
              </div>
              <div class="text-h6" data-test="clone-alert-title">
                {{ t("alerts.cloneTitle") }}
              </div>
            </div>
          </div>
          <q-card-section>
            <q-form @submit="submitForm">
              <q-input
                data-test="to-be-clone-alert-name"
                v-model="toBeCloneAlertName"
                label="Alert Name"
              />
              <q-select
                data-test="to-be-clone-stream-type"
                v-model="toBeClonestreamType"
                label="Stream Type"
                :options="streamTypes"
                @update:model-value="updateStreams()"
              />
              <q-select
                data-test="to-be-clone-stream-name"
                v-model="toBeClonestreamName"
                :loading="isFetchingStreams"
                :disable="!toBeClonestreamType"
                label="Stream Name"
                :options="streamNames"
                @change="updateStreamName"
                @filter="filterStreams"
                use-input
                fill-input
                hide-selected
                :input-debounce="400"
              />
              <div class="flex justify-center q-mt-lg">
                <q-btn
                  data-test="clone-alert-cancel-btn"
                  v-close-popup="true"
                  class="q-mb-md text-bold"
                  :label="t('alerts.cancel')"
                  text-color="light-text"
                  padding="sm md"
                  no-caps
                />
                <q-btn
                  data-test="clone-alert-submit-btn"
                  :label="t('alerts.save')"
                  class="q-mb-md text-bold no-border q-ml-md"
                  color="secondary"
                  padding="sm xl"
                  type="submit"
                  :disable="isSubmitting"
                  no-caps
                />
              </div>
            </q-form>
          </q-card-section>
        </q-card>
      </q-dialog>
    </template>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  ref,
  onBeforeMount,
  onActivated,
  watch,
  defineAsyncComponent,
} from "vue";
import type { Ref } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import useStreams from "@/composables/useStreams";

import { QTable, date, useQuasar, type QTableProps } from "quasar";
import { useI18n } from "vue-i18n";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import alertsService from "@/services/alerts";
import destinationService from "@/services/alert_destination";
import templateService from "@/services/alert_templates";
import NoData from "@/components/shared/grid/NoData.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import segment from "@/services/segment_analytics";
import config from "@/aws-exports";
import {
  getImageURL,
  getUUID,
  verifyOrganizationStatus,
  convertUnixToQuasarFormat,
} from "@/utils/zincutils";
import type { Alert, AlertListItem } from "@/ts/interfaces/index";
import {
  outlinedDelete,
  outlinedPause,
  outlinedPlayArrow,
} from "@quasar/extras/material-icons-outlined";
import actions from "@/services/action_scripts";

interface ActionScriptList {
  "#": string | number; // If this represents a serial number or row index
  id: any; // The unique identifier, specify the type (e.g., string, number) if known
  name: any; // Name of the action script
  uuid: any; // Unique UUID for the action script
  created_by: any; // The user who created the script
  created_at: string; // Creation timestamp, in ISO format or a specific format
  last_run_at: string; // Timestamp of the last run, in ISO format or a specific format
  last_successful_at: string; // Timestamp of the last successful run
  status: any; // Current status of the action script
}

// import alertList from "./alerts";

// TODO code clean up needs to be done
export default defineComponent({
  name: "AlertList",
  components: {
    QTablePagination,
    EditScript: defineAsyncComponent(
      () => import("@/components/actionScripts/EditScript.vue"),
    ),
    NoData,
    ConfirmDialog,
  },
  emits: [
    "updated:fields",
    "update:changeRecordPerPage",
    "update:maxRecordToReturn",
  ],
  setup() {
    const store = useStore();
    const { t } = useI18n();
    const $q = useQuasar();
    const router = useRouter();
    const alerts: Ref<Alert[]> = ref([]);
    const actionsScriptRows: Ref<ActionScriptList[]> = ref([]);
    const formData: Ref<Alert | {}> = ref({});
    const toBeClonedAlert: Ref<Alert | {}> = ref({});
    const showAddActionScriptDialog: any = ref(false);
    const qTable: Ref<InstanceType<typeof QTable> | null> = ref(null);
    const selectedDelete: any = ref(null);
    const isUpdated: any = ref(false);
    const confirmDelete = ref<boolean>(false);
    const splitterModel = ref(220);
    const showForm = ref(false);
    const indexOptions = ref([]);
    const schemaList = ref([]);
    const streams: any = ref({});
    const isFetchingStreams = ref(false);
    const isSubmitting = ref(false);

    const { getStreams } = useStreams();

    const toBeCloneAlertName = ref("");
    const toBeCloneUUID = ref("");
    const toBeClonestreamType = ref("");
    const toBeClonestreamName = ref("");
    const streamTypes = ref(["logs", "metrics", "traces"]);
    const streamNames: Ref<string[]> = ref([]);
    const alertStateLoadingMap: Ref<{ [key: string]: boolean }> = ref({});
    const folders = ref([
      {
        name: "folder1",
      },
      {
        name: "folder2",
      },
    ]);
    const columns: any = ref<QTableProps["columns"]>([
      {
        name: "#",
        label: "#",
        field: "#",
        align: "left",
      },
      {
        name: "name",
        field: "name",
        label: t("alerts.name"),
        align: "left",
        sortable: true,
      },
      {
        name: "created_by",
        field: "created_by",
        label: t("alerts.createdBy"),
        align: "center",
        sortable: true,
      },
      {
        name: "created_at",
        field: "created_at",
        label: t("alerts.createdAt"),
        align: "left",
        sortable: true,
      },
      {
        name: "execution_details_type",
        field: "execution_details_type",
        label: t("actions.type"),
        align: "left",
        sortable: true,
      },
      {
        name: "last_run_at",
        field: "last_run_at",
        label: t("alerts.lastRunAt"),
        align: "left",
        sortable: true,
      },
      {
        name: "last_successful_at",
        field: "last_successful_at",
        label: t("alerts.lastSuccessfulAt"),
        align: "left",
        sortable: true,
      },
      {
        name: "status",
        field: "status",
        label: t("alerts.status"),
        align: "left",
        sortable: true,
      },
      {
        name: "actions",
        field: "actions",
        label: t("alerts.actions"),
        align: "center",
        sortable: false,
      },
    ]);
    const activeTab: any = ref("alerts");
    const destinations = ref([0]);
    const templates = ref([0]);
    const getActionScripts = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading actions...",
      });
      actions
        .list(store.state.selectedOrganization.identifier)
        .then((res) => {
          var counter = 1;
          resultTotal.value = res.data.length;
          alerts.value = res.data.map((alert: any) => {
            return {
              ...alert,
              uuid: getUUID(),
            };
          });
          actionsScriptRows.value = alerts.value.map((data: any) => {
            if (data.execution_details_type == "repeat")
              data.execution_details_type = "Cron Job";
            return {
              "#": counter <= 9 ? `0${counter++}` : counter++,
              id: data.id,
              name: data.name,
              uuid: data.uuid,
              created_by: data.created_by,
              created_at: data.created_at
                ? convertUnixToQuasarFormat(data.created_at)
                : "-",
              last_run_at: data.last_run_at
                ? convertUnixToQuasarFormat(data.last_run_at)
                : "-",
              last_successful_at: data.last_successful_at
                ? convertUnixToQuasarFormat(data.last_successful_at)
                : "-",
              status: data.status,
              execution_details_type: data.execution_details_type,
            };
          });
          actionsScriptRows.value.forEach((alert: ActionScriptList) => {
            alertStateLoadingMap.value[alert.uuid as string] = false;
          });
          if (router.currentRoute.value.query.action == "add") {
            showAddUpdateFn({ row: undefined });
          }
          if (router.currentRoute.value.query.action == "update") {
            const alertName = router.currentRoute.value.query.id as string;
            showAddUpdateFn({
              row: getAlertByName(alertName),
            });
          }
          dismiss();
        })
        .catch((e) => {
          console.error(e);
          dismiss();
          $q.notify({
            type: "negative",
            message: "Error while pulling Actions.",
            timeout: 2000,
          });
        });
    };
    const getAlertByName = (id: string) => {
      return alerts.value.find((alert) => alert.id === id);
    };
    if (!alerts.value.length) {
      getActionScripts();
    }
    onBeforeMount(async () => {
      // await getTemplates();
      // getDestinations();
    });
    // onActivated(() => getDestinations());
    watch(
      () => router.currentRoute.value.query.action,
      (action) => {
        if (!action) showAddActionScriptDialog.value = false;
      },
    );
    // const getDestinations = async () => {
    //   destinationService
    //     .list({
    //       org_identifier: store.state.selectedOrganization.identifier,
    //     })
    //     .then((res) => {
    //       destinations.value = res.data;
    //     })
    //     .catch(() =>
    //       $q.notify({
    //         type: "negative",
    //         message: "Error while fetching destinations.",
    //         timeout: 3000,
    //       }),
    //     );
    // };

    // const getTemplates = () => {
    //   templateService
    //     .list({
    //       org_identifier: store.state.selectedOrganization.identifier,
    //     })
    //     .then((res) => {
    //       templates.value = res.data;
    //     })
    //     .catch(() =>
    //       $q.notify({
    //         type: "negative",
    //         message: "Error while fetching templates.",
    //         timeout: 3000,
    //       }),
    //     );
    // };
    const perPageOptions: any = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
    ];
    const resultTotal = ref<number>(0);
    const maxRecordToReturn = ref<number>(100);
    const selectedPerPage = ref<number>(20);
    const pagination: any = ref({
      rowsPerPage: 20,
    });
    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value?.setPagination(pagination.value);
    };
    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
    };

    const addAlert = () => {
      showAddActionScriptDialog.value = true;
    };

    const duplicateAlert = (row: any) => {
      toBeCloneUUID.value = row.uuid;
      toBeCloneAlertName.value = row.name;
      toBeClonestreamName.value = "";
      toBeClonestreamType.value = "";

      showForm.value = true;
    };
    const submitForm = async () => {
      const alertToBeCloned = alerts.value.find(
        (alert) => alert.uuid === toBeCloneUUID.value,
      ) as Alert;

      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      if (!alertToBeCloned) {
        $q.notify({
          type: "negative",
          message: "Alert not found",
          timeout: 2000,
        });
        return;
      }
      if (!toBeClonestreamType.value) {
        $q.notify({
          type: "negative",
          message: "Please select stream type ",
          timeout: 2000,
        });
        return;
      }
      if (!toBeClonestreamName.value) {
        $q.notify({
          type: "negative",
          message: "Please select stream name",
          timeout: 2000,
        });
        return;
      }
      isSubmitting.value = true;

      alertToBeCloned.name = toBeCloneAlertName.value;
      alertToBeCloned.stream_name = toBeClonestreamName.value;
      alertToBeCloned.stream_type = toBeClonestreamType.value;

      try {
        alertsService
          .create(
            store.state.selectedOrganization.identifier,
            alertToBeCloned.stream_name,
            alertToBeCloned.stream_type,
            alertToBeCloned,
          )
          .then((res) => {
            dismiss();
            if (res.data.code == 200) {
              $q.notify({
                type: "positive",
                message: "Alert Cloned Successfully",
                timeout: 2000,
              });
              showForm.value = false;
              getActionScripts();
            } else {
              $q.notify({
                type: "negative",
                message: res.data.message,
                timeout: 2000,
              });
            }
          })
          .catch((e: any) => {
            if (e.response?.status == 403) {
              showForm.value = false;
              isSubmitting.value = false;
              return;
            }
            dismiss();
            $q.notify({
              type: "negative",
              message: e.response.data.message,
              timeout: 2000,
            });
          })
          .finally(() => {
            isSubmitting.value = false;
          });
      } catch (e: any) {
        showForm.value = true;
        isSubmitting.value = false;
        $q.notify({
          type: "negative",
          message: e.data.message,
          timeout: 2000,
        });
      }
    };
    const showAddUpdateFn = (props: any) => {
      formData.value = alerts.value.find(
        (alert: any) => alert.uuid === props.row?.uuid,
      ) as Alert;
      //use this comment for testing multi_time_range shifts
      // if( formData.value){
      //   formData.value.query_condition.multi_time_range = [{offSet:"30m"}];
      // }
      let action;
      if (!props.row) {
        isUpdated.value = false;
        action = "Add Alert";
        router.push({
          name: "actionScripts",
          query: {
            action: "add",
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      } else {
        isUpdated.value = true;
        action = "Update Alert";
        router.push({
          name: "actionScripts",
          query: {
            action: "update",
            id: props.row.id,
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
      addAlert();
      if (config.enableAnalytics == "true") {
        segment.track("Button Click", {
          button: action,
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          page: "Alerts",
        });
      }
    };
    const refreshList = () => {
      getActionScripts();
      hideForm();
    };
    const hideForm = () => {
      showAddActionScriptDialog.value = false;
      router.push({
        name: "actionScripts",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };
    const deleteAlert = () => {
      actions
        .delete(
          store.state.selectedOrganization.identifier,
          selectedDelete.value.id,
        )
        .then((res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              type: "positive",
              message: res.data.message,
              timeout: 2000,
            });
            getActionScripts();
          } else {
            $q.notify({
              type: "negative",
              message: res.data.message,
              timeout: 2000,
            });
          }
        })
        .catch((err) => {
          if (err.response?.status == 403) {
            return;
          }
          $q.notify({
            type: "negative",
            message: err?.data?.message || "Error while deleting alert.",
            timeout: 2000,
          });
        });
      if (config.enableAnalytics == "true") {
        segment.track("Button Click", {
          button: "Delete Alert",
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          alert_name: selectedDelete.value.name,
          page: "Alerts",
        });
      }
    };
    const showDeleteDialogFn = (props: any) => {
      selectedDelete.value = props.row;
      confirmDelete.value = true;
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
    const updateStreamName = (selectedOption: any) => {
      toBeClonestreamName.value = selectedOption;
    };
    const updateStreams = (resetStream = true) => {
      if (resetStream) toBeClonestreamName.value = "";
      if (streams.value[toBeClonestreamType.value]) {
        schemaList.value = streams.value[toBeClonestreamType.value];
        indexOptions.value = streams.value[toBeClonestreamType.value].map(
          (data: any) => {
            return data.name;
          },
        );
        updateStreamName(toBeClonestreamName.value);

        return;
      }

      if (!toBeClonestreamType.value) return Promise.resolve();

      isFetchingStreams.value = true;
      return getStreams(toBeClonestreamType.value, false)
        .then((res: any) => {
          streams.value[toBeClonestreamType.value] = res.list;
          schemaList.value = res.list;
          indexOptions.value = res.list.map((data: any) => {
            return data.name;
          });

          return Promise.resolve();
        })
        .catch(() => Promise.reject())
        .finally(() => (isFetchingStreams.value = false));
    };
    const filterStreams = (val: string, update: any) => {
      streamNames.value = filterColumns(indexOptions.value, val, update);
    };

    // const toggleAlertState = (row: any) => {
    //   alertStateLoadingMap.value[row.uuid] = true;
    //   const alert: Alert = alerts.value.find(
    //     (alert) => alert.uuid === row.uuid,
    //   ) as Alert;
    //   alertsService
    //     .toggleState(
    //       store.state.selectedOrganization.identifier,
    //       alert.stream_name,
    //       alert.name,
    //       !alert?.enabled,
    //       alert.stream_type,
    //     )
    //     .then(() => {
    //       alert.enabled = !alert.enabled;
    //       actionsScriptRows.value.forEach((alert) => {
    //         alert.uuid === row.uuid ? (alert.enabled = !alert.enabled) : null;
    //       });
    //     })
    //     .finally(() => {
    //       alertStateLoadingMap.value[row.uuid] = false;
    //     });
    // };

    const routeTo = (name: string) => {
      router.push({
        name: name,
        query: {
          action: "add",
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    // const refreshDestination = async () => {
    //   await getDestinations();
    // };

    return {
      t,
      qTable,
      store,
      router,
      alerts,
      columns,
      formData,
      hideForm,
      confirmDelete,
      selectedDelete,
      updateStreams,
      updateStreamName,
      getActionScripts,
      pagination,
      resultTotal,
      refreshList,
      perPageOptions,
      selectedPerPage,
      addAlert,
      deleteAlert,
      isUpdated,
      showAddUpdateFn,
      showDeleteDialogFn,
      duplicateAlert,
      changePagination,
      maxRecordToReturn,
      showAddActionScriptDialog,
      showForm,
      toBeCloneAlertName,
      toBeCloneUUID,
      toBeClonestreamType,
      toBeClonestreamName,
      streamTypes,
      filterColumns,
      filterStreams,
      streamNames,
      submitForm,
      schemaList,
      indexOptions,
      streams,
      isFetchingStreams,
      isSubmitting,
      changeMaxRecordToReturn,
      outlinedDelete,
      filterQuery: ref(""),
      filterData(rows: any, terms: any) {
        var filtered = [];
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (
            rows[i]["name"].toLowerCase().includes(terms) ||
            (rows[i]["stream_name"] != null &&
              rows[i]["stream_name"].toLowerCase().includes(terms)) ||
            (rows[i]["owner"] != null &&
              rows[i]["owner"].toLowerCase().includes(terms)) ||
            (rows[i]["enabled"] != null &&
              rows[i]["enabled"].toString().toLowerCase().includes(terms)) ||
            (rows[i]["alert_type"] != null &&
              rows[i]["alert_type"].toString().toLowerCase().includes(terms)) ||
            (rows[i]["stream_type"] != null &&
              rows[i]["stream_type"]
                .toString()
                .toLowerCase()
                .includes(terms)) ||
            (rows[i]["description"] != null &&
              rows[i]["description"].toString().toLowerCase().includes(terms))
          ) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
      getImageURL,
      activeTab,
      destinations,
      verifyOrganizationStatus,
      folders,
      splitterModel,
      outlinedPause,
      outlinedPlayArrow,
      actionsScriptRows,
      alertStateLoadingMap,
      templates,
      routeTo,
    };
  },
});
</script>

<style lang="scss">
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}

.alert-list-table {
  th:last-child,
  td:last-child {
    position: sticky;
    right: 0;
    z-index: 1;
    background: #ffffff;
    box-shadow: -4px 0px 4px 0 rgba(0, 0, 0, 0.1);
  }
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

.alerts-tabs {
  .q-tabs {
    &--vertical {
      margin: 1.5rem 1rem 0 0;

      .q-tab {
        justify-content: flex-start;
        padding: 0 1rem 0 1.25rem;
        border-radius: 0.5rem;
        margin-bottom: 0.5rem;

        &__content.tab_content {
          .q-tab {
            &__icon + &__label {
              padding-left: 0.875rem;
              font-weight: 600;
            }
          }
        }

        &--active {
          background-color: $accent;
        }
      }
    }
  }
}
.clone-alert-popup {
  width: 400px;
}
</style>
