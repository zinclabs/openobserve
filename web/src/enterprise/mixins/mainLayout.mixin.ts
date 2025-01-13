import { ref, markRaw, Ref } from "vue";
import { useRouter } from "vue-router";
import config from "@/aws-exports";
import { useStore } from "vuex";

import { getUserInfo, getImageURL, useLocalOrganization } from "@/utils/zincutils";
import organizationService from "@/services/organizations";
import billingService from "@/services/billings";
import userService from "@/services/users";
import PipelineIcon from "@/components/icons/PipelineIcon.vue";

const MainLayoutCloudMixin = {
  setup() {
    const router = useRouter();
    const store = useStore();
    const orgOptions = ref([{ label: Number, value: String }]);
    const selectedOrg: Ref<string> = ref("");

    /**
     * Add function & organization menu in left navigation
     * @param linksList
     * @returns linksList.value
     */
    const leftNavigationLinks = (linksList: any, t: any) => {
      linksList.value.splice(5, 0, {
        title: t("menu.pipeline"),
        iconComponent: markRaw(PipelineIcon),
        link: "/pipeline",
        name: "pipeline",
      });

      if(!store.state.zoConfig?.custom_hide_menus
          ?.split(",")
          ?.includes("billings")
      ) {
        linksList.value.splice(10, 0, {
          title: t("menu.billings"),
          icon: "payments",
          link: "/billings",
        });
      }

      return linksList.value;
    };

    /**
     * Get default organization
     */
    const getDefaultOrganization = async (store: any) => {
      await organizationService
        .list(0, 100000, "id", false, "")
        .then((res: any) => {
          store.dispatch("setOrganizations", res.data.data);
          const localOrg: any = useLocalOrganization();
          orgOptions.value = res.data.data.map(
            (data: {
              id: any;
              name: any;
              type: any;
              identifier: any;
              UserObj: any;
            }) => {
              const optiondata: any = {
                label: data.name,
                id: data.id,
                identifier: data.identifier,
                user_email: store.state.userInfo.email,
              };

              if (
                (Object.keys(selectedOrg.value).length == 0 &&
                  (data.type == "default" || data.id == "1") &&
                  store.state.userInfo.email == data.UserObj.email) ||
                res.data.data.length == 1
              ) {
                selectedOrg.value = localOrg.value && Object.keys(localOrg.value).length > 0
                  ? localOrg.value
                  : optiondata;
                useLocalOrganization(selectedOrg.value);
                store.dispatch("setSelectedOrganization", selectedOrg.value);
              }
              return optiondata;
            },
          );
        })
        .catch((error) => console.log(error));
    };

    /**
     * Get organization plan
     * if plan is free, get the threshold and extract search and ingest threshold
     * if one of the threshold exceed the threshold, show the warning message else show error message
     */
    const getOrganizationThreshold = async (store: any) => {
      const organization: {
        identifier: string;
        subscription_type: string;
      } = store.state.selectedOrganization;
      if (
        organization.subscription_type == config.freePlan ||
        organization.subscription_type == ""
      ) {
        // await billingService
        //   .get_quota_threshold(organization.identifier)
        //   .then((res: any) => {
        //     const searchData: number = res.data.data.search;
        //     const ingestData: number = res.data.data.ingest;
        //     // res.data.data.forEach((element: any) => {
        //     //   if (element.event == "search") {
        //     //     searchData += element.size;
        //     //   } else if (element.event == "multi" || element.event == "bulk") {
        //     //     ingestData += element.size;
        //     //   }
        //     // });
        //     const searchNearThreshold = Math.floor(
        //       (store.state.selectedOrganization.search_threshold *
        //         parseInt(config.zincQuotaThreshold)) /
        //         100
        //     );

        //     const ingestNearThreshold = Math.floor(
        //       (store.state.selectedOrganization.ingest_threshold *
        //         parseInt(config.zincQuotaThreshold)) /
        //         100
        //     );
        //     let usageMessage = "";
        //     if (
        //       searchData > searchNearThreshold ||
        //       ingestData > ingestNearThreshold
        //     ) {
        //       if (searchNearThreshold >= 100 || ingestNearThreshold >= 100) {
        //         usageMessage =
        //           "You’ve exceeded monthly free limit. Search: [SEARCH_USAGE]%, Ingestion: [INGEST_USAGE]%";
        //       } else {
        //         usageMessage =
        //           "You’re approaching monthly free limit. Search: [SEARCH_USAGE]%, Ingestion: [INGEST_USAGE]%";
        //       }

        //       const percentageSearchQuota: any =
        //         store.state.selectedOrganization.search_threshold > 0
        //           ? (
        //               (searchData /
        //                 store.state.selectedOrganization.search_threshold) *
        //               100
        //             ).toFixed(2)
        //           : 0;

        //       const percentageIngestQuota: any =
        //         store.state.selectedOrganization.ingest_threshold > 0
        //           ? (
        //               (ingestData /
        //                 store.state.selectedOrganization.ingest_threshold) *
        //               100
        //             ).toFixed(2)
        //           : 0;

        //       usageMessage = usageMessage.replace(
        //         "[SEARCH_USAGE]",
        //         percentageSearchQuota <= 100 ? percentageSearchQuota : 100
        //       );
        //       usageMessage = usageMessage.replace(
        //         "[INGEST_USAGE]",
        //         percentageIngestQuota <= 100 ? percentageIngestQuota : 100
        //       );
        //     }
        //     // quotaThresholdMsg.value = usageMessage;
        //     store.dispatch("setQuotaThresholdMsg", usageMessage);
        //   })
        //   .catch((error) => console.log(error));
      }
    };

    /**
     * Get refresh token
     */
    const getRefreshToken = () => {
      userService
        .getRefreshToken()
        .then((res) => {
          const sessionUserInfo: any = getUserInfo(
            "#id_token=" + res.data.data.id_token
          );

          const userInfo = sessionUserInfo !== null ? sessionUserInfo : null;
          if (userInfo !== null) {
            store.dispatch("login", {
              loginState: true,
              userInfo: userInfo,
            });

            store.dispatch("setUserInfo", userInfo);
          }
        })
        .catch((e) => {
          console.log("Error while fetching refresh token:", e);
        });
    };

    return {
      store,
      orgOptions,
      getImageURL,
      leftNavigationLinks,
      getRefreshToken,
      getDefaultOrganization,
      getOrganizationThreshold,
    };
  },
};

export default MainLayoutCloudMixin;
