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
  <div class="scheduled-alerts">
    <div v-if="!disableQueryTypeSelection" class="scheduled-alert-tabs q-mb-lg">
      <q-tabs
        data-test="scheduled-alert-tabs"
        v-model="tab"
        no-caps
        outside-arrows
        size="sm"
        mobile-arrows
        class="bg-white text-primary"
        @update:model-value="updateTab"
      >
        <q-tab
          data-test="scheduled-alert-custom-tab"
          name="custom"
          :label="t('alerts.quick')"
        />
        <q-tab
          data-test="scheduled-alert-sql-tab"
          name="sql"
          :label="t('alerts.sql')"
        />
        <q-tab
          data-test="scheduled-alert-metrics-tab"
          v-if="alertData.stream_type === 'metrics'"
          name="promql"
          :label="t('alerts.promql')"
        />
      </q-tabs>
    </div>
    <template v-if="tab === 'custom'">
      <fields-input
        class="q-mt-md"
        :stream-fields="columns"
        :fields="conditions"
        @add="addField"
        @remove="removeField"
        @input:update="(name, field) => emits('input:update', name, field)"
      />
    </template>
    <template v-else>
      <div class="tw-flex tw-justify-start tw-items-center">
        <div class="text-bold q-mr-sm q-my-sm">
          {{ tab === "promql" ? "Promql" : "SQL" }}
        </div>
        <q-toggle
          v-if="!disableVrlFunction"
          data-test="logs-search-bar-show-query-toggle-btn"
          v-model="isVrlFunctionEnabled"
          :icon="'img:' + getImageURL('images/common/function.svg')"
          title="Toggle Function Editor"
          class="q-pl-xs"
          size="30px"
          @update:model-value="updateFunctionVisibility"
          :disable="tab === 'promql'"
        />
      </div>

      <div class="flex">
        <template v-if="tab === 'sql'">
          <div>
            <query-editor
              data-test="scheduled-alert-sql-editor"
              ref="queryEditorRef"
              editor-id="alerts-query-editor"
              class="monaco-editor"
              :debounceTime="300"
              v-model:query="query"
              :class="
                query == '' && queryEditorPlaceholderFlag ? 'empty-query' : ''
              "
              @update:query="updateQueryValue"
              @focus="queryEditorPlaceholderFlag = false"
              @blur="onBlurQueryEditor"
            />
            <div class="text-negative q-mb-xs invalid-sql-error">
              <span v-show="!!sqlQueryErrorMsg">
                Error: {{ sqlQueryErrorMsg }}</span
              >
            </div>
          </div>
        </template>
        <template v-if="tab === 'promql'">
          <query-editor
            data-test="scheduled-alert-promql-editor"
            ref="queryEditorRef"
            editor-id="alerts-query-editor"
            class="monaco-editor q-mb-md"
            :debounceTime="300"
            v-model:query="promqlQuery"
            @update:query="updateQueryValue"
          />
        </template>

        <div
          data-test="logs-vrl-function-editor"
          v-show="!disableVrlFunction && isVrlFunctionEnabled && tab === 'sql'"
        >
          <div style="height: 40px; width: 100%">
            <div style="display: flex; height: 40px">
              <q-select
                v-model="selectedFunction"
                label="Use Saved function"
                :options="functionOptions"
                data-test="dashboard-use-saved-vrl-function"
                input-debounce="0"
                behavior="menu"
                use-input
                filled
                borderless
                dense
                hide-selected
                menu-anchor="top left"
                fill-input
                option-label="name"
                option-value="name"
                @filter="filterFunctionOptions"
                @update:modelValue="onFunctionSelect"
                style="width: 100%"
              >
                <template #no-option>
                  <q-item>
                    <q-item-section> {{ t("search.noResult") }}</q-item-section>
                  </q-item>
                </template>
              </q-select>
            </div>
          </div>
          <query-editor
            data-test="logs-vrl-function-editor"
            ref="fnEditorRef"
            editor-id="fnEditor"
            class="monaco-editor"
            :debounceTime="300"
            v-model:query="vrlFunctionContent"
            :class="
              vrlFunctionContent == '' && functionEditorPlaceholderFlag
                ? 'empty-function'
                : ''
            "
            language="ruby"
            @focus="functionEditorPlaceholderFlag = false"
            @blur="onBlurFunctionEditor"
          />

          <div
            class="text-subtitle2 q-pb-sm"
            style="min-height: 21px; width: 500px"
          >
            <div v-if="vrlFunctionError">
              <div class="text-negative q-mb-xs flex items-center">
                <q-btn
                  :icon="
                    isFunctionErrorExpanded ? 'expand_more' : 'chevron_right'
                  "
                  dense
                  size="xs"
                  flat
                  class="q-mr-xs"
                  data-test="table-row-expand-menu"
                  @click.stop="toggleExpandFunctionError"
                />
                <div>
                  <span v-show="vrlFunctionError">Invalid VRL function</span>
                </div>
              </div>
              <div
                v-if="isFunctionErrorExpanded"
                class="q-px-sm q-pb-sm"
                :class="
                  store.state.theme === 'dark' ? 'bg-grey-10' : 'bg-grey-2'
                "
              >
                <pre class="q-my-none" style="white-space: pre-wrap">{{
                  vrlFunctionError
                }}</pre>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <div class="q-mt-sm">
      <div
        v-if="
          alertData.stream_type === 'metrics' &&
          tab === 'promql' &&
          promqlCondition
        "
        class="flex justify-start items-center text-bold q-mb-lg o2-input"
      >
        <div style="width: 190px">Trigger if the value is</div>
        <div class="flex justify-start items-center">
          <div data-test="scheduled-alert-promlq-condition-operator-select">
            <q-select
              v-model="promqlCondition.operator"
              :options="triggerOperators"
              color="input-border"
              bg-color="input-bg"
              class="no-case q-py-none q-mr-xs"
              filled
              borderless
              dense
              use-input
              hide-selected
              fill-input
              style="width: 88px; border-right: none"
              @update:model-value="updatePromqlCondition"
            />
          </div>
          <div
            data-test="scheduled-alert-promlq-condition-value"
            style="width: 160px; margin-left: 0 !important"
            class="silence-notification-input o2-input"
          >
            <q-input
              v-model="promqlCondition.value"
              type="number"
              dense
              filled
              min="0"
              style="background: none"
              placeholder="Value"
              @update:model-value="updatePromqlCondition"
            />
          </div>
        </div>
      </div>
      <div
        v-if="tab === 'custom'"
        class="flex justify-start items-center text-bold q-mb-lg"
      >
        <div data-test="scheduled-alert-aggregation-title" style="width: 172px">
          Aggregation
        </div>
        <q-toggle
          data-test="scheduled-alert-aggregation-toggle"
          v-model="_isAggregationEnabled"
          size="sm"
          color="primary"
          class="text-bold q-pl-0"
          :disable="tab === 'sql' || tab === 'promql'"
          @update:model-value="updateAggregation"
        />
      </div>
      <div
        v-if="_isAggregationEnabled && aggregationData"
        class="flex items-center no-wrap q-mr-sm q-mb-sm"
      >
        <div
          data-test="scheduled-alert-group-by-title"
          class="text-bold"
          style="width: 190px"
        >
          {{ t("alerts.groupBy") }}
        </div>
        <div
          class="flex justify-start items-center flex-wrap"
          style="width: calc(100% - 190px)"
        >
          <template
            v-for="(group, index) in aggregationData.group_by"
            :key="group"
          >
            <div
              :data-test="`scheduled-alert-group-by-${index + 1}`"
              class="flex justify-start items-center no-wrap o2-input"
            >
              <div data-test="scheduled-alert-group-by-column-select">
                <q-select
                  v-model="aggregationData.group_by[index]"
                  :options="filteredFields"
                  color="input-border"
                  bg-color="input-bg"
                  class="no-case q-py-none q-mb-sm"
                  filled
                  borderless
                  dense
                  use-input
                  emit-value
                  hide-selected
                  placeholder="Select column"
                  fill-input
                  :input-debounce="400"
                  @filter="
                    (val: string, update: any) => filterFields(val, update)
                  "
                  :rules="[(val: any) => !!val || 'Field is required!']"
                  style="width: 200px"
                  @update:model-value="updateTrigger"
                />
              </div>
              <q-btn
                data-test="scheduled-alert-group-by-delete-btn"
                :icon="outlinedDelete"
                class="iconHoverBtn q-mb-sm q-ml-xs q-mr-sm"
                :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
                padding="xs"
                unelevated
                size="sm"
                round
                flat
                :title="t('alert_templates.delete')"
                @click="deleteGroupByColumn(index)"
                style="min-width: auto"
              />
            </div>
          </template>
          <q-btn
            data-test="scheduled-alert-group-by-add-btn"
            icon="add"
            class="iconHoverBtn q-mb-sm q-ml-xs q-mr-sm"
            :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
            padding="xs"
            unelevated
            size="sm"
            round
            flat
            :title="t('common.add')"
            @click="addGroupByColumn()"
            style="min-width: auto"
          />
        </div>
      </div>
      <div
        v-if="!disableThreshold"
        class="flex justify-start items-center q-mb-xs no-wrap q-pb-md"
      >
        <div
          data-test="scheduled-alert-threshold-title"
          class="text-bold flex items-center"
          style="width: 190px"
        >
          {{ t("alerts.threshold") + " *" }}

          <q-icon
            :name="outlinedInfo"
            size="17px"
            class="q-ml-xs cursor-pointer"
            :class="
              store.state.theme === 'dark' ? 'text-grey-5' : 'text-grey-7'
            "
          >
            <q-tooltip
              anchor="center right"
              self="center left"
              max-width="300px"
            >
              <span style="font-size: 14px"
                >The threshold above/below which the alert will trigger. <br />
                e.g. if the threshold is >100 and the query returns a value of
                101 then the alert will trigger.</span
              >
            </q-tooltip>
          </q-icon>
        </div>
        <div style="width: calc(100% - 190px)" class="position-relative">
          <template v-if="_isAggregationEnabled && aggregationData">
            <div class="flex justify-start items-center">
              <div
                data-test="scheduled-alert-threshold-function-select"
                class="threshould-input q-mr-xs o2-input"
              >
                <q-select
                  v-model="aggregationData.function"
                  :options="aggFunctions"
                  color="input-border"
                  bg-color="input-bg"
                  class="no-case q-py-none"
                  filled
                  borderless
                  dense
                  use-input
                  hide-selected
                  fill-input
                  style="width: 120px"
                  @update:model-value="updateAggregation"
                />
              </div>
              <div
                class="threshould-input q-mr-xs o2-input"
                data-test="scheduled-alert-threshold-column-select"
              >
                <q-select
                  v-model="aggregationData.having.column"
                  :options="filteredNumericColumns"
                  color="input-border"
                  bg-color="input-bg"
                  class="no-case q-py-none"
                  filled
                  borderless
                  dense
                  use-input
                  emit-value
                  hide-selected
                  fill-input
                  @filter="filterNumericColumns"
                  style="width: 250px"
                  @update:model-value="updateAggregation"
                />
              </div>
              <div
                data-test="scheduled-alert-threshold-operator-select"
                class="threshould-input q-mr-xs o2-input q-mt-sm"
              >
                <q-select
                  v-model="aggregationData.having.operator"
                  :options="triggerOperators"
                  color="input-border"
                  bg-color="input-bg"
                  class="no-case q-py-none"
                  filled
                  borderless
                  dense
                  use-input
                  hide-selected
                  fill-input
                  style="width: 120px"
                  @update:model-value="updateAggregation"
                />
              </div>
              <div class="flex items-center q-mt-sm">
                <div
                  data-test="scheduled-alert-threshold-value-input"
                  style="width: 250px; margin-left: 0 !important"
                  class="silence-notification-input o2-input"
                >
                  <q-input
                    v-model="aggregationData.having.value"
                    type="number"
                    dense
                    filled
                    min="0"
                    style="background: none"
                    placeholder="Value"
                    @update:model-value="updateAggregation"
                  />
                </div>
              </div>
            </div>
            <div
              data-test="scheduled-alert-threshold-error-text"
              v-if="
                !aggregationData.function ||
                !aggregationData.having.column ||
                !aggregationData.having.operator ||
                !aggregationData.having.value.toString().trim().length
              "
              class="text-red-8 q-pt-xs absolute"
              style="font-size: 11px; line-height: 12px"
            >
              Field is required!
            </div>
          </template>
          <template v-else>
            <div class="flex justify-start items-center">
              <div
                class="threshould-input"
                data-test="scheduled-alert-threshold-operator-select"
              >
                <q-select
                  v-model="triggerData.operator"
                  :options="triggerOperators"
                  color="input-border"
                  bg-color="input-bg"
                  class="showLabelOnTop no-case q-py-none"
                  filled
                  borderless
                  dense
                  use-input
                  hide-selected
                  fill-input
                  :rules="[(val: any) => !!val || 'Field is required!']"
                  style="width: 88px; border: 1px solid rgba(0, 0, 0, 0.05)"
                  @update:model-value="updateTrigger"
                />
              </div>
              <div
                class="flex items-center"
                style="border: 1px solid rgba(0, 0, 0, 0.05); border-left: none"
              >
                <div
                  style="width: 89px; margin-left: 0 !important"
                  class="silence-notification-input"
                  data-test="scheduled-alert-threshold-value-input"
                >
                  <q-input
                    v-model="triggerData.threshold"
                    type="number"
                    dense
                    filled
                    min="1"
                    style="background: none"
                    debounce="300"
                    @update:model-value="updateTrigger"
                  />
                </div>
                <div
                  data-test="scheduled-alert-threshold-unit"
                  style="
                    min-width: 90px;
                    margin-left: 0 !important;
                    height: 40px;
                    font-weight: normal;
                  "
                  :class="
                    store.state.theme === 'dark' ? 'bg-grey-10' : 'bg-grey-2'
                  "
                  class="flex justify-center items-center"
                >
                  {{ t("alerts.times") }}
                </div>
              </div>
            </div>
            <div
              data-test="scheduled-alert-threshold-error-text"
              v-if="!triggerData.operator || !Number(triggerData.threshold)"
              class="text-red-8 q-pt-xs absolute"
              style="font-size: 11px; line-height: 12px"
            >
              Field is required!
            </div>
          </template>
        </div>
      </div>
      <div class="flex items-center q-mr-sm">
        <div
          data-test="scheduled-alert-period-title"
          class="text-bold flex items-center"
          style="width: 190px"
        >
          {{ t("alerts.period") + " *" }}
          <q-icon
            :name="outlinedInfo"
            size="17px"
            class="q-ml-xs cursor-pointer"
            :class="
              store.state.theme === 'dark' ? 'text-grey-5' : 'text-grey-7'
            "
          >
            <q-tooltip
              anchor="center right"
              self="center left"
              max-width="300px"
            >
              <span style="font-size: 14px"
                >Period for which the query should run.<br />
                e.g. 10 minutes means that whenever the query will run it will
                use the last 10 minutes of data. If the query runs at 4:00 PM
                then it will use the data from 3:50 PM to 4:00 PM.</span
              >
            </q-tooltip>
          </q-icon>
        </div>
        <div style="min-height: 58px">
          <div
            class="flex items-center q-mr-sm"
            style="border: 1px solid rgba(0, 0, 0, 0.05); width: fit-content"
          >
            <div
              data-test="scheduled-alert-period-input"
              style="width: 87px; margin-left: 0 !important"
              class="silence-notification-input"
            >
              <q-input
                v-model="triggerData.period"
                type="number"
                dense
                filled
                min="1"
                style="background: none"
                debounce="300"
                @update:model-value="updateTrigger"
              />
            </div>
            <div
              data-test="scheduled-alert-period-unit"
              style="
                min-width: 90px;
                margin-left: 0 !important;
                height: 40px;
                font-weight: normal;
              "
              :class="store.state.theme === 'dark' ? 'bg-grey-10' : 'bg-grey-2'"
              class="flex justify-center items-center"
            >
              {{ t("alerts.minutes") }}
            </div>
          </div>
          <div
            data-test="scheduled-alert-period-error-text"
            v-if="!Number(triggerData.period)"
            class="text-red-8 q-pt-xs"
            style="font-size: 11px; line-height: 12px"
          >
            Field is required!
          </div>
        </div>
      </div>
      <div v-if="tab == 'sql'">
        <div class="flex items-center q-mr-sm">
          <div
            data-test="multi-time-range-alerts-title"
            class="text-bold q-py-md flex items-center"
            style="width: 190px"
          >
            Multi Window Selection
            <q-btn
              no-caps
              padding="xs"
              class=""
              size="sm"
              flat
              icon="info_outline"
              data-test="multi-time-range-alerts-info-btn"
            >
              <q-tooltip
                anchor="bottom middle"
                self="top middle"
                style="font-size: 14px"
                max-width="300px"
              >
                <span
                  >Additional timeframe for query execution: <br />
                  For example, selecting "past 10 hours" means that each time
                  the query runs, it will retrieve data from 10 hours prior,
                  using the last 10 minutes of that period. <br />
                  If the query is scheduled from 4:00 PM to 4:10 PM,
                  additionally it will pull data from 6:00 AM to 6:10 AM.
                </span>
              </q-tooltip>
            </q-btn>
          </div>
        </div>
        <div
          v-for="(picker, index) in dateTimePicker"
          :key="index"
          class="q-mb-md"
        >
          <div class="flex">
            <CustomDateTimePicker
              v-model="picker.offSet"
              :picker="picker"
              :isFirstEntry="false"
              @update:model-value="updateDateTimePicker"
            />
            <q-btn
              data-test="multi-time-range-alerts-delete-btn"
              :icon="outlinedDelete"
              class="iconHoverBtn q-ml-xs q-mr-sm"
              :class="store.state?.theme === 'dark' ? 'icon-dark' : ''"
              padding="xs"
              unelevated
              size="sm"
              round
              flat
              :title="t('alert_templates.delete')"
              @click="removeTimeShift(index)"
              style="min-width: auto"
            />
          </div>
        </div>
        <div>
          <q-btn
            data-test="multi-time-range-alerts-add-btn"
            label="Add Time Shift"
            size="sm"
            class="text-bold add-variable"
            icon="add"
            style="
              border-radius: 4px;
              text-transform: capitalize;
              background: #f2f2f2 !important;
              color: #000 !important;
            "
            @click="addTimeShift"
          />
        </div>
      </div>
      <div class="flex items-center q-mr-sm">
        <div
          data-test="scheduled-alert-cron-toggle-title"
          class="text-bold flex items-center"
          style="width: 190px"
        >
          {{ t("alerts.crontitle") + " *" }}
          <q-icon
            :name="outlinedInfo"
            size="17px"
            class="q-ml-xs cursor-pointer"
            :class="
              store.state.theme === 'dark' ? 'text-grey-5' : 'text-grey-7'
            "
          >
            <q-tooltip
              anchor="center right"
              self="center left"
              max-width="300px"
            >
              <span style="font-size: 14px"
                >Configure the option to enable a cron job.</span
              >
            </q-tooltip>
          </q-icon>
        </div>
        <div style="min-height: 58px">
          <div class="flex items-center q-mr-sm" style="width: fit-content">
            <div
              data-test="scheduled-alert-cron-input"
              style="width: 87px; margin-left: 0 !important"
              class="silence-notification-input"
            >
              <q-toggle
                data-test="scheduled-alert-cron-toggle-btn"
                class="q-mt-sm"
                v-model="triggerData.frequency_type"
                :true-value="'cron'"
                :false-value="'minutes'"
                @update:model-value="updateTrigger"
              />
            </div>
          </div>
        </div>
      </div>
      <div class="flex items-center q-mr-sm">
        <div
          data-test="scheduled-alert-frequency-title"
          class="text-bold flex items-center"
          style="width: 190px"
        >
          {{ t("alerts.frequency") + " *" }}
          <q-icon
            :name="outlinedInfo"
            size="17px"
            class="q-ml-xs cursor-pointer"
            :class="
              store.state.theme === 'dark' ? 'text-grey-5' : 'text-grey-7'
            "
          >
            <q-tooltip
              anchor="center right"
              self="center left"
              max-width="auto"
            >
              <span
                style="font-size: 14px"
                v-if="triggerData.frequency_type == 'minutes'"
                >How often the task should be executed.<br />
                e.g., 2 minutes means that the task will run every 2 minutes and
                will be processed based on the other parameters provided.</span
              >
              <span style="font-size: 14px" v-else>
                Pattern: * * * * * * means every second.
                <br />
                Format: [Second (optional) 0-59] [Minute 0-59] [Hour 0-23] [Day
                of Month 1-31, 'L'] [Month 1-12] [Day of Week 0-7 or '1L-7L', 0
                and 7 for Sunday].
                <br />
                Use '*' to represent any value, 'L' for the last day/weekday.
                <br />
                Example: 0 0 12 * * ? - Triggers at 12:00 PM daily. It specifies
                second, minute, hour, day of month, month, and day of week,
                respectively.</span
              >
            </q-tooltip>
          </q-icon>
          <template
            v-if="triggerData.frequency_type == 'cron' && showTimezoneWarning"
          >
            <q-icon
              :name="outlinedWarning"
              size="18px"
              class="cursor-pointer tw-ml-[8px]"
              :class="
                store.state.theme === 'dark'
                  ? 'tw-text-orange-500'
                  : 'tw-text-orange-500'
              "
            >
              <q-tooltip
                anchor="center right"
                self="center left"
                max-width="auto"
                class="tw-text-[14px]"
              >
                Warning: The displayed timezone is approximate. Verify and
                select the correct timezone manually.
              </q-tooltip>
            </q-icon>
          </template>
        </div>
        <div style="min-height: 78px">
          <div class="flex items-center" style="width: fit-content">
            <div
              data-test="scheduled-alert-frequency-input"
              :style="
                triggerData.frequency_type == 'minutes'
                  ? 'width: 87px; margin-left: 0 !important'
                  : 'width: fit-content !important'
              "
              class="silence-notification-input"
            >
              <q-input
                data-test="scheduled-alert-frequency-input-field"
                v-if="triggerData.frequency_type == 'minutes'"
                v-model="triggerData.frequency"
                type="number"
                dense
                filled
                :min="
                  Math.ceil(
                    store.state?.zoConfig?.min_auto_refresh_interval / 60,
                  ) || 1
                "
                style="background: none"
                debounce="300"
                @update:model-value="updateTrigger"
              />
              <div v-else class="tw-flex tw-items-center o2-input">
                <q-input
                  data-test="scheduled-alert-cron-input-field"
                  v-model="triggerData.cron"
                  dense
                  filled
                  :label="t('reports.cron') + ' *'"
                  style="background: none; width: 180px"
                  class="showLabelOnTop"
                  stack-label
                  outlined
                  debounce="300"
                  @update:model-value="updateTrigger"
                />
                <q-select
                  data-test="add-report-schedule-start-timezone-select"
                  v-model="triggerData.timezone"
                  :options="filteredTimezone"
                  @blur="
                    browserTimezone =
                      browserTimezone == ''
                        ? Intl.DateTimeFormat().resolvedOptions().timeZone
                        : browserTimezone
                  "
                  use-input
                  @filter="timezoneFilterFn"
                  input-debounce="0"
                  dense
                  filled
                  emit-value
                  fill-input
                  hide-selected
                  :title="triggerData.timezone"
                  :label="t('logStream.timezone') + ' *'"
                  :display-value="`Timezone: ${browserTimezone}`"
                  class="timezone-select showLabelOnTop q-ml-sm"
                  stack-label
                  outlined
                  style="width: 210px"
                />
              </div>
            </div>
            <div
              v-if="triggerData.frequency_type == 'minutes'"
              data-test="scheduled-alert-frequency-unit"
              style="
                min-width: 90px;
                margin-left: 0 !important;
                height: 40px;
                font-weight: normal;
              "
              :class="store.state.theme === 'dark' ? 'bg-grey-10' : 'bg-grey-2'"
              class="flex justify-center items-center"
            >
              {{ t("alerts.minutes") }}
            </div>
          </div>
          <div
            data-test="scheduled-alert-frequency-error-text"
            v-show="
              (!Number(triggerData.frequency) &&
                triggerData.frequency_type == 'minutes') ||
              (triggerData.frequency_type == 'cron' &&
                (triggerData.cron == '' || !triggerData.timezone)) ||
              cronJobError
            "
            class="text-red-8 q-pt-xs"
            style="font-size: 11px; line-height: 12px"
          >
            {{ cronJobError || "Field is required!" }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, defineAsyncComponent } from "vue";
