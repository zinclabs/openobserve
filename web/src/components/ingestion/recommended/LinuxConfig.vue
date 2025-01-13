<template>
  <div class="q-pa-md">
    <ContentCopy :content="getCommand"></ContentCopy>
    <br />
    <hr />
    <div>
      <div class="text-subtitle1 q-pl-xs q-mt-md">
        Once you have installed the OpenObserve collector, it will:
        <ul class="tw-list-disc tw-ml-5">
          <li>Collect system logs</li>
          <li>Collect host metrics</li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, type Ref } from "vue";
import type { Endpoint } from "@/ts/interfaces";
import ContentCopy from "@/components/CopyContent.vue";
import { useStore } from "vuex";
import { b64EncodeStandard } from "../../../utils/zincutils";

const store = useStore();

const props = defineProps({
  currOrgIdentifier: {
    type: String,
  },
  currUserEmail: {
    type: String,
  },
});

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

const accessKey = computed(() => {
  return b64EncodeStandard(
    `${props.currUserEmail}:${store.state.organizationData.organizationPasscode}`,
  );
});

const getCommand = computed(() => {
  return `curl -O https://raw.githubusercontent.com/openobserve/agents/main/linux/install.sh && chmod +x install.sh && sudo ./install.sh ${endpoint.value.url}/api/${props.currOrgIdentifier}/ [BASIC_PASSCODE]`;
});
</script>

<style scoped></style>
