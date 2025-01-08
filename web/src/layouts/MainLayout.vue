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
  <q-layout
    view="hHh Lpr lff"
    :class="[store.state.printMode === true ? 'printMode' : '']"
  >
    <q-header
      :class="[store?.state?.theme == 'dark' ? 'dark-mode' : 'bg-white']"
    >
      <q-toolbar class="o2-bg-color">
        <div
          class="flex relative-position q-mr-sm"
          v-if="
            (config.isEnterprise == 'true' &&
              store.state.zoConfig.hasOwnProperty('custom_logo_text') &&
              store.state.zoConfig.custom_logo_text != '') ||
            (config.isEnterprise == 'true' &&
              store.state.zoConfig.hasOwnProperty('custom_logo_img') &&
              store.state.zoConfig.custom_logo_img != null)
          "
        >
          <span
            v-if="
              store.state.zoConfig.hasOwnProperty('custom_logo_text') &&
              store.state.zoConfig?.custom_logo_text != ''
            "
            class="text-h6 text-bold q-pa-none cursor-pointer q-mr-sm"
            @click="goToHome"
            >{{ store.state.zoConfig.custom_logo_text }}</span
          >
          <img
            v-if="
              store.state.zoConfig.hasOwnProperty('custom_logo_img') &&
              store.state.zoConfig?.custom_logo_img != null
            "
            :src="
              `data:image; base64, ` + store.state.zoConfig?.custom_logo_img
            "
            style="max-width: 150px; max-height: 31px"
          />
          <img
            v-if="store.state.zoConfig.custom_hide_self_logo == false"
            class="appLogo"
            :src="
              store?.state?.theme == 'dark'
                ? getImageURL('images/common/open_observe_logo_2.svg')
                : getImageURL('images/common/open_observe_logo.svg')
            "
            @click="goToHome"
          />
        </div>
        <div v-else class="flex relative-position q-mr-sm">
          <img
            class="appLogo"
            :src="getImageURL('images/common/open_observe_logo.svg')"
            @click="goToHome"
          />
          <span v-if="config.isCloud == 'true'" class="absolute beta-text"
            >Beta</span
          >
        </div>

        <q-toolbar-title></q-toolbar-title>
        <div
          class="headerMenu float-left"
          v-if="store.state.organizationData.quotaThresholdMsg"
        >
          <div
            type="warning"
            icon="cloud"
            class="warning-msg"
            style="display: inline"
          >
            <q-icon name="warning" size="xs" class="warning" />{{
              store.state.organizationData.quotaThresholdMsg
            }}
          </div>
          <q-btn
            color="secondary"
            size="sm"
            style="display: inline; padding: 5px 10px"
            rounded
            borderless
            dense
            class="q-ma-xs"
            @click="router.replace('/billings/plans')"
            >Upgrade to PRO Plan</q-btn
          >
        </div>
        <div
          data-test="navbar-organizations-select"
          class="q-mx-sm current-organization"
        >
          <q-select
            v-model="selectedOrg"
            borderless
            dense
            :options="orgOptions"
            option-label="identifier"
            class="q-px-none q-py-none q-mx-none q-my-none organizationlist"
            @update:model-value="updateOrganization()"
          />
        </div>
        <!-- <div>
          <q-btn-dropdown
            data-test="language-dropdown"
            unelevated
            no-caps
            dense
            flat
            class="q-pa-xs q-ma-none"
            :icon="selectedLanguage.icon"
          >
            <q-list class="languagelist q-pa-none">
              <q-item
                data-test="language-dropdown-item"
                v-for="lang in langList"
                :key="lang.code"
                v-ripple="true"
                v-close-popup="true"
                clickable
                dense
                v-bind="lang"
                active-class="activeLang"
                @click="changeLanguage(lang)"
              >
                <q-item-section avatar>
                  <q-icon size="xs" :name="lang.icon"