import FieldsInput from "@/components/alerts/FieldsInput.vue";
import { useI18n } from "vue-i18n";
import {
  outlinedDelete,
  outlinedInfo,
  outlinedWarning,
} from "@quasar/extras/material-icons-outlined";
import { useStore } from "vuex";
import {
  getImageURL,
  useLocalTimezone,
  getCronIntervalDifferenceInSeconds,
  isAboveMinRefreshInterval,
} from "@/utils/zincutils";
import { useQuasar } from "quasar";
import CustomDateTimePicker from "@/components/CustomDateTimePicker.vue";

const QueryEditor = defineAsyncComponent(
  () => import("@/components/QueryEditor.vue"),
);

const props = defineProps([
  "columns",
  "conditions",
  "trigger",
  "sql",
  "query_type",
  "aggregation",
  "isAggregationEnabled",
  "alertData",
  "promql",
  "promql_condition",
  "vrl_function",
  "showVrlFunction",
  "sqlQueryErrorMsg",
  "disableThreshold",
  "disableVrlFunction",
  "disableQueryTypeSelection",
  "vrlFunctionError",
  "showTimezoneWarning",
  "multi_time_range",
]);

const emits = defineEmits([
  "field:add",
  "field:remove",
  "update:trigger",
  "update:query_type",
  "update:sql",
  "update:aggregation",
  "update:isAggregationEnabled",
  "input:update",
  "update:promql",
  "update:promql_condition",
  "update:vrl_function",
  "update:showVrlFunction",
  "validate-sql",
  "update:multi_time_range",
]);

