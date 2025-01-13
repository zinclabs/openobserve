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
        <q-td :props="props">
          {{ props.row.role }}
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

export default defineComponent({
  name: "UserPageOpenSource",
  components: { QTablePagination, NoData },
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

    onBeforeMount(() => {
      getOrgMembers();
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
    const options = ["admin"];
    const selectedRole = ref(options[0]);
    const currentUserRole = ref("");
    let deleteUserEmail = "";

    const getOrgMembers = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading users...",
      });

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
            };
          });

          dismiss();
        });
    };

    if (usersState.users.length == 0) {
      getOrgMembers();
    }

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

    const updateMember = (data: any) => {
      if (data.data != undefined) {
        getOrgMembers();
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

    const addMember = (res: any, data: any, operationType: string) => {
      showAddUserDialog.value = false;
      if (res.code == 200) {
        $q.notify({
          color: "positive",
          message: "User added successfully.",
        });

        if (operationType == "created") {
          if (
            store.state.selectedOrganization.identifier == data.organization
          ) {
            usersState.users.push({
              "#":
                usersState.users.length + 1 <= 9
                  ? `0${usersState.users.length + 1}`
                  : usersState.users.length + 1,
              email: data.email,
              first_name: data.first_name,
              last_name: data.last_name,
              role: data.role,
            });
          }
        } else {
          usersState.users.forEach((member: any, key: number) => {
            if (member.email == data.email) {
              usersState.users[key] = {
                ...usersState.users[key],
                ...data,
              };
            }
          });
        }
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

    const deleteUser = () => {
      usersService
        .delete(store.state.selectedOrganization.identifier, deleteUserEmail)
        .then((res: any) => {
          if (res.data.code == 200) {
            $q.notify({
              color: "positive",
              message: "User deleted successfully.",
            });
            getOrgMembers();
          }
        })
        .catch((err: any) => {
          $q.notify({
            color: "negative",
            message: "Error while deleting user.",
          });
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