class="padding-none" />
                </q-item-section>

                <q-item-section
                  :data-test="`language-dropdown-item-${lang.code}`"
                >
                  <q-item-label>{{ lang.label }}</q-item-label>
                </q-item-section>
              </q-item>
            </q-list>
          </q-btn-dropdown>
        </div> -->

        <ThemeSwitcher></ThemeSwitcher>

        <q-btn
          round
          flat
          dense
          :ripple="false"
          @click="openSlack"
          data-test="menu-link-slack-item"
        >
          <div class="row items-center no-wrap">
            <q-icon
              ><component :is="slackIcon" size="25px" class="header-icon"
            /></q-icon>
          </div>
          <q-tooltip anchor="top middle" self="bottom middle">
            {{ t("menu.slack") }}
          </q-tooltip>
        </q-btn>
        <q-btn round flat dense :ripple="false">
          <div class="row items-center no-wrap">
            <q-icon
              name="help_outline"
              size="25px"
              class="header-icon"
            ></q-icon>
            <q-tooltip anchor="top middle" self="bottom middle">
              {{ t("menu.help") }}
            </q-tooltip>
          </div>

          <q-menu
            fit
            anchor="bottom right"
            self="top right"
            transition-show="jump-down"
            transition-hide="jump-up"
            class="header-menu-bar"
          >
            <q-list style="min-width: 250px">
              <div
                v-if="
                  config.isCloud !== 'true' &&
                  !store.state.zoConfig?.custom_hide_menus
                    ?.split(',')
                    ?.includes('openapi')
                "
              >
                <q-item clickable @click="navigateToOpenAPI(zoBackendUrl)">
                  <q-item-section>
                    <q-item-label>
                      {{ t(`menu.openapi`) }}
                    </q-item-label>
                  </q-item-section>
                </q-item>
                <q-separator />
              </div>
              <q-item clickable @click="navigateToDocs()">
                <q-item-section>
                  <q-item-label>
                    {{ t(`menu.docs`) }}
                  </q-item-label>
                </q-item-section>
              </q-item>
            </q-list>
          </q-menu>
        </q-btn>

        <q-btn
          data-test="menu-link-settings-item"
          round
          flat
          dense
          :ripple="false"
          @click="router.push({ name: 'settings' })"
        >
          <div class="row items-center no-wrap">
            <q-icon
              :name="outlinedSettings"
              size="25px"
              class="header-icon"
            ></q-icon>
          </div>
          <q-tooltip anchor="top middle" self="bottom middle">
            {{ t("menu.settings") }}
          </q-tooltip>
        </q-btn>

        <q-btn
          round
          flat
          dense
          :ripple="false"
          data-test="header-my-account-profile-icon"
        >
          <div class="row items-center no-wrap">
            <q-icon
              :name="user.picture ? user.picture : 'person'"
              size="25px"
              class="header-icon"
            ></q-icon>
            <q-tooltip anchor="top middle" self="bottom middle">
              {{
                user.given_name
                  ? user.given_name + " " + user.family_name
                  : user.email
              }}</q-tooltip
            >
          </div>

          <q-menu
            fit
            anchor="bottom right"
            self="top right"
            transition-show="jump-down"
            transition-hide="jump-up"
            class="header-menu-bar"
          >
            <q-list style="min-width: 250px">
              <q-item>
                <q-item-section avatar>
                  <q-icon
                    :name="user.picture ? user.picture : 'person'"
                    size="xs"
                  ></q-icon>
                </q-item-section>
                <q-item-section>
                  <q-item-label>{{
                    user.given_name
                      ? user.given_name + " " + user.family_name
                      : user.email
                  }}</q-item-label>
                </q-item-section>
              </q-item>
              <q-separator />
              <div v-if="config.isCloud == 'true'">
                <q-item
                  v-ripple="true"
                  v-close-popup="true"
                  clickable
                  :to="{ path: '/settings' }"
                >
                  <q-item-section avatar>
                    <q-avatar
                      size="md"
                      icon="settings"
                      color="red"
                      text-color="white"
                    />
                  </q-item-section>
                  <q-item-section>
                    <q-item-label>{{ t("menu.settings") }}</q-item-label>
                  </q-item-section>
                </q-item>
                <q-separator />
              </div>
              <q-item clickable>
                <q-item-section avatar>
                  <q-icon size="xs" name="language" class="padding-none" />
                </q-item-section>
                <q-item-section>
                  <q-item-label>{{ t("menu.language") }}</q-item-label>
                </q-item-section>
                <q-item-section></q-item-section>
                <q-item-section side>
                  <div class="q-gutter-xs">
                    <q-icon
                      size="xs"
                      :name="selectedLanguage.icon"
                      class="padding-none"
                    />
                    <span
                      class="cursor-pointer vertical-bottom q-mt-sm selected-lang-label"
                      >{{ selectedLanguage.label }}</span
                    >
                  </div>
                </q-item-section>
                <q-item-section side style="padding-left: 0px">
                  <q-icon
                    class="icon-ley-arrow-right"
                    name="keyboard_arrow_right"
                  />
                </q-item-section>

                <q-menu
                  auto-close
                  anchor="top end"
                  self="top start"
                  data-test="language-dropdown-item"
                  class="header-menu-bar"
                >
                  <q-list>
                    <q-item
                      v-for="lang in langList"
                      :key="lang.code"
                      v-bind="lang"
                      dense
                      clickable
                      @click="changeLanguage(lang)"
                    >
                      <q-item-section avatar>
                        <q-icon
                          size="xs"
                          :name="lang.icon"
                          class="padding-none"
                        />
                      </q-item-section>

                      <q-item-section
                        :data-test="`language-dropdown-item-${lang.code}`"
                      >
                        <q-item-label>{{ lang.label }}</q-item-label>
                      </q-item-section>
                    </q-item>
                  </q-list>
                </q-menu>
              </q-item>
              <q-separator />
              <q-item
                data-test="menu-link-logout-item"
                v-ripple="true"
                v-close-popup="true"
                clickable
                @click="signout"
              >
                <q-item-section avatar>
                  <q-icon size="xs" name="exit_to_app" class="padding-none" />
                </q-item-section>
                <q-item-section>
                  <q-item-label>{{ t("menu.signOut") }}</q-item-label>
                </q-item-section>
              </q-item>
            </q-list>
          </q-menu>
        </q-btn>
      </q-toolbar>
    </q-header>

    <q-drawer
      v-model="drawer"
      show-if-above
      :width="80"
      :breakpoint="500"
      bordered
      class="o2-bg-color"
    >
      <q-list class="leftNavList">
        <menu-link
          v-for="nav in linksList"
          :key="nav.title"
          :link-name="nav.name"
          v-bind="{ ...nav, mini: miniMode }"
        />
      </q-list>
    </q-drawer>
    <q-page-container
      :key="store.state.selectedOrganization?.identifier"
      v-if="isLoading"
    >
      <router-view v-slot="{ Component }">
        <template v-if="$route.meta.keepAlive">
          <keep-alive>
            <component :is="Component" />
          </keep-alive>
        </template>
        <template v-else>
          <component :is="Component" />
        </template>
      </router-view>
    </q-page-container>
  </q-layout>