const { t } = useI18n();

const triggerData = ref(props.trigger);

const query = ref(props.sql);

const promqlQuery = ref(props.promql);

const tab = ref(props.query_type || "custom");

const q = useQuasar();

const store = useStore();

const functionEditorPlaceholderFlag = ref(true);

const queryEditorPlaceholderFlag = ref(true);

const isFunctionErrorExpanded = ref(false);

const metricFunctions = ["p50", "p75", "p90", "p95", "p99"];
const regularFunctions = ["avg", "max", "min", "sum", "count"];

const aggFunctions = computed(() =>
  props.alertData.stream_type === "metrics"
    ? [...regularFunctions, ...metricFunctions]
    : [...regularFunctions],
);

const _isAggregationEnabled = ref(
  tab.value === "custom" && props.isAggregationEnabled,
);

const promqlCondition = ref(props.promql_condition);

const aggregationData = ref(props.aggregation);

const filteredFields = ref(props.columns);

const fnEditorRef = ref<any>(null);

const filteredTimezone: any = ref([]);

const cronJobError = ref("");

const getNumericColumns = computed(() => {
  if (
    _isAggregationEnabled.value &&
    aggregationData &&
    aggregationData.value.function === "count"
  )
    return props.columns;
  else
    return props.columns.filter((column: any) => {
      return column.type !== "Utf8";
    });
});

