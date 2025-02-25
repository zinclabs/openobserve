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
  <q-page class="q-pa-none" style="min-height: inherit">
    <div v-if="!showDestinationEditor && !showImportDestination">
      <q-table
        data-test="alert-destinations-list-table"
        ref="qTable"
        :rows="destinations"
        :columns="columns"
        row-key="id"
        style="width: 100%"
        :pagination="pagination"
        :filter="filterQuery"
        :filter-method="filterData"
      >
        <template #no-data>
          <div
            v-if="!templates.length"
            class="full-width flex column justify-center items-center text-center"
          >
            <div style="width: 600px" class="q-mt-xl">
              <template v-if="!templates.length">
                <div class="text-subtitle1">
                  It looks like you haven't created any Templates yet. To create
                  an Alert, you'll need to have at least one Destination and one
                  Template in place
                </div>
                <q-btn
                  class="q-mt-md"
                  label="Create Template"
                  size="md"
                  color="primary"
                  no-caps
                  style="border-radius: 4px"
                  @click="routeTo('alertTemplates')"
                />
              </template>
            </div>
          </div>
          <template v-else>
            <NoData />
          </template>
        </template>
        <template v-slot:body-cell-actions="props">
          <q-td :props="props">
            <q-btn
              icon="download"
              title="Export Destination"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              @click.stop="exportDestination(props.row)"
              data-test="destination-export"
            ></q-btn>
            <q-btn
              :data-test="`alert-destination-list-${props.row.name}-update-destination`"
              icon="edit"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alert_destinations.edit')"
              @click="editDestination(props.row)"
            ></q-btn>
            <q-btn
              :data-test="`alert-destination-list-${props.row.name}-delete-destination`"
              :icon="outlinedDelete"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alert_destinations.delete')"
              @click="conformDeleteDestination(props.row)"
            ></q-btn>
          </q-td>
        </template>
        <template #top="scope">
          <div class="q-table__title" data-test="alert-destinations-list-title">
            {{ t("alert_destinations.header") }}
          </div>
          <q-input
            data-test="destination-list-search-input"
            v-model="filterQuery"
            borderless
            filled
            dense
            class="q-ml-auto q-mb-xs no-border"
            :placeholder="t('alert_destinations.search')"
          >
            <template #prepend>
              <q-icon name="search" class="cursor-pointer" />
            </template>
          </q-input>
          <q-btn
            class="q-ml-md text-bold"
            padding="sm lg"
            outline
            no-caps
            :label="t(`dashboard.import`)"
            @click="importDestination"
            data-test="alert-import"
          />
          <q-btn
            data-test="alert-destination-list-add-alert-btn"
            class="q-ml-md q-mb-xs text-bold no-border"
            padding="sm lg"
            color="secondary"
            no-caps
            :disable="!templates.length"
            :label="t(`alert_destinations.add`)"
            @click="editDestination(null)"
          />

          <QTablePagination
            :scope="scope"
            :pageTitle="t('alert_destinations.header')"
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
    <div v-else-if="showDestinationEditor && !showImportDestination">
      <AddDestination
        :is-alerts="true"
        :destination="editingDestination"
        :templates="templates"
        @cancel:hideform="toggleDestinationEditor"
        @get:destinations="getDestinations"
      />
    </div>
    <div v-else>
      <ImportDestination
      :destinations="destinations"
      :templates="templates"
      @update:destinations="getDestinations"
       />

    </div>

    <ConfirmDialog
      title="Delete Destination"
      message="Are you sure you want to delete destination?"
      @update:ok="deleteDestination"
      @update:cancel="cancelDeleteDestination"
      v-model="confirmDelete.visible"
    />
  </q-page>
</template>
<script lang="ts">
import { ref, onBeforeMount, onActivated, watch, defineComponent, onMounted } from "vue"; 
import type { Ref } from "vue";
import { useI18n } from "vue-i18n";
import { useQuasar, type QTableProps } from "quasar";
import NoData from "../shared/grid/NoData.vue";
import { getImageURL } from "@/utils/zincutils";
import AddDestination from "./AddDestination.vue";
import destinationService from "@/services/alert_destination";
import templateService from "@/services/alert_templates";
import { useStore } from "vuex";
import ConfirmDialog from "../ConfirmDialog.vue";
import { useRouter } from "vue-router";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import type { DestinationPayload } from "@/ts/interfaces";
import type { Template } from "@/ts/interfaces/index";

import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import ImportDestination from "./ImportDestination.vue";