</template>

<script lang="ts">
import {
  QPage,
  QPageContainer,
  QLayout,
  QDrawer,
  QList,
  QItem,
  QItemLabel,
  QItemSection,
  QBtn,
  QBtnDropdown,
  QToolbarTitle,
  QHeader,
  QToolbar,
  QAvatar,
  QIcon,
  QSelect,
  useQuasar,
} from "quasar";
import MenuLink from "../components/MenuLink.vue";
import { useI18n } from "vue-i18n";
import {
  useLocalCurrentUser,
  useLocalOrganization,
  useLocalUserInfo,
  getImageURL,
  invlidateLoginData,
  getLogoutURL,
} from "../utils/zincutils";

import {
  ref,
  defineComponent,
  KeepAlive,
  computed,
  onMounted,
  watch,
  markRaw,
  nextTick,
  onBeforeMount,
} from "vue";
import { useStore } from "vuex";
import { useRouter, RouterView } from "vue-router";
import config from "../aws-exports";

import { setLanguage } from "../utils/cookies";
import { getLocale } from "../locales";

import MainLayoutOpenSourceMixin from "@/mixins/mainLayout.mixin";
import MainLayoutCloudMixin from "@/enterprise/mixins/mainLayout.mixin";

import configService from "@/services/config";
import streamService from "@/services/stream";
import billings from "@/services/billings";
import ThemeSwitcher from "../components/ThemeSwitcher.vue";
import {
  outlinedHome,
  outlinedSearch,
  outlinedBarChart,
  outlinedAccountTree,
  outlinedDashboard,
  outlinedWindow,
  outlinedReportProblem,
  outlinedFilterAlt,
  outlinedPerson,
  outlinedFormatListBulleted,
  outlinedSettings,
  outlinedManageAccounts,
  outlinedDescription,
} from "@quasar/extras/material-icons-outlined";
import SlackIcon from "@/components/icons/SlackIcon.vue";
import ManagementIcon from "@/components/icons/ManagementIcon.vue";
import organizations from "@/services/organizations";
import useStreams from "@/composables/useStreams";
import { openobserveRum } from "@openobserve/browser-rum";

let mainLayoutMixin: any = null;
if (config.isCloud == "true") {
  mainLayoutMixin = MainLayoutCloudMixin;
} else {
  mainLayoutMixin = MainLayoutOpenSourceMixin;
}