const filteredNumericColumns = ref(getNumericColumns.value);

const addField = () => {
  emits("field:add");
};

const updateDateTimePicker = (data: any) => {
  emits("update:multi_time_range", dateTimePicker.value);
  console.log("data", dateTimePicker.value);
};

const removeTimeShift = (index: any) => {
  dateTimePicker.value.splice(index, 1);
};

var triggerOperators: any = ref(["=", "!=", ">=", "<=", ">", "<"]);

const selectedFunction = ref("");

const currentTimezone =
  useLocalTimezone() || Intl.DateTimeFormat().resolvedOptions().timeZone;

const browserTimezone = ref(currentTimezone);

// @ts-ignore
let timezoneOptions = Intl.supportedValuesOf("timeZone").map((tz: any) => {
  return tz;
});

filteredTimezone.value = [...timezoneOptions];

const browserTime =
  "Browser Time (" + Intl.DateTimeFormat().resolvedOptions().timeZone + ")";

// Add the UTC option
timezoneOptions.unshift("UTC");
timezoneOptions.unshift(browserTime);

const timezoneFilterFn = (val: string, update: Function) => {
  filteredTimezone.value = filterColumns(timezoneOptions, val, update);
};

const removeField = (field: any) => {
  emits("field:remove", field);
};

const updateQueryValue = (value: string) => {
  query.value = value;

  if (tab.value === "sql") emits("update:sql", value);
  if (tab.value === "promql") emits("update:promql", value);

  emits("input:update", "query", value);
};

