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
  <q-page class="page">
    <div class="head q-table__title q-mx-md q-my-sm">
      {{ t("settings.header") }}
    </div>
    <q-separator class="separator" />
    <q-splitter
      v-model="splitterModel"
      unit="px"
      style="min-height: calc(100vh - 104px)"
    >
      <template v-slot:before>
        <q-tabs
          v-model="settingsTab"
          indicator-color="transparent"
          inline-label
          vertical
        >
          <q-route-tab
            default
            name="queryManagement"
            :to="'/settings/query_management'"
            icon="query_stats"
            :label="t('settings.queryManagement')"
            content-class="tab_content"
            v-if="isMetaOrg"
          />
          <q-route-tab
            name="general"
            :to="'/settings/general'"
            :icon="outlinedSettings"
            :label="t('settings.generalLabel')"
            content-class="tab_content"
          />
          <q-route-tab
            name="organization"
            :to="'/settings/organization'"
            icon="business"
            :label="t('settings.orgLabel')"
            content-class="tab_content"
          />
          <q-route-tab
            data-test="alert-destinations-tab"
            name="destinations"
            :to="{
              name: 'alertDestinations',
              query: {
                org_identifier: store.state.selectedOrganization.identifier,
              },
            }"
            icon="location_on"
            :label="t('alert_destinations.header')"
            content-class="tab_content"
          />
          <q-route-tab
            data-test="alert-templates-tab"
            name="templates"
            :to="{
              name: 'alertTemplates',
              query: {
                org_identifier: store.state.selectedOrganization.identifier,
              },
            }"
            icon="description"
            :label="t('alert_templates.header')"
            content-class="tab_content"
          />
          <q-route-tab
            data-test="management-cipher-key-tab"
            name="cipher-keys"
            :to="{
              name: 'cipherKeys',
              query: {
                org_identifier: store.state.selectedOrganization.identifier,
              },
            }"
            icon="key"
            :label="t('settings.cipherKeys')"
            content-class="tab_content"
          />
        </q-tabs>
      </template>

      <template v-slot:after>
        <router-view title=""> </router-view>
      </template>
    </q-splitter>
  </q-page>
</template>

<script lang="ts">
// @ts-ignore
import {
  defineComponent,
  ref,
  onBeforeMount,
  onActivated,
  onUpdated,
} from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useRouter } from "vue-router";
import { useQuasar } from "quasar";
import config from "@/aws-exports";
import { outlinedSettings } from "@quasar/extras/material-icons-outlined";
import useIsMetaOrg from "@/composables/useIsMetaOrg";

export default defineComponent({
  name: "AppSettings",
  setup() {
    const { t } = useI18n();
    const store = useStore();
    const q = useQuasar();
    const router: any = useRouter();
    const settingsTab = ref("general");
    const { isMetaOrg } = useIsMetaOrg();

    const handleSettingsRouting = () => {
      if (router.currentRoute.value.name === "settings") {
        if (isMetaOrg.value && config.isEnterprise === "true") {
          settingsTab.value = "queryManagement";
          router.push({
            path: "/settings/query_management",
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          });
        } else {
          settingsTab.value = "general";
          router.push({
            path: "/settings/general",
            query: {
              org_identifier: store.state.selectedOrganization.identifier,
            },
          });
        }
      }
    };

    onBeforeMount(() => {
      handleSettingsRouting();
    });

    onActivated(() => {
      handleSettingsRouting();
    });

    onUpdated(() => {
      handleSettingsRouting();
    });

    return {
      t,
      store,
      router,
      config,
      settingsTab,
      splitterModel: ref(250),
      outlinedSettings,
      isMetaOrg,
    };
  },
});
</script>
<style scoped lang="scss">
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
</style>
