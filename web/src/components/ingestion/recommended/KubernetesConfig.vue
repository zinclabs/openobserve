<!-- eslint-disable no-useless-escape -->
<template>
  <div class="q-pa-md kubernetes-config-section">
    <div class="text-subtitle1 q-pl-xs q-mt-md">Install cert-manager</div>
    <ContentCopy
      class="q-mt-sm"
      content="kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.16.1/cert-manager.yaml"
    />

    <div class="text-subtitle1 q-pl-xs q-mt-md">
      Wait for 2 minutes after installing cert-manger for the webhook to be
      ready.
    </div>

    <div class="text-subtitle1 q-pl-xs q-mt-md">Update helm repo</div>
    <ContentCopy class="q-mt-sm" :content="helmUpdateCmd" />

    <div class="text-subtitle1 q-pl-xs q-mt-md">
      Install Prometheus operator CRDs(Required by Opentelemetry operator)
    </div>
    <ContentCopy class="q-mt-sm" :content="crdCommand" />

    <div class="text-subtitle1 q-pl-xs q-mt-md">
      Install OpenTelemetry operator
    </div>
    <ContentCopy
      class="q-mt-sm"
      content="kubectl apply -f https://github.com/open-telemetry/opentelemetry-operator/releases/latest/download/opentelemetry-operator.yaml"
    />

    <div class="text-subtitle1 q-pl-xs q-mt-md">Create namespace</div>
    <ContentCopy
      class="q-mt-sm"
      content="kubectl create ns openobserve-collector"
    />

    <div class="text-subtitle1 q-pl-xs q-mt-md">
      Install OpenObserve collector
    </div>
    <div v-if="config.isCloud == 'true'">
      <ContentCopy class="q-mt-sm" :content="collectorCmd" />
    </div>
    <div v-else>
      <q-tabs v-model="tab" horizontalalign="left" no-caps>
        <q-tab
          data-test="kubernetes-default-tab"
          name="external"
          :label="t('ingestion.external')"
        />
        <q-tab
          data-test="kubernetes-this-tab"
          name="internal"
          :label="t('ingestion.internal')"
        >
          <q-tooltip>
            {{ t("ingestion.internalLabel") }}
          </q-tooltip>
        </q-tab>
      </q-tabs>
      <q-separator />
      <q-tab-panels
        v-model="tab"
        animated
        swipeable
        vertical
        transition-prev="jump-up"
        transition-next="jump-up"
      >
        <q-tab-panel name="internal" data-test="kubernetes-tab-panels-this">
          <ContentCopy class="q-mt-sm" :content="collectorCmdThisCluster" />
          <pre>
