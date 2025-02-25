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
    <div v-if=" !showImportTemplate  && !showTemplateEditor">
      <q-table
        data-test="alert-templates-list-table"
        ref="q-table"
        :rows="templates"
        :columns="columns"
        row-key="id"
        style="width: 100%"
        :rows-per-page-options="[0]"
        :pagination="pagination"
        :filter="filterQuery"
        :filter-method="filterData"
      >
        <template #no-data>
          <NoData />
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
              @click.stop="exportTemplate(props.row)"
              data-test="destination-export"
            ></q-btn>
            <q-btn
              :data-test="`alert-template-list-${props.row.name}-update-template`"
              icon="edit"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alert_templates.edit')"
              @click="editTemplate(props.row)"
            ></q-btn>
            <q-btn
              :data-test="`alert-template-list-${props.row.name}-delete-template`"
              :icon="outlinedDelete"
              class="q-ml-xs"
              padding="sm"
              unelevated
              size="sm"
              round
              flat
              :title="t('alert_templates.delete')"
              @click="conformDeleteDestination(props.row)"
            ></q-btn>
          </q-td>
        </template>
        <template #top>
          <div class="q-table__title" data-test="alert-templates-list-title">
            {{ t("alert_templates.header") }}
          </div>
          <q-input
            data-test="template-list-search-input"
            v-model="filterQuery"
            borderless
            filled
            dense
            class="q-ml-auto q-mb-xs no-border"
            :placeholder="t('alert_templates.search')"
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
            @click="importTemplate"
            data-test="alert-import"
          />
          <q-btn
            data-test="alert-template-list-add-alert-btn"
            class="q-ml-md q-mb-xs text-bold no-border"
            padding="sm lg"
            color="secondary"
            no-caps
            :label="t(`alert_templates.add`)"
            @click="editTemplate(null)"
          />
        </template>
      </q-table>
    </div>
    <div v-else-if="!showImportTemplate && showTemplateEditor">
      <AddTemplate
        :template="editingTemplate"
        @cancel:hideform="toggleTemplateEditor"
        @get:templates="getTemplates"
      />
    </div>
    <div v-else>
      <ImportTemplate
      :templates="templates"
      @update:templates="getTemplates"
       />
    </div>



    <ConfirmDialog
      title="Delete Template"
      message="Are you sure you want to delete template?"
      @update:ok="deleteTemplate"
      @update:cancel="cancelDeleteTemplate"
      v-model="confirmDelete.visible"
    />
  </q-page>
</template>
<script lang="ts" setup>
import { ref, onActivated, onMounted, watch, defineAsyncComponent } from "vue";
import type { Ref } from "vue";
import { useI18n } from "vue-i18n";
import { useQuasar, type QTableProps } from "quasar";
import NoData from "../shared/grid/NoData.vue";
import templateService from "@/services/alert_templates";
import ConfirmDialog from "../ConfirmDialog.vue";
import type { TemplateData, Template } from "@/ts/interfaces";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";
import ImportTemplate from "./ImportTemplate.vue";

const AddTemplate = defineAsyncComponent(
  () => import("@/components/alerts/AddTemplate.vue")
);

const store = useStore();
const { t } = useI18n();
const router = useRouter();
const q = useQuasar();
const templates: Ref<Template[]> = ref([]);
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
    label: t("alert_templates.name"),
    align: "left",
    sortable: true,
  },
  {
    name: "actions",
    field: "actions",
    label: t("alert_templates.actions"),
    align: "center",
    sortable: false,
    style: "width: 110px",
  },
]);
const showTemplateEditor = ref(false);
const showImportTemplate = ref(false);
const editingTemplate: Ref<TemplateData | null> = ref(null);
const confirmDelete: Ref<{
  visible: boolean;
  data: any;
}> = ref({ visible: false, data: null });
const pagination = {
  page: 1,
  rowsPerPage: 0, // 0 means all rows
};
const filterQuery = ref("");
onActivated(() => {
  if (!templates.value.length) updateRoute();
});
onMounted(() => {
  getTemplates();
});