const updateTrigger = () => {
  cronJobError.value = "";
  validateFrequency(triggerData.value);

  emits("update:trigger", triggerData.value);
  emits("input:update", "period", triggerData.value);
};

const updateTab = () => {
  updateQuery();
  updateAggregationToggle();
  emits("update:query_type", tab.value);
  emits("input:update", "query_type", tab.value);
};

const getDefaultPromqlCondition = () => {
  return {
    column: "value",
    operator: ">=",
    value: 0,
  };
};

const onFunctionSelect = (_function: any) => {
  selectedFunction.value = _function.name;
  vrlFunctionContent.value = _function.function;
};

const functionsList = computed(() => store.state.organizationData.functions);
const dateTimePicker = ref(props.multi_time_range || []);

const functionOptions = ref<any[]>([]);

watch(
  () => functionsList.value,
  (functions: any[]) => {
    functionOptions.value = [...functions];
  },
);

const vrlFunctionContent = computed({
  get() {
    return props.vrl_function || "";
  },
  set(value) {
    emits("update:vrl_function", value);
  },
});

const isVrlFunctionEnabled = computed({
  get() {
    return props.showVrlFunction;
  },
  set(value) {
    emits("update:showVrlFunction", value);
    updateFunctionVisibility(value);
  },
});

const updateFunctionVisibility = (isEnabled: boolean) => {
  if (!isEnabled) {
    vrlFunctionContent.value = null;
    selectedFunction.value = "";
  }
};

