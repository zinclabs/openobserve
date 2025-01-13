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
  <div>
    <div
      data-test="iam-groups-section-title"
      style="font-size: 18px; padding-top: 12px"
      class="q-mb-md q-px-md"
    >
      {{ t("iam.groups") }}
    </div>

    <div>
      <div class="q-px-md q-mb-sm q-pb-xs row items-center justify-between">
        <div date-test="iam-groups-search-input">
          <q-input
            v-model="filterQuery"
            filled
            dense
            class="q-pr-sm"
            style="width: 500px"
            :placeholder="t('iam.searchGroup')"
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
          :label="t(`iam.addGroup`)"
          @click="addGroup"
        />
      </div>
      <div
        data-test="iam-groups-table-title"
        class="q-px-md q-pt-sm q-ml-xs text-bold"
        style="font-size: 14px; padding-bottom: 12px"
      >
        {{ rows.length }} {{ t("iam.groups") }}
      </div>
      <app-table
        data-test="iam-groups-table-section"
        class="iam-table"
        :rows="rows"
        :columns="columns"
        pagination
        :rows-per-page="25"
        :filter="{
          value: filterQuery,
          method: filterGroups,
        }"
        :bordered="false"
      >
        <template v-slot:actions="slotProps: any">
          <div>
            <q-icon
              :data-test="`iam-groups-edit-${slotProps.column.row.group_name}-role-icon`"
              size="14px"
              name="edit"
              class="cursor-pointer q-mr-md"
              :title="t('common.edit')"
              @click="editGroup(slotProps.column.row)"
            />
            <q-icon
              :data-test="`iam-groups-delete-${slotProps.column.row.group_name}-role-icon`"
              size="14px"
              name="delete"
              class="cursor-pointer"
              :title="t('common.delete')"
              @click="showConfirmDialog(slotProps.column.row)"
            />
          </div>
        </template>
      </app-table>
    </div>

    <q-dialog v-model="showAddGroup" position="right" full-height maximized>
      <AddGroup
        style="width: 30vw"
        :org_identifier="store.state.selectedOrganization.identifier"
        @cancel:hideform="hideAddGroup"
        @added:group="setupGroups"
      />
    </q-dialog>
    <ConfirmDialog
      title="Delete Group"
      :message="`Are you sure you want to delete '${deleteConformDialog?.data?.group_name as string}'?`"
      @update:ok="_deleteGroup"
      @update:cancel="deleteConformDialog.show = false"
      v-model="deleteConformDialog.show"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onBeforeMount } from "vue";
import AddGroup from "./AddGroup.vue";
import { useI18n } from "vue-i18n";
import AppTable from "@/components/AppTable.vue";
import { cloneDeep } from "lodash-es";
import { useRouter } from "vue-router";
import { useStore } from "vuex";
import { getGroups, deleteGroup } from "@/services/iam";
import usePermissions from "@/composables/iam/usePermissions";
import { useQuasar } from "quasar";
import ConfirmDialog from "@/components/ConfirmDialog.vue";

const showAddGroup = ref(false);

const { t } = useI18n();

const rows: any = ref([]);

const router = useRouter();

const store = useStore();

const { groupsState } = usePermissions();

const filterQuery = ref("");

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
    name: "group_name",
    field: "group_name",
    label: t("iam.groupName"),
    align: "left",
    sortable: true,
  },
  {
    name: "actions",
    field: "actions",
    label: t("alerts.actions"),
    align: "center",
    sortable: false,
    slot: true,
    slotName: "actions",
    style: "width: 400px",
  },
];

const groups = ref([]);

onBeforeMount(() => {
  setupGroups();
});

const updateTable = () => {
  let counter = 1;
  rows.value = cloneDeep(
    groupsState.groups.map((group: { group_name: string }, index: number) => ({
      ...group,
      "#": counter <= 9 ? `0${counter++}` : counter++,
    }))
  );
};

const addGroup = () => {
  showAddGroup.value = true;
};

const editGroup = (group: any) => {
  router.push({
    name: "editGroup",
    params: {
      group_name: group.group_name,
    },
  });
};

const setupGroups = async () => {
  await getGroups(store.state.selectedOrganization.identifier)
    .then((res) => {
      groupsState.groups = res.data.map((group: string) => ({
        group_name: group,
      }));
      updateTable();
    })
    .catch((err) => {
      console.log(err);
    });
};

const hideAddGroup = () => {
  showAddGroup.value = false;
};

const filterGroups = (rows: any, terms: any) => {
  var filtered = [];
  terms = terms.toLowerCase();
  for (var i = 0; i < rows.length; i++) {
    if (rows[i]["group_name"].toLowerCase().includes(terms)) {
      filtered.push(rows[i]);
    }
  }
  return filtered;
};

const deleteUserGroup = (group: any) => {
  deleteGroup(group.group_name, store.state.selectedOrganization.identifier)
    .then(() => {
      q.notify({
        message: "Role deleted successfully!",
        color: "positive",
        position: "bottom",
      });
      setupGroups();
    })
    .catch((error: any) => {
      if (error.response.status != 403) {
        q.notify({
          message: "Error while deleting group!",
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

const _deleteGroup = () => {
  deleteUserGroup(deleteConformDialog.value.data);
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
