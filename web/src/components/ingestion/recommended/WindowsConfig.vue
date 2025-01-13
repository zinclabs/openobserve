<template>
  <div class="q-pa-md">
    <div class="text-subtitle1 q-pl-xs q-mt-md">
      Run the powershell terminal as administrator and execute the following
      command:
    </div>
    <ContentCopy class="q-mt-sm" :content="getCommand" />
    <br />
    <hr />
    <div>
      <div class="text-subtitle1 q-pl-xs q-mt-md">
        Once you have installed the OpenObserve collector, it will:
        <ul class="tw-list-disc tw-ml-5">
          <li>Collect logs from Windows event log</li>
          <li>Collect metrics from Windows performance counters</li>
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
  return `Invoke-WebRequest -Uri https://raw.githubusercontent.com/openobserve/agents/main/windows/install.ps1 -OutFile install.ps1 ; .\\install.ps1 -URL ${endpoint.value.url}/api/${props.currOrgIdentifier}/ -AUTH_KEY [BASIC_PASSCODE]`;
});
</script>

<style scoped></style>
