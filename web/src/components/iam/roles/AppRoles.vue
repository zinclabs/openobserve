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
    data-test="iam-roles-section-title"
    style="font-size: 18px; padding-top: 12px"
    class="q-mb-md q-px-md"
  >
    {{ t("iam.roles") }}
  </div>

  <div>
    <div class="q-px-md q-mb-sm q-pb-xs row items-center justify-between">
      <div date-test="iam-roles-search-input">
        <q-input
          v-model="filterQuery"
          filled
          dense
          class="q-pr-sm"
          style="width: 500px"
          :placeholder="t('iam.searchRole')"
        >
          <template #prepend>
            <q-icon name="search" />
          </template>
        </q-input>
      </div>

      <q-btn
        data-test="alert-list-add-alert-btn"
        class="q-ml-md q-mb-xs text-bold no-border q-mr-sm"
        padding="sm lg"
        color="secondary"
        no-caps
        :label="t(`iam.addRole`)"
        @click="addRole"
      />
    </div>
    <div
      data-test="iam-roles-table-title"
      class="q-px-md q-pt-sm q-ml-xs text-bold"
      style="font-size: 14px; padding-bottom: 12px"
    >
      {{ rows.length }} {{ t("iam.roles") }}
      <!-- Add QTablePagination here -->
    </div>
    <app-table
      data-test="iam-roles-table-section"
      class="iam-table"
      :rows="rows"
      :columns="columns"
      pagination
      :rows-per-page="25"
      :filter="{
        value: filterQuery,
        method: filterRoles,
      }"
      :bordered="false"
    >
      <template v-slot:actions="slotProps: any">
        <div>
          <q-icon
            :data-test="`iam-roles-edit-${slotProps.column.row.role_name}-role-icon`"
            size="14px"
            name="edit"
            class="cursor-pointer q-mr-md"
            :title="t('common.edit')"
            @click="() => editRole(slotProps.column.row)"
          />
          <q-icon
            :data-test="`iam-roles-delete-${slotProps.column.row.role_name}-role-icon`"
            size="14px"
            name="delete"
            class="cursor-pointer"
            :title="t('common.delete')"
            @click="() => showConfirmDialog(slotProps.column.row)"
          />
        </div>
      </template>
    </app-table>
  </div>
  <q-dialog v-model="showAddGroup" position="right" full-height maximized>
    <AddRole
      style="width: 30vw"
      @cancel:hideform="hideForm"
      @added:role="setupRoles"
    />
  </q-dialog>
  <ConfirmDialog
    title="Delete Role"
    :message="`Are you sure you want to delete '${deleteConformDialog?.data?.role_name as string}' role?`"
    @update:ok="_deleteRole"
    @update:cancel="deleteConformDialog.show = false"
    v-model="deleteConformDialog.show"
  />
</template>

<script setup lang="ts">
import { onBeforeMount, ref } from "vue";
import AddRole from "./AddRole.vue";
import { useI18n } from "vue-i18n";
import AppTable from "@/components/AppTable.vue";
import { cloneDeep } from "lodash-es";
import { useRouter } from "vue-router";
import { getRoles, deleteRole } from "@/services/iam";
import { useStore } from "vuex";
import usePermissions from "@/composables/iam/usePermissions";
import { useQuasar } from "quasar";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

const { t } = useI18n();

const showAddGroup = ref(false);

const rows: any = ref([]);

const router = useRouter();

const store = useStore();

const q = useQuasar();

const deleteConformDialog = ref({
  show: false,
  data: null as any,
});

const columns: any = [
  {
    name: "#",
    label: "#",
    field: "#",
    align: "left",
  },
  {
    name: "role_name",
    field: "role_name",
    label: t("iam.roleName"),
    align: "left",
    sortable: true,
  },
  {
    name: "actions",
    field: "actions",
    label: t("common.actions"),
    align: "center",
    sortable: false,
    slot: true,
    slotName: "actions",
    style: "width: 400px",
  },
];

const { rolesState } = usePermissions();

onBeforeMount(() => {
  setupRoles();
});

const filterQuery = ref("");

const updateTable = () => {
   let counter = 1;
  rows.value = cloneDeep(
    rolesState.roles.map((role: { role_name: string }, index) => ({
      ...role,
      // "#": index + 1,
       "#": counter <= 9 ? `0${counter++}` : counter++,
    }))
  );
};

const addRole = () => {
  showAddGroup.value = true;
};

const editRole = (role: any) => {
  router.push({
    name: "editRole",
    params: {
      role_name: role.role_name,
    },
  });
};

const setupRoles = async () => {
  await getRoles(store.state.selectedOrganization.identifier)
    .then((res) => {
      rolesState.roles = res.data.map((role: string) => ({
        role_name: role,
      }));
      updateTable();
    })
    .catch((err) => {
      console.log(err);
    });
};

const filterRoles = (rows: any, terms: any) => {
  var filtered = [];
  terms = terms.toLowerCase();
  for (var i = 0; i < rows.length; i++) {
    if (rows[i]["role_name"].toLowerCase().includes(terms)) {
      filtered.push(rows[i]);
    }
  }
  return filtered;
};

const hideForm = () => {
  showAddGroup.value = false;
};

const deleteUserRole = (role: any) => {
  deleteRole(role.role_name, store.state.selectedOrganization.identifier)
    .then(() => {
      q.notify({
        message: "Role deleted successfully!",
        color: "positive",
        position: "bottom",
      });
      setupRoles();
    })
    .catch((error: any) => {
      if (error.response.status != 403) {
        q.notify({
          message: "Error while deleting role!",
          color: "negative",
          position: "bottom",
        });
      }
    });
};

const showConfirmDialog = (row: any) => {
  deleteConformDialog.value.show = true;
  deleteConformDialog.value.data = row;
};

const _deleteRole = () => {
  deleteUserRole(deleteConformDialog.value.data);
  deleteConformDialog.value.data = null;
};
</script>

<style scoped></style>
<style lang="scss">
.iam-table {
  .thead-sticky,
  .tfoot-sticky {
    position: sticky;
    top: 0;
    opacity: 1;
    z-index: 1;
    background: transparent !important;
  }

  .q-table--dark .thead-sticky,
  .q-table--dark .tfoot-sticky {
    background: transparent !important;
  }
}
</style>