export default defineComponent({
  name: "MainLayout",
  mixins: [mainLayoutMixin],
  components: {
    "menu-link": MenuLink,
    "keep-alive": KeepAlive,
    "q-page": QPage,
    "q-page-container": QPageContainer,
    "q-layout": QLayout,
    "q-drawer": QDrawer,
    "q-list": QList,
    "q-item": QItem,
    "q-item-label": QItemLabel,
    "q-item-section": QItemSection,
    "q-btn": QBtn,
    "q-btn-dropdown": QBtnDropdown,
    "q-toolbar-title": QToolbarTitle,
    "q-header": QHeader,
    "q-toolbar": QToolbar,
    "router-view": RouterView,
    "q-avatar": QAvatar,
    "q-icon": QIcon,
    "q-select": QSelect,
    SlackIcon,
    ManagementIcon,
    ThemeSwitcher,
  },
  methods: {
    navigateToDocs() {
      let docURL = "https://openobserve.ai/docs";
      if (
        this.config.isEnterprise == "true" &&
        this.store.state.zoConfig.custom_docs_url != ""
      ) {
        docURL = this.store.state.zoConfig.custom_docs_url;
      }
      window.open(docURL, "_blank");
    },
    navigateToOpenAPI(zoBackendUrl: string) {
      window.open(zoBackendUrl + "/swagger/index.html", "_blank");
    },
    signout() {
      if (config.isEnterprise == "true") {
        invlidateLoginData();
      }

      const logoutURL = getLogoutURL();
      this.store.dispatch("logout");

      useLocalCurrentUser("", true);
      useLocalUserInfo("", true);

      if (config.isCloud == "true") {
        window.location.href = logoutURL;
      }
      this.$router.push("/logout");
    },
    goToHome() {
      this.$router.push("/");
    },
    changeLanguage(item: { code: string; label: string; icon: string }) {
      setLanguage(item.code);
      window.location.reload();
    },
  },
  setup() {
    const store: any = useStore();
    const router: any = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const miniMode = ref(false);
    const zoBackendUrl = store.state.API_ENDPOINT;
    const isLoading = ref(false);
    const { getStreams, resetStreams } = useStreams();

    const isMonacoEditorLoaded = ref(false);

    let customOrganization = router.currentRoute.value.query.hasOwnProperty(
      "org_identifier",
    )
      ? router.currentRoute.value.query.org_identifier
      : undefined;
    const selectedOrg = ref(store.state.selectedOrganization);
    const excludeParentRedirect = [
      "pipeline",
      "functionList",
      "streamFunctions",
      "enrichmentTables",
      "alertList",
      "alertDestinations",
      "alertTemplates",
      "/ingestion/",
    ];

    const orgOptions = ref([{ label: Number, value: String }]);
    let slackURL = "https://short.openobserve.ai/community";
    if (
      config.isEnterprise == "true" &&
      store.state.zoConfig.custom_slack_url != ""
    ) {
      slackURL = store.state.zoConfig.custom_slack_url;
    }

    let user = store.state.userInfo;

    var linksList = ref([
      {
        title: t("menu.home"),
        icon: outlinedHome,
        link: "/",
        exact: true,
        name: "home",
      },
      {
        title: t("menu.search"),
        icon: outlinedSearch,
        link: "/logs",
        name: "logs",
      },
      {
        title: t("menu.metrics"),
        icon: outlinedBarChart,
        link: "/metrics",
        name: "metrics",
      },
      {
        title: t("menu.traces"),
        icon: outlinedAccountTree,
        link: "/traces",
        name: "traces",
      },
      {
        title: t("menu.rum"),
        icon: "devices",
        link: "/rum/performance/overview",
        name: "rum",
      },
      {
        title: t("menu.dashboard"),
        icon: outlinedDashboard,
        link: "/dashboards",
        name: "dashboards",
      },
      {
        title: t("menu.index"),
        icon: outlinedWindow,
        link: "/streams",
        name: "streams",
      },
      {
        title: t("menu.alerts"),
        icon: outlinedReportProblem,
        link: "/alerts",
        name: "alertList",
      },
      {
        title: t("menu.ingestion"),
        icon: outlinedFilterAlt,
        link: "/ingestion",
        name: "ingestion",
      },
      {
        title: t("menu.iam"),
        icon: outlinedManageAccounts,
        link: "/iam",
        display: store.state?.currentuser?.role == "admin" ? true : false,
        name: "iam",
      },
      {
        title: t("menu.about"),
        icon: outlinedFormatListBulleted,
        link: "/about",
        name: "about",
      },
    ]);

    const langList = [
      {
        label: "English",
        code: "en-gb",
        icon: "img:" + getImageURL("images/language_flags/en-gb.svg"),
      },
      {
        label: "Türkçe",
        code: "tr-turk",
        icon: "img:" + getImageURL("images/language_flags/tr-turk.svg"),
      },
      {
        label: "简体中文",
        code: "zh-cn",
        icon: "img:" + getImageURL("images/language_flags/zh-cn.svg"),
      },
      {
        label: "Français",
        code: "fr",
        icon: "img:" + getImageURL("images/language_flags/fr.svg"),
      },
      {
        label: "Español",
        code: "es",
        icon: "img:" + getImageURL("images/language_flags/es.svg"),
      },
      {
        label: "Deutsch",
        code: "de",
        icon: "img:" + getImageURL("images/language_flags/de.svg"),
      },
      {
        label: "Italiano",
        code: "it",
        icon: "img:" + getImageURL("images/language_flags/it.svg"),
      },
      {
        label: "日本語",
        code: "ja",
        icon: "img:" + getImageURL("images/language_flags/ja.svg"),
      },
      {
        label: "한국어",
        code: "ko",
        icon: "img:" + getImageURL("images/language_flags/ko.svg"),
      },
      {
        label: "Nederlands",
        code: "nl",
        icon: "img:" + getImageURL("images/language_flags/nl.svg"),
      },
      {
        label: "Português",
        code: "pt",
        icon: "img:" + getImageURL("images/language_flags/pt.svg"),
      },
    ];

    onBeforeMount(() => {
      try {
        const url = new URL(window.location.href);
        const localOrg: any = useLocalOrganization();
        if (
          Object.keys(localOrg.value).length == 0 &&
          url.searchParams.get("org_identifier") != null
        ) {
          localOrg.value = {
            identifier: url.searchParams.get("org_identifier"),
            user_email: store.state.userInfo.email,
          };

          selectedOrg.value = localOrg.value;
          useLocalOrganization(localOrg.value);
          store.dispatch("setSelectedOrganization", localOrg.value);
        }
      } catch (error) {
        console.error("Error in onBeforeMount:", error);
      }
    });

    onMounted(async () => {
      filterMenus();

      // TODO OK : Clean get config functions which sets rum user and functions menu. Move it to common method.
      if (
        !store.state.zoConfig.hasOwnProperty("version") ||
        store.state.zoConfig.version == ""
      ) {
        getConfig();
      } else {
        if (config.isCloud == "false") {
          linksList.value = mainLayoutMixin
            .setup()
            .leftNavigationLinks(linksList, t);
          filterMenus();
        }
        await nextTick();
        // if rum enabled then setUser to capture session details.
        if (store.state.zoConfig.rum?.enabled) {
          setRumUser();
        }
      }
    });

    const selectedLanguage: any =
      langList.find((l) => l.code == getLocale()) || langList[0];

    const filterMenus = () => {
      const disableMenus = new Set(
        store.state.zoConfig?.custom_hide_menus
          ?.split(",")
          ?.filter((val: string) => val?.trim()) || [],
      );

      store.dispatch("setHiddenMenus", disableMenus);

      linksList.value = linksList.value.filter(
        (link: { name: string }) => !disableMenus.has(link.name),
      );
    };

    // additional links based on environment and conditions
    if (config.isCloud == "true") {
      linksList.value = mainLayoutMixin
        .setup()
        .leftNavigationLinks(linksList, t);
      filterMenus();
    } else {
      linksList.value.splice(7, 0, {
        title: t("menu.report"),
        icon: outlinedDescription,
        link: "/reports",
        name: "reports",
      });
    }

    //orgIdentifier query param exists then clear the localstorage and store.
    if (store.state.selectedOrganization != null) {
      if (
        mainLayoutMixin.setup().customOrganization != undefined &&
        mainLayoutMixin.setup().customOrganization !=
          store.state.selectedOrganization?.identifier
      ) {
        useLocalOrganization("");
        store.dispatch("setSelectedOrganization", {});
      }
    }

    const triggerRefreshToken = () => {
      const expirationTimeUnix = store.state.userInfo.exp;

      // Convert the expiration time to milliseconds
      const expirationTimeMilliseconds = expirationTimeUnix * 1000;

      // Get the current time in milliseconds
      const currentTimeMilliseconds = Date.now();

      // Calculate the time difference
      const timeUntilNextAPICall =
        expirationTimeMilliseconds - currentTimeMilliseconds - 100;

      // Convert the time difference from milliseconds to seconds
      const timeUntilNextAPICallInSeconds = timeUntilNextAPICall / 1000;

      setTimeout(() => {
        mainLayoutMixin.setup().getRefreshToken();
      }, timeUntilNextAPICallInSeconds);
    };

    //get refresh token for cloud environment
    if (store.state.hasOwnProperty("userInfo") && store.state.userInfo.email) {
      if (config.isCloud == "true") {
        triggerRefreshToken();
      }
    }

    const updateOrganization = async () => {
      resetStreams();
      store.dispatch("setIsDataIngested", false);
      const orgIdentifier = selectedOrg.value.identifier;
      const queryParams =
        router.currentRoute.value.path.indexOf(".logs") > -1
          ? router.currentRoute.value.query
          : {};
      router.push({
        path: router.currentRoute.value.path,
        query: {
          ...queryParams,
          org_identifier: orgIdentifier,
        },
      });
      useLocalOrganization(selectedOrg.value);
      store.dispatch("setSelectedOrganization", { ...selectedOrg.value });
      // setSelectedOrganization();
      // if (
      //   config.isCloud &&
      //   selectedOrg.value.subscription_type == config.freePlan
      // ) {
      //   await billings
      //     .list_subscription(selectedOrg.value.identifier)
      //     .then(async (res: any) => {
      //       if (res.data.data.length == 0) {
      //         router.push({ name: "plans" });
      //       } else if (
      //         res.data.data.CustomerBillingObj.customer_id == null ||
      //         res.data.data.CustomerBillingObj.customer_id == ""
      //       ) {
      //         router.push({ name: "plans" });
      //       } else {
      //         await verifyStreamExist(selectedOrg.value);
      //       }
      //     });
      // } else {
      if (
        store.state.zoConfig.hasOwnProperty(
          "restricted_routes_on_empty_data",
        ) &&
        store.state.zoConfig.restricted_routes_on_empty_data == true &&
        store.state.organizationData.isDataIngested == false
      ) {
        await verifyStreamExist(selectedOrg.value);
      }
    };

    const verifyStreamExist = async (selectedOrgData: any) => {
      await getStreams("", false).then((response: any) => {
        store.dispatch("setSelectedOrganization", {
          ...selectedOrgData,
        });
        if (response.list.length == 0) {
          store.dispatch("setIsDataIngested", false);
          $q.notify({
            type: "warning",
            message:
              "You haven't initiated the data ingestion process yet. To explore other pages, please start the data ingestion.",
            timeout: 5000,
          });
          router.push({ name: "ingestion" });
        } else {
          store.dispatch("setIsDataIngested", true);
        }
      });
    };

    const setSelectedOrganization = async () => {
      try {
        customOrganization = router.currentRoute.value.query.hasOwnProperty(
          "org_identifier",
        )
          ? router.currentRoute.value.query.org_identifier
          : "";
        let tempDefaultOrg = {};
        let localOrgFlag = false;
        const url = new URL(window.location.href);
        if (store.state.organizations?.length > 0) {
          const localOrg: any = useLocalOrganization();
          if (
            Object.keys(localOrg.value).length == 0 &&
            url.searchParams.get("org_identifier") != null
          ) {
            localOrg.value = {
              identifier: url.searchParams.get("org_identifier"),
              user_email: store.state.userInfo.email,
            };
          }
          orgOptions.value = store.state.organizations.map(
            (data: {
              id: any;
              name: any;
              type: any;
              identifier: any;
              UserObj: any;
              ingest_threshold: number;
              search_threshold: number;
              CustomerBillingObj: { subscription_type: string; note: string };
              status: string;
            }) => {
              const optiondata: any = {
                label: data.name,
                id: data.id,
                identifier: data.identifier,
                user_email: store.state.userInfo.email,
                ingest_threshold: data.ingest_threshold,
                search_threshold: data.search_threshold,
                subscription_type: data.hasOwnProperty("CustomerBillingObj")
                  ? data.CustomerBillingObj.subscription_type
                  : "",
                status: data.status,
                note: data.hasOwnProperty("CustomerBillingObj")
                  ? data.CustomerBillingObj.note
                  : "",
              };

              if (
                config.isCloud == "true" &&
                localOrg.value?.identifier == data?.identifier &&
                (customOrganization == "" || customOrganization == undefined)
              ) {
                // localOrg.value.subscription_type =
                //   data.CustomerBillingObj.subscription_type;
                // useLocalOrganization(localOrg.value);
                useLocalOrganization(optiondata);
              }

              if (
                localOrg.value.identifier == data.identifier ||
                url.searchParams.get("org_identifier") == data.identifier
              ) {
                localOrgFlag = true;
              }

              if (
                (Object.keys(selectedOrg.value).length == 0 &&
                  data.type == "default" &&
                  store.state.userInfo.email == data.UserObj.email &&
                  (customOrganization == "" ||
                    customOrganization == undefined)) ||
                (store.state.organizations?.length == 1 &&
                  (customOrganization == "" || customOrganization == undefined))
              ) {
                selectedOrg.value = localOrg.value
                  ? localOrg.value
                  : optiondata;
                useLocalOrganization(optiondata);
                store.dispatch("setSelectedOrganization", optiondata);
              } else if (data.identifier == customOrganization) {
                selectedOrg.value = optiondata;
                useLocalOrganization(optiondata);
                store.dispatch("setSelectedOrganization", optiondata);
              }

              if (data.type == "default") {
                tempDefaultOrg = optiondata;
              }

              return optiondata;
            },
          );
        }

        if (localOrgFlag == false) {
          selectedOrg.value = tempDefaultOrg;
          useLocalOrganization(tempDefaultOrg);
          store.dispatch("setSelectedOrganization", tempDefaultOrg);
        }

        if (
          Object.keys(selectedOrg.value).length == 0 &&
          store.state.organizations.length > 0
        ) {
          let data = store.state.organizations[0];
          let optiondata = {
            label: data.name,
            id: data.id,
            identifier: data.identifier,
            user_email: store.state.userInfo.email,
            ingest_threshold: data.ingest_threshold,
            search_threshold: data.search_threshold,
            subscription_type: data.hasOwnProperty("CustomerBillingObj")
              ? data.CustomerBillingObj.subscription_type
              : "",
            status: data.status,
            note: data.hasOwnProperty("CustomerBillingObj")
              ? data.CustomerBillingObj.note
              : "",
          };
          selectedOrg.value = optiondata;
          useLocalOrganization(optiondata);
          store.dispatch("setSelectedOrganization", optiondata);
        }

        if (router.currentRoute.value.query.action == "subscribe") {
          router.push({
            name: "plans",
          });
        }

        if (selectedOrg.value.identifier != "" && config.isCloud == "true") {
          mainLayoutMixin.setup().getOrganizationThreshold(store);
        }

        if (
          Object.keys(selectedOrg.value).length > 0 &&
          selectedOrg.value.identifier != "" &&
          selectedOrg.value.identifier != undefined
        ) {
          await getOrganizationSettings();
          isLoading.value = true;
        }
      } catch (error) {
        console.error("Error in setSelectedOrganization:", error);
      }
    };

    // get organizations settings on first load and identifier change
    const getOrganizationSettings = async () => {
      try {
        //get organizations settings
        const orgSettings: any = await organizations.get_organization_settings(
          store.state?.selectedOrganization?.identifier,
        );

        //set settings in store
        //scrape interval will be in number
        store.dispatch("setOrganizationSettings", {
          scrape_interval: orgSettings?.data?.data?.scrape_interval ?? 15,
          span_id_field_name:
            orgSettings?.data?.data?.span_id_field_name ?? "spanId",
          trace_id_field_name:
            orgSettings?.data?.data?.trace_id_field_name ?? "traceId",
          toggle_ingestion_logs:
            orgSettings?.data?.data?.toggle_ingestion_logs ?? false,
          enable_websocket_search:
            orgSettings?.data?.data?.enable_websocket_search ?? false,
        });
      } catch (error) {
        console.error("Error in getOrganizationSettings:", error);
      }
      return;
    };

    /**
     * Get configuration from the backend.
     * @return {"version":"","instance":"","commit_hash":"","build_date":"","default_fts_keys":["field1","field2"],"telemetry_enabled":true,"default_functions":[{"name":"function name","text":"match_all('v')"}}
     * @throws {Error} If the request fails.
     */
    const getConfig = async () => {
      await configService
        .get_config()
        .then(async (res: any) => {
          if (config.isCloud == "false") {
            linksList.value = mainLayoutMixin
              .setup()
              .leftNavigationLinks(linksList, t);
            filterMenus();
          }
          store.dispatch("setConfig", res.data);
          await nextTick();
          // if rum enabled then setUser to capture session details.
          if (res.data.rum.enabled) {
            setRumUser();
          }
        })
        .catch((error) => console.log(error));
    };

    if (config.isCloud == "true") {
      mainLayoutMixin.setup().getDefaultOrganization(store);
    }

    const setRumUser = () => {
      if (store.state.zoConfig?.rum?.enabled == true) {
        const userInfo = store.state.userInfo;
        openobserveRum.setUser({
          name: userInfo.given_name + " " + userInfo.family_name,
          email: userInfo.email,
        });
      }
    };

    const prefetch = () => {
      const href = "/web/assets/editor.api.v1.js";
      const existingLink = document.querySelector(
        `link[rel="prefetch"][href="${href}"]`,
      );

      if (!existingLink) {
        // Create a new link element
        isMonacoEditorLoaded.value = true;
        const link = document.createElement("link");
        link.rel = "prefetch";
        link.href = href;
        document.head.appendChild(link);
      }
    };

    const expandMenu = () => {
      // miniMode.value = false;
      if (!isMonacoEditorLoaded.value) prefetch();
    };

    const openSlack = () => {
      window.open(slackURL, "_blank");
    };

    return {
      t,
      router,
      store,
      config,
      langList,
      selectedLanguage,
      linksList,
      selectedOrg,
      orgOptions,
      leftDrawerOpen: false,
      miniMode,
      user,
      zoBackendUrl,
      isLoading,
      getImageURL,
      updateOrganization,
      setSelectedOrganization,
      getOrganizationSettings,
      resetStreams,
      triggerRefreshToken,
      prefetch,
      expandMenu,
      slackIcon: markRaw(SlackIcon),
      openSlack,
      outlinedSettings,
    };
  },
  computed: {
    changeOrganization() {
      return this.store?.state?.organizations;
    },
    changeOrganizationIdentifier() {
      return this.store?.state?.selectedOrganization?.identifier;
    },
    forceFetchOrganization() {
      return this.router?.currentRoute?.value?.query?.update_org;
    },
    changeUserInfo() {
      return this.store?.state?.userInfo;
    },
  },
  watch: {
    forceFetchOrganization() {
      mainLayoutMixin.setup().getDefaultOrganization(this.store);
    },
    changeOrganization: {
      handler() {
        this.setSelectedOrganization();
      },
      deep: true,
      immediate: true,
    },
    async changeOrganizationIdentifier() {
      this.isLoading = false;
      this.resetStreams();
      this.store.dispatch("setOrganizationPasscode", "");
      this.store.dispatch("resetOrganizationData", {});

      await this.getOrganizationSettings();

      this.isLoading = true;
    },
    changeUserInfo(newVal) {
      if (JSON.stringify(newVal) != "{}") {
        this.triggerRefreshToken();
      }
    },
  },
});
</script>

