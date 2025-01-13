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
  <div class="relative-position full-height" data-test="edit-role-page">
    <!-- TODO OK : Add button to delete role in toolbar -->
    <div
      data-test="edit-role-title"
      style="font-size: 18px"
      class="q-py-sm q-px-md"
    >
      {{ editingRole }}
    </div>

    <div class="full-width bg-grey-4" style="height: 1px" />

    <AppTabs
      data-test="edit-role-tabs"
      :tabs="tabs"
      :active-tab="activeTab"
      @update:active-tab="updateActiveTab"
    />

    <q-separator />

    <template v-if="isFetchingInitialRoles">
      <div data-test="edit-role-page-loading-spinner" style="margin-top: 64px">
        <q-spinner-hourglass
          color="primary"
          size="40px"
          style="margin: 0 auto; display: block"
        />
        <div class="text-center full-width">
          Hold on tight, we're fetching your role details...
        </div>
      </div>
    </template>
    <template v-else>
      <div style="min-height: calc(100% - (39px + 55px + 46px))">
        <GroupUsers
          data-test="edit-role-users-section"
          v-show="activeTab === 'users'"
          :groupUsers="roleUsers"
          :activeTab="activeTab"
          :added-users="addedUsers"
          :removed-users="removedUsers"
        />
        <GroupServiceAccounts
          data-test="edit-role-users-section"
          v-show="activeTab === 'serviceAccounts'"
          :groupUsers="roleUsers"
          :activeTab="activeTab"
          :added-users="addedUsers"
          :removed-users="removedUsers"
        />

        <div
          v-show="activeTab === 'permissions'"
          data-test="edit-role-permissions-section"
        >
          <div
            class="flex justify-between items-center"
            :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
            :style="{
              'box-shadow':
                store.state.theme === 'dark'
                  ? 'rgb(45 45 45) 0px 4px 7px 0px'
                  : 'rgb(240 240 240) 0px 4px 7px 0px',
              height: '56px',
            }"
          >
            <div
              v-show="permissionsUiType === 'table'"
              data-test="edit-role-permissions-filters"
              class="o2-input flex items-start q-px-md q-py-sm justify-start"
              style="position: sticky; top: 0px; z-index: 2"
            >
              <div
                data-test="edit-role-permissions-show-toggle"
                class="flex items-center q-pt-xs q-mr-md"
              >
                <span
                  data-test="edit-role-permissions-show-text"
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
                  <template
                    v-for="visual in permissionDisplayOptions"
                    :key="visual.value"
                  >
                    <q-btn
                      :data-test="`edit-role-permissions-show-${visual.value}-btn`"
                      :color="
                        visual.value === filter.permissions ? 'primary' : ''
                      "
                      :flat="visual.value === filter.permissions ? false : true"
                      dense
                      no-caps
                      size="11px"
                      class="q-px-md visual-selection-btn"
                      @click="updateTableData(visual.value)"
                    >
                      {{ visual.label }}</q-btn
                    >
                  </template>
                </div>
              </div>
              <div data-test="edit-role-permissions-search-input">
                <q-input
                  v-model="filter.value"
                  borderless
                  :debounce="500"
                  filled
                  dense
                  class="q-mb-xs no-border q-mr-md"
                  :placeholder="t('common.search')"
                  style="width: 300px"
                  @update:model-value="onResourceChange"
                >
                  <template #prepend>
                    <q-icon name="search" class="cursor-pointer" />
                  </template>
                </q-input>
              </div>
              <div data-test="edit-role-permissions-resource-select-input">
                <q-select
                  v-model="filter.resource"
                  :options="filteredResources"
                  color="input-border"
                  bg-color="input-bg"
                  class="q-mr-sm"
                  placeholder="Select Resource"
                  map-options
                  use-input
                  emit-value
                  fill-input
                  hide-selected
                  outlined
                  filled
                  dense
                  clearable
                  style="width: 200px"
                  @filter="filterResourceOptions"
                  @update:model-value="onResourceChange"
                />
              </div>
            </div>
            <div></div>
            <div
              data-test="edit-role-permissions-ui-type-toggle"
              class="q-mr-md"
              style="
                border: 1px solid #d7d7d7;
                width: fit-content;
                border-radius: 2px;
              "
            >
              <template
                v-for="visual in permissionUiOptions"
                :key="visual.value"
              >
                <q-btn
                  :data-test="`edit-role-permissions-show-${visual.value}-btn`"
                  :color="visual.value === permissionsUiType ? 'primary' : ''"
                  :flat="visual.value === permissionsUiType ? false : true"
                  dense
                  no-caps
                  size="11px"
                  class="q-px-md visual-selection-btn"
                  @click="updatePermissionsUi(visual.value)"
                >
                  {{ visual.label }}</q-btn
                >
              </template>
            </div>
          </div>

          <div
            data-test="edit-role-permissions-table-section"
            class="q-px-md q-my-sm"
          >
            <div v-show="permissionsUiType === 'table'">
              <permissions-table
                ref="permissionTableRef"
                :rows="permissionsState.permissions"
                :customFilteredPermissions="filteredPermissions"
                :filter="filter"
                :visibleResourceCount="countOfVisibleResources"
                :selected-permissions-hash="selectedPermissionsHash"
                @updated:permission="handlePermissionChange"
                @expand:row="expandPermission"
              />
            </div>
            <div v-show="permissionsUiType === 'json'">
              <div class="flex items-center justify-between">
                <div class="q-mb-md text-bold">
                  {{ selectedPermissionsHash.size }} Permission
                </div>
                <div
                  class="flex items-center cursor-pointer"
                  :title="t('menu.help')"
                  @click="toggleHelpSection"
                >
                  <q-icon name="help" size="17px" />
                  <span class="q-ml-xs"> Help </span>
                </div>
              </div>
              <div class="flex no-wrap">
                <div
                  :style="
                    isHelpOpen
                      ? { width: 'calc(100% - 350px)' }
                      : { width: '100%' }
                  "
                >
                  <query-editor
                    data-test="logs-vrl-function-editor"
                    editor-id="add-function-editor"
                    class="monaco-editor q-mt-sm"
                    language="json"
                    ref="permissionJsonEditorRef"
                    v-model:query="permissionsJsonValue"
                    style="height: calc(100vh - 328px)"
                  />
                </div>
                <div v-if="isHelpOpen" style="width: 350px" class="q-pa-sm">
                  <div class="flex justify-between items-center q-px-sm">
                    <div style="font-size: 16px">Quick Reference</div>
                    <q-icon
                      class="cursor-pointer"
                      name="close"
                      size="14px"
                      :title="t('common.close')"
                      @click="toggleHelpSection"
                    />
                  </div>
                  <q-separator class="q-mt-sm q-mb-md" />
                  <div class="q-mt-sm q-px-sm">
                    <div>
                      Configure access with JSON objects specifying "object"
                      (resource) and "permission" (access level).
                    </div>
                    <pre style="font-size: 12px">
{
  "object": "MainResource:ChildResource",
  "permission": "AccessType"
}</pre
                    >
                    <div>
                      <span class="text-bold">Child Resource:</span> <br />
                      Specific instance or
                      <span class="text-bold">organizationID</span> for all
                      instances within a main resource.
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
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
          data-test="edit-role-cancel-btn"
          class="text-bold"
          :label="t('alerts.cancel')"
          text-color="light-text"
          padding="sm md"
          no-caps
          @click="cancelPermissionsUpdate"
        />
        <q-btn
          data-test="edit-role-save-btn"
          :label="t('alerts.save')"
          class="text-bold no-border q-ml-md"
          color="secondary"
          padding="sm xl"
          no-caps
          @click="saveRole"
        />
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { cloneDeep } from "lodash-es";
import { defineAsyncComponent, ref, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import type { Resource, Entity, Permission } from "@/ts/interfaces";
import PermissionsTable from "@/components/iam/roles/PermissionsTable.vue";
import { useStore } from "vuex";
import usePermissions from "@/composables/iam/usePermissions";
import { useRouter } from "vue-router";
import { onBeforeMount } from "vue";
import {
  updateRole,
  getResources,
  getResourcePermission,
  getRoleUsers,
} from "@/services/iam";
import { useQuasar } from "quasar";
import type { AxiosPromise } from "axios";
import streamService from "@/services/stream";
import pipelineService from "@/services/pipelines";
import alertService from "@/services/alerts";
import reportService from "@/services/reports";
import templateService from "@/services/alert_templates";
import destinationService from "@/services/alert_destination";
import jsTransformService from "@/services/jstransform";
import organizationsService from "@/services/organizations";
import savedviewsService from "@/services/saved_views";
import dashboardService from "@/services/dashboards";
import serviceAccountService from "@/services/service_accounts";
import useStreams from "@/composables/useStreams";
import { getGroups, getRoles } from "@/services/iam";
import AppTabs from "@/components/common/AppTabs.vue";
import GroupUsers from "../groups/GroupUsers.vue";
import { nextTick } from "vue";
import GroupServiceAccounts from "../groups/GroupServiceAccounts.vue";
import cipherKeysService from "@/services/cipher_keys";

const QueryEditor = defineAsyncComponent(
  () => import("@/components/QueryEditor.vue")
);

onBeforeMount(() => {
  permissionsState.permissions = [];
  editingRole.value = router.currentRoute.value.params.role_name as string;
  getRoleDetails();
});

const permissionTableRef: any = ref(null);

const { t } = useI18n();

const { permissionsState } = usePermissions();

const router = useRouter();

const q = useQuasar();

const store = useStore();

const isHelpOpen = ref(false);

const permissionJsonEditorRef: any = ref(null);

const activeTab = ref("permissions");

const editingRole = ref("");

const permissions: Ref<Permission[]> = ref([]);

const permissionsHash = ref(new Set()); // Saved permissions of role

const selectedPermissionsHash = ref(new Set()); // Saved + new added permission hash

const addedPermissions: any = ref({});

const resourceMapper: Ref<{ [key: string]: Resource }> = ref({});

const removedPermissions: any = ref({});

const countOfVisibleResources = ref(0);

const isFetchingInitialRoles = ref(false);

const filteredPermissions: Ref<{ [key: string]: Entity[] }> = ref({});

const heavyResourceEntities: Ref<{ [key: string]: Entity[] }> = ref({});
const permissionsJsonValue = ref("");

const addedUsers = ref(new Set());
const removedUsers = ref(new Set());

const roleUsers: Ref<string[]> = ref([]);

const permissionsUiType = ref("table");

const { getStreams } = useStreams();

const tabs = [
  {
    value: "permissions",
    label: "Permissions",
  },
  {
    value: "users",
    label: "Users",
  },
  {
    value: "serviceAccounts",
    label: "Service Accounts",
  },
];

const permissionDisplayOptions = [
  {
    label: "All",
    value: "all",
  },
  {
    label: "Selected",
    value: "selected",
  },
];

const permissionUiOptions = [
  {
    label: "Table",
    value: "table",
  },
  {
    label: "JSON",
    value: "json",
  },
];

const filter = ref({
  resource: "",
  value: "",
  permissions: "selected",
  method: filterResources,
});

const filteredResources: Ref<any[]> = ref([]);

const resourceOptions: Ref<any[]> = ref([]);

const updateActiveTab = (tab: string) => {
  if (!tab) return;
  activeTab.value = tab;
};

const getRoleDetails = () => {
  isFetchingInitialRoles.value = true;

  getResources(store.state.selectedOrganization.identifier)
    .then(async (res) => {
      permissionsState.resources = res.data
        .sort((a: any, b: any) => a.order - b.order)
        .filter((resource: any) => resource.visible);

      setDefaultPermissions();

      filteredResources.value = permissionsState.resources
        .map((r) => {
          return {
            label: r.display_name,
            value: r.key,
          };
        })
        .filter((r) => r.value !== "dashboard");

      resourceOptions.value = cloneDeep(filteredResources.value);

      await getResourcePermissions();
      await getUsers();
      savePermissionHash();
      await updateRolePermissions(permissions.value);
      isFetchingInitialRoles.value = false;

      updateTableData();
    })
    .catch(() => {
      isFetchingInitialRoles.value = false;
    });
};

const getUsers = () => {
  return new Promise((resolve, reject) => {
    getRoleUsers(editingRole.value, store.state.selectedOrganization.identifier)
      .then((res) => {
        roleUsers.value = res.data;
        resolve(true);
      })
      .catch((err) => {
        reject(err);
      });
  });
};

const getResourceByName = (
  resources: Resource[],
  resourceName: string,
  level: number = 0
): Resource | null | undefined => {
  for (let i = 0; i < resources.length; i++) {
    if (resources[i].resourceName === resourceName) return resources[i];
    else if (resources[i].childs.length) {
      const isFound = getResourceByName(
        resources[i].childs,
        resourceName,
        level + 1
      );
      if (isFound) return isFound;
    }
  }

  if (!level) return null;
};

const setDefaultPermissions = () => {
  //TODO: Need to make it recursive to support multi level nested resources
  permissionsState.resources.forEach((resource: any) => {
    const resourcePermission = getDefaultResource();
    resourcePermission.name = resource.key;
    resourcePermission.resourceName = resource.key;
    resourcePermission.display_name = resource.display_name;
    resourcePermission.top_level = resource.top_level;

    if (resource.has_entities) resourcePermission.has_entities = true;

    resourcePermission.parent = resource.parent;

    resourceMapper.value[resourcePermission.name] = resourcePermission;

    if (resource.parent) {
      const parentResource = getResourceByName(
        permissionsState.permissions,
        resource.parent
      );
      if (parentResource) {
        parentResource.childs.push(resourcePermission as Resource);
        return;
      }
    }

    modifyResourcePermissions(resourcePermission);

    permissionsState.permissions.push(resourcePermission as Resource);
  });

  permissionsState.permissions = permissionsState.permissions.filter(
    (resource) => !resource.parent
  );
};

const modifyResourcePermissions = (resource: Resource) => {
  if (resource.resourceName === "settings") {
    resource.permission.AllowList.show = false;
    resource.permission.AllowDelete.show = false;
    resource.permission.AllowPost.show = false;
  }
};

const getResourcePermissions = () => {
  const promises: AxiosPromise<any>[] = [];
  permissionsState.resources.forEach((resource) => {
    promises.push(
      getResourcePermission({
        role_name: editingRole.value,
        org_identifier: store.state.selectedOrganization.identifier,
        resource: resource.key,
      })
    );
  });

  return new Promise((resolve, reject) => {
    Promise.all(promises)
      .then((res) => {
        res.forEach((resourcePermissions: { data: Permission[] }) => {
          permissions.value.push(...resourcePermissions.data);
        });
        promises.length = 0;
        resolve(true);
      })
      .catch((err) => {
        reject(err);
      });
  });
};

const getDefaultResource = (): Resource => {
  return {
    name: "",
    permission: {
      AllowAll: {
        show: true,
        value: false,
      },
      AllowList: {
        show: true,
        value: false,
      },
      AllowGet: {
        show: true,
        value: false,
      },
      AllowDelete: {
        show: true,
        value: false,
      },
      AllowPost: {
        show: true,
        value: false,
      },
      AllowPut: {
        show: true,
        value: false,
      },
    },
    display_name: "",
    parent: "",
    childs: [],
    type: "Type",
    resourceName: "",
    entities: [],
    has_entities: false,
    is_loading: false,
    top_level: true,
  };
};

const getOrgId = () => {
  return store.state.selectedOrganization.identifier;
};

const savePermissionHash = () => {
  permissions.value.forEach((permission: Permission) => {
    const { resource, entity } = decodePermission(permission.object);

    // Creating permissions hash to check if permission is selected at the time of save as we need to send only added and removed permissions
    const permissionHash = `${resource}:${entity}:${permission.permission}`;
    permissionsHash.value.add(permissionHash);
    selectedPermissionsHash.value.add(permissionHash);
  });
};

const updateRolePermissions = async (permissions: Permission[]) => {
  let resourceMapper: { [key: string]: Resource } = {};
  for (let i = 0; i < permissions.length; i++) {
    try {
      let {
        resource,
        entity,
      }: {
        resource: string;
        entity: string;
      } = decodePermission(permissions[i].object);

      if (!resourceMapper[resource]) {
        resourceMapper[resource] = getResourceByName(
          permissionsState.permissions,
          resource
        ) as Resource;
      }

      // Added it intentionally, as to get parent resource for dashboard, before getting dashboard permissions
      if (!resourceMapper[resource] && resource === "dashboard") {
        if (!resourceMapper["dfolder"])
          resourceMapper["dfolder"] = getResourceByName(
            permissionsState.permissions,
            "dfolder"
          ) as Resource;

        await getResourceEntities(resourceMapper["dfolder"]);

        if (!resourceMapper[resource]) {
          resourceMapper[resource] = getResourceByName(
            permissionsState.permissions,
            resource
          ) as Resource;
        }
      }

      if (!resourceMapper[resource]) continue;

      if (
        resourceMapper[resource].parent &&
        !resourceMapper[resourceMapper[resource].parent]
      ) {
        resourceMapper[resourceMapper[resource].parent] = getResourceByName(
          permissionsState.permissions,
          resourceMapper[resource].parent
        ) as Resource;
      }

      if (entity === "_all_" + getOrgId()) {
        resourceMapper[resource].permission[permissions[i].permission].value =
          true;

        continue;
      }

      if (resourceMapper[resource].parent)
        await getResourceEntities(
          resourceMapper[resourceMapper[resource].parent]
        );

      // This is just to handle dashboard permissions, need to fix this
      if (resource === "dashboard") {
        const [folderId, dashboardId] = entity.split("/");

        const dashResource = resourceMapper["dfolder"].entities.find(
          (e: Entity) => e.name === folderId
        );
        await getResourceEntities(dashResource as Entity);
      } else if (
        resource === "logs" ||
        resource === "metrics" ||
        resource === "traces" ||
        resource === "index"
      ) {
        const streamResource = resourceMapper["stream"].entities.find(
          (e: Entity) => e.name === resource
        );
        await getResourceEntities(streamResource as Entity);
      } else {
        await getResourceEntities(resourceMapper[resource]);
      }
    } catch (err) {
      console.log(err);
    }
  }

  resourceMapper = {};

  return new Promise((resolve) => {
    resolve(true);
  });
};

const decodePermission = (permission: string) => {
  const [resource, entity] = permission.split(":");
  return { resource, entity };
};

const cancelPermissionsUpdate = () => {
  router.push({ name: "roles" });
};

const handlePermissionChange = (row: any, permission: string) => {
  let entity = "";
  let resourceName = row.resourceName;

  // As there can be conflict in resource name and org id, as they can be same.
  // So we are adding _all_ prefix to org id to differentiate between org id and resource name

  if (row.type === "Type")
    entity = "_all_" + store.state.selectedOrganization.identifier;
  else entity = row.name;

  if (row.type === "Resource" && row.top_level) {
    resourceName = row.name;
    entity = "_all_" + store.state.selectedOrganization.identifier;
  }

  const permissionHash = `${resourceName}:${entity}:${permission}`;

  // Add permission to addedPermissions if not present
  updatePermissionMappings(permissionHash);
};

const updatePermissionMappings = (permissionHash: string) => {
  const permissionSplit = permissionHash.split(":");
  const object = permissionSplit[0] + ":" + permissionSplit[1];
  const permission = permissionSplit[2];

  if (
    !addedPermissions.value[permissionHash] &&
    !permissionsHash.value.has(permissionHash)
  ) {
    selectedPermissionsHash.value.add(permissionHash);
    addedPermissions.value[permissionHash] = {
      object,
      permission: permission,
    };
    return;
  }

  // Remove permission from removedPermissions if present
  if (
    removedPermissions.value[permissionHash] &&
    permissionsHash.value.has(permissionHash) &&
    !selectedPermissionsHash.value.has(permissionHash)
  ) {
    delete removedPermissions.value[permissionHash];
    selectedPermissionsHash.value.add(permissionHash);
    return;
  }

  // Remove permission from removedPermissions if present
  if (removedPermissions.value[permissionHash]) {
    delete removedPermissions.value[permissionHash];
    return;
  }

  // Remove permission from addedPermissions if present
  if (permissionsHash.value.has(permissionHash)) {
    selectedPermissionsHash.value.delete(permissionHash);
    removedPermissions.value[permissionHash] = {
      object,
      permission: permission,
    };

    return;
  }

  // Remove permission from addedPermissions if present
  if (addedPermissions.value[permissionHash]) {
    selectedPermissionsHash.value.delete(permissionHash);
    delete addedPermissions.value[permissionHash];
    return;
  }
};

const updateTableData = async (value: string = filter.value.permissions) => {
  filter.value.permissions = value;

  await nextTick();

  setTimeout(() => {
    updatePermissionVisibility(permissionsState.permissions);
    countVisibleResources(permissionsState.permissions);
  }, 0);
};

const updatePermissionsUi = async (value: string) => {
  permissionsUiType.value = value;
  if (value === "json") {
    const permissions: {
      object: string;
      permission: string;
    }[] = [];
    selectedPermissionsHash.value.forEach((permission: any) => {
      const [resource, entity, _permission] = permission.split(":");
      permissions.push({
        object: `${resource}:${entity}`,
        permission: _permission,
      });
    });

    permissionsJsonValue.value = JSON.stringify(permissions);
    permissionJsonEditorRef.value.setValue(permissionsJsonValue.value);
    await nextTick();
    permissionJsonEditorRef.value.formatDocument();
  } else if (value === "table") {
    updateJsonInTable();
  }
};

const updateJsonInTable = () => {
  const permissions = JSON.parse(permissionsJsonValue.value);

  const permissionsHash = new Set(
    permissions.map((p: any) => p.object + ":" + p.permission)
  );
  let hash = "";
  let permission;
  let resource = "";
  let entity = "";
  let resourceDetails: Entity | Resource;

  // Update added permissions
  updateRolePermissions(permissions);

  permissions.forEach((permission: any) => {
    [resource, entity] = permission.object.split(":");
    hash = permission.object + ":" + permission.permission;
    if (!selectedPermissionsHash.value.has(hash)) {
      updatePermissionMappings(hash);

      resourceDetails = resourceMapper.value[resource];

      if (resource === "dashboard") {
        const [folderId] = entity.split("/");

        resourceDetails = resourceMapper.value["dfolder"].entities.find(
          (e: Entity) => e.name === folderId
        ) as Entity;
      } else if (entity === "_all_" + getOrgId()) {
        resourceDetails.permission[permission.permission as "AllowAll"].value =
          selectedPermissionsHash.value.has(
            getPermissionHash(resource, permission.permission, entity)
          );
      } else if (
        resource === "logs" ||
        resource === "metrics" ||
        resource === "traces" ||
        resource === "index"
      ) {
        resourceDetails = resourceMapper.value["stream"].entities.find(
          (e: Entity) => e.name === resource
        ) as Entity;
      }

      updateEntityPermission(
        resourceDetails,
        resource,
        entity,
        permission.permission
      );
    }
  });

  // Update removed permissions
  selectedPermissionsHash.value.forEach(async (permissionHash: any) => {
    permission = permissionHash.split(":");
    resource = permission[0];
    entity = permission[1];
    permission = {
      object: permission[0] + ":" + permission[1],
      permission: permission[2] as "AllowAll",
    };

    if (!permissionsHash.has(permissionHash)) {
      updatePermissionMappings(permissionHash);

      resourceDetails = resourceMapper.value[resource];

      if (resource === "dashboard") {
        const [folderId, dashboardId] = entity.split("/");

        resourceDetails = resourceMapper.value["dfolder"].entities.find(
          (e: Entity) => e.name === folderId
        ) as Entity;
      } else if (entity === "_all_" + getOrgId()) {
        resourceDetails.permission[permission.permission as "AllowAll"].value =
          selectedPermissionsHash.value.has(
            getPermissionHash(resource, permission.permission, entity)
          );
      } else if (
        resource === "logs" ||
        resource === "metrics" ||
        resource === "traces" ||
        resource === "index"
      ) {
        resourceDetails = resourceMapper.value["stream"].entities.find(
          (e: Entity) => e.name === resource
        ) as Entity;
      }

      updateEntityPermission(
        resourceDetails,
        resource,
        entity,
        permission.permission
      );
    }
  });
};

const updateExpandedResources = (resources: (Resource | Entity)[]) => {
  resources.forEach(async (resource) => {
    // Check if the current item is an object and has the 'expand' key
    if (
      typeof resource === "object" &&
      resource.expand &&
      resource.has_entities
    ) {
      resource.is_loading = true;
      await getResourceEntities(resource);
      resource.is_loading;
      // Perform additional actions as needed
    }

    // If the item itself contains a nested array, call the function recursively
    if (Array.isArray(resource.entities)) {
      updateExpandedResources(resource.entities);
    }
  });
};

const countVisibleResources = (permissions: (Resource | Entity)[]): number => {
  let count = 0;

  permissions.forEach((permission: Entity | Resource) => {
    if (permission.show) {
      count += 1;
    }

    // Recursively count in nested entities
    if (permission.entities?.length) {
      count += countVisibleResources(permission.entities);
    }
  });

  countOfVisibleResources.value = count;
  return count;
};

const updatePermissionVisibility = (
  permissions: (Resource | Entity)[],
  forceShow: boolean = false,
  level: number = 0,
  relations: string[] = []
): void => {
  permissions.forEach((permission: Entity | Resource) => {
    // Check if any permission value is true

    const parentRelations = [...relations];

    if (permission.type === "Type")
      parentRelations.push(permission.resourceName);

    const showResource = Object.values(permission.permission).some(
      (permDetail) => permDetail.value
    );

    let isResourceFiltered = true;

    if (filter.value.resource)
      isResourceFiltered = parentRelations.includes(filter.value.resource);

    if (filter.value.value) {
      isResourceFiltered =
        isResourceFiltered &&
        (permission.display_name || permission.name)
          .toLowerCase()
          .includes(filter.value.value.toLowerCase());
    }

    permission.show =
      filter.value.permissions === "all"
        ? isResourceFiltered
        : showResource && isResourceFiltered;

    if (forceShow) permission.show = true;

    // Recursively update the show property for entities
    if (!permission.entities?.length) return;

    if (
      permission.name === "logs" ||
      permission.name === "metrics" ||
      permission.name === "traces" ||
      permission.name === "index"
    ) {
      updatePermissionVisibility(
        heavyResourceEntities.value[permission.name] || [],
        permission.show,
        level + 1,
        parentRelations
      );
    } else {
      if (permission.entities?.length)
        updatePermissionVisibility(
          permission.entities || [],
          permission.show,
          level + 1,
          parentRelations
        );
    }

    let filteredEntities: Entity[] = [];

    if (
      permission.name === "logs" ||
      permission.name === "metrics" ||
      permission.name === "traces" ||
      permission.name === "index"
    ) {
      filteredEntities =
        heavyResourceEntities.value[permission.name]?.filter(
          (entity: any) => entity.show
        ) || [];
    } else {
      filteredEntities =
        permission.entities?.filter((entity) => entity.show) || [];
    }

    // Update the permission object to add `show` property

    // If we need to show child by default show parent

    if (
      permission.show &&
      (permission.name === "logs" ||
        permission.name === "metrics" ||
        permission.name === "traces" ||
        permission.name === "index")
    ) {
      permission.entities =
        filter.value.permissions === "all"
          ? [...filteredEntities.slice(0, 50)]
          : [...filteredEntities];
    }

    if (filteredEntities?.length) {
      permission.show = true;
    }

    filteredEntities.length = 0;
  });
};

// const updateFilteredPermissions = (
//   permissions: (Resource | Entity)[]
// ): void => {
//   permissions.forEach((permission: Entity | Resource) => {
//     // Check if any permission value is true
//     const showResource = Object.values(permission.permission).some(
//       (permDetail) => permDetail.value
//     );

//     // Recursively update the show property for entities
//     if (permission.entities?.length) {
//       updatePermissionVisibility(permission.entities);
//     }

//     const filteredEntities = permission.entities?.filter(
//       (entity) => entity.show
//     );

//     // Update the permission object to add `show` property

//     let isResourceFiltered = true;
//     if (filter.value.resource)
//       isResourceFiltered = filter.value.resource === permission.resourceName;

//     if (filter.value.value) {
//       isResourceFiltered =
//         isResourceFiltered &&
//         (permission.display_name || permission.name)
//           .toLowerCase()
//           .includes(filter.value.value.toLowerCase());
//     }

//     // If we need to show child by default show parent
//     if (filteredEntities) {
//       permission.entities = [...filteredEntities];
//     } else {
//       permission.show =
//         filter.value.permissions === "all"
//           ? isResourceFiltered
//           : showResource && isResourceFiltered;
//     }
//   });
// };

const filterRowsByResourceName = (
  rows: (Resource | Entity)[],
  resourceName: string
) => {
  return rows.reduce(
    (filteredRows: (Resource | Entity)[], row: Resource | Entity) => {
      // Check if the current row matches the filter
      if (row.resourceName === resourceName) {
        // If the row has nested rows, filter those as well
        if (row.entities && row.entities.length) {
          row.entities = filterRowsByResourceName(
            row.entities,
            resourceName
          ) as Entity[];
        }
        // Add the row to the filtered list
        filteredRows.push(row);
      } else if (row.entities && row.entities.length) {
        // Even if the current row doesn't match, there might be nested rows that do
        const filteredEntities = filterRowsByResourceName(
          row.entities,
          resourceName
        );
        // Only add the row if it has matching nested rows
        if (filteredEntities.length) {
          // Optionally, you might want to clone the row here to avoid mutating the original
          const newRow = { ...row, entities: filteredEntities };
          filteredRows.push(newRow as Resource);
        }
      }
      return filteredRows;
    },
    []
  );
};

const onResourceChange = async () => {
  updatePermissionVisibility(permissionsState.permissions);
  countVisibleResources(permissionsState.permissions);
};

function filterResources(rows: any, terms: any) {
  var filtered = [];
  terms = terms.toLowerCase();
  for (var i = 0; i < rows.length; i++) {
    let isAdded = false;
    if (rows[i]["display_name"].toLowerCase().includes(terms)) {
      filtered.push(rows[i]);
      isAdded = true;
      continue;
    }
    for (var j = 0; j < rows[i].entities.length; j++) {
      if (
        !isAdded &&
        rows[i].entities[j]["display_name"].toLowerCase().includes(terms)
      ) {
        filtered.push(rows[i]);
        break;
      }
    }
  }
  return filtered;
}

const expandPermission = async (resource: any) => {
  const expand = !resource.expand;

  resource.expand = expand;
  if (expand) {
    try {
      await getResourceEntities(resource);
    } catch (err) {
      console.log(err);
    }
  }
};

const getPermissionHash = (
  resourceName: string,
  permission: string,
  entity?: string
) => {
  if (!entity) entity = "_all_" + store.state.selectedOrganization.identifier;

  return `${resourceName}:${entity}:${permission}`;
};

/**
 *
 * @param resource
 * @param typeOf - Type to assign the new entities that we get from the server
 */
const getResourceEntities = (resource: Resource | Entity) => {
  const listEntitiesFnMap: {
    [key: string]: (resource: Resource | Entity) => Promise<any>;
  } = {
    stream: getStreamsTypes,
    stream_type: getStreamsTypes,
    logs: getLogs,
    metrics: getMetrics,
    traces: getTraces,
    index: getIndexStreams,
    alert: getAlerts,
    template: getTemplates,
    destination: getDestinations,
    pipeline: getPipelines,
    enrichment_table: getEnrichmentTables,
    function: getFunctions,
    org: getOrgs,
    savedviews: getSavedViews,
    group: _getGroups,
    role: _getRoles,
    dfolder: getFolders,
    dashboard: getDashboards,
    metadata: getMetadataStreams,
    report: getReports,
    service_accounts: getServiceAccounts,
    cipher_keys: getCipherKeys,
  };

  return new Promise(async (resolve, reject) => {
    if (!resource.entities?.length) {
      resource.is_loading = true;
      if (resource.childName) {
        await listEntitiesFnMap[resource.childName](resource);
      } else {
        await listEntitiesFnMap[resource.resourceName](resource);
      }

      // unncecessaryly we are updating the all resource entities, fix to update the current resource
      updatePermissionVisibility(permissionsState.permissions);
      resource.is_loading = false;
    }

    resolve(true);
  });
};

const getEnrichmentTables = async () => {
  const data: any = await getStreams("enrichment_tables", false);

  updateResourceEntities("enrichment_table", ["name"], data.list);

  return new Promise((resolve, reject) => {
    resolve(true);
  });
};

const getDashboards = async (resource: Entity | Resource) => {
  let dashboards: any = await dashboardService.list(
    0,
    10000,
    "name",
    false,
    "",
    store.state.selectedOrganization.identifier,
    resource.name,
    ""
  );

  updateEntityEntities(
    resource,
    ["dashboardId"],
    [
      ...dashboards.data.dashboards.map(
        (dash: any) => Object.values(dash).filter((dash) => dash)[0]
      ),
    ],
    false,
    "title"
  );

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getOrgs = async () => {
  const orgs = await organizationsService.list(0, 100000, "name", false, "");

  updateResourceEntities("org", ["identifier"], [...orgs.data.data]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getSavedViews = async () => {
  const savedViews = await savedviewsService.get(
    store.state.selectedOrganization.identifier
  );
  updateResourceEntities(
    "savedviews",
    ["view_id"],
    [...savedViews.data.views],
    false,
    "view_name"
  );

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getFolders = async () => {
  const folders: any = await dashboardService.list_Folders(
    store.state.selectedOrganization.identifier
  );

  let isDefaultPresent = folders.data.list.find(
    (folder: any) => folder.folderId === "default"
  );

  if (!isDefaultPresent) {
    folders.data.list.unshift({ folderId: "default", name: "default" });
  }

  updateResourceEntities(
    "dfolder",
    ["folderId"],
    [...folders.data.list],
    true,
    "name",
    "dashboard"
  );
  return new Promise((resolve) => {
    resolve(true);
  });
};

const _getGroups = async () => {
  const groups = await getGroups(store.state.selectedOrganization.identifier);
  updateResourceEntities("group", [], [...groups.data]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const _getRoles = async () => {
  const roles = await getRoles(store.state.selectedOrganization.identifier);
  updateResourceEntities("role", [], [...roles.data]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getFunctions = async () => {
  const functions = await jsTransformService.list(
    1,
    100000,
    "name",
    false,
    "",
    store.state.selectedOrganization.identifier
  );

  updateResourceEntities("function", ["name"], [...functions.data.list]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getDestinations = async () => {
  const destinations = await destinationService.list({
    sort_by: "name",
    org_identifier: store.state.selectedOrganization.identifier,
  });

  updateResourceEntities("destination", ["name"], [...destinations.data]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getTemplates = async () => {
  const templates = await templateService.list({
    org_identifier: store.state.selectedOrganization.identifier,
  });

  updateResourceEntities("template", ["name"], [...templates.data]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getPipelines = async () => {
  const pipelines = await pipelineService.getPipelines(
    store.state.selectedOrganization.identifier
  );

  updateResourceEntities("pipeline", ["name"], [...pipelines.data.list]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getAlerts = async () => {
  const alerts = await alertService.list(
    1,
    10000,
    "name",
    false,
    "",
    store.state.selectedOrganization.identifier
  );

  updateResourceEntities("alert", ["name"], [...alerts.data.list]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getLogs = async (resource: Resource | Entity) => {
  const logs: any = await getStreams("logs", false);

  updateEntityEntities(resource, ["name"], logs.list);

  return new Promise((resolve, reject) => {
    resolve(true);
  });
};

const getIndexStreams = async (resource: Resource | Entity) => {
  const indices: any = await getStreams("index", false);

  updateEntityEntities(resource, ["name"], indices.list);

  return new Promise((resolve, reject) => {
    resolve(true);
  });
};

const getMetrics = async (resource: Resource | Entity) => {
  const metrics: any = await getStreams("metrics", false);

  updateEntityEntities(resource, ["name"], metrics.list);

  return new Promise((resolve, reject) => {
    resolve(true);
  });
};

const getTraces = async (resource: Resource | Entity) => {
  const traces: any = await getStreams("traces", false);

  updateEntityEntities(resource, ["name"], traces.list);

  return new Promise((resolve, reject) => {
    resolve(true);
  });
};

const getMetadataStreams = async (resource: Resource | Entity) => {
  const metadata: any = await getStreams("metadata", false);

  updateResourceEntities("metadata", ["name"], metadata.list);

  return new Promise((resolve, reject) => {
    resolve(true);
  });
};

const getStreamsTypes = async () => {
  const streams = [
    { stream_type: "logs", name: "Logs" },
    { stream_type: "traces", name: "Traces" },
    { stream_type: "metrics", name: "Metrics" },
    { stream_type: "index", name: "Indices" },
  ];

  streams.forEach((stream) => {
    updateResourceResource(
      stream.stream_type,
      "stream",
      ["stream_type"],
      [stream],
      true,
      "name"
    );
  });

  return new Promise((resolve, reject) => {
    resolve(true);
  });
};

const getReports = async () => {
  const reports = await reportService.list(
    store.state.selectedOrganization.identifier
  );

  updateResourceEntities("report", ["name"], [...reports.data]);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getServiceAccounts = async () => {
  const accounts = await serviceAccountService.list(
    store.state.selectedOrganization.identifier
  );

  updateResourceEntities("service_accounts", ["email"], accounts.data.data);

  return new Promise((resolve) => {
    resolve(true);
  });
};

const getCipherKeys = async () => {
  const data: any = await cipherKeysService.list(
    store.state.selectedOrganization.identifier,
  );

  updateResourceEntities("cipher_keys", ["name"], [...data.data.keys]);

  return new Promise((resolve, reject) => {
    resolve(true);
  });
};

const updateEntityEntities = (
  entity: Entity | Resource,
  entityNameKeys: string[],
  data: any[],
  hasEntities: boolean = false,
  displayNameKey?: string
) => {
  if (!entity) return;

  const entities: Entity[] = data.map((_entity: any) => {
    let entityName = "";
    if (typeof _entity === "string") entityName = _entity;

    if (typeof _entity === "object") {
      entityName = entityNameKeys.reduce((acc, curr) => {
        return acc ? acc + "/" + (_entity[curr] || curr) : _entity[curr];
      }, "");

      if (entity.childName === "dashboard") {
        entityName = entity["name"] + "/" + _entity["dashboardId"];
      }
    }

    return {
      name: entityName,
      permission: {
        AllowAll: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              entity.childName as string,
              "AllowAll",
              entityName
            )
          ),
          show: true,
        },
        AllowGet: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              entity.childName as string,
              "AllowGet",
              entityName
            )
          ),
          show: true,
        },
        AllowDelete: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              entity.childName as string,
              "AllowDelete",
              entityName
            )
          ),
          show: true,
        },
        AllowPut: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              entity.childName as string,
              "AllowPut",
              entityName
            )
          ),
          show: true,
        },
        AllowList: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              entity.childName as string,
              "AllowList",
              entityName
            )
          ),
          show: hasEntities,
        },
        AllowPost: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              entity.childName as string,
              "AllowPost",
              entityName
            )
          ),
          show: hasEntities,
        },
      },
      entities: [],
      type: "Resource",
      resourceName: entity.childName as string,
      has_entities: hasEntities,
      display_name: displayNameKey ? _entity[displayNameKey] : entityName,
      show: true,
      top_level: false,
    };
  });

  if (
    entity.name === "logs" ||
    entity.name === "metrics" ||
    entity.name === "traces" ||
    entity.name === "index"
  ) {
    heavyResourceEntities.value[entity.name] = [...entities];
    if (entity.entities) entity.entities.push(...entities.slice(0, 50));
  } else {
    if (entity.entities) entity.entities.push(...entities);
  }
};

const updateResourceEntities = (
  resourceName: string,
  entityNameKeys: string[],
  data: any[],
  hasEntities: boolean = false,
  displayNameKey?: string,
  childName?: string
) => {
  const resource: Resource | null | undefined = getResourceByName(
    permissionsState.permissions,
    resourceName
  );

  if (!resource) return;

  data.forEach((_entity: any) => {
    let entityName = "";
    if (typeof _entity === "string") entityName = _entity;

    if (typeof _entity === "object") {
      entityName = entityNameKeys.reduce((acc, curr) => {
        return acc ? acc + "/" + (_entity[curr] || curr) : _entity[curr];
      }, "");
    }

    resource.entities.push({
      name: entityName,
      permission: {
        AllowAll: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(resourceName, "AllowAll", entityName)
          ),
          show: true,
        },
        AllowGet: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(resourceName, "AllowGet", entityName)
          ),
          show: true,
        },
        AllowDelete: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(resourceName, "AllowDelete", entityName)
          ),
          show: true,
        },
        AllowPut: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(resourceName, "AllowPut", entityName)
          ),
          show: true,
        },
        AllowList: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(resourceName, "AllowList", entityName)
          ),
          show: hasEntities,
        },
        AllowPost: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(resourceName, "AllowPost", entityName)
          ),
          show: hasEntities,
        },
      },
      entities: [],
      type: "Resource",
      resourceName: resourceName,
      has_entities: hasEntities,
      display_name: displayNameKey ? _entity[displayNameKey] : entityName,
      show: true,
      childName: childName || "",
      top_level: !!resource.childs.find((child) => child.name === childName)
        ?.top_level,
    });
  });
};

