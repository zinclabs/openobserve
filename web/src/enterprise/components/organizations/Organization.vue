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
  <q-page class="q-pa-none">
    <q-table
      ref="qTable"
      :rows="organizations"
      :columns="columns"
      row-key="id"
      :pagination="pagination"
      :filter="filterQuery"
      :filter-method="filterData"
      :loading="loading"
    >
      <template #no-data><NoData /></template>

      <!-- <template #body-cell-actions="props">
        <q-td :props="props">
          <q-btn
            v-if="props.row.role == 'Admin'"
            icon="group"
            :title="t('organization.invite')"
            padding="sm"
            unelevated
            size="sm"
            round
            flat
            @click="redirectToInviteMember(props)"
          ></q-btn>
        </q-td>
      </template> -->

      <template #top="scope">
        <div
          class="full-width flex justify-between items-start"
          style="margin-bottom: 2px; height: 44px"
        >
          <div class="q-table__title">{{ t("organization.header") }}</div>
          <q-btn
            class="q-ml-md q-mb-xs text-bold no-border"
            padding="sm lg"
            color="secondary"
            no-caps
            dense
            :label="t(`organization.add`)"
            @click="addOrganization"
          />
        </div>
        <q-input
          v-model="filterQuery"
          filled
          dense
          class="q-ml-none q-mb-xs"
          style="width: 400px"
          :placeholder="t('organization.search')"
        >
          <template #prepend>
            <q-icon name="search" />
          </template>
        </q-input>

        <QTablePagination
          :scope="scope"
          :pageTitle="t('organization.header')"
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
          :maxRecordToReturn="maxRecordToReturn"
          position="bottom"
          @update:changeRecordPerPage="changePagination"
          @update:maxRecordToReturn="changeMaxRecordToReturn"
        />
      </template>
    </q-table>
    <q-dialog
      v-model="showAddOrganizationDialog"
      position="right"
      full-height
      maximized
      @before-hide="hideAddOrgDialog"
    >
      <add-update-organization @updated="updateOrganizationList" />
    </q-dialog>

    <q-dialog
      v-model="showJoinOrganizationDialog"
      position="right"
      full-height
      maximized
      @before-hide="hideJoinOrgDialog"
    >
      <join-organization v-model="organization" @updated="joinOrganization" />
    </q-dialog>
    <q-dialog v-model="showOrgAPIKeyDialog">
      <q-card>
        <q-card-section>
          <div class="text-h6">Organization API Key</div>
        </q-card-section>

        <q-card-section class="q-pt-none" wrap>
          <q-item>
            <q-item-section
              ><q-item-label lines="3" style="word-wrap: break-word">{{
                organizationAPIKey
              }}</q-item-label></q-item-section
            >
            <q-item-section side>
              <q-btn
                unelevated
                round
                flat
                padding="sm"
                size="sm"
                icon="content_copy"
                @click="copyAPIKey"
                :title="t('organization.copyapikey')"
              />
            </q-item-section>
          </q-item>
        </q-card-section>
      </q-card>
    </q-dialog>
  </q-page>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref, onMounted, onUpdated, watch } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, date, copyToClipboard } from "quasar";
import { useI18n } from "vue-i18n";

import organizationsService from "@/services/organizations";
import AddUpdateOrganization from "./AddUpdateOrganization.vue";
import JoinOrganization from "./JoinOrganization.vue";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import NoData from "@/components/shared/grid/NoData.vue";
import segment from "@/services/segment_analytics";
import { convertToTitleCase } from "@/utils/zincutils";
import config from "@/aws-exports";