const updateQuery = () => {
  if (tab.value === "promql") {
    const condition = !props.promql_condition
      ? getDefaultPromqlCondition()
      : props.promql_condition;
    promqlCondition.value = condition;
    emits("update:promql_condition", condition);
    promqlQuery.value = props.promql;
  }

  if (tab.value === "sql") query.value = props.sql;
};
const addTimeShift = () => {
  const newTimeShift = {
    offSet: "15m",
    data: {
      selectedDate: {
        relative: {
          value: 15,
          period: "m",
          label: "Minutes",
        },
      },
    },
  };

  dateTimePicker.value.push({ offSet: newTimeShift.offSet });
};

const updatePromqlCondition = () => {
  emits("update:promql_condition", promqlCondition.value);
  emits("input:update", "promql_condition", promqlCondition.value);
};

const addGroupByColumn = () => {
  const aggregationDataCopy = { ...aggregationData.value };
  aggregationDataCopy.group_by.push("");
  emits("update:aggregation", aggregationDataCopy);
  emits("input:update", "aggregation", aggregationDataCopy);
};

const deleteGroupByColumn = (index: number) => {
  const aggregationDataCopy = { ...aggregationData.value };
  aggregationDataCopy.group_by.splice(index, 1);
  emits("update:aggregation", aggregationDataCopy);
  emits("input:update", "aggregation", aggregationDataCopy);
};

