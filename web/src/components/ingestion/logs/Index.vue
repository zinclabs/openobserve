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

<!-- eslint-disable vue/x-invalid-end-tag -->
<template>
  <q-splitter
    v-model="splitterModel"
    unit="px"
    style="min-height: calc(100vh - 130px)"
  >
    <template v-slot:before>
      <q-tabs
        v-model="ingestiontabs"
        indicator-color="transparent"
        inline-label
        vertical
      >
        <q-route-tab
          name="curl"
          :to="{
            name: 'curl',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          icon="data_object"
          label="Curl"
          content-class="tab_content"
        />
        <q-route-tab
          name="filebeat"
          :to="{
            name: 'filebeat',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          :icon="'img:' + getImageURL('images/ingestion/filebeat.png')"
          label="Filebeat"
          content-class="tab_content"
        />
        <q-route-tab
          default
          name="fluentbit"
          :to="{
            name: 'fluentbit',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          :icon="'img:' + getImageURL('images/ingestion/fluentbit_icon.png')"
          label="FluentBit"
          content-class="tab_content"
        />
        <q-route-tab
          name="fluentd"
          :to="{
            name: 'fluentd',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          :icon="'img:' + getImageURL('images/ingestion/fluentd_icon.svg')"
          label="Fluentd"
          content-class="tab_content"
        />
        <q-route-tab
          name="vector"
          :to="{
            name: 'vector',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          :icon="'img:' + getImageURL('images/ingestion/vector.png')"
          label="Vector"
          content-class="tab_content"
        />
        <q-route-tab
          name="ingestLogsFromOtel"
          :to="{
            name: 'ingestLogsFromOtel',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          :icon="'img:' + getImageURL('images/ingestion/otlp.svg')"
          label="OTEL Collector"
          content-class="tab_content"
        />
        <q-route-tab
          name="logstash"
          :to="{
            name: 'logstash',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          :icon="'img:' + getImageURL('images/ingestion/logstash.svg')"
          label="Logstash"
          content-class="tab_content"
        />
        <q-route-tab
          name="syslogNg"
          :to="{
            name: 'syslogNg',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          icon="plagiarism"
          label="Syslog-ng"
          content-class="tab_content"
        />
        <q-route-tab
          v-if="showSyslog"
          name="syslog"
          :to="{
            name: 'syslog',
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          }"
          icon="plagiarism"
          label="Syslog"
          content-class="tab_content"
        />
      </q-tabs>
    </template>

    <template v-slot:after>
      <router-view
        :title="ingestiontabs"
        :currOrgIdentifier="currOrgIdentifier"
        :currUserEmail="currentUserEmail"
        @copy-to-clipboard-fn="copyToClipboardFn"
      >
      </router-view>
    </template>
  </q-splitter>
</template>

<script lang="ts">
// @ts-ignore
import { defineComponent, ref, onBeforeMount, computed, onUpdated } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { copyToClipboard, useQuasar } from "quasar";
import config from "../../../aws-exports";
import segment from "@/services/segment_analytics";
import { getImageURL, verifyOrganizationStatus } from "@/utils/zincutils";

export default defineComponent({
  name: "IngestLogs",
  props: {
    currOrgIdentifier: {
      type: String,
      default: "",
    },
  },
  setup() {
    const { t } = useI18n();
    const store = useStore();
    const q = useQuasar();
    const router: any = useRouter();
    const rowData: any = ref({});
    const confirmUpdate = ref<boolean>(false);
    const ingestiontabs = ref("");
    const currentOrgIdentifier: any = ref(
      store.state.selectedOrganization.identifier,
    );
    const ingestRoutes = [
      "curl",
      "fluentbit",
      "fluentd",
      "vector",
      "syslog",
      "syslogNg",
    ];

    onBeforeMount(() => {
      if (ingestRoutes.includes(router.currentRoute.value.name)) {
        router.push({
          name: router.currentRoute.value.name,
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
        return;
      }
      if (router.currentRoute.value.name === "ingestLogs") {
        router.push({
          name: "curl",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
        return;
      }
    });
    onUpdated(() => {
      if (router.currentRoute.value.name === "ingestLogs") {
        router.push({
          name: "curl",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
        return;
      }
    });

    const copyToClipboardFn = (content: any) => {
      copyToClipboard(content.innerText)
        .then(() => {
          q.notify({
            type: "positive",
            message: "Content Copied Successfully!",
            timeout: 5000,
          });
        })
        .catch(() => {
          q.notify({
            type: "negative",
            message: "Error while copy content.",
            timeout: 5000,
          });
        });

      segment.track("Button Click", {
        button: "Copy to Clipboard",
        ingestion: router.currentRoute.value.name,
        user_org: store.state.selectedOrganization.identifier,
        user_id: store.state.userInfo.email,
        page: "Ingestion",
      });
    };

    const showUpdateDialogFn = () => {
      confirmUpdate.value = true;
    };

    const showSyslog = computed(() => {
      return config.isCloud !== "true";
    });

    const showCloudIngestionOptions = computed(() => {
      return config.isCloud === "true";
    });

    return {
      t,
      store,
      router,
      config,
      rowData,
      splitterModel: ref(250),
      currentUserEmail: store.state.userInfo.email,
      currentOrgIdentifier,
      copyToClipboardFn,
      showUpdateDialogFn,
      confirmUpdate,
      getImageURL,
      verifyOrganizationStatus,
      ingestiontabs,
      showSyslog,
      showCloudIngestionOptions,
    };
  },
});
</script>

<style scoped lang="scss">
.ingestionPage {
  padding: 1.5rem 0 0;
  .head {
    padding-bottom: 1rem;
  }
  .q-tabs {
    &--vertical {
      margin: 1.5rem 1rem 0 1rem;
      .q-tab {
        justify-content: flex-start;
        padding: 0 0.6rem 0 0.6rem;
        border-radius: 0.5rem;
        margin-bottom: 0.5rem;
        text-transform: capitalize;

        &__content.tab_content {
          .q-tab {
            &__icon + &__label {
              padding-left: 0.875rem;
              font-weight: 600;
            }
          }
        }
        &--active {
          color: black;
          background-color: $accent;
        }
      }
    }
  }
}
</style>
<style lang="scss">
.ingestionPage {
  .q-tab-panel {
    padding: 0 !important;
    .tab_content {
      .q-tab__label {
        overflow: hidden !important;
        text-overflow: ellipsis !important;
        white-space: nowrap !important;
      }
    }
  }

  .q-icon > img {
    height: auto !important;
  }
}
</style>