export default defineComponent({
  name: "PageOrganization",
  components: {
    AddUpdateOrganization,
    JoinOrganization,
    QTablePagination,
    NoData,
  },
  setup() {
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const organizations = ref([]);
    const organization = ref({});
    const showAddOrganizationDialog = ref(false);
    const showJoinOrganizationDialog = ref(false);
    const showOrgAPIKeyDialog = ref(false);
    const organizationAPIKey = ref("");
    const qTable: any = ref(null);
    const columns = ref<QTableProps["columns"]>([
      {
        name: "#",
        label: "#",
        field: "#",
        align: "left",
      },
      {
        name: "name",
        field: "name",
        label: t("organization.name"),
        align: "left",
        sortable: true,
      },
      {
        name: "identifier",
        field: "identifier",
        label: t("organization.identifier"),
        align: "left",
        sortable: true,
      },
      {
        name: "role",
        field: "role",
        label: t("organization.role"),
        align: "left",
        sortable: true,
      },
      {
        name: "type",
        field: "type",
        label: t("organization.type"),
        align: "left",
        sortable: true,
      },
      {
        name: "owner",
        field: "owner",
        label: t("organization.owner"),
        align: "left",
        sortable: true,
      },
      {
        name: "plan_type",
        field: "plan_type",
        label: t("organization.subscription_type"),
        align: "left",
        sortable: true,
      },
      {
        name: "status",
        field: "status",
        label: t("organization.status"),
        align: "left",
        sortable: true,
      },
      {
        name: "created",
        field: "created",
        label: t("organization.created"),
        align: "left",
        sortable: true,
      },
      // {
      //   name: "actions",
      //   field: "actions",
      //   label: t("organization.actions"),
      //   align: "center",
      // },
    ]);
    const perPageOptions = [
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

    watch(
      () => router.currentRoute.value.query?.action,
      (action) => {
        if (action == "add") {
          showAddOrganizationDialog.value = true;
        }
      }
    );

    onMounted(() => {
      if (router.currentRoute.value.query.action == "add") {
        showAddOrganizationDialog.value = true;
      }
    });

    onUpdated(() => {
      if (router.currentRoute.value.query.action == "add") {
        showAddOrganizationDialog.value = true;
      }

      if (router.currentRoute.value.query.action == "invite") {
        organizations.value.map((org) => {
          if (org.identifier == router.currentRoute.value.query.id) {
            organization.value = org;
            showJoinOrganizationDialog.value = true;
          }
        });
      }
    });

    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };
    const changeMaxRecordToReturn = (val: any) => {
      maxRecordToReturn.value = val;
      getOrganizations();
    };

    const addOrganization = (evt) => {
      router.push({
        query: {
          action: "add",
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });

      if (evt) {
        let button_txt = evt.target.innerText;
        segment.track("Button Click", {
          button: button_txt,
          user_org: store.state.selectedOrganization.identifier,
          user_id: store.state.userInfo.email,
          page: "Organizations",
        });
      }
    };

    const getOrganizations = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading organizations...",
      });
      organizationsService.list(0, 100000, "name", false, "").then((res) => {
        // Updating store so that organizations in navbar also gets updated
        store.dispatch("setOrganizations", res.data.data);

        resultTotal.value = res.data.data.length;
        let counter = 1;
        organizations.value = res.data.data.map((data: any) => {
          const memberrole = data.OrganizationMemberObj.filter(
            (v: any) =>
              v.user_id == store.state.currentuser.id && v.role == "admin"
          );

          if (
            router.currentRoute.value.query.action == "invite" &&
            data.identifier == router.currentRoute.value.query.id
          ) {
            const props: { row: any } = {
              row: {
                id: data.id,
                name: data.name,
                identifier: data.identifier,
                role: data.role,
                member_lists: [],
              },
            };

            inviteTeam(props);
          }

          const role = memberrole.length ? memberrole[0].role : "member";
          return {
            "#": counter <= 9 ? `0${counter++}` : counter++,
            id: data.id,
            name: data.name,
            identifier: data.identifier,
            type: convertToTitleCase(data.type),
            owner:
              data.UserObj.first_name != ""
                ? data.UserObj.first_name
                : data.UserObj.email,
            created: date.formatDate(data.created_at, "YYYY-MM-DDTHH:mm:ssZ"),
            role: convertToTitleCase(role),
            status: convertToTitleCase(data.status),
            plan_type:
              data.CustomerBillingObj.subscription_type == config.freePlan ||
              data.CustomerBillingObj.subscription_type == ""
                ? "Developer"
                : "Pro",
          };
        });

        dismiss();
      });
    };

    const onAddTeam = (props: any) => {
      console.log(props);
    };

    const inviteTeam = (props: any) => {
      organization.value = {
        id: props.row.id,
        name: props.row.name,
        role: props.row.role,
        identifier: props.row.identifier,
        member_lists: [],
      };
      showJoinOrganizationDialog.value = true;

      segment.track("Button Click", {
        button: "Invite Member",
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        page: "Organizations",
      });
    };

    getOrganizations();

    const redirectToInviteMember = (props) => {
      router.push({
        name: "organizations",
        query: {
          action: "invite",
          id: props.row.identifier,
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    const hideAddOrgDialog = () => {
      router.push({
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    const hideJoinOrgDialog = () => {
      router.push({
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };

    return {
      t,
      store,
      router,
      qTable,
      loading: ref(false),
      organizations,
      organization,
      columns,
      showAddOrganizationDialog,
      showJoinOrganizationDialog,
      showOrgAPIKeyDialog,
      organizationAPIKey,
      addOrganization,
      getOrganizations,
      inviteTeam,
      onAddTeam,
      pagination,
      resultTotal,
      perPageOptions,
      selectedPerPage,
      changePagination,
      maxRecordToReturn,
      changeMaxRecordToReturn,
      filterQuery: ref(""),
      filterData(rows: string | any[], terms: string) {
        const filtered = [];
        terms = terms.toLowerCase();
        for (let i = 0; i < rows.length; i++) {
          if (rows[i]["name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
        return filtered;
      },
      redirectToInviteMember,
      hideAddOrgDialog,
      hideJoinOrgDialog,
    };
  },
  methods: {
    updateOrganizationList() {
      this.router.push({
        name: "organizations",
        query: {
          org_identifier: this.store.state.selectedOrganization.identifier,
        },
      });
      this.showAddOrganizationDialog = false;
      this.getOrganizations();

      this.$q.notify({
        type: "positive",
        message: `Organization added successfully.`,
      });
    },
    joinOrganization() {
      this.$q.notify({
        type: "positive",
        message: "Request completed successfully.",
        timeout: 5000,
      });
      this.showJoinOrganizationDialog = false;
    },
    copyAPIKey() {
      copyToClipboard(this.organizationAPIKey)
        .then(() => {
          this.$q.notify({
            type: "positive",
            message: "API Key Copied Successfully!",
            timeout: 5000,
          });
        })
        .catch(() => {
          this.$q.notify({
            type: "negative",
            message: "Error while copy API Key.",
            timeout: 5000,
          });
        });
    },
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
</style>