<style lang="scss">
@import "../styles/app.scss";

.printMode {
  .q-header {
    display: none;
  }

  .q-drawer {
    display: none;
  }

  .q-page-container {
    padding-left: 0px !important;
  }
}

.warning-msg {
  background-color: var(--q-warning);
  padding: 5px;
  border-radius: 5px;
}

.alert-msg {
  background-color: var(--q-alert);
  padding: 5px;
  border-radius: 5px;
}

.q-header .q-btn-dropdown__arrow {
  margin-left: -4px;
}

.q-header {
  color: unset;
  @extend .border-bottom;

  .beta-text {
    font-size: 11px;
    right: 1px;
    bottom: -9px;
  }

  .appLogo {
    margin-left: 0.5rem;
    margin-right: 0;
    width: 150px;
    cursor: pointer;

    &__mini {
      margin-right: 0.25rem;
      // margin-left: 0.25rem;
      height: 30px;
      width: 30px;
    }
  }
}

.q-toolbar {
  min-height: 40px;
}

.headerMenu {
  margin-right: 1rem;

  .block {
    font-weight: 700;
    color: #404040;
  }
}

.q-item {
  min-height: 30px;
  padding: 8px 8px;
}

.o2-bg-color {
  background-color: rgba(89, 96, 178, 0.08);
}