const updateAggregation = () => {
  if (!props.aggregation) {
    aggregationData.value = {
      group_by: [""],
      function: "avg",
      having: {
        column: "",
        operator: "=",
        value: "",
      },
    };
  }
  emits("update:aggregation", aggregationData.value);
  emits("update:isAggregationEnabled", _isAggregationEnabled.value);
  emits("input:update", "aggregation", aggregationData.value);
};

const filterFields = (val: string, update: Function) => {
  filteredFields.value = filterColumns(props.columns, val, update);
};

const filterColumns = (options: string[], val: string, update: Function) => {
  let filteredOptions: any[] = [];

  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
  }

  console.log("options", options);

  update(() => {
    const value = val.toLowerCase();
    filteredOptions = options.filter((column: any) => {
      // Check if type of column is object or string and then filter
      if (typeof column === "object") {
        return column.value.toLowerCase().indexOf(value) > -1;
      }

      if (typeof column === "string") {
        return column.toLowerCase().indexOf(value) > -1;
      }
    });
  });

  return filteredOptions;
};

const filterNumericColumns = (val: string, update: Function) => {
  if (val === "") {
    update(() => {
      filteredNumericColumns.value = [...getNumericColumns.value];
    });
  }
  update(() => {
    const value = val.toLowerCase();
    filteredNumericColumns.value = getNumericColumns.value.filter(
      (column: any) => column.value.toLowerCase().indexOf(value) > -1,
    );
  });
};