watch(
  () => router.currentRoute.value.query.action,
  (action) => {
    if (!action){ 
      showTemplateEditor.value = false;
      showImportTemplate.value = false;
    };
  }
);

const getTemplates = () => {
  const dismiss = q.notify({
    spinner: true,
    message: "Please wait while loading templates...",
  });
  templateService
    .list({
      org_identifier: store.state.selectedOrganization.identifier,
    })
    .then((res) => {
      templates.value = res.data.map((data: any, index: number) => ({
        ...data,
        "#": index + 1 <= 9 ? `0${index + 1}` : index + 1,
      }));
      updateRoute();
    })
    .catch((err) => {
      dismiss();
      if(err.response.status != 403){
        q.notify({
          type: "negative",
          message: "Error while pulling templates.",
          timeout: 2000,
        });
      }
    })
    .finally(() => {
      dismiss();
    });
};
const updateRoute = () => {
  if (router.currentRoute.value.query.action === "add") editTemplate();
  if (router.currentRoute.value.query.action === "update")
    editTemplate(
      getTemplateByName(router.currentRoute.value.query.name as string)
    );
  if(router.currentRoute.value.query.action === "import") {
    showImportTemplate.value = true
  }
};
const getTemplateByName = (name: string) => {
  return templates.value.find((template) => template.name === name);
};
const editTemplate = (template: any = null) => {
  resetEditingTemplate();
  toggleTemplateEditor();

  const query: { [key: string]: string } = {
    action: template ? "update" : "add",
    org_identifier: store.state.selectedOrganization.identifier,
  };

  if (template) query.name = template.name;

  if (router.currentRoute.value.query.type)
    query.type = router.currentRoute.value.query.type.toString() as string;

  if (!template) {
    router.push({
      name: "alertTemplates",
      query,
    });
  } else {
    editingTemplate.value = { ...template };
    router.push({
      name: "alertTemplates",
      query: {
        action: "update",
        name: template.name,
        org_identifier: store.state.selectedOrganization.identifier,
      },
    });
  }
};
const resetEditingTemplate = () => {
  editingTemplate.value = null;
};
const deleteTemplate = () => {
  if (confirmDelete.value?.data?.name) {
    templateService
      .delete({
        org_identifier: store.state.selectedOrganization.identifier,
        template_name: confirmDelete.value.data.name,
      })
      .then(() => {
        q.notify({
          type: "positive",
          message: `Template ${confirmDelete.value.data.name} deleted successfully`,
          timeout: 2000,
        });

        getTemplates();
      })
      .catch((err) => {
        if (err.response.data.code === 409) {
          q.notify({
            type: "negative",
            message: err.response.data.message,
            timeout: 2000,
          });
        }
      });
  }
};
const importTemplate = () => {
  showImportTemplate.value = true;
  router.push({
        name: "alertTemplates",
        query: {
          action: "import",
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
}
const conformDeleteDestination = (destination: any) => {
  confirmDelete.value.visible = true;
  confirmDelete.value.data = destination;
};
const cancelDeleteTemplate = () => {
  confirmDelete.value.visible = false;
  confirmDelete.value.data = null;
};
const toggleTemplateEditor = () => {
  showTemplateEditor.value = !showTemplateEditor.value;
  if (!showTemplateEditor.value)
    router.push({
      name: "alertTemplates",
      query: {
        org_identifier: store.state.selectedOrganization.identifier,
      },
    });
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
const exportTemplate = (row: any) => {
  const findTemplate: any = getTemplateByName(row.name);
      const templateByName = {...findTemplate}
      if(templateByName.hasOwnProperty("#")) delete templateByName["#"];
      const templateJson = JSON.stringify(templateByName,null,2);
      const blob = new Blob([templateJson], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        // Create an anchor element to trigger the download
        const link = document.createElement('a');
        link.href = url;

        // Set the filename of the download
        link.download = `${templateByName.name}.json`;

        // Trigger the download by simulating a click
        link.click();

        // Clean up the URL object after download
        URL.revokeObjectURL(url);

}
</script>
<style lang=""></style>