const updateResourceResource = (
  resourceName: string,
  parentResourceName: string,
  entityNameKeys: string[],
  data: any[],
  hasEntities: boolean = false,
  displayNameKey?: string
) => {
  const resource: Resource | null | undefined = getResourceByName(
    permissionsState.permissions,
    parentResourceName
  );

  if (!resource) return;

  data.forEach((_entity: any) => {
    let entityName = "";
    if (typeof _entity === "string") entityName = _entity;

    if (typeof _entity === "object") {
      entityName = entityNameKeys.reduce((acc, curr) => {
        return acc ? acc + "/" + (_entity[curr] || curr) : _entity[curr];
      }, "");
    }

    resource.entities.push({
      name: entityName,
      permission: {
        AllowAll: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              resourceName,
              "AllowAll",
              "_all_" + store.state.selectedOrganization.identifier
            )
          ),
          show: true,
        },
        AllowGet: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              resourceName,
              "AllowGet",
              "_all_" + store.state.selectedOrganization.identifier
            )
          ),
          show: true,
        },
        AllowDelete: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              resourceName,
              "AllowDelete",
              "_all_" + store.state.selectedOrganization.identifier
            )
          ),
          show: true,
        },
        AllowPut: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              resourceName,
              "AllowPut",
              "_all_" + store.state.selectedOrganization.identifier
            )
          ),
          show: true,
        },
        AllowList: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              resourceName,
              "AllowList",
              "_all_" + store.state.selectedOrganization.identifier
            )
          ),
          show: hasEntities,
        },
        AllowPost: {
          value: selectedPermissionsHash.value.has(
            getPermissionHash(
              resourceName,
              "AllowPost",
              "_all_" + store.state.selectedOrganization.identifier
            )
          ),
          show: hasEntities,
        },
      },
      entities: [],
      type: "Type",
      resourceName: resourceName,
      has_entities: hasEntities,
      childName: resourceName,
      display_name: displayNameKey ? _entity[displayNameKey] : entityName,
      show: true,
      top_level: true,
    });
  });
};

