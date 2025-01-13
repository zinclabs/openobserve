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
  <div class="q-ma-md">
    <CopyContent class="q-mt-sm" :content="content" />
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import type { Ref } from "vue";
import config from "../../../aws-exports";
import { useStore } from "vuex";
import { getImageURL, maskText } from "../../../utils/zincutils";
import CopyContent from "@/components/CopyContent.vue";
export default defineComponent({
  name: "curl-mechanism",
  props: {
    currOrgIdentifier: {
      type: String,
    },
    currUserEmail: {
      type: String,
    },
  },
  components: { CopyContent },
  setup() {
    const store = useStore();
    const endpoint: any = ref({
      url: "",
      host: "",
      port: "",
      protocol: "",
      tls: "",
    });

    let ingestionURL: string = store.state.API_ENDPOINT;
    if (
      Object.hasOwn(store.state.zoConfig, "ingestion_url") &&
      store.state.zoConfig.ingestion_url !== ""
    ) {
      ingestionURL = store.state.zoConfig.ingestion_url;
    }
    const url = new URL(ingestionURL);

    endpoint.value = {
      url: ingestionURL,
      host: url.hostname,
      port: url.port || (url.protocol === "https:" ? "443" : "80"),
      protocol: url.protocol.replace(":", ""),
      tls: url.protocol === "https:" ? "On" : "Off",
    };

    const content = `curl -u [EMAIL]:[PASSCODE] -k ${endpoint.value.url}/api/${store.state.selectedOrganization.identifier}/default/_json -d '[{"level":"info","job":"test","log":"test message for openobserve"}]'`;

    return {
      store,
      config,
      endpoint,
      getImageURL,
      maskText,
      content,
    };
  },
});
</script>
