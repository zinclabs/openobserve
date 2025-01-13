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
  <div class="col q-pr-xs">
    <div
      data-test="iam-users-selection-filters"
      class="flex justify-start items-center bordered q-px-md q-py-sm"
      :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
      :style="{
        'box-shadow':
          store.state.theme === 'dark'
            ? 'rgb(45 45 45) 0px 4px 7px 0px'
            : 'rgb(240 240 240) 0px 4px 7px 0px',
      }"
      style="position: sticky; top: 0px; z-index: 2"
    >
      <div data-test="iam-users-selection-show-toggle" class="q-mr-md">
        <div class="flex items-center q-pt-xs">
          <span
            data-test="iam-users-selection-show-text"
            style="font-size: 14px"
          >
            Show
          </span>
          <div
            class="q-ml-xs"
            style="
              border: 1px solid #d7d7d7;
              width: fit-content;
              border-radius: 2px;
            "
          >
            <template v-for="visual in usersDisplayOptions" :key="visual.value">
              <q-btn
                :data-test="`iam-users-selection-show-${visual.value}-btn`"
                :color="visual.value === usersDisplay ? 'primary' : ''"
                :flat="visual.value === usersDisplay ? false : true"
                dense
                no-caps
                size="11px"
                class="q-px-md visual-selection-btn"
                @click="updateUserTable(visual.value)"
              >
                {{ visual.label }}</q-btn
              >
            </template>
          </div>
        </div>
      </div>
      <div
        data-test="iam-users-selection-search-input"
        class="o2-input q-mr-md"
        style="width: 400px"
      >
        <q-input
          data-test="alert-list-search-input"
          v-model="userSearchKey"
          borderless
          filled
          dense
          class="q-ml-auto q-mb-xs no-border"
          placeholder="Search User"
        >
          <template #prepend>
            <q-icon name="search" class="cursor-pointer" />
          </template>
        </q-input>
      </div>

      <div
          class="q-mx-sm current-organization"
        >
        <q-select
          v-if="
            store.state.selectedOrganization.identifier ===
              store.state.zoConfig.meta_org &&
            usersDisplay == 'all'
          "
          v-model="selectedOrg"
          borderless
          use-input
          input-debounce="300"
          :options="orgList"
          option-label="label"
          option-value="value"
          class="q-px-none q-py-none q-mx-none q-my-none organizationlist"
          @filter="filterOrganizations"
          @update:model-value="updateOrganization"
          placeholder="Select Organization"
          virtual-scroll
        />

        </div>
    </div>
    <div data-test="iam-users-selection-table" class="q-px-md">
      <div
        data-test="iam-users-selection-table-title"
        class="q-my-sm text-bold"
      >
        {{ rows.length }} Users
      </div>
      <template v-if="rows.length">
        <app-table
          :rows="rows"
          :columns="columns"
          :dense="true"
          :virtual-scroll="false"
          style="height: fit-content"
          :filter="{
            value: userSearchKey,
            method: filterUsers,
          }"
        >
          <template v-slot:select="slotProps: any">
            <q-checkbox
              :data-test="`iam-users-selection-table-body-row-${slotProps.column.row.email}-checkbox`"
              size="xs"
              v-model="slotProps.column.row.isInGroup"
              class="filter-check-box cursor-pointer"
              @click="toggleUserSelection(slotProps.column.row)"
            />
          </template>
        </app-table>
      </template>
      <div
        data-test="iam-users-selection-table-no-users-text"
        v-if="!rows.length"
        class="q-mt-md text-bold q-pl-md"
      >
        No users added
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppTable from "@/components/AppTable.vue";
import usePermissions from "@/composables/iam/usePermissions";
import { cloneDeep } from "lodash-es";
import { watch, computed } from "vue";
import type { Ref } from "vue";
import { ref, onBeforeMount } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
// show selected users in the table
// Add is_selected to the user object
const props = defineProps({
  groupUsers: {
    type: Array,
    default: () => [],
  },
  activeTab: {
    type: String,
    default: "users",
  },
  addedUsers: {
    type: Set,
    default: () => new Set(),
  },
  removedUsers: {
    type: Set,
    default: () => new Set(),
  },
});

const users: Ref<any[]> = ref([]);

const rows: Ref<any[]> = ref([]);

const usersDisplay = ref("selected");

const store = useStore();
const orgOptions = ref([{ label: "All", value: "all" }]);
const selectedOrg = ref(orgOptions.value[0]);
const orgList = ref([...orgOptions.value]);
const usersDisplayOptions = [
  {
    label: "All",
    value: "all",
  },
  {
    label: "Selected",
    value: "selected",
  },
];
const filterOrganizations = (val: string, update: (fn: () => void) => void) => {
  // Filter logic
  update(() => {
    const needle = val.toLowerCase();
    orgList.value = orgOptions.value.filter((org) =>
      org.label.toLowerCase().includes(needle)
    );
  });
};
const { t } = useI18n();

