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
  <div class="q-ma-md" :key="rumToken">
    <div v-if="rumToken">
      <div class="text-h6 q-mt-xs" data-test="rumweb-title-text">
        {{ t("ingestion.npmStepTitle") }}
      </div>
      <q-separator class="q-my-sm"></q-separator>

      <SanitizedHtmlRenderer
        class="text-subtitle1 q-mt-xs"
        :htmlContent="npmStep1"
      />

      <copy-content
        content="npm i @openobserve/browser-rum @openobserve/browser-logs"
      ></copy-content>

      <br />
      <SanitizedHtmlRenderer
        class="text-subtitle1 q-mt-xs"
        :htmlContent="npmStep2"
      />
      <CopyContent
        :key="displayConfiguration"
        :content="initConfiguration"
        :displayContent="displayConfiguration"
      ></CopyContent>
    </div>
    <div v-else class="q-mt-xs">
      {{ t("ingestion.generateRUMTokenMessage") }}
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, onMounted, onUpdated, onActivated } from "vue";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { getImageURL, maskText } from "../../../utils/zincutils";
import CopyContent from "../../CopyContent.vue";
import SanitizedHtmlRenderer from "@/components/SanitizedHtmlRenderer.vue";

export default defineComponent({
  name: "rum-web-page",
  components: {
    CopyContent,
    SanitizedHtmlRenderer,
  },
  props: {
    currOrgIdentifier: {
      type: String,
    },
    currUserEmail: {
      type: String,
    },
  },
  setup(props) {
    const store = useStore();
    const { t } = useI18n();

    const npmStep1 = ref(
      "<b>Step1: </b>Add <a href='https://www.npmjs.com/package/&#64;openobserve/browser-rum' style='color:darkorange' target='_blank'>&#64;openobserve/browser-rum</a> and <a href='https://www.npmjs.com/package/&#64;openobserve/browser-logs' style='color:darkorange' target='_blank'>&#64;openobserve/browser-logs</a> to your package.json file, or run the following command:"
    );
    const npmStep2 = ref(
      "<b>Step2: </b>Initialize the OpenObserve RUM and Logs SDKs in your application entry point (e.g. index.js or main.js)."
    );
    const rumToken = ref("");
    const defaultConfig = `
import { openobserveRum } from '@openobserve/browser-rum';
import { openobserveLogs } from '@openobserve/browser-logs';

const options = {
  clientToken: '<OPENOBSERVE_CLIENT_TOKEN>',
  applicationId: 'web-application-id',
  site: '<OPENOBSERVE_SITE>',
  service: 'my-web-application',
  env: 'production',
  version: '0.0.1',
  organizationIdentifier: '<OPENOBSERVE_ORGANIZATION_IDENTIFIER>',
  insecureHTTP: <INSECUREHTTP>,
  apiVersion: 'v1',
};

openobserveRum.init({
  applicationId: options.applicationId, // required, any string identifying your application
  clientToken: options.clientToken,
  site: options.site,
  organizationIdentifier: options.organizationIdentifier,
  service: options.service,
  env: options.env,
  version: options.version,
  trackResources: true,
  trackLongTasks: true,
  trackUserInteractions: true,
  apiVersion: options.apiVersion,
  insecureHTTP: options.insecureHTTP,
  defaultPrivacyLevel: 'allow' // 'allow' or 'mask-user-input' or 'mask'. Use one of the 3 values.
});

openobserveLogs.init({
  clientToken: options.clientToken,
  site: options.site,
  organizationIdentifier: options.organizationIdentifier,
  service: options.service,
  env: options.env,
  version: options.version,
  forwardErrorsToLogs: true,
  insecureHTTP: options.insecureHTTP,
  apiVersion: options.apiVersion,
});

// You can set a user context
openobserveRum.setUser({
  id: "1",
  name: "Captain Hook",
  email: "captainhook@example.com",
});

openobserveRum.startSessionReplayRecording();`;
    const initConfiguration = ref(defaultConfig);
    const displayConfiguration = ref(defaultConfig);

    onMounted(() => {
      if (store.state.organizationData.rumToken) {
        rumToken.value = store.state.organizationData.rumToken.rum_token;
      }
    });

    onUpdated(() => {
      if (store.state.organizationData.rumToken) {
        replaceStaticValues();
      }
    });

    onActivated(() => {
      replaceStaticValues();
    });

    const replaceStaticValues = () => {
      rumToken.value = store.state.organizationData.rumToken.rum_token;
      let configData = defaultConfig;

      let ingestionURL: string = store.state.API_ENDPOINT;
      if (
        Object.hasOwn(store.state.zoConfig, "ingestion_url") &&
        store.state.zoConfig.ingestion_url !== ""
      ) {
        ingestionURL = store.state.zoConfig.ingestion_url;
      }

      configData = configData.replace(
        /<OPENOBSERVE_SITE>/g,
        ingestionURL
          .replace("https://", "")
          .replace("http://", "")
          .replace(/\/$/, "")
      );

      configData = configData.replace(
        /<OPENOBSERVE_ORGANIZATION_IDENTIFIER>/g,
        store.state.selectedOrganization.identifier
      );

      if (store.state.API_ENDPOINT.indexOf("https://") > -1) {
        configData = configData.replace(/<INSECUREHTTP>/g, "false");
      } else {
        configData = configData.replace(/<INSECUREHTTP>/g, "true");
      }

      initConfiguration.value = configData.replace(
        /<OPENOBSERVE_CLIENT_TOKEN>/g,
        rumToken.value
      );

      displayConfiguration.value = configData.replace(
        /<OPENOBSERVE_CLIENT_TOKEN>/g,
        maskText(rumToken.value)
      );
    };

    return {
      t,
      store,
      getImageURL,
      rumToken,
      npmStep1,
      npmStep2,
      initConfiguration,
      displayConfiguration,
      replaceStaticValues,
    };
  },
  computed: {
    checkRUMToken() {
      return this.store.state.organizationData.rumToken;
    },
  },
  watch: {
    checkRUMToken() {
      this.rumToken = this.store.state.organizationData.rumToken.key;
      this.replaceStaticValues();
    },
  },
});
</script>
