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
  <div
    data-test="add-action-script-section"
    class="full-width create-report-page"
  >
    <div class="row items-center no-wrap q-mx-md q-my-sm">
      <div class="flex items-center">
        <div
          data-test="add-action-script-back-btn"
          class="flex justify-center items-center q-mr-md cursor-pointer"
          style="
            border: 1.5px solid;
            border-radius: 50%;
            width: 22px;
            height: 22px;
          "
          title="Go Back"
          @click="router.back()"
        >
          <q-icon name="arrow_back_ios_new" size="14px" />
        </div>
        <div
          v-if="isEditingActionScript"
          class="text-h6"
          data-test="add-action-script-title"
        >
          {{ t("actions.update") }}
        </div>
        <div v-else class="text-h6" data-test="add-action-script-title">
          {{ t("actions.add") }}
        </div>
      </div>
    </div>
    <q-separator />
    <div class="flex" style="height: calc(100vh - 162px); overflow: auto">
      <div ref="addAlertFormRef" class="q-px-lg q-my-md" style="width: 1024px">
        <q-form
          class="create-report-form"
          ref="addActionScriptFormRef"
          @submit="onSubmit"
        >
          <div
            data-test="add-action-script-name-input"
            class="report-name-input o2-input q-px-sm"
            style="padding-top: 12px"
          >
            <q-input
              v-model.trim="formData.name"
              :label="t('alerts.name') + ' *'"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[
                (val: any) =>
                  !!val
                    ? isValidResourceName(val) ||
                      `Characters like :, ?, /, #, and spaces are not allowed.`
                    : t('common.nameRequired'),
              ]"
              tabindex="0"
              style="width: 400px"
            >
              <template v-slot:hint>
                Characters like :, ?, /, #, and spaces are not allowed.
              </template>
            </q-input>
          </div>
          <div
            data-test="add-action-script-description-input"
            class="report-name-input o2-input q-px-sm q-pb-md"
          >
            <q-input
              v-model="formData.description"
              :label="t('reports.description')"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              tabindex="0"
              style="width: 600px"
            />
          </div>

          <q-stepper
            v-model="step"
            vertical
            color="primary"
            animated
            class="q-mt-md"
            header-nav
          >
            <q-step
              data-test="add-action-script-step-1"
              :name="1"
              title="Upload Script Zip"
              :icon="outlinedDashboard"
              :done="step > 1"
            >
              <div
                data-test="add-action-script-file-input"
                class="flex items-center o2-input"
              >
                <q-file
                  v-if="!isEditingActionScript || formData.fileNameToShow == ''"
                  ref="fileInput"
                  color="primary"
                  filled
                  v-model="formData.codeZip"
                  :label="t('actions.uploadCodeZip')"
                  bg-color="input-bg"
                  class="tw-w-[300px] q-pt-md q-pb-sm showLabelOnTop lookup-table-file-uploader"
                  stack-label
                  outlined
                  dense
                  accept=".zip"
                  :rules="[
                    (val: any) => {
                      if (!isEditingActionScript) {
                        return !!val || 'CSV File is required!';
                      }
                      return true;
                    },
                  ]"
                >
                  <template v-slot:prepend>
                    <q-icon name="attachment" />
                  </template>
                  <template v-slot:hint>
                    Note: Only .zip files are accepted and it may contain
                    various resources such as .py, .txt and main.py file etc.
                  </template>
                </q-file>

                <div
                  v-else-if="
                    isEditingActionScript && formData.fileNameToShow != ''
                  "
                >
                  {{ formData.fileNameToShow }}
                  <q-btn
                    data-test="add-action-script-edit-file-btn"
                    @click="editFileToUpload"
                    icon="edit"
                    no-caps
                    dense
                    flat
                    size="14px"
                  />
                </div>
                <div
                  v-if="isEditingActionScript && formData.fileNameToShow == ''"
                  class="q-pt-md q-mt-xs q-pl-md"
                >
                  <q-btn
                    data-test="cancel-upload-new-btn-file"
                    @click="cancelUploadingNewFile"
                    color="red"
                    label="Cancel"
                    no-caps
                  />
                </div>
              </div>

              <q-stepper-navigation>
                <q-btn
                  data-test="add-action-script-step1-continue-btn"
                  @click="step = 2"
                  color="secondary"
                  label="Continue"
                  no-caps
                  class="q-mt-sm"
                />
              </q-stepper-navigation>
            </q-step>

            <q-step
              data-test="add-action-script-step-2"
              :name="2"
              title="Schedule"
              icon="schedule"
              :done="step > 2"
              class="q-mt-md"
            >
              <div class="q-my-sm q-px-sm">
                <div
                  style="font-size: 14px"
                  class="text-bold text-grey-8 q-mb-sm"
                  data-test="add-action-script-frequency-title"
                >
                  Frequency
                </div>
                <div
                  style="
                    border: 1px solid #d7d7d7;
                    width: fit-content;
                    border-radius: 2px;
                  "
                >
                  <template v-for="visual in frequencyTabs" :key="visual.value">
                    <q-btn
                      :data-test="`add-action-script-schedule-frequency-${visual.value}-btn`"
                      :color="visual.value === frequency.type ? 'primary' : ''"
                      :flat="visual.value === frequency.type ? false : true"
                      dense
                      no-caps
                      size="12px"
                      class="q-px-lg visual-selection-btn"
                      style="padding-top: 4px; padding-bottom: 4px"
                      @click="frequency.type = visual.value"
                    >
                      {{ visual.label }}</q-btn
                    >
                  </template>
                </div>

                <div
                  v-if="frequency.type === 'once'"
                  class="flex justify-start items-center q-mt-md"
                  data-test="add-action-script-frequency-info"
                >
                  <q-icon name="event" class="q-mr-sm" />
                  <div style="font-size: 14px">
                    The script will be triggered immediately after it is saved
                  </div>
                </div>

                <template v-if="frequency.type === 'repeat'">
                  <div class="flex items-center justify-start q-mt-md o2-input">
                    <div
                      data-test="add-action-script-cron-input"
                      class="q-mr-sm"
                      style="padding-top: 0; width: 320px"
                    >
                      <div
                        class="q-mb-xs text-bold text-grey-8"
                        data-test="add-action-script-cron-expression-title"
                      >
                        {{ t("reports.cronExpression") + " *" }}
                        <q-icon
                          data-test="add-action-script-cron-info"
                          :name="outlinedInfo"
                          size="17px"
                          class="q-ml-xs cursor-pointer"
                          :class="
                            store.state.theme === 'dark'
                              ? 'text-grey-5'
                              : 'text-grey-7'
                          "
                        >
                          <q-tooltip anchor="center right" self="center left">
                            <span style="font-size: 14px">
                              Pattern: * * * * * means every minute .
                              <br />
                              Format: [Minute 0-59] [Hour 0-23] [Day of Month
                              1-31, 'L'] [Month 1-12] [Day of Week 0-7 or
                              '1L-7L', 0 and 7 for Sunday].
                              <br />
                              Use '*' to represent any value, 'L' for the last
                              day/weekday. <br />
                              Example: 0 12 * * ? - Triggers at 12:00 PM daily.
                              It specifies minute, hour, day of month, month,
                              and day of week, respectively.</span
                            >
                          </q-tooltip>
                        </q-icon>
                      </div>

                      <q-input
                        filled
                        v-model="frequency.cron"
                        color="input-border"
                        bg-color="input-bg"
                        type="text"
                        outlined
                        :rules="[
                          (val: any) =>
                            !!val.length
                              ? cronError.length
                                ? cronError
                                : true
                              : 'Field is required!',
                        ]"
                        dense
                        debounce="300"
                        style="width: 100%"
                        @update:model-value="validateFrequency(frequency.cron)"
                      />
                    </div>
                    <q-select
                      data-test="add-action-script-timezone-select"
                      v-model="formData.timezone"
                      :options="['UTC']"
                      :label="t('actions.timezone') + ' *'"
                      :loading="isFetchingServiceAccounts"
                      :popup-content-style="{ textTransform: 'lowercase' }"
                      color="input-border"
                      bg-color="input-bg"
                      class="showLabelOnTop no-case"
                      filled
                      stack-label
                      outlined
                      dense
                      use-input
                      hide-selected
                      fill-input
                      :input-debounce="400"
                      behavior="menu"
                      disable
                      :rules="[(val: any) => !!val || 'Field is required!']"
                      style="
                        min-width: 250px !important;
                        width: 250px !important;
                      "
                    />
                  </div>
                </template>
              </div>

              <q-stepper-navigation>
                <div>
                  <q-btn
                    data-test="add-action-script-step2-continue-btn"
                    @click="step = 3"
                    color="secondary"
                    label="Continue"
                    class="q-ml-sm"
                    no-caps
                  />
                  <q-btn
                    data-test="add-action-script-step2-back-btn"
                    @click="step = 1"
                    flat
                    color="primary"
                    label="Back"
                    class="q-ml-sm"
                    no-caps
                  />
                </div>
              </q-stepper-navigation>
            </q-step>

            <q-step
              data-test="add-action-script-step-3"
              :name="3"
              title="Select Service Account"
              :icon="outlinedDashboard"
              :done="step > 3"
              class="q-mt-md"
            >
              <div class="flex items-center o2-input">
                <div class="o2-input service-account-selector">
                  <div
                    data-test="add-action-script-service-account-title"
                    class="q-mb-xs text-bold text-grey-8"
                  >
                    {{ t("actions.serviceAccount") + " *" }}
                    <q-icon
                      :name="outlinedInfo"
                      size="17px"
                      class="q-ml-xs cursor-pointer"
                      :class="
                        store.state.theme === 'dark'
                          ? 'text-grey-5'
                          : 'text-grey-7'
                      "
                    >
                      <q-tooltip anchor="center right" self="center left">
                        <span style="font-size: 14px">
                          Make sure service account has permissions to access
                          Actions.
                        </span>
                      </q-tooltip>
                    </q-icon>
                  </div>
                  <q-select
                    data-test="add-action-script-service-account-select"
                    v-model="formData.service_account"
                    :options="filteredServiceAccounts"
                    :loading="isFetchingServiceAccounts"
                    :popup-content-style="{ textTransform: 'lowercase' }"
                    color="input-border"
                    bg-color="input-bg"
                    class="q-py-sm no-case"
                    filled
                    outlined
                    dense
                    use-input
                    hide-selected
                    fill-input
                    :input-debounce="400"
                    @filter="filterServiceAccounts"
                    behavior="menu"
                    :rules="[(val: any) => !!val || 'Field is required!']"
                    style="min-width: 250px !important; width: 250px !important"
                  />
                </div>
              </div>

              <q-stepper-navigation>
                <q-btn
                  data-test="add-action-script-step3-continue-btn"
                  @click="step = 4"
                  color="secondary"
                  label="Continue"
                  no-caps
                  class="q-mt-sm"
                />
                <q-btn
                  data-test="add-action-script-step3-back-btn"
                  @click="step = 2"
                  flat
                  color="primary"
                  label="Back"
                  class="q-ml-sm"
                  no-caps
                />
              </q-stepper-navigation>
            </q-step>
            <q-step
              data-test="add-action-script-step-4"
              :name="4"
              title="Environmental Variables"
              icon="lock"
              :done="step > 4"
              class="q-mt-md"
            >
              <div
                v-for="(header, index) in environmentalVariables"
                :key="header.uuid"
                class="row q-col-gutter-sm o2-input"
                data-test="add-action-script-env-variable"
              >
                <div class="col-5 q-ml-none">
                  <q-input
                    :data-test="`add-action-script-header-${header['key']}-key-input`"
                    v-model="header.key"
                    color="input-border"
                    bg-color="input-bg"
                    stack-label
                    outlined
                    filled
                    :placeholder="'Key'"
                    dense
                    tabindex="0"
                  />
                </div>
                <div class="col-5 q-ml-none q-mb-sm">
                  <q-input
                    :data-test="`add-action-script-header-${header['key']}-value-input`"
                    v-model="header.value"
                    :placeholder="t('alert_destinations.api_header_value')"
                    color="input-border"
                    bg-color="input-bg"
                    stack-label
                    outlined
                    filled
                    dense
                    isUpdatingDestination
                    tabindex="0"
                  />
                </div>
                <div class="col-2 q-ml-none">
                  <q-btn
                    :data-test="`add-action-script-header-${header['key']}-delete-btn`"
                    icon="delete"
                    class="q-ml-xs iconHoverBtn"
                    padding="sm"
                    unelevated
                    size="sm"
                    round
                    flat
                    :title="t('alert_templates.delete')"
                    @click="deleteApiHeader(header)"
                  />
                  <q-btn
                    data-test="add-action-script-add-header-btn"
                    v-if="index === environmentalVariables.length - 1"
                    icon="add"
                    class="q-ml-xs iconHoverBtn"
                    padding="sm"
                    unelevated
                    size="sm"
                    round
                    flat
                    :title="t('alert_templates.edit')"
                    @click="addApiHeader()"
                  />
                </div>
              </div>
              <q-stepper-navigation>
                <q-btn
                  data-test="add-action-script-step4-back-btn"
                  flat
                  @click="step = 3"
                  color="primary"
                  label="Back"
                  class="q-ml-sm"
                  no-caps
                />
              </q-stepper-navigation>
            </q-step>
          </q-stepper>
        </q-form>
      </div>
    </div>
    <div
      class="flex justify-end q-px-md q-py-sm full-width"
      style="position: sticky; bottom: 0px; z-index: 2"
      :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
      :style="{
        'box-shadow':
          store.state.theme === 'dark'
            ? 'rgb(45 45 45) 0px -4px 7px 0px'
            : 'rgb(240 240 240) 0px -4px 7px 0px',
      }"
    >
      <q-btn
        data-test="add-action-script-cancel-btn"
        class="text-bold"
        :label="t('alerts.cancel')"
        text-color="light-text"
        padding="sm md"
        no-caps
        @click="openCancelDialog"
      />
      <q-btn
        data-test="add-action-script-save-btn"
        :label="t('alerts.save')"
        class="text-bold no-border q-ml-md"
        color="secondary"
        padding="sm xl"
        no-caps
        @click="saveActionScript"
      />
    </div>
  </div>
  <ConfirmDialog
    v-model="dialog.show"
    :title="dialog.title"
    :message="dialog.message"
    @update:ok="dialog.okCallback"
    @update:cancel="dialog.show = false"
  />
