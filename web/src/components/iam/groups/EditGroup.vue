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
  <div data-test="edit-group-section" class="relative-position full-height">
    <div
      data-test="edit-group-section-title"
      style="font-size: 18px"
      class="q-py-sm q-px-md"
    >
      {{ groupDetails.group_name }}
    </div>

    <div class="full-width bg-grey-4" style="height: 1px" />

    <AppTabs
      data-test="edit-group-tabs"
      :tabs="tabs"
      :active-tab="activeTab"
      @update:active-tab="updateActiveTab"
    />

    <q-separator />
    <div style="min-height: calc(100% - (39px + 55px + 43px))">
      <GroupUsers
        v-show="activeTab === 'users'"
        :groupUsers="groupDetails.users"
        :activeTab="activeTab"
        :added-users="addedUsers"
        :removed-users="removedUsers"
      />

      <GroupRoles
        v-show="activeTab === 'roles'"
        :groupRoles="groupDetails.roles"
        :activeTab="activeTab"
        :added-roles="addedRoles"
        :removed-roles="removedRoles"
      />
      <GroupServiceAccounts
        v-if="config.isCloud == 'false'"
        v-show="activeTab === 'serviceAccounts'"
        :groupUsers="groupDetails.users"
        :activeTab="activeTab"
        :added-users="addedUsers"
        :removed-users="removedUsers"
      />
    </div>
    <div
      class="flex justify-end q-px-md q-py-sm full-width"
      style="position: sticky; bottom: 0px; z-index: 2"
      :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
      :style="{
        'box-shadow':
          store.state.theme === 'dark'
            ? 'rgb(45 45 45) 0px -4px 7px 0px'
            : 'rgb(240 240 240) 0px -4px 7px 0px',
      }"
    >
      <q-btn
        data-test="add-alert-cancel-btn"
        class="text-bold"
        :label="t('alerts.cancel')"
        text-color="light-text"
        padding="sm md"
        no-caps
        @click="cancelEditGroup"
      />
      <q-btn
        data-test="add-alert-submit-btn"
        :label="t('alerts.save')"
        class="text-bold no-border q-ml-md"
        color="secondary"
        padding="sm xl"
        no-caps
        @click="saveGroupChanges"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import GroupRoles from "./GroupRoles.vue";
import GroupUsers from "./GroupUsers.vue";
import AppTabs from "@/components/common/AppTabs.vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { onBeforeMount } from "vue";
import { getGroup, updateGroup } from "@/services/iam";
import { useStore } from "vuex";
import usePermissions from "@/composables/iam/usePermissions";
import { useQuasar } from "quasar";
import GroupServiceAccounts from "./GroupServiceAccounts.vue";
import config from "@/aws-exports";

onBeforeMount(() => {
  getGroupDetails();
});

const { groupsState } = usePermissions();

const activeTab = ref("roles");

const store = useStore();

const router = useRouter();

const { t } = useI18n();

const q = useQuasar();

const groupDetails = ref({
  group_name: "dev",
  roles: [] as string[],
  users: [] as string[],
});

const addedUsers = ref(new Set());
const removedUsers = ref(new Set());

const addedRoles = ref(new Set());
const removedRoles = ref(new Set());

const tabs = [
  {
    value: "roles",
    label: "Roles",
  },
  {
    value: "users",
    label: "Users",
  },
];

if (config.isCloud == "false") {
  tabs.push({
    value: "serviceAccounts",
    label: "Service Accounts",
  });
}

const getGroupDetails = () => {
  const groupName: string = router.currentRoute.value.params
    .group_name as string;

  getGroup(groupName, store.state.selectedOrganization.identifier)
    .then((res) => {
      console.log(res,'res in get group')
      groupDetails.value = {
        ...res.data,
        group_name: res.data.name,
      };
      groupsState.groups[groupName] = groupDetails.value;
    })
    .catch((err) => {
      console.log(err);
    });
};

const updateActiveTab = (tab: string) => {
  if (!tab) return;
  activeTab.value = tab;
};

const saveGroupChanges = () => {
  const payload = {
    add_users: Array.from(addedUsers.value) as string[],
    remove_users: Array.from(removedUsers.value) as string[],
    add_roles: Array.from(addedRoles.value) as string[],
    remove_roles: Array.from(removedRoles.value) as string[],
  };

  if (
    !(
      payload.add_users.length ||
      payload.remove_users.length ||
      payload.add_roles.length ||
      payload.remove_roles.length
    )
  ) {
    q.notify({
      type: "info",
      message: `No updates detected.`,
      timeout: 3000,
    });

    return;
  }

  updateGroup({
    group_name: groupDetails.value.group_name,
    org_identifier: store.state.selectedOrganization.identifier,
    payload,
  })
    .then((res) => {
      q.notify({
        type: "positive",
        message: `Updated group successfully!`,
        timeout: 3000,
      });

      // Reset Roles
      groupDetails.value.roles = groupDetails.value.roles.filter(
        (user) => !removedRoles.value.has(user)
      );

      addedRoles.value.forEach((value: any) => {
        groupDetails.value.roles.push(value as string);
      });

      addedRoles.value = new Set([]);

      removedRoles.value = new Set([]);

      // Reset Users
      groupDetails.value.users = groupDetails.value.users.filter(
        (user) => !removedUsers.value.has(user)
      );

      addedUsers.value.forEach((value: any) => {
        groupDetails.value.users.push(value as string);
      });

      addedUsers.value = new Set([]);

      removedUsers.value = new Set([]);
    })
    .catch((err) => {
      if(err.response.status != 403){
        q.notify({
          type: "negative",
          message: "Error while updating group!",
          timeout: 3000,
        });
      }
    });
};

const cancelEditGroup = () => {
  router.push({ name: "groups" });
};
</script>

<style scoped></style>
