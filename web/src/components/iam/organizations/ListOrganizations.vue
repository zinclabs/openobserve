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
        </div>
        <div class="full-width row q-mb-xs items-start">
          <div class="col">
            <q-input
              v-model="filterQuery"
              filled
              dense
              class="col-6 q-pr-sm"
              style="width: 400px"
              :placeholder="t('organization.search')"
            >
              <template #prepend>
                <q-icon name="search" />
              </template>
            </q-input>
          </div>
          <div class="col-3">
            <q-btn
              v-if="config.isEnterprise == 'true' || config.isCloud == 'true'"
              class="q-ml-md q-mb-xs text-bold no-border float-right"
              padding="sm lg"
              color="secondary"
              no-caps
              dense
              :label="t(`organization.add`)"
              @click="addOrganization"
              data-test="Add Organization"
            />
          </div>
        </div>

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
          position="bottom"
          @update:changeRecordPerPage="changePagination"
        />
        <!-- :maxRecordToReturn="maxRecordToReturn" -->
        <!-- @update:maxRecordToReturn="changeMaxRecordToReturn" -->
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
  </q-page>
</template>

<script lang="ts">
// @ts-nocheck
import { defineComponent, ref, watch, onMounted, onBeforeMount, onUpdated } from "vue";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar, date, copyToClipboard } from "quasar";
import { useI18n } from "vue-i18n";

import organizationsService from "@/services/organizations";
import JoinOrganization from "./JoinOrganization.vue";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import AddUpdateOrganization from "@/components/iam/organizations/AddUpdateOrganization.vue";
import NoData from "@/components/shared/grid/NoData.vue";
import segment from "@/services/segment_analytics";
import { convertToTitleCase } from "@/utils/zincutils";
import config from "@/aws-exports";

export default defineComponent({
  name: "PageOrganization",
  components: {
    AddUpdateOrganization,
    QTablePagination,
    NoData,
    AddUpdateOrganization,
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
        name: "type",
        field: "type",
        label: t("organization.type"),
        align: "left",
        sortable: true,
      },
    ]);

    if (config.isCloud == "true") {
      columns.value.push({
        name: "plan",
        field: "plan",
        label: t("organization.subscription_plan"),
        align: "center",
        sortable: true,
      });
    }

    const perPageOptions = [
      { label: "25", value: 25 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "250", value: 250 },
      { label: "500", value: 500 },
    ];
    const resultTotal = ref<number>(0);
    // const maxRecordToReturn = ref<number>(500);
    const selectedPerPage = ref<number>(25);
    const pagination: any = ref({
      rowsPerPage: 25,
    });
    const isCloudOrEnterprise = () => {
      return config.isCloud === "true" || config.isEnterprise === "true";
    };

    watch(
      () => router.currentRoute.value.query?.action,
      (action) => {
        if (action == "add" && isCloudOrEnterprise()) {
          showAddOrganizationDialog.value = true;
        }
      },
    );

    onMounted(() => {
      if (
        router.currentRoute.value.query.action == "add" &&
        isCloudOrEnterprise()
      ) {
        showAddOrganizationDialog.value = true;
      }
    });

    onUpdated(() => {
      if (
        router.currentRoute.value.query.action == "add" &&
        isCloudOrEnterprise()
      ) {
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

    onMounted(() => {
      if (router.currentRoute.value.query?.action == "add") {
        showAddOrganizationDialog.value = true;
      }
    });

    watch(
      () => router.currentRoute.value.query?.action,
      (value) => {
        if (value == "add") {
          showAddOrganizationDialog.value = true;
        }
      },
    );

    const changePagination = (val: { label: string; value: any }) => {
      selectedPerPage.value = val.value;
      pagination.value.rowsPerPage = val.value;
      qTable.value.setPagination(pagination.value);
    };

    const getOrganizations = () => {
      const dismiss = $q.notify({
        spinner: true,
        message: "Please wait while loading organizations...",
      });
      organizationsService.list(0, 1000000, "name", false, "").then((res) => {
        // Updating store so that organizations in navbar also gets updated
        store.dispatch("setOrganizations", res.data.data);

        resultTotal.value = res.data.data.length;
        let counter = 1;
        const billingPlans = {
          0: "Free Forever",
          1: "Pay As You Go",
          2: "Enterprise",
        };
        organizations.value = res.data.data.map((data) => {
          // Common fields for all configurations

          const commonOrganization = {
            "#": counter <= 9 ? `0${counter++}` : counter++,
            name: data.name,
            identifier: data.identifier,
            type: convertToTitleCase(data.type),
            plan: billingPlans[data.plan] || "-",
          };

          // Additional fields and logic for cloud configuration
          // if (config.isCloud === "true") {
          //   const memberrole = data.OrganizationMemberObj.filter(
          //     (v) =>
          //       v.user_id === store.state.currentuser.id && v.role === "admin",
          //   );

          //   // If invited, pass props to inviteTeam function
          //   // if (
          //   //   router.currentRoute.value.query.action === "invite" &&
          //   //   data.identifier === router.currentRoute.value.query.id
          //   // ) {
          //   //   const props = {
          //   //     row: {
          //   //       id: data.id,
          //   //       name: data.name,
          //   //       identifier: data.identifier,
          //   //       role: data.role,
          //   //       member_lists: [],
          //   //     },
          //   //   };
          //   //   inviteTeam(props);
          //   // }

          //   const role = memberrole.length ? memberrole[0].role : "member";

          //   // Extend common fields with cloud-specific data
          //   return {
          //     ...commonOrganization,
          //     id: data.id,
          //     created: date.formatDate(data.created_at, "YYYY-MM-DDTHH:mm:ssZ"),
          //     role: convertToTitleCase(role),
          //     status: convertToTitleCase(data.status),
          //     plan_type:
          //       data.CustomerBillingObj.subscription_type === config.freePlan ||
          //       data.CustomerBillingObj.subscription_type === ""
          //         ? "Developer"
          //         : "Pro",
          //   };
          // }

          // For open-source or enterprise, return only common fields
          return commonOrganization;
        });

        dismiss();
      });
    };

    getOrganizations();

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

    const hideAddOrgDialog = () => {
      router.push({
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
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

    return {
      t,
      store,
      router,
      qTable,
      config,
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
      pagination,
      resultTotal,
      perPageOptions,
      selectedPerPage,
      changePagination,
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
      hideAddOrgDialog,
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