</template>

<script setup lang="ts">
import { ref, nextTick, onMounted, watch } from "vue";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import DateTime from "@/components/DateTime.vue";
import {
  getUUID,
  useLocalTimezone,
  isValidResourceName,
  getCronIntervalDifferenceInSeconds,
  isAboveMinRefreshInterval,
} from "@/utils/zincutils";
import VariablesInput from "@/components/alerts/VariablesInput.vue";
import { useStore } from "vuex";
import { outlinedDashboard } from "@quasar/extras/material-icons-outlined";
import dashboardService from "@/services/dashboards";
import { onBeforeMount } from "vue";
import type { Ref } from "vue";
import { DateTime as _DateTime } from "luxon";
import actions from "@/services/action_scripts";
import { useQuasar } from "quasar";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import cronParser from "cron-parser";
import { outlinedInfo } from "@quasar/extras/material-icons-outlined";
import { convertDateToTimestamp } from "@/utils/date";
import service_accounts from "@/services/service_accounts";

defineProps({
  report: {
    type: Object,
    default: null,
  },
});
const emit = defineEmits(["getActionScripts"]);

const defaultActionScript = {
  codeZip: null,
  description: "",
  enabled: true,
  name: "",
  orgId: "",
  start: 0,
  frequency: {
    interval: 1,
    type: "repeat",
    cron: "",
  },
  environment_variables: {},
  execution_details: "",
  timezone: "UTC",
  timezoneOffset: 0,
  lastTriggeredAt: null,
  createdAt: "",
  updatedAt: "",
  owner: "",
  lastEditedBy: "",
  fileNameToShow: "",
  id: "",
  service_account: "",
};