Format of the URL is: http://&lt;helm-release-name&gt;-openobserve-router.&lt;namespace&gt;.svc.cluster.local 
Make changes accordingly to the above URL.
          </pre>
        </q-tab-panel>

        <q-tab-panel name="external" data-test="kubernetes-tab-panels-default">
          <ContentCopy class="q-mt-sm" :content="collectorCmd" />
        </q-tab-panel>
      </q-tab-panels>
    </div>
    <br />
    <hr />
    <div class="text-subtitle1 q-pl-xs q-mt-md">
      Once you have installed the OpenObserve collector, it will:
      <ul class="tw-list-disc tw-ml-5">
        <li>Collect metrics from your Kubernetes cluster</li>
        <li>Collect events from your Kubernetes cluster</li>
        <li>Collect logs from your Kubernetes cluster</li>
        <li>
          Allow you to capture traces without instrumenting your applications
          that are written in following languages using OpenTelemetry
          auto-instrumentation. It can be done by setting the following to the
          pod/namespace annotations:
          <ul class="tw-list-disc tw-ml-5">
            <li>
              <b>Java:</b> instrumentation.opentelemetry.io/inject-java:
              "openobserve-collector/openobserve-java"
            </li>
            <li>
              <b>DotNet:</b> instrumentation.opentelemetry.io/inject-dotnet:
              "openobserve-collector/openobserve-dotnet"
            </li>
            <li>
              <b>NodeJS:</b> instrumentation.opentelemetry.io/inject-nodejs:
              "openobserve-collector/openobserve-nodejs"
            </li>
            <li>
              <b>Python:</b> instrumentation.opentelemetry.io/inject-python:
              "openobserve-collector/openobserve-python"
            </li>
            <li>
              <b>Go (Uses eBPF):</b>
              <ul class="tw-list-disc tw-ml-5">
                <li>
                  instrumentation.opentelemetry.io/inject-go:
                  "openobserve-collector/openobserve-go"
                </li>
                <li>
                  instrumentation.opentelemetry.io/otel-go-auto-target-exe:
                  "/path/to/container/executable"
                </li>
              </ul>
            </li>
          </ul>
        </li>
      </ul>
      You can refer and install
      <a
        href="https://github.com/openobserve/hotcommerce"
        class="hover:tw-underline text-primary"
        >HOT commerce</a
      >
      app as an example to understand how this works in practice. Refer to
      <a
        href="https://github.com/open-telemetry/opentelemetry-operator"
        class="hover:tw-underline text-primary"
        >OpenTelemetry operator</a
      >
      for further documentation.
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, type Ref } from "vue";
import type { Endpoint } from "@/ts/interfaces";
import ContentCopy from "@/components/CopyContent.vue";
import { useStore } from "vuex";
import { b64EncodeStandard } from "../../../utils/zincutils";
import config from "@/aws-exports";
import { useI18n } from "vue-i18n";

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
const tab = ref("external");
const { t } = useI18n();

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

const collectorCmd = computed(() => {
  return `helm --namespace openobserve-collector \\
  install o2c openobserve/openobserve-collector \\
  --set k8sCluster=cluster1  \\
  --set exporters."otlphttp/openobserve".endpoint=${endpoint.value.url}/api/${props.currOrgIdentifier}  \\
  --set exporters."otlphttp/openobserve".headers.Authorization="Basic [BASIC_PASSCODE]"  \\
  --set exporters."otlphttp/openobserve_k8s_events".endpoint=${endpoint.value.url}/api/${props.currOrgIdentifier}  \\
  --set exporters."otlphttp/openobserve_k8s_events".headers.Authorization="Basic [BASIC_PASSCODE]"`;
});

const collectorCmdThisCluster = computed(() => {
  return `helm --namespace openobserve-collector \\
  install o2c openobserve/openobserve-collector \\
  --set k8sCluster=cluster1  \\
  --set exporters."otlphttp/openobserve".endpoint=http://o2-openobserve-router.openobserve.svc.cluster.local:5080/api/${props.currOrgIdentifier}  \\
  --set exporters."otlphttp/openobserve".headers.Authorization="Basic [BASIC_PASSCODE]"  \\
  --set exporters."otlphttp/openobserve_k8s_events".endpoint=http://o2-openobserve-router.openobserve.svc.cluster.local:5080/api/${props.currOrgIdentifier}  \\
  --set exporters."otlphttp/openobserve_k8s_events".headers.Authorization="Basic [BASIC_PASSCODE]" \\
  --create-namespace`;
});

const crdCommand = computed(() => {
  // Club kubectl create crd commands from template
  return `kubectl create -f https://raw.githubusercontent.com/prometheus-operator/prometheus-operator/main/example/prometheus-operator-crd/monitoring.coreos.com_servicemonitors.yaml
kubectl create -f https://raw.githubusercontent.com/prometheus-operator/prometheus-operator/main/example/prometheus-operator-crd/monitoring.coreos.com_podmonitors.yaml
kubectl create -f https://raw.githubusercontent.com/prometheus-operator/prometheus-operator/refs/heads/main/example/prometheus-operator-crd/monitoring.coreos.com_scrapeconfigs.yaml
kubectl create -f https://raw.githubusercontent.com/prometheus-operator/prometheus-operator/refs/heads/main/example/prometheus-operator-crd/monitoring.coreos.com_probes.yaml`;
});

const helmUpdateCmd = computed(() => {
  return `helm repo add openobserve https://charts.openobserve.ai
helm repo update`;
});
</script>

<style scoped lang="scss"></style>
