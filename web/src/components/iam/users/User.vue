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
      ref="qTable"
      :rows="usersState.users"
      :columns="columns"
      row-key="id"
      :pagination="pagination"
      :filter="filterQuery"
      :filter-method="filterData"
    >
      <template #no-data>
        <NoData></NoData>
      </template>
      <template #body-cell-role="props">
        <q-td :props="props" v-if="props?.row?.enableChangeRole">
          <q-select
            dense
            borderless
            v-model="props.row.role"
            :options="options"
            emit-value
            map-options
            style="width: 70px"
            @update:model-value="updateUserRole(props.row)"
          />
        </q-td>
        <q-td :props="props" v-else>
          {{ props.row.role }}
        </q-td>
      </template>
      <template #body-cell-actions="props">
        <q-td :props="props" side>
          <q-btn
            v-if="props.row.enableDelete"
            :icon="outlinedDelete"
            :title="t('user.delete')"
            class="q-ml-xs"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="confirmDeleteAction(props)"
            style="cursor: pointer !important"
          />
          <q-btn
            v-if="props.row.enableEdit"
            icon="edit"
            :title="t('user.update')"
            class="q-ml-xs"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="addRoutePush(props)"
            style="cursor: pointer !important"
          />
        </q-td>
      </template>
      <template #top="scope">
        <div
          class="q-table__title full-width q-mb-md"
          data-test="user-title-text"
        >
          {{ t("user.header") }}
        </div>
        <div class="full-width row q-mb-xs items-start">
          <q-input
            v-model="filterQuery"
            filled
            dense
            class="col-6 q-pr-sm"
            :placeholder="t('user.search')"
          >
            <template #prepend>
              <q-icon name="search" />
            </template>
          </q-input>

          <div class="col-6">
            <q-btn
              v-if="showAddUserBtn"
              class="q-ml-md q-mb-xs text-bold no-border"
              style="float: right; cursor: pointer !important"
              padding="sm lg"
              color="secondary"
              no-caps
              dense
              :label="t(`user.add`)"
              @click="addRoutePush({})"
            />
          </div>
        </div>
        <QTablePagination
          :scope="scope"
          :pageTitle="t('user.header')"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          position="top"
          @update:changeRecordPerPage="changePagination"
        />
      </template>

      <template #bottom="scope">
        <QTablePagination
          :scope="scope"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          position="bottom"
          @update:changeRecordPerPage="changePagination"
        />
      </template>
    </q-table>
    <q-dialog
      v-model="showUpdateUserDialog"
      position="right"
      full-height
      maximized
    >
      <update-user-role v-model="selectedUser" @updated="updateMember" />
    </q-dialog>

    <q-dialog
      v-model="showAddUserDialog"
      position="right"
      full-height
      maximized
    >
      <add-user
        style="width: 35vw"
        v-model="selectedUser"
        :isUpdated="isUpdated"
        :userRole="currentUserRole"
        :roles="options"
        @updated="addMember"
        @cancel:hideform="hideForm"
      />
    </q-dialog>

    <q-dialog v-model="confirmDelete">
      <q-card style="width: 240px">
        <q-card-section class="confirmBody">
          <div class="head">{{ t("user.confirmDeleteHead") }}</div>
          <div class="para">{{ t("user.confirmDeleteMsg") }}</div>
        </q-card-section>

        <q-card-actions class="confirmActions">
          <q-btn v-close-popup="true" unelevated no-caps class="q-mr-sm">
            {{ t("user.cancel") }}
          </q-btn>
          <q-btn
            v-close-popup="true"
            unelevated
            no-caps
            class="no-border"
            color="primary"
            @click="deleteUser"
          >
            {{ t("user.ok") }}
          </q-btn>
        </q-card-actions>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<script lang="ts">