const { t } = useI18n();
const router = useRouter();

const fileInput = ref(null);

const originalActionScriptData: Ref<string> = ref("");

const addActionScriptFormRef: Ref<any> = ref(null);

const step = ref(1);

const formData = ref(defaultActionScript);

const q = useQuasar();

const dialog = ref({
  show: false,
  title: "",
  message: "",
  okCallback: () => {},
});

const timeTabs = [
  {
    label: "Schedule now",
    value: "scheduleNow",
  },
  {
    label: "Schedule later",
    value: "scheduleLater",
  },
];

const frequencyTabs = [
  {
    label: "Cron Job",
    value: "repeat",
  },
  {
    label: "Once",
    value: "once",
  },
];

const selectedTimeTab = ref("scheduleNow");

const store = useStore();

const filteredTimezone: any = ref([]);

const folderOptions: Ref<{ label: string; value: string }[]> = ref([]);

const dashboardOptions: Ref<
  { label: string; value: string; tabs: any[]; version: number }[]
> = ref([]);

const dashboardTabOptions: Ref<{ label: string; value: string }[]> = ref([]);

const options: Ref<{ [key: string]: any[] }> = ref({});

const emails = ref("");

const isEditingActionScript = ref(false);

const isFetchingActionScript = ref(false);

const environmentalVariables = ref([{ key: "", value: "", uuid: getUUID() }]);
const cronError = ref("");