.q-list {
  &.leftNavList {
    padding-bottom: 0px;

    .q-item {
      margin: 5px 5px 5px 5px;
      display: list-item;
      text-align: center;
      list-style: none;
      padding: 5px 2px;
      border-radius: 5px;

      .q-icon {
        height: 1.5rem;
        width: 1.5rem;
      }

      &.q-router-link--active {
        .q-icon img {
          filter: brightness(100);
        }

        .q-item__label {
          color: white;
        }
        color: white;
      }

      &__label {
        font-size: 12px;
        font-weight: 600;
        color: grey;
      }
    }
  }

  .flagIcon img {
    border-radius: 3px;
    object-fit: cover;
    display: block;
    height: 16px;
    width: 24px;
  }

  .q-item {
    &__section {
      &--avatar {
        padding-right: 0px !important;
        min-width: 1.5rem;
        display: list-item;
        text-align: center;
        list-style: none;
      }
    }

    &__label {
      font-weight: 400;
    }

    &.activeLang {
      &__label {
        font-weight: 600;
        color: $primary;
      }
    }
  }
}

.userInfo {
  align-items: flex-start;
  flex-direction: column;
  margin-left: 0.875rem;
  margin-right: 1rem;
  display: flex;

  .userName {
    line-height: 1.25rem;
    font-weight: 700;
  }

  .userRole {
    font-size: 0.75rem;
    line-height: 1rem;
    color: #565656;
    font-weight: 600;
  }
}