const saveRole = () => {
  if (permissionsUiType.value === "json") updateJsonInTable();

  const payload = {
    add: Object.values(addedPermissions.value),
    remove: Object.values(removedPermissions.value),
    add_users: Array.from(addedUsers.value) as string[],
    remove_users: Array.from(removedUsers.value) as string[],
  };

  if (
    !(
      payload.add.length ||
      payload.remove.length ||
      payload.add_users.length ||
      payload.remove_users.length
    )
  ) {
    q.notify({
      type: "info",
      message: `No updates detected.`,
      timeout: 3000,
    });

    return;
  }

  updateRole({
    role_id: editingRole.value,
    org_identifier: store.state.selectedOrganization.identifier,
    payload,
  })
    .then(async (res) => {
      // combine permissionsHash and selectedPermissionsHash

      q.notify({
        type: "positive",
        message: `Updated role successfully!`,
        timeout: 3000,
      });

      // Resetting permissions state on save

      Object.keys(removedPermissions.value).forEach((permission) => {
        if (permissionsHash.value.has(permission))
          permissionsHash.value.delete(permission);

        if (selectedPermissionsHash.value.has(permission))
          selectedPermissionsHash.value.delete(permission);
      });

      permissionsHash.value = new Set([
        ...Array.from(permissionsHash.value),
        ...Array.from(selectedPermissionsHash.value),
      ]);

      selectedPermissionsHash.value = cloneDeep(permissionsHash.value);

      addedPermissions.value = {};

      removedPermissions.value = {};

      roleUsers.value = roleUsers.value.filter(
        (user) => !removedUsers.value.has(user)
      );

      addedUsers.value.forEach((value: any) => {
        roleUsers.value.push(value);
      });

      addedUsers.value = new Set([]);

      removedUsers.value = new Set([]);
    })
    .catch((err) => {
      if(err.response.status != 403){
        q.notify({
          type: "negative",
          message: `Error while updating role!`,
          timeout: 3000,
        });
      }
      console.log(err);
    });
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
      (column: any) => column.label.toLowerCase().indexOf(value) > -1
    );
  });
  return filteredOptions;
};

const filterResourceOptions = (val: string, update: any) => {
  filteredResources.value = filterColumns(
    resourceOptions.value,
    val,
    update
  ) as any[];
};

const updateEntityPermission = (
  resource: Resource | Entity,
  resourceName: string,
  entityName: string,
  permission:
    | "AllowAll"
    | "AllowList"
    | "AllowGet"
    | "AllowDelete"
    | "AllowPost"
) => {
  if (resource?.entities)
    resource.entities.forEach((entity: Entity) => {
      if (entity.name === entityName) {
        entity.permission[permission].value = selectedPermissionsHash.value.has(
          getPermissionHash(resourceName, permission, entityName)
        );
      }
    });
};

const toggleHelpSection = async () => {
  isHelpOpen.value = !isHelpOpen.value;

  await nextTick();
  await nextTick();

  permissionJsonEditorRef.value.resetEditorLayout();
};
</script>
<style scoped></style>