interface ConformDelete {
  visible: boolean;
  data: any;
}
export default defineComponent({
  name: "PageAlerts",
  components: { AddDestination, NoData, ConfirmDialog, QTablePagination ,ImportDestination},
  setup() {
    const qTable = ref();
    const store = useStore();
    const editingDestination: Ref<DestinationPayload | null> = ref(null);
    const { t } = useI18n();
    const q = useQuasar();
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
        label: t("alert_destinations.name"),
        align: "left",
        sortable: true,
      },
      {
        name: "url",
        field: "url",
        label: t("alert_destinations.url"),
        align: "left",
        sortable: false,
      },
      {
        name: "method",
        field: "method",
        label: t("alert_destinations.method"),
        align: "left",
        sortable: true,
      },
      {
        name: "actions",
        field: "actions",
        label: t("alert_destinations.actions"),
        align: "center",
        sortable: false,
        style: "width: 110px",
      },
    ]);
    const destinations: Ref<DestinationPayload[]> = ref([]);
    const templates: Ref<Template[]> = ref([
      { name: "test", body: "", type: "http" },
    ]);
    const confirmDelete: Ref<ConformDelete> = ref({
      visible: false,
      data: null,
    });
    const showDestinationEditor = ref(false);
    const showImportDestination = ref(false);
    const router = useRouter();
    const filterQuery = ref("");
    const perPageOptions: any = [
      { label: "5", value: 5 },
      { label: "10", value: 10 },
      { label: "20", value: 20 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "All", value: 0 },
    ];
    const resultTotal = ref(0);
    const selectedPerPage = ref(20);
    const pagination: any = ref({
      rowsPerPage: 20,
    });
    onActivated(() => {
      getTemplates();
      if (!destinations.value.length) getDestinations();
    });
    onBeforeMount(() => {
      getDestinations();
      getTemplates();
    });

    watch(
      () => router.currentRoute.value.query.action,
      (action) => {
        if (!action) {
          showDestinationEditor.value = false
          showImportDestination.value = false
        };
      }
    );

    onMounted(()=>{
      updateRoute();
    })

    const getDestinations = () => {
      const dismiss = q.notify({
        spinner: true,
        message: "Please wait while loading destinations...",
      });
      destinationService
        .list({
          page_num: 1,
          page_size: 100000,
          sort_by: "name",
          desc: false,
          org_identifier: store.state.selectedOrganization.identifier,
          module: "alert",
        })
        .then((res) => {
          res.data = res.data.filter(
            (destination: any) =>
              destination.type == "http" || destination.type == "email",
          );
          resultTotal.value = res.data.length;
          destinations.value = res.data.map((data: any, index: number) => ({
            ...data,
            "#": index + 1 <= 9 ? `0${index + 1}` : index + 1,
          }));
          updateRoute();
        })
        .catch((err) => {
          if(err.response.status != 403){
            q.notify({
              type: "negative",
              message: "Error while pulling destinations.",
              timeout: 2000,
            });
          }
          dismiss();
        })
        .finally(() => dismiss());
    };
    const getTemplates = () => {
      templateService
        .list({
          org_identifier: store.state.selectedOrganization.identifier,
        })
        .then((res) => (templates.value = res.data));
    };
    const updateRoute = () => {
      if (router.currentRoute.value.query.action === "add")
        editDestination(null);
      if (router.currentRoute.value.query.action === "update")
        editDestination(
          getDestinationByName(router.currentRoute.value.query.name as string)
        );
      if(router.currentRoute.value.query.action === "import")
        showImportDestination.value = true;
    };
    const getDestinationByName = (name: string) => {
      return destinations.value.find(
        (destination) => destination.name === name
      );
    };
    const editDestination = (destination: any) => {
      toggleDestinationEditor();
      resetEditingDestination();
      if (!destination) {
        router.push({
          name: "alertDestinations",
          query: {
            action: "add",
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      } else {
        editingDestination.value = { ...destination };
        router.push({
          name: "alertDestinations",
          query: {
            action: "update",
            name: destination.name,
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
    };
    const resetEditingDestination = () => {
      editingDestination.value = null;
    };
    const deleteDestination = () => {
      if (confirmDelete.value?.data?.name) {
        destinationService
          .delete({
            org_identifier: store.state.selectedOrganization.identifier,
            destination_name: confirmDelete.value.data.name,
          })
          .then(() => {
            q.notify({
              type: "positive",
              message: `Destination ${confirmDelete.value.data.name} deleted successfully`,
              timeout: 2000,
            });
            getDestinations();
          })
          .catch((err) => {
            if (err.response.data.code === 409) {
              const message =
                err.response.data?.message ||
                err.response.data?.error ||
                "Error while deleting destination";
              q.notify({
                type: "negative",
                message,
                timeout: 2000,
              });
            }
          });
      }
    };
    const conformDeleteDestination = (destination: any) => {
      confirmDelete.value.visible = true;
      confirmDelete.value.data = destination;
    };
    const cancelDeleteDestination = () => {
      confirmDelete.value.visible = false;
      confirmDelete.value.data = null;
    };
    const toggleDestinationEditor = () => {
      showDestinationEditor.value = !showDestinationEditor.value;
      if (!showDestinationEditor.value)
        router.push({
          name: "alertDestinations",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
    };
    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };
    const filterData = (rows: any, terms: any) => {
      var filtered = [];
      terms = terms.toLowerCase();
      for (var i = 0; i < rows.length; i++) {
        if (rows[i]["name"].toLowerCase().includes(terms)) {
          filtered.push(rows[i]);
        }
      }
      return filtered;
    };

    const routeTo = (name: string) => {
      router.push({
        name: name,
        query: {
          action: "add",
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    const exportDestination = (row: any) =>{

      const findDestination: any = getDestinationByName(row.name);
      const destinationByName = {...findDestination}
      if(destinationByName.hasOwnProperty("#")) delete destinationByName["#"];
      const destinationJson = JSON.stringify(destinationByName,null,2);
      const blob = new Blob([destinationJson], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        // Create an anchor element to trigger the download
        const link = document.createElement('a');
        link.href = url;

        // Set the filename of the download
        link.download = `${destinationByName.name}.json`;

        // Trigger the download by simulating a click
        link.click();

        // Clean up the URL object after download
        URL.revokeObjectURL(url);
    }
    const importDestination = () =>{
      showImportDestination.value = true;
      router.push({
        name: "alertDestinations",
        query: {
          action: "import",
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    }

    return {
      t,
      qTable,
      showDestinationEditor,
      destinations,
      columns,
      editDestination,
      getImageURL,
      conformDeleteDestination,
      filterQuery,
      filterData,
      editingDestination,
      templates,
      toggleDestinationEditor,
      getDestinations,
      deleteDestination,
      cancelDeleteDestination,
      confirmDelete,
      changePagination,
      perPageOptions,
      resultTotal,
      pagination,
      outlinedDelete,
      routeTo,
      exportDestination,
      showImportDestination,
      importDestination,
    };
  },
});
</script>
<style lang=""></style>