const userSearchKey = ref("");

const hasFetchedOrgUsers = ref(false);

const groupUsersMap = ref(new Set());

const { usersState } = usePermissions();



const columns = computed(() => {
  const baseColumns = [
    {
      name: "select",
      field: "",
      label: "",
      align: "left",
      sortable: false,
      slot: true,
      slotName: "select",
    },
    {
      name: "email",
      field: "email",
      label: t("iam.userName"),
      align: "left",
      sortable: true,
    },
  ];

  // Add "Organizations" column only if the selected organization is "meta"
  if (store.state.selectedOrganization.identifier === store.state.zoConfig.meta_org) {
    baseColumns.push({
      name: "organization",
      field: "org",
      label: "Organizations",
      align: "left",
      sortable: true,
    });
  }

  return baseColumns;
});


onBeforeMount(async () => {  
  groupUsersMap.value = new Set(props.groupUsers);
  await getchOrgUsers();
  updateUserTable(usersDisplay.value);

  if (store.state.organizations.length > 0) {
    const otherOrgOptions = store.state.organizations.map((data: any) => ({
      label: data.name,
      id: data.id,
      identifier: data.identifier,
      user_email: store.state.userInfo.email,
      ingest_threshold: data.ingest_threshold,
      search_threshold: data.search_threshold,
      subscription_type: data.CustomerBillingObj?.subscription_type || "",
      status: data.status,
      note: data.CustomerBillingObj?.note || "",
    }));

    // Sort the organization options alphabetically by label
    otherOrgOptions.sort((a:any, b:any) => a.label.localeCompare(b.label));

    // Prepend "All" option to the sorted list
    orgOptions.value = [{ label: "All", value: "all" }, ...otherOrgOptions];
  }
  selectedOrg.value = orgOptions.value[0]; // Default to "All"
});


watch(
  () => props.groupUsers,
  async () => {
    hasFetchedOrgUsers.value = false;
    groupUsersMap.value = new Set(props.groupUsers);
    await getchOrgUsers();
    updateUserTable(usersDisplay.value);
    selectedOrg.value = orgOptions.value[0];
  },
  {
    deep: true,
  }
);

const updateUserTable = async (value: string) => {
  usersDisplay.value = value;

  if (!hasFetchedOrgUsers.value && value === "all") {
    await getchOrgUsers();
  }

  if (usersDisplay.value === "all") {
    rows.value = users.value;
  } else {
    rows.value = users.value.filter((user: any) => user.isInGroup);
  }
};

const updateOrganization = () => {
  if (selectedOrg.value.value === "all") {
    // Show all users when "All" is selected
    rows.value =
      usersDisplay.value === "all"
        ? users.value
        : users.value.filter((user) => user.isInGroup);
  } else {
    // Filter users based on selected organization or root role
    rows.value = users.value.filter((user) => {
      const isRootRole = user.role === "root"; // Check if user has "root" role
      const matchesOrg = user.org.includes(selectedOrg.value.label); // Match organization name
      return isRootRole || matchesOrg; // Include if "root" or matches org
    });
  }
};

const getchOrgUsers = async () => {
  // fetch group users
  hasFetchedOrgUsers.value = true;
  return new Promise(async (resolve) => {
    const data: any = await usersState.getOrgUsers(
      store.state.selectedOrganization.identifier , { list_all: true }
    );

    usersState.users = cloneDeep(
      data.map((user: any, index: number) => {  
        return {
          email: user.email,
          "#": index + 1,
          isInGroup: groupUsersMap.value.has(user.email),
          org: user.orgs?.length > 0 ? user.orgs.map((org:{ org_name: string }) => org.org_name).join(", ") : "", // Set default "N/A" for users with no orgs
          role:user.role
        };
      })
    );

    users.value = cloneDeep(usersState.users).map(
      (user: any, index: number) => {
        return {
          "#": index + 1,
          email: user.email,
          isInGroup: groupUsersMap.value.has(user.email),
          org: user.org,
          role:user.role
        };
      }
    );
    resolve(true);
  });
};

const toggleUserSelection = (user: any) => {
  if (user.isInGroup && !groupUsersMap.value.has(user.email)) {
    props.addedUsers.add(user.email);
  } else if (!user.isInGroup && groupUsersMap.value.has(user.email)) {
    props.removedUsers.add(user.email);
  }

  if (!user.isInGroup && props.addedUsers.has(user.email)) {
    props.addedUsers.delete(user.email);
  }

  if (!user.isInGroup && props.addedUsers.has(user.email)) {
    props.removedUsers.delete(user.email);
  }
};

const filterUsers = (rows: any[], term: string) => {
  var filtered = [];
  for (var i = 0; i < rows.length; i++) {
    var user = rows[i];
    if (user.email.toLowerCase().indexOf(term.toLowerCase()) > -1) {
      filtered.push(user);
    }
  }
  return filtered;
};
</script>

<style scoped></style>