const frequency = ref({
  type: "once",
  custom: {
    interval: 1,
    period: "days",
  },
  cron: "",
});

watch(
  () => router.currentRoute.value.query?.id,
  async (action_id) => {
    await handleActionScript();
  },
);

onBeforeMount(async () => {
  await handleActionScript();
});

const onSubmit = () => {};

const scheduling = ref({
  date: "",
  time: "",
  timezone: "",
});

const currentTimezone =
  useLocalTimezone() || Intl.DateTimeFormat().resolvedOptions().timeZone;
const timezone = ref(currentTimezone);

const timezoneFilterFn = (val: string, update: Function) => {
  filteredTimezone.value = filterColumns(timezoneOptions, val, update);
};

const filterColumns = (options: any[], val: String, update: Function) => {
  let filteredOptions: any[] = [];
  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
    return filteredOptions;
  }
  update(() => {
    const value = val.toLowerCase();
    filteredOptions = options.filter(
      (column: any) => column.toLowerCase().indexOf(value) > -1,
    );
  });
  return filteredOptions;
};

// @ts-ignore
let timezoneOptions = Intl.supportedValuesOf("timeZone").map((tz: any) => {
  return tz;
});

const browserTime =
  "Browser Time (" + Intl.DateTimeFormat().resolvedOptions().timeZone + ")";

