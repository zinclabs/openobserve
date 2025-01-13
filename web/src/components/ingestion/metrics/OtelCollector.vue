<template>
  <div>
    <div class="q-pa-md">
      <div class="text-subtitle1 text-bold q-mt-md q-pl-xs">OTLP HTTP</div>
      <ContentCopy class="q-mt-sm" :content="getOtelHttpConfig" />
    </div>
    <div class="q-pa-md">
      <div class="text-subtitle1 text-bold q-mt-md q-pl-xs">OTLP gRPC</div>
      <ContentCopy class="q-mt-sm" :content="getOtelGrpcConfig" />
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
    `${props.currUserEmail}:${store.state.organizationData.organizationPasscode}`
  );
});

const getOtelGrpcConfig = computed(() => {
  return `exporters:
  otlp/openobserve:
      endpoint: ${endpoint.value.host}:5081
      headers:
        Authorization: "Basic [BASIC_PASSCODE]"
        organization: ${props.currOrgIdentifier}
        stream-name: default
      tls:
        insecure: true`;
});

const getOtelHttpConfig = computed(() => {
  return `exporters:
  otlphttp/openobserve:
    endpoint: ${endpoint.value.url}/api/${props.currOrgIdentifier}
    headers:
      Authorization: Basic [BASIC_PASSCODE]
      stream-name: default`;
});
</script>

<style scoped></style>