const updateAggregationToggle = () => {
  _isAggregationEnabled.value =
    tab.value === "custom" && props.isAggregationEnabled;
};

const filterFunctionOptions = (val: string, update: any) => {
  update(() => {
    functionOptions.value = functionsList.value.filter((fn: any) => {
      return fn.name.toLowerCase().indexOf(val.toLowerCase()) > -1;
    });
  });
};

const onBlurQueryEditor = async () => {
  queryEditorPlaceholderFlag.value = true;
  emits("validate-sql");
};

const validateInputs = (notify: boolean = true) => {
  if (
    Number(triggerData.value.period) < 1 ||
    isNaN(Number(triggerData.value.period))
  ) {
    notify &&
      q.notify({
        type: "negative",
        message: "Period should be greater than 0",
        timeout: 1500,
      });
    return false;
  }

  if (aggregationData.value) {
    if (
      !props.disableThreshold &&
      (isNaN(triggerData.value.threshold) ||
        !aggregationData.value.having.value.toString().trim().length ||
        !aggregationData.value.having.column ||
        !aggregationData.value.having.operator)
    ) {
      notify &&
        q.notify({
          type: "negative",
          message: "Threshold should not be empty",
          timeout: 1500,
        });
      return false;
    }

    return true;
  }

  if (
    !props.disableThreshold &&
    (isNaN(triggerData.value.threshold) ||
      triggerData.value.threshold < 1 ||
      !triggerData.value.operator)
  ) {
    notify &&
      q.notify({
        type: "negative",
        message: "Threshold should not be empty",
        timeout: 1500,
      });
    return false;
  }

  return true;
};

const onBlurFunctionEditor = async () => {
  functionEditorPlaceholderFlag.value = true;
  emits("validate-sql");
};

const toggleExpandFunctionError = () => {
  isFunctionErrorExpanded.value = !isFunctionErrorExpanded.value;
};

const validateFrequency = (frequency: {
  frequency_type: string;
  cron: string;
  frequency: number;
}) => {
  if (frequency.frequency_type === "cron") {
    try {
      const intervalInSecs = getCronIntervalDifferenceInSeconds(frequency.cron);

      if (
        typeof intervalInSecs === "number" &&
        !isAboveMinRefreshInterval(intervalInSecs, store.state?.zoConfig)
      ) {
        const minInterval =
          Number(store.state?.zoConfig?.min_auto_refresh_interval) || 1;
        cronJobError.value = `Frequency should be greater than ${minInterval - 1} seconds.`;
        return;
      }
    } catch (e) {
      cronJobError.value = "Invalid cron expression";
    }
  }

  if (frequency.frequency_type === "minutes") {
    const intervalInMins = Math.ceil(
      store.state?.zoConfig?.min_auto_refresh_interval / 60,
    );

    if (frequency.frequency < intervalInMins) {
      cronJobError.value =
        "Frequency should be greater than " + (intervalInMins - 1);
      return;
    }
  }
};

defineExpose({
  tab,
  validateInputs,
  cronJobError,
  validateFrequency,
});
</script>

<style lang="scss" scoped>
.scheduled-alert-tabs {
  border: 1px solid $primary;
  width: 300px;
  border-radius: 4px;
  overflow: hidden;
}
</style>
<style lang="scss" >
.scheduled-alert-tabs {
  .q-tab--active {
    background-color: $primary;
    color: $white;
  }

  .q-tab__indicator {
    display: none;
  }

  .q-tab {
    height: 28px;
    min-height: 28px;
  }
}
.scheduled-alerts {
  .monaco-editor {
    width: 500px !important;
    height: 100px !important;
    border: 1px solid $border-color;
  }

  .q-btn {
    &.icon-dark {
      filter: none !important;
    }
  }

  .empty-query .monaco-editor-background {
    background-image: url("../../assets/images/common/query-editor.png");
    background-repeat: no-repeat;
    background-size: 115px;
  }

  .empty-function .monaco-editor-background {
    background-image: url("../../assets/images/common/vrl-function.png");
    background-repeat: no-repeat;
    background-size: 170px;
  }
}

.invalid-sql-error {
  min-height: 21px;
}
</style>
