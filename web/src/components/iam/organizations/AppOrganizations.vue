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
    <component v-if="loadComponent" :is="componentName" />
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import OrganizationsEnterprise from "@/components/iam/organizations/ListOrganizations.vue";

import config from "@/aws-exports";
import { watch } from "vue";

export default defineComponent({
  name: "AppOrganizations",
  components: {
    OrganizationsEnterprise,
  },
  setup() {
    const store = useStore();
    const { t } = useI18n();

    const componentName = ref("OrganizationsEnterprise");

    const loadComponent = ref(false);

    watch(
      () => store.state.zoConfig,
      (zoConfig) => {
        if (
          zoConfig.sso_enabled ||
          config.isEnterprise == "true" ||
          config.isCloud == "true"
        ) {
          componentName.value = "OrganizationsEnterprise";
        }

        loadComponent.value = true;
      },
      {
        immediate: true,
      },
    );

    return { store, t, componentName, loadComponent };
  },
});
</script>