.headerMenu {
  margin-right: 1rem;

  .block {
    font-weight: 700;
    color: #404040;
  }
}
.q-list {
  &.leftNavList {
    .q-item {
      .q-icon {
        height: 1.5rem;
        width: 1.5rem;
      }

      &.q-router-link--active {
        .q-icon img {
          filter: brightness(100);
        }
      }
    }
  }

  .flagIcon img {
    border-radius: 3px;
    object-fit: cover;
    display: block;
    height: 16px;
    width: 24px;
  }

  .q-item {
    &__section {
      &--avatar {
        padding-right: 0.875rem;
        min-width: 1.5rem;
      }
    }

    &__label {
      font-weight: 400;
    }

    &.activeLang {
      &__label {
        font-weight: 600;
        color: $primary;
      }
    }
  }
}

.userInfo {
  align-items: flex-start;
  flex-direction: column;
  margin-left: 0.875rem;
  margin-right: 1rem;
  display: flex;

  .userName {
    line-height: 1.25rem;
    font-weight: 700;
  }

  .userRole {
    font-size: 0.75rem;
    line-height: 1rem;
    color: #565656;
    font-weight: 600;
  }
}

.dark-mode {
  background-color: $dark-page;
}

.languagelist {
  .q-item {
    padding: 4px 8px;
  }
}

.text-powered-by {
  float: left;
  display: inline-block;
  position: absolute;
  margin-top: 16px;
  margin-left: 0px;
}

.custom-text-logo {
  display: inline-block;
  float: left;
  position: absolute;
  margin-left: 72px !important;
  margin-top: 16px !important;
  width: 80px !important;
}

.header-icon {
  opacity: 0.7;
}
</style>