// Add the UTC option
timezoneOptions.unshift("UTC");
timezoneOptions.unshift(browserTime);

const saveActionScript = async () => {
  // If frequency is cron, then we set the start timestamp as current time and timezone as browser timezone
  // if (frequency.value.type === "Repeat") {
  //   const now = new Date();

  //   // Get the day, month, and year from the date object
  //   const day = String(now.getDate()).padStart(2, "0");
  //   const month = String(now.getMonth() + 1).padStart(2, "0"); // January is 0!
  //   const year = now.getFullYear();

  //   // Combine them in the DD-MM-YYYY format
  //   scheduling.value.date = `${day}-${month}-${year}`;

  //   // Get the hours and minutes, ensuring they are formatted with two digits
  //   const hours = String(now.getHours()).padStart(2, "0");
  //   const minutes = String(now.getMinutes()).padStart(2, "0");

  //   // Combine them in the HH:MM format
  //   scheduling.value.time = `${hours}:${minutes}`;
  // }

  // // If the user has selected to schedule now, we set the timezone to the current timezone
  // if (formData.value.execution_details === "Once") {
  //   scheduling.value.timezone = timezone.value;
  // }
  let form;

  // Determine if FormData is needed
  const useFormData =
    !isEditingActionScript.value ||
    (isEditingActionScript.value && formData.value.codeZip);
  // Initialize form as FormData or plain object
  form = useFormData ? new FormData() : {};

  // Common fields
  const commonFields: Record<string, any> = {
    name: formData.value.name,
    description: formData.value.description,
    execution_details: frequency.value.type,
    service_account: formData.value.service_account,
  };
  // const convertedDateTime = convertDateToTimestamp(
  //   scheduling.value.date,
  //   scheduling.value.time,
  //   scheduling.value.timezone,
  // );

  // Add cron expression if needed
  if (frequency.value.type === "repeat") {
    commonFields.cron_expr = frequency.value.cron.toString().trim();
    // commonFields.timezoneOffset = convertedDateTime.offset.toString();
    // commonFields.timezone = scheduling.value.timezone;
  }

  if (useFormData) {
    commonFields.owner = store.state.userInfo.email;
  }
  // if (frequency.value.type == "Once") {
  //   commonFields.timezone = null;
  //   commonFields.timezoneOffset = null;
  // }

  // Add environment variables if present
  if (environmentalVariables.value.length > 0) {
    // Convert environmentalVariables to an object, ignoring conflicts with mandatory keys
    const environment_variables = environmentalVariables.value.reduce(
      (acc: any, curr: any) => {
        // Check if the key exists in mandatoryKeys
        if (curr.key) {
          // Add the key only if it is not a mandatory key
          acc[curr.key] = curr.value;
        }

        return acc;
      },
      {},
    );

    // Assign to commonFields
    commonFields.environment_variables = environment_variables;
  }

  // Populate form (either FormData or plain object)
  Object.entries(commonFields).forEach(([key, value]) => {
    if (useFormData) {
      (form as FormData).append(
        key,
        typeof value === "object" ? JSON.stringify(value) : value,
      );
    } else {
      (form as Record<string, any>)[key] = value;
    }
  });

  // Add file fields if using FormData
  if (useFormData && formData.value.codeZip) {
    (form as FormData).append("file", formData.value.codeZip || "");
    (form as FormData).append(
      "filename",
      (formData.value.codeZip as File).name || "",
    );
  }

  if (isEditingActionScript.value && formData.value.codeZip) {
    (form as FormData).append("id", formData.value.id);
  }

  // Check if all report input fields are valid
  try {
    validateActionScriptData();
    await nextTick();
    await nextTick();
    const isValidForm = await addActionScriptFormRef.value.validate();
    if (!isValidForm) return;
  } catch (err) {
    console.log(err);
  }

  const updateAction =
    isEditingActionScript.value && !formData.value.codeZip
      ? actions.update
      : actions.create;

  const dismiss = q.notify({
    spinner: true,
    message: "Please wait...",
    timeout: 2000,
  });
  const actionId: string = (router.currentRoute.value.query?.id ||
    "") as string;

  updateAction(store.state.selectedOrganization.identifier, actionId, form)
    .then(() => {
      q.notify({
        type: "positive",
        message: `Action ${
          isEditingActionScript.value ? "updated" : "saved"
        } successfully.`,
        timeout: 3000,
      });
      goToActionScripts();
      emit("getActionScripts");
    })
    .catch((error) => {
      step.value = 3;
      if (error.response.status != 403) {
        q.notify({
          type: "negative",
          message:
            error?.response?.data?.message ||
            `Error while ${
              isEditingActionScript.value ? "updating" : "saving"
            } Action.`,
          timeout: 4000,
        });
      }
    })
    .finally(() => {
      dismiss();
    });
};

