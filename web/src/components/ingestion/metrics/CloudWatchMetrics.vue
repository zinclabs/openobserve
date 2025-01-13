<!-- Copyright 2024 OpenObserve Inc.

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
    <div class="q-ma-md">
      <CopyContent class="q-mt-sm" :content="content" />
    </div>
    <div>
      <a
        href="https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch-metric-streams-setup-datalake.html"
        class="q-ml-lg text-bold"
        style="padding-right: 2px"
        target="_blank"
        title="AWS CloudWatch Metrics - Set up a custom metric stream with Firehose"
      >
        Click here</a
      >
      to explore the process of setting up a CloudWatch custom metric stream
      with Data Firehose to OpenObserve. You may choose JSON or OpenTelemetry
      1.0 as the output format.
      <p class="q-ml-lg text-italic" style="padding-right: 2px">
        Note: Output is available under Logs with stream name
        'cloudwatch_metrics'.
      </p>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, type Ref } from "vue";
import config from "../../../aws-exports";
import { useStore } from "vuex";
import { getImageURL } from "../../../utils/zincutils";
import CopyContent from "@/components/CopyContent.vue";

export default defineComponent({
  name: "cloudwatchMetrics",
  props: {
    currOrgIdentifier: {
      type: String,
    },
    currUserEmail: {
      type: String,
    },
  },
  components: { CopyContent },
  setup(props) {
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

    const content = `HTTP Endpoint: ${endpoint.value.url}/aws/${store.state.selectedOrganization.identifier}/cloudwatch_metrics/_kinesis_firehose
Access Key: [BASIC_PASSCODE]`;
    return {
      store,
      config,
      endpoint,
      content,
      getImageURL,
    };
  },
});
</script>