import { defineComponent, ref, onActivated, onBeforeMount } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, type QTableProps, date } from "quasar";
import { useI18n } from "vue-i18n";
import config from "@/aws-exports";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import usersService from "@/services/users";
import UpdateUserRole from "@/components/iam/users/UpdateRole.vue";
import AddUser from "@/components/iam/users/AddUser.vue";
import NoData from "@/components/shared/grid/NoData.vue";
import organizationsService from "@/services/organizations";
import segment from "@/services/segment_analytics";
import {
  getImageURL,
  verifyOrganizationStatus,
  maskText,
} from "@/utils/zincutils";
import { outlinedDelete } from "@quasar/extras/material-icons-outlined";

// @ts-ignore
import usePermissions from "@/composables/iam/usePermissions";
import { computed, nextTick } from "vue";
import { getRoles } from "@/services/iam";

export default defineComponent({
  name: "UserPageOpenSource",
  components: { QTablePagination, UpdateUserRole, NoData, AddUser },
  emits: [
    "updated:fields",
    "deleted:fields",
    "updated:dates",
    "update:changeRecordPerPage",
    "update:maxRecordToReturn",
  ],
  setup(props, { emit }) {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const resultTotal = ref<number>(0);
    const showUpdateUserDialog: any = ref(false);
    const showAddUserDialog: any = ref(false);
    const confirmDelete = ref<boolean>(false);
    const selectedUser: any = ref({});
    const orgData: any = ref(store.state.selectedOrganization);
    const isUpdated: any = ref(false);
    const qTable: any = ref(null);
    const { usersState } = usePermissions();
    const isEnterprise = ref(false);
    const isCurrentUserInternal = ref(false);

    onActivated(() => {
      if (router.currentRoute.value.query.action == "add") {
        addUser({}, false);
      } else {
        usersState.users.map((member: any) => {
          if (member.email == router.currentRoute.value.query.email) {
            addUser({ row: member }, true);
          }
        });
      }
    });

    onBeforeMount(async () => {
      isEnterprise.value = config.isEnterprise == "true";
      await getOrgMembers();
      if (isEnterprise.value) await getRoles();

      if (
        (isEnterprise.value && isCurrentUserInternal.value) ||
        !isEnterprise.value
      ) {
        columns.value.push({
          name: "actions",
          field: "actions",
          label: t("user.actions"),
          align: "left",
        });
      }

      updateUserActions();
    });

    const columns: any = ref<QTableProps["columns"]>([
      {
        name: "#",
        label: "#",
        field: "#",
        align: "left",
      },
      {
        name: "email",
        field: "email",
        label: t("user.email"),
        align: "left",
        sortable: true,
      },
      {
        name: "first_name",
        field: "first_name",
        label: t("user.firstName"),
        align: "left",
        sortable: true,
      },
      {
        name: "last_name",
        field: "last_name",
        label: t("user.lastName"),
        align: "left",
        sortable: true,
      },
      {
        name: "role",
        field: "role",
        label: t("user.role"),
        align: "left",
        sortable: true,
      },
    ]);
    const userEmail: any = ref("");
    const options = ref([{ label: "Admin", value: "admin" }]);
    const selectedRole = ref(options.value[0].value);
    const currentUserRole = ref("");
    let deleteUserEmail = "";

    const getRoles = () => {
      return new Promise((resolve) => {
        usersService
          .getRoles(store.state.selectedOrganization.identifier)
          .then((res) => {
            options.value = res.data;
          })
          .finally(() => resolve(true));
      });
    };

    const getOrgMembers = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading users...",
      });

      return new Promise((resolve, reject) => {
        usersService
          .orgUsers(
            0,
            1000,
            "email",
            false,
            "",
            store.state.selectedOrganization.identifier
          )
          .then((res) => {
            resultTotal.value = res.data.data.length;
            let counter = 1;
            currentUserRole.value = "";
            usersState.users = res.data.data.map((data: any) => {
              if (store.state.userInfo.email == data.email) {
                currentUserRole.value = data.role;
                isCurrentUserInternal.value = !data.is_external;
              }

              if (data.email == router.currentRoute.value.query.email) {
                addUser({ row: data }, true);
              }

              return {
                "#": counter <= 9 ? `0${counter++}` : counter++,
                email: maskText(data.email),
                first_name: data.first_name,
                last_name: data.last_name,
                role: data.role,
                member_created: date.formatDate(
                  parseInt(data.member_created),
                  "YYYY-MM-DDTHH:mm:ssZ"
                ),
                member_updated: date.formatDate(
                  parseInt(data.member_updated),
                  "YYYY-MM-DDTHH:mm:ssZ"
                ),
                org_member_id: data.org_member_id,
                isLoggedinUser: store.state.userInfo.email == data.email,
                isExternal: !!data.is_external,
                enableEdit: false,
                enableChangeRole: false,
                enableDelete: false,
              };
            });

            dismiss();

            resolve(true);
          })
          .catch(() => {
            dismiss();
            reject(false);
          });
      });
    };

    interface OptionType {
      label: String;
      value: number | String;
    }
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

    const showAddUserBtn = computed(() => {
      if (isEnterprise.value) {
        return (
          isCurrentUserInternal.value &&
          (currentUserRole.value == "admin" || currentUserRole.value == "root")
        );
      } else {
        return (
          currentUserRole.value == "admin" || currentUserRole.value == "root"
        );
      }
    });

    const currentUser = computed(() => store.state.userInfo.email);

    const updateUserActions = () => {
      usersState.users.forEach((member: any) => {
        member.enableEdit = shouldAllowEdit(member);
        member.enableChangeRole = shouldAllowChangeRole(member);
        member.enableDelete = shouldAllowDelete(member);
      });
    };

    const shouldAllowEdit = (user: any) => {
      if (isEnterprise.value) {
        return (
          isCurrentUserInternal.value &&
          !user.isExternal &&
          (currentUserRole.value == "root" ||
            (currentUserRole.value == "admin" && user.role !== "root"))
        );
      } else {
        return (
          user.isLoggedinUser ||
          currentUserRole.value == "root" ||
          (currentUserRole.value == "admin" && user.role !== "root")
        );
      }
    };

    const shouldAllowChangeRole = (user: any) => {
      if (isEnterprise.value) {
        return (
          isCurrentUserInternal.value &&
          !user.isExternal &&
          user.role !== "root" &&
          (currentUserRole.value == "root" || currentUserRole.value == "admin")
        );
      } else {
        return (
          ((currentUserRole.value == "admin" && user.role !== "root") ||
            currentUserRole.value == "root") &&
          !user.isLoggedinUser
        );
      }
    };

    const shouldAllowDelete = (user: any) => {
      if (isEnterprise.value) {
        return (
          isCurrentUserInternal.value &&
          !user.isExternal &&
          user.role !== "root" &&
          (currentUserRole.value == "root" ||
            currentUserRole.value == "admin") &&
          !user.isLoggedinUser
        );
      } else {
        return (
          (currentUserRole.value == "admin" ||
            currentUserRole.value == "root") &&
          !user.isLoggedinUser &&
          user.role !== "root"
        );
      }
    };

    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
    };

    const updateUser = (props: any) => {
      selectedUser.value = props.row;
      showUpdateUserDialog.value = true;
    };

    const addUser = (props: any, is_updated: boolean) => {
      isUpdated.value = is_updated;
      selectedUser.value.organization =
        store.state.selectedOrganization.identifier;

      if (props.row != undefined) {
        selectedUser.value = props.row;
        segment.track("Button Click", {
          button: "Actions",
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          update_user: props.row.email,
          page: "Users",
        });
      } else {
        selectedUser.value = {};
      }
      setTimeout(() => {
        showAddUserDialog.value = true;
      }, 100);
    };

    const addRoutePush = (props: any) => {
      if (props.row != undefined) {
        router.push({
          name: "users",
          query: {
            action: "update",
            org_identifier: store.state.selectedOrganization.identifier,
            email: props.row.email,
          },
        });
        addUser(
          {
            row: props.row,
          },
          true
        );
      } else {
        addUser({}, false);
        router.push({
          name: "users",
          query: {
            action: "add",
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
      }
    };

    const updateMember = async (data: any) => {
      if (data.data != undefined) {
        try {
          await getOrgMembers();
        } catch (error) {
          $q.notify({
            color: "negative",
            message: "Failed to refresh user list",
          });
        }
        updateUserActions();
        showUpdateUserDialog.value = false;
      }
    };

    const hideForm = () => {
      showAddUserDialog.value = false;
      router.replace({
        name: "users",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    const addMember = async (res: any, data: any, operationType: string) => {
      showAddUserDialog.value = false;
      if (res.code == 200) {
        $q.notify({
          color: "positive",
          message: "User added successfully.",
        });
        await getOrgMembers();
        updateUserActions();
      }
      router.replace({
        name: "users",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    const confirmDeleteAction = (props: any) => {
      confirmDelete.value = true;
      deleteUserEmail = props.row.email;
    };

    const deleteUser = async () => {
      usersService
        .delete(store.state.selectedOrganization.identifier, deleteUserEmail)
        .then(async (res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              color: "positive",
              message: "User deleted successfully.",
            });
            await getOrgMembers();
            updateUserActions();
          }
        })
        .catch((err: any) => {
          if(err.response.status != 403){
            $q.notify({
              color: "negative",
              message: "Error while deleting user.",
            });
          }
        });
    };

    const updateUserRole = (row: any) => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      organizationsService
        .update_member_role(
          {
            id: parseInt(row.orgMemberId ? row.orgMemberId : row.org_member_id),
            role: row.role,
            email: row.email,
            organization_id: parseInt(store.state.selectedOrganization.id),
          },
          store.state.selectedOrganization.identifier
        )
        .then((res: { data: any }) => {
          if (res.data.error_members != null) {
            const message = `Error while updating organization member`;
            $q.notify({
              type: "negative",
              message: message,
              timeout: 15000,
            });
          } else {
            $q.notify({
              type: "positive",
              message: "Organization member updated successfully.",
              timeout: 3000,
            });
          }
          dismiss();
        })
        .catch((error) => {
          dismiss();
          console.log(error);
        });

      segment.track("Button Click", {
        button: "Update Role",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        update_user: row.email,
        page: "Users",
      });
    };
    return {
      t,
      qTable,
      router,
      store,
      config,
      usersState,
      columns,
      orgData,
      confirmDelete,
      deleteUser,
      confirmDeleteAction,
      getOrgMembers,
      updateUser,
      updateMember,
      addUser,
      addRoutePush,
      addMember,
      hideForm,
      isUpdated,
      showAddUserDialog,
      pagination,
      resultTotal,
      selectedUser,
      perPageOptions,
      selectedPerPage,
      changePagination,
      maxRecordToReturn,
      showUpdateUserDialog,
      changeMaxRecordToReturn,
      outlinedDelete,
      filterQuery: ref(""),
      filterData(rows: any, terms: any) {
        var filtered = [];
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (
            rows[i]["first_name"]?.toLowerCase().includes(terms) ||
            rows[i]["last_name"]?.toLowerCase().includes(terms) ||
            rows[i]["email"]?.toLowerCase().includes(terms) ||
            rows[i]["role"].toLowerCase().includes(terms)
          ) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
      userEmail,
      selectedRole,
      options,
      currentUserRole,
      updateUserRole,
      getImageURL,
      verifyOrganizationStatus,
      isEnterprise,
      isCurrentUserInternal,
      showAddUserBtn,
    };
  },
});
</script>

<style lang="scss" scoped>
.q-table {
  &__top {
    border-bottom: 1px solid $border-color;
    justify-content: flex-end;
  }
}

.iconHoverBtn {
  cursor: pointer !important;
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

.non-selectable {
  cursor: default !important;
}

.invite-user {
  background: $input-bg;
  border-radius: 4px;

  .separator {
    width: 1px;
  }
}

.inputHint {
  font-size: 11px;
  color: $light-text;
}
</style>