const validateActionScriptData = async () => {
  if (!formData.value.codeZip && !isEditingActionScript.value) {
    step.value = 1;
    return;
  }

  if (formData.value.execution_details === "repeat") {
    try {
      cronError.value = "";
      cronParser.parseExpression(frequency.value.cron);
      validateFrequency(frequency.value.cron);
    } catch (err) {
      cronError.value = "Invalid cron expression!";
      return;
    }
  }

  if (formData.value.execution_details === "repeat" && cronError.value) {
    step.value = 2;
    return;
  }

  if (!formData.value.execution_details) {
    step.value = 2;
    return;
  }

  // if (!formData.value.timezone) {
  //   step.value = 2;
  //   return;
  // }
};

const validateFrequency = (value: string) => {
  cronError.value = "";

  try {
    const intervalInSecs = getCronIntervalDifferenceInSeconds(value);

    if (
      typeof intervalInSecs === "number" &&
      !isAboveMinRefreshInterval(intervalInSecs, store.state?.zoConfig)
    ) {
      const minInterval =
        Number(store.state?.zoConfig?.min_auto_refresh_interval) || 1;
      cronError.value = `Frequency should be greater than ${minInterval - 1} seconds.`;
      return;
    }
  } catch (err) {
    cronError.value = "Invalid cron expression!";
  }
};

