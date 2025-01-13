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
  <q-page>
    <div
      style="text-align: center; width: 100%; font-size: 30px; margin: 40px 0px"
    >
      Member Subscription
    </div>
    <div v-if="status == 'processing'">{{ message }}</div>
    <div
      v-else-if="status == 'error' && error == ''"
      style="text-align: center"
    >
      Error while processing member subscription request.<br /><br />
    </div>

    <SanitizedHtmlRenderer
      v-else-if="status == 'error' && error !== ''"
      :htmlContent="error"
      class="subscription_message"
    />

    <div v-else>Thank you for your subscription.</div>

    <!-- <div
      v-if="status == 'error' && error !== ''"
      class="subscription_message q-btn-primary"
    >
      <b>Please click the button below to proceed with your subscription after taking above mentioned action.</b><br />
      <q-btn @click="ProcessSubscription(queryString, 'confirm')" class="q-mt-md">Confirm Member Subscription</q-btn>
    </div> -->
  </q-page>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { useQuasar } from "quasar";
import { useRouter } from "vue-router";
import { useStore } from "vuex";
import { useLocalOrganization, getPath } from "../utils/zincutils";

import organizationsService from "../services/organizations";
import SanitizedHtmlRenderer from "@/components/SanitizedHtmlRenderer.vue";

export default defineComponent({
  name: "PageUser",
  components: {
    SanitizedHtmlRenderer,
  },
  data() {
    return {
      status: "processing",
      message: "Please wait while we process your request...",
      error: "",
      queryString: this.$route.hash.split("=")[1],
    };
  },
  created() {
    const propArr = this.$route.hash.split("=");
    this.ProcessSubscription(propArr[1], "confirm");
  },
  methods: {
    async ProcessSubscription(s: any, action: string) {
      const baseURL = getPath();
      const route = this.$router.resolve({ name: "organizations" });
      const redirectURI = route.href || baseURL;
      await organizationsService
        .process_subscription(s, action, this.$store.state.selectedOrganization.identifier)
        .then((res) => {
          this.status = "completed";
          const dismiss = this.$q.notify({
            type: "positive",
            message: res.data.message,
          });

          if (res.data.hasOwnProperty("data")) {
            res.data.data.label = res.data.data.name;
            useLocalOrganization(res.data.data);

            window.location.href = redirectURI;
          } else {
            window.location.href = redirectURI;
          }
          return res;
        })
        .catch((e) => {
          this.status = "error";
          this.error = e.response.data.message.replaceAll("[BASE_URL]", baseURL);
        });
    },
  },
  setup() {
    const $store = useStore();
    const $q = useQuasar();
    const $router = useRouter();

    return {
      $router,
      $store,
    };
  },
});
</script>

<style lang="scss" scoped>
.subscription_message {
  font-size: 16px;
  line-height: 30px;
  width: 70%;
  margin: auto;
  text-align: left;
}
</style>