const goToActionScripts = () => {
  router.replace({
    name: "actionScripts",
    query: {
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const setupEditingActionScript = async (report: any) => {
  formData.value = {
    ...report,
    timezone: report.timezone || "UTC",
  };
  formData.value.fileNameToShow = report.zip_file_name;

  // set date, time and timezone in scheduling
  // const date = new Date(report.start / 1000);

  // // Get the day, month, and year from the date object
  // const day = String(date.getDate()).padStart(2, "0");
  // const month = String(date.getMonth() + 1).padStart(2, "0"); // January is 0!
  // const year = date.getFullYear();

  // // Combine them in the DD-MM-YYYY format
  // scheduling.value.date = `${day}-${month}-${year}`;

  // // Get the hours and minutes, ensuring they are formatted with two digits
  // const hours = String(date.getHours()).padStart(2, "0");
  // const minutes = String(date.getMinutes()).padStart(2, "0");

  // // Combine them in the HH:MM format
  // scheduling.value.time = `${hours}:${minutes}`;

  // scheduling.value.timezone = report.timezone;

  // // set selectedTimeTab to scheduleLater
  // selectedTimeTab.value = "scheduleLater";

  // set frequency
  if (report.execution_details == "repeat") {
    frequency.value.type = "repeat";
    frequency.value.cron =
      report.cron_expr.split(" ").length === 7
        ? report.cron_expr.split(" ").slice(0, 6).join(" ")
        : report.cron_expr;
  } else {
    frequency.value.type = "once";
  }
  if (Object.keys(formData.value.environment_variables).length) {
    environmentalVariables.value = [];
    Object.entries(formData.value.environment_variables).forEach(
      ([key, value]: [string, any]) => {
        addApiHeader(key, value);
      },
    );
  }
};

const openCancelDialog = () => {
  if (originalActionScriptData.value === JSON.stringify(formData.value)) {
    goToActionScripts();
    return;
  }
  dialog.value.show = true;
  dialog.value.title = "Discard Changes";
  dialog.value.message = "Are you sure you want to cancel Action changes?";
  dialog.value.okCallback = goToActionScripts;
};
const editFileToUpload = () => {
  formData.value.fileNameToShow = "";
};
const cancelUploadingNewFile = () => {
  formData.value.fileNameToShow = JSON.parse(
    originalActionScriptData.value,
  ).zip_file_name;
};
const addApiHeader = (key: string = "", value: string = "") => {
  environmentalVariables.value.push({
    key: key,
    value: value,
    uuid: getUUID(),
  });
};
const deleteApiHeader = (header: any) => {
  environmentalVariables.value = environmentalVariables.value.filter(
    (_header) => _header.uuid !== header.uuid,
  );
  if (
    (formData.value.environment_variables as { [key: string]: any })[header.key]
  ) {
    delete (formData.value.environment_variables as { [key: string]: any })[
      header.key
    ];
  }

  if (!environmentalVariables.value.length) addApiHeader();
};
const isRequiredKey = (value: any) => {
  return value && value.trim() !== "" ? true : "Key is required";
};

const isRequiredValue = (value: any) => {
  return value && value.trim() !== "" ? true : "Value is required";
};

const handleActionScript = async () => {
  isEditingActionScript.value = !!router.currentRoute.value.query?.id;

  if (isEditingActionScript.value) {
    isEditingActionScript.value = true;
    isFetchingActionScript.value = true;

    const actionId: string = (router.currentRoute.value.query?.id ||
      "") as string;
    actions
      .get_by_id(store.state.selectedOrganization.identifier, actionId)
      .then((res: any) => {
        setupEditingActionScript(res.data);

        originalActionScriptData.value = JSON.stringify(formData.value);
      })
      .catch((err) => {
        if (err.response.status != 403) {
          q.notify({
            type: "negative",
            message: err?.data?.message || "Error while fetching Action!",
            timeout: 4000,
          });
        }
      })
      .finally(() => {
        isFetchingActionScript.value = false;
      });
  } else {
    originalActionScriptData.value = JSON.stringify(formData.value);
  }
};

const filteredServiceAccounts: Ref<{ label: string; value: string }[]> = ref(
  [],
);
const isFetchingServiceAccounts = ref(false);

const filterServiceAccounts = (val: string, update: Function) => {
  filteredServiceAccounts.value = filterColumns(
    serviceAccountsOptions,
    val,
    update,
  );
};

const serviceAccountsOptions: any[] = [];

const getServiceAccounts = async () => {
  isFetchingServiceAccounts.value = true;
  try {
    const res = await service_accounts.list(
      store.state.selectedOrganization.identifier,
    );
    serviceAccountsOptions.push(
      ...res.data.data.map((account: any) => account.email),
    );
    filteredServiceAccounts.value = [...serviceAccountsOptions];
  } catch (err: any) {
    if (err.response?.status != 403) {
      q.notify({
        type: "negative",
        message:
          err.response?.data?.message ||
          "Error while fetching service accounts.",
        timeout: 3000,
      });
    }
  } finally {
    isFetchingServiceAccounts.value = false;
  }
};

onMounted(async () => {
  await getServiceAccounts();
});
</script>

<style lang="scss">
.create-report-page {
  .q-expansion-item .q-item {
    padding: 0 8px;
  }
}
</style>

<style scoped lang="scss">
.lookup-table-file-uploader {
  :deep(.q-field__label) {
    left: -30px;
  }
}

.service-account-selector {
  :deep(.q-field__control-container .q-field__native > :first-child) {
    text-transform: none !important;
  }
}
</style>
