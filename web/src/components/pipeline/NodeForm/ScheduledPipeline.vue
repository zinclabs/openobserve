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
  <div class="full-height full-width scheduled-pipeline-container">
    <div class="flex items-center justify-between q-pb-sm">
        <div class="flex items-center">
          <q-btn icon="close" size="12px" class="q-pt-sm" flat dense   @click="$emit('cancel:form')"/>
                <div class=" q-pb-sm stream-routing-title  q-pl-xs">
                  {{ t("pipeline.query") }}
                </div>
        </div>
          <div class="flex items-center">
            <div class="flex items-center">
              <q-tabs
                data-test="scheduled-pipeline-tabs"
                v-model="tab"
                no-caps
                outside-arrows
                size="sm"
                mobile-arrows
                class="bg-white text-primary scheduled-pipeline-tabs q-mr-sm"
                @update:model-value="updateTab"
                style="height: 34px !important;"
                :disable="selectedStreamType != 'metrics'"
              >
                <q-tab
                  data-test="scheduled-pipeline-sql-tab"
                  name="sql"
                  :label="t('alerts.sql')"
                />
                <q-tab
                  data-test="scheduled-pipeline-metrics-tab"
                  name="promql"
                  :disable="selectedStreamType !== 'metrics'"
                  :label="t('alerts.promql')"
                >
                  <q-tooltip v-if="selectedStreamType !== 'metrics'">
                    Promql is only available for metrics stream type
                  </q-tooltip>
                </q-tab>
              </q-tabs>
            </div>
            <DateTime
            style="height: 34px !important; border-radius: 3px;"
            @on:date-change="updateDateChange"
            class="q-mr-sm" />
              <q-btn
                data-test="logs-search-bar-refresh-btn"
                data-cy="search-bar-refresh-button"
                dense
                flat
                :title="t('search.runQuery')"
                class="q-pa-none q-mr-sm search-button-pipeline"
                @click="{
                  expandState.output = true;
                  expandState.query = false;
                  runQuery();
                }"
                >{{ t("search.runQuery") }}</q-btn
              >

          <q-btn
              data-test="add-function-fullscreen-btn"
              :text-color="store.state.theme === 'dark' ? 'grey-1' : 'primary'"
              dense
              style="height: 34px;" 
              no-caps
              :icon="isFullscreen ? 'fullscreen_exit' : 'fullscreen'"
              @click="handleFullScreen"
            />

         
          </div>



  </div>
  <q-separator />

    <div  class="q-mb-sm stepper-header" >
            <div  class="" style="height: 100% !important;">
  
           
  
           <q-splitter
              v-model="splitterModel"
              style="width: 100%;"
              class="full-height"
            >
            <template   #before>
              <q-splitter
              v-model="sideBarSplitterModel"
              style="width: 100%; height: calc(100vh - 90px) !important;"
              class="full-height"
              horizontal
            >
            <template #before>
                            <!-- fieldlist section -->
                <FullViewContainer
                  name="query"
                  v-model:is-expanded="expandState.buildQuery"
                  label="Build Query"
                  class="tw-mt-1"
                  @click="handleBuildQuery"
                />
              <div class="q-pt-sm" v-show="expandState.buildQuery"  >
                
              <div >
                <q-select
                v-model="selectedStreamType"
                :options="streamTypes"
                option-label="label"
                option-value="value"
                :label="t('alerts.stream_name') + ' *'"
                :popup-content-style="{ textTransform: 'lowercase' }"
                color="input-border"
                bg-color="input-bg"
                class="  no-case full-width"
                filled
                stack-label
                dense
                use-input
                hide-selected
                fill-input
                input-debounce="300"
                @update:model-value="getStreamList"
               />

                <q-select
                v-model="selectedStreamName"
                :options="filteredStreams"
                option-label="label"
                option-value="value"
                :label="t('alerts.stream_name') + ' *'"
                :popup-content-style="{ textTransform: 'lowercase' }"
                color="input-border"
                bg-color="input-bg"
                class="q-py-xs  no-case full-width"
                filled
                stack-label
                dense
                use-input
                hide-selected
                fill-input
                input-debounce="300"
                @update:model-value="getStreamFields"
                @filter="filterStreams"
                @popup-show="getStreamList"
                map-options
                emit-value
               />

              </div>
                
                <FieldList 
                  :fields="streamFields"
                  :stream-name="selectedStreamName"
                  @event-emitted="handleSidebarEvent"
                  :time-stamp="{
                      startTime: dateTime.startTime,
                      endTime: dateTime.endTime,
                    }"
                    :hideIncludeExlcude="true"
                    :hideCopyValue="false"
                />
               
                
              </div>
            </template>
            <template #after>
              <!-- set variables part -->
              <FullViewContainer
                  name="query"
                  v-model:is-expanded="expandState.setVariables"
                  label="Set Variables"
                  class="tw-mt-1"
                  @click="handleSetVariables"
                />
              <div v-show="expandState.setVariables"  class="flex justify-between q-pl-sm full-height" style="overflow-y: auto !important;">
              <div>
              <div
                v-if="
                  selectedStreamType === 'metrics' &&
                  tab === 'promql' &&
                  promqlCondition
                "
                class="flex justify-start items-center text-bold q-mb-sm q-mt-md o2-input"
              >
                <div style="width: 130px">Trigger
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
                        >Based upon the condition of trigger the pipeline will get trigger <br />
                        e.g. if the trigger value is >100 and the query returns a value of
                        101 then the pipeline will trigger.</span
                      >
                    </q-tooltip>
                  </q-icon>
                </div>
                <div class="flex justify-start items-center">
                  <div data-test="scheduled-pipeline-promlq-condition-operator-select">
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
                    data-test="scheduled-pipeline-promlq-condition-value"
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
                <div
                  data-test="scheduled-pipeline-aggregation-title"
                  style="width: 172px"
                >
                  Aggregation
                </div>
                <q-toggle
                  data-test="scheduled-pipeline-aggregation-toggle"
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
                  data-test="scheduled-pipeline-group-by-title"
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
                      :data-test="`scheduled-pipeline-group-by-${index + 1}`"
                      class="flex justify-start items-center no-wrap o2-input"
                    >
                      <div data-test="scheduled-pipeline-group-by-column-select">
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
                          @filter="filterFields"
                          :rules="[(val: any) => !!val || 'Field is required!']"
                          style="width: 200px"
                          @update:model-value="updateTrigger"
                        />
                      </div>
                      <q-btn
                        data-test="scheduled-pipeline-group-by-delete-btn"
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
                    data-test="scheduled-pipeline-group-by-add-btn"
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
                  data-test="scheduled-pipeline-threshold-title"
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
                        data-test="scheduled-pipeline-threshold-function-select"
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
                        data-test="scheduled-pipeline-threshold-column-select"
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
                        data-test="scheduled-pipeline-threshold-operator-select"
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
                          data-test="scheduled-pipeline-threshold-value-input"
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
                      data-test="scheduled-pipeline-threshold-error-text"
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
                        data-test="scheduled-pipeline-threshold-operator-select"
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
                          data-test="scheduled-pipeline-threshold-value-input"
                        >
                          <q-input
                            v-model="triggerData.threshold"
                            type="number"
                            dense
                            filled
                            min="1"
                            style="background: none"
                            @update:model-value="updateTrigger"
                          />
                        </div>
                        <div
                          data-test="scheduled-pipeline-threshold-unit"
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
                      data-test="scheduled-pipeline-threshold-error-text"
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
                  data-test="scheduled-pipeline-cron-toggle-title"
                  class="text-bold flex items-center"
                  style="width: 130px"
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
                        >Configure the option to enable a cron expression.</span
                      >
                    </q-tooltip>
                  </q-icon>
                </div>
                <div style="min-height: 58px">
                  <div class="flex items-center q-mr-sm" style="width: fit-content">
                    <div
                      data-test="scheduled-pipeline-cron-input"
                      class="silence-notification-input"
                    >
                      <q-toggle
                        data-test="scheduled-pipeline-cron-toggle-btn"
                        class="q-mt-sm"
                        v-model="triggerData.frequency_type"
                        :true-value="'cron'"
                        :false-value="'minutes'"
                      />
                    </div>
                  </div>
                </div>
              </div>
              <div class="flex items-center justify-start q-mr-sm">
                <div
                  data-test="scheduled-pipeline-frequency-title"
                  class="text-bold flex items-center q-mr-xs"
                  :style="{
                    width: triggerData.frequency_type == 'minutes' ? '130px' : '100px',
                  }"
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
                <div style="max-height: 50px;" class="q-mb-sm">
                  <div class="flex items-center q-mr-sm" style="width: fit-content">
                    <div
                      data-test="scheduled-pipeline-frequency-input"
                      :style="
                        triggerData.frequency_type == 'minutes'
                          ? 'width: 87px; margin-left: 0 !important;margin-top: 10px'
                          : 'width: fit-content !important'
                      "
                      class="silence-notification-input"
                    >
                      <q-input
                        data-test="scheduled-pipeline-frequency-input-field"
                        v-if="triggerData.frequency_type == 'minutes'"
                        v-model="triggerData.frequency"
                        type="number"
                        dense
                        filled
                        min="1"
                        style="background: none"
                        @update:model-value="updateFrequency"
                      />
                      <div v-else  class="tw-flex tw-items-center o2-input">
                        <q-input
                          data-test="scheduled-pipeline-cron-input-field"
                          v-model="triggerData.cron"
                          dense
                          filled
                          :label="t('reports.cronExpression') + ' *'"
                          style="background: none; width: 130px;"
                          class="showLabelOnTop"
                          stack-label
                          outlined
                          @update:model-value="updateCron"
                          required
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
                          style="width: 200px"
                        />
                      </div>
                    </div>
                    <div
                      v-if="triggerData.frequency_type == 'minutes'"
                      data-test="scheduled-pipeline-frequency-unit"
                      style="
                        margin-left: 0 !important;
                        height: 40px;
                        font-weight: normal;
                        margin-top: 10px;
                        width: 87px
                      "
                      :class="store.state.theme === 'dark' ? 'bg-grey-10' : 'bg-grey-2'"
                      class="flex justify-center items-center"
                    >
                      {{ t("alerts.minutes") }}
                    </div>
                  </div>
                  <div
                    data-test="scheduled-pipeline-frequency-error-text"
                    v-if="
                      (!Number(triggerData.frequency) &&
                        triggerData.frequency_type == 'minutes') ||
                      (triggerData.frequency_type == 'cron' &&
                        triggerData.cron == '') ||
                      cronJobError
                    "
                    class="text-red-8 q-pt-xs"
                    style="font-size: 11px; line-height: 12px"
                  >
                    {{ cronJobError || "Field is required!" }}
                  </div>
                </div>
              </div>
              <div class="flex  items-center q-mr-sm q-mt-lg">
                <div
                  data-test="scheduled-pipeline-period-title"
                  class="text-bold flex items-center q-pb-sm"
                  style="width: 130px"
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
                      data-test="scheduled-pipeline-period-input"
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
                        v-bind:readonly="triggerData.frequency_type == 'minutes'"
                        v-bind:disable="triggerData.frequency_type == 'minutes'"
                        @update:model-value="updateTrigger"
                      />
                    </div>
                    <div
                      data-test="scheduled-pipeline-period-unit"
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

                </div>
                <div
                    data-test="scheduled-pipeline-period-error-text"
                    v-if="!Number(triggerData.period)"
                    class="text-red-8 q-pt-xs"
                    style="font-size: 11px; line-height: 12px"
                  >
                    Field is required!
                  </div>
                  <div
                    data-test="scheduled-pipeline-period-warning-text"
                    v-else
                    class="text-primary q-pt-xs"
                    style="font-size: 12px; line-height: 12px; padding: 2px 0px"
                  >
                    Note: The period should be the same as frequency.
                  </div>
          
              </div>
              </div>

                <div
                  class="flex justify-start items-end q-mt-lg q-pb-lg full-width"
                  :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
                >
                
                </div>

      
                 </div>
            </template>
            </q-splitter>

            </template>
            <template #separator>
              <q-btn
                    data-test="logs-search-field-list-collapse-btn"
                    :icon="
                      collapseFields
                        ? 'chevron_right'
                        : 'chevron_left'
                    "
                    :title="
                     collapseFields
                        ? 'Collapse Fields'
                        : 'Open Fields'
                    "
                    dense
                    size="20px"
                    round
                    class="q-mr-xs field-list-collapse-btn"
                    color="primary"
                    style="left: 10px; position: absolute; overflow: auto !important; top: 0px; z-index: 100 !important;"
                   
                    @click="collapseFieldList"
                  ></q-btn>
            </template>
            <template #after>
             
              <div class="full-width" style="height: calc(100vh - 140px) !important;" >
              <div class="query-editor-container scheduled-pipelines">
                <FullViewContainer
                  name="query"
                  v-model:is-expanded="expandState.query"
                  :label="tab === 'sql' ? 'Sql Query' : 'PromQL Query'"
                  class="tw-mt-1"
                  @click="{
                    expandState.query = !expandState.query;
                    expandState.output = false;
                  }"
                />
                <query-editor
                v-show="expandState.query"
                data-test="scheduled-pipeline-sql-editor"
                ref="pipelineEditorRef"
                editor-id="pipeline-query-editor"
                class="monaco-editor"
                v-model:query="query"
                :class="query == '' && queryEditorPlaceholderFlag ? 'empty-query' : ''"
                @update:query="updateQueryValue"
                @focus="queryEditorPlaceholderFlag = false"
                @blur="onBlurQueryEditor"
                style="height: calc(100vh - 190px) !important;"
              />
              
             
              </div>
              <div>
                <FullViewContainer
                  name="output"
                  v-model:is-expanded="expandState.output"
                  label="Output"
                  class="tw-mt-1"
                  @click="{
                    expandState.output = !expandState.output;
                    expandState.query = false;
                  }"
                />
                  <TenstackTable
                  v-if="rows.length > 0 && tab == 'sql'"
                    style="height: calc(100vh - 190px) !important;"
                    v-show="expandState.output"
                    ref="searchTableRef"
                    :columns="getColumns"
                    :rows="rows"
                  />
                  <div v-if="loading" style="height: calc(100vh - 190px) !important;" class="flex justify-center items-center" >
                    <q-spinner-hourglass color="primary" size="lg" />

                  </div>

                  <div v-else-if="rows.length == 0 && expandState.output && tab == 'sql'" style="height: calc(100vh - 190px) !important;" >
                    <h6
                    v-if="selectedStreamName == ''"
                    data-test="logs-search-no-stream-selected-text"
                    class="text-center col-10 q-mx-none"
                  >
                    <q-icon name="info" color="primary" size="md" /> Select a
                    stream and press 'Run query' to continue. Additionally, you
                    can apply additional filters and adjust the date range to
                    enhance search.
                  </h6>
                  <h6
                  v-else
                    data-test="logs-search-no-stream-selected-text"
                    class="text-center col-10 q-mx-none"
                  >
                    <q-icon name="info" color="primary" size="md" />
                    {{ t("search.applySearch") }}
                    
                  </h6>
                  </div>

                  <div v-else-if="tab == 'promql'">
                    <PreviewPromqlQuery
                    ref="previewPromqlQueryRef"
                    :query="query"
                    :stream_name="selectedStreamName"
                    :stream_type="selectedStreamType"
                    :dateTime="dateTime"
                    />
                  </div>
  
              </div>

             
            </div>
            <div class="flex justify-end q-mt-md">

          <q-btn
                  data-test="stream-routing-query-cancel-btn"
                  class="text-bold q-ml-md no border"
                  :label="t('alerts.cancel')"
                  text-color="light-text"
                  padding="sm md"
                  no-caps
                  @click="$emit('cancel:form')"
                />
                <q-btn
                  data-test="stream-routing-query-save-btn"
                  :label="t('alerts.save')"
                  class="text-bold no-border q-ml-md"
                  color="secondary"
                  padding="sm xl"
                  no-caps
                  type="submit"
                  @click="$emit('submit:form')"
                />
                <q-btn
                  v-if="pipelineObj.isEditNode"
                  data-test="stream-routing-query-delete-btn"
                  :label="t('pipeline.deleteNode')"
                  class="text-bold no-border q-ml-md"
                  color="negative"
                  padding="sm xl"
                  no-caps
                  @click="$emit('delete:node')"
                />
          </div>   

            
            </template>
          
          </q-splitter>

                



           <!-- query-eidtor-part -->
            
          </div>





   </div>   

  </div>
  
</template>

<script setup lang="ts">
import {
  ref,
  watch,
  computed,
  type Ref,
  defineAsyncComponent,
  nextTick,
  onMounted,
} from "vue";
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
  timestampToTimezoneDate,
} from "@/utils/zincutils";
import useQuery from "@/composables/useQuery";
import searchService from "@/services/search";
import { useQuasar } from "quasar";
import cronParser from "cron-parser";
import useDragAndDrop from "@/plugins/pipelines/useDnD";
import IndexList from "@/plugins/logs/IndexList.vue";
import { split } from "postcss/lib/list";
import FullViewContainer from "@/components/functions/FullViewContainer.vue";
import SearchResult from "@/plugins/logs/SearchResult.vue";

import DateTime from "@/components/DateTime.vue";

import FieldList from "@/components/common/sidebar/FieldList.vue";
import useStreams from "@/composables/useStreams";

import TenstackTable from "@/plugins/logs/TenstackTable.vue";
import PreviewPromqlQuery from "./PreviewPromqlQuery.vue";

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
  "isValidSqlQuery",
  "disableThreshold",
  "disableVrlFunction",
  "disableQueryTypeSelection",
  "showTimezoneWarning",
  "streamType"
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
  "update:frequency",
  "update:stream_type",
  "submit:form",
  "cancel:form",
  "delete:node",
  "update:fullscreen",
  "update:stream_type",
]);
const {  pipelineObj } = useDragAndDrop();

const { getStream, getStreams } = useStreams ();

const selectedStreamName = ref("");

const streamOptions = ref([]);

const getColumns = computed(() => {
  return [
    {
      name: "_timestamp",
      id: "_timestamp",
      accessorFn: (row: any) => timestampToTimezoneDate(
        row["_timestamp"] / 1000,
        store.state.timezone,
        "yyyy-MM-dd HH:mm:ss.SSS"
      ),
      prop: (row: any) => timestampToTimezoneDate(
        row["_timestamp"] / 1000,
        store.state.timezone,
        "yyyy-MM-dd HH:mm:ss.SSS"
      ),
      label: t("search.timestamp") + ` (${store.state.timezone})`,
      header: t("search.timestamp") + ` (${store.state.timezone})`,
      align: "left",
      sortable: true,
      enableResizing: false,
      meta: {
        closable: false,
        showWrap: false,
        wrapContent: true
      },
      size: 200
    },
    {
      name: "source",
      id: "source",
      accessorFn: (row: any) => JSON.stringify(row),
      cell: (info: any) => info.getValue(),
      header: "source",
      sortable: true,
      enableResizing: false,
      meta: {
        closable: false,
        showWrap: false,
        wrapContent: false
      }
    }
  ];
});





const { t } = useI18n();

const triggerData = ref(props.trigger);

const query = ref(props.sql);

const promqlQuery = ref(props.promql);

const tab = ref(props.query_type || "custom");
const stream_type = ref(props.streamType || "logs");
const collapseFields = ref(false);

const q = useQuasar();

const store = useStore();

const functionEditorPlaceholderFlag = ref(true);

const queryEditorPlaceholderFlag = ref(true);
const  pipelineEditorRef : any = ref(null);
const splitterModel = ref(30);
const step = ref(1);
const dateTime  = ref({
  startTime: null,
  endTime: null,
  relativeTimePeriod: null,
  valueType: "absolute",
});
const streamFields: any = ref([]);
const previewPromqlQueryRef : any = ref(null);

const loading = ref(false);

const selectedStreamType = ref(props.streamType || "logs");


const filteredStreams = ref<any[]>([]);
const streams = ref([]);

watch(()=> collapseFields.value ,  (val)=>{
  if(val == true){
    splitterModel.value = 0;
  }
},{
  immediate: true,
  deep: true,
})
watch(()=> splitterModel.value ,  (val)=>{
  if(val == 10){
   splitterModel.value = 0;
  }
})


onMounted(async ()=>{
  getStreamList();
})
const standardOutput = ref(`
{
  "data": {
    "results": [
      {
        "event": {
          "_timestamp": 1735128523652186,
          "job": "test",
          "level": "info",
          "log": "test message for openobserve",
        },
      },
      {
        "event": {
          "log": "test message for openobserve",
        },
        "message": "Error in event",
      },
    ],
  },
}
`)

const filteredTimezone: any = ref([]);
const expandState =ref( {
  output: false,
  query: true,
  buildQuery: true,
  setVariables: true

})

const sideBarSplitterModel = ref(60);

watch(()=> sideBarSplitterModel.value ,  (val)=>{
  if(val == 10){
   sideBarSplitterModel.value = 5;
  }
  if(val == 90){
    sideBarSplitterModel.value = 95;
  }
})

watch(()=> selectedStreamType.value, (val)=>{
  if(val != "metrics"){
    tab.value = "sql";
  }
  selectedStreamName.value = "";
  streamFields.value = [];
  query.value = "";
  expandState.value.query = true;
  expandState.value.output = false;
  emits("update:stream_type", val);
})

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

const cronJobError = ref("");

const filteredNumericColumns = ref(getNumericColumns.value);

const currentTimezone =
  useLocalTimezone() || Intl.DateTimeFormat().resolvedOptions().timeZone;

const browserTimezone = ref(currentTimezone);
const streamTypes = ["logs", "metrics", "traces"];

const rows = ref([ ])

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

const addField = () => {
  emits("field:add");
};

var triggerOperators: any = ref(["=", "!=", ">=", "<=", ">", "<"]);

const selectedFunction = ref("");

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
  emits("update:trigger", triggerData.value);
  emits("input:update", "period", triggerData.value);
};
const updateStreamType = () => {
  if (stream_type.value != "metrics") {
    tab.value = "sql";
  }
  emits("update:stream_type", stream_type.value);
};

const updateFrequency = async () => {
  cronJobError.value = "";

  validateFrequency();

  triggerData.value.period = Number(triggerData.value.frequency);

  emits("update:trigger", triggerData.value);
  emits("input:update", "period", triggerData.value);
};

function convertCronToMinutes(cronExpression: string) {
  cronJobError.value = "";
  // Parse the cron expression using cron-parser
  try {
    const interval = cronParser.parseExpression(cronExpression);
    // Get the first and second execution times
    const firstExecution = interval.next();
    const secondExecution = interval.next();

    // Calculate the difference in milliseconds
    const diffInMs = secondExecution.getTime() - firstExecution.getTime();

    // Convert milliseconds to minutes
    const diffInMinutes = diffInMs / (1000 * 60);

    return diffInMinutes;
  } catch (err) {
    cronJobError.value = "Invalid cron expression";
    return -1;
  }
}

const updateCron = () => {
  cronJobError.value = "";

  let minutes = 0;
  try {
    minutes = convertCronToMinutes(triggerData.value.cron);
    validateFrequency();

    if (minutes < 0) return;

    // Check if the number is a float by checking if the value has a decimal part
    if (minutes % 1 !== 0) {
      // If it's a float, fix it to 2 decimal places
      minutes = Number(minutes.toFixed(2));
    } else {
      // If it's an integer, return it as is
      minutes = Number(minutes.toString());
    }
  } catch (err) {
    console.log(err);
    return;
  }

  triggerData.value.period = minutes;

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

const functionOptions = ref<any[]>([]);

watch(
  () => functionsList.value,
  (functions: any[]) => {
    functionOptions.value = [...functions];
  },
);

const vrlFunctionContent = computed({
  get() {
    return props.vrl_function;
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
  },
});

const updateQuery = () => {
  if (tab.value === "promql") {
    query.value = `${selectedStreamName.value}{}`
  }

  if (tab.value === "sql") query.value = props.sql;
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

const onBlurQueryEditor = () => {
  queryEditorPlaceholderFlag.value = true;

  // emits("validate-sql");
};

const validateInputs = (notify: boolean = true) => {
  validateFrequency();

  if (cronJobError.value) {
    notify &&
      q.notify({
        type: "negative",
        message: cronJobError.value,
        timeout: 2000,
      });
    return false;
  }

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

const validateFrequency = () => {
  if (triggerData.value.frequency_type === "cron") {
    try {
      const intervalInSecs = getCronIntervalDifferenceInSeconds(
        triggerData.value.cron,
      );

      if (
        typeof intervalInSecs === "number" &&
        !isAboveMinRefreshInterval(intervalInSecs, store.state?.zoConfig)
      ) {
        const minInterval =
          Number(store.state?.zoConfig?.min_auto_refresh_interval) || 1;
        cronJobError.value = `Frequency should be greater than ${minInterval - 1} seconds.`;
        return;
      }
    } catch (err) {
      cronJobError.value = "Invalid cron expression";
    }
  }

  if (triggerData.value.frequency_type === "minutes") {
    const intervalInMins = Math.ceil(
      store.state?.zoConfig?.min_auto_refresh_interval / 60,
    );

    if (triggerData.value.frequency < intervalInMins) {
      cronJobError.value =
        "Frequency should be greater than " + (intervalInMins - 1);
      return;
    }
  }
};
const collapseFieldList = () => {
  splitterModel.value = collapseFields.value ? 30 : 0;
  collapseFields.value = !collapseFields.value;
}

const getStreamFields = () => {
  return new Promise((resolve) => {
    getStream(selectedStreamName.value, selectedStreamType.value, true)
      .then((stream: any) => {
        streamFields.value = [];
        stream.schema?.forEach((field: any) => {
            streamFields.value.push({
              ...field,
              showValues: true,
            });
          
        });
      })
      .finally(() => {
        if(tab.value === 'sql'){
          query.value = `SELECT * FROM "${selectedStreamName.value}"`;
        }
        else if (tab.value === 'promql'){
          query.value = `${selectedStreamName.value}{}`;
        }
        expandState.value.query = true;
        expandState.value.output = false;

        resolve(true);
      });
  });
};
const filterStreams = (val: string, update: any) => {
  update(() => {
    // Only try to get stream list if we haven't already attempted
    if (!val || val === '') {
      if (filteredStreams.value.length === 0 && !streamListFetched.value) {
        getStreamList();
      }
    } else {
      // Filter existing streams based on the search value
      filteredStreams.value = filteredStreams.value.filter((stream: any) => {
        return stream.label.toLowerCase().indexOf(val.toLowerCase()) > -1;
      });
    }
  });
};

// Add a ref to track if we've attempted to fetch streams
const streamListFetched = ref(false);

async function getStreamList() {
  try {
    streamListFetched.value = true; // Mark that we've attempted to fetch
    const res: any = await getStreams(selectedStreamType.value, false);
    filteredStreams.value = res.list?.map((stream: any) => ({
      label: stream.name,
      value: stream.name
    })) || [];
  } 
  catch(err) {
    console.log(err);
    filteredStreams.value = [];
  }
}

const handleSidebarEvent = (event: string, value: any) => {
  
  if (pipelineEditorRef.value) {
    const cursorPosition = pipelineEditorRef.value?.getCursorIndex();
    
    // Split the value by '=' and take the first part
    const insertValue = value.split('=')[0].trim();
    
    // Add spaces before and after the value
    const valueToInsert = ` ${insertValue} `;
    
    // Get current query value
    const currentQuery : any = pipelineEditorRef.value.getValue();
    
    // Insert at cursor position
    const newQuery = 
      currentQuery.slice(0, cursorPosition+1) + 
      valueToInsert + 
      currentQuery.slice(cursorPosition+1);
    
    // Set the new value
    pipelineEditorRef.value.setValue(newQuery);
  } else {
    console.log('Could not find editor instance');
  }
};
const updateDateChange = (date: any) => {
  if (JSON.stringify(date) === JSON.stringify(dateTime.value)) return;
  dateTime.value = {
    startTime: date.startTime,
    endTime: date.endTime,
    relativeTimePeriod: date.relativeTimePeriod,
    valueType: date.relativeTimePeriod ? "relative" : "absolute",
  };
};

const runQuery = async () => {

  if(tab.value == 'sql'){
    loading.value = true;

  const queryReq = {
    sql: query.value,
    start_time: dateTime.value.startTime,
    end_time: dateTime.value.endTime,
    from: 0,
    size: 10,
  }
  searchService.search({
          org_identifier: store.state.selectedOrganization.identifier,
            query: {query: queryReq},
            page_type: selectedStreamType.value,
  }
  ).then((res: any) => {
    if(res.data.hits.length > 0){
    rows.value = res.data.hits
    }
    else{
      rows.value = [];
    }

  }).catch((err: any) => {
    console.log(err, 'err');
  }).finally(() => {
    loading.value = false;
  })
}
else if(tab.value == 'promql'){
  previewPromqlQueryRef.value.refreshData();
  // searchService.metrics_query_range({
  //   org_identifier: store.state.selectedOrganization.identifier,
  //   query: query.value,
  //   start_time: dateTime.value.startTime,
  //   end_time: dateTime.value.endTime,
  // }).then((res: any) => {
  //   console.log(res, 'res')
  // }).catch((err: any) => {
  //   console.log(err, 'err')
  // })
}
}

const handleBuildQuery = () => {
  expandState.value.buildQuery = !expandState.value.buildQuery;
  if(expandState.value.buildQuery == false){
    sideBarSplitterModel.value = 0;
  }
  else{
    sideBarSplitterModel.value = 60;
  }
}
const handleSetVariables = () => {
  expandState.value.setVariables = !expandState.value.setVariables;
  if(expandState.value.setVariables == false){
    sideBarSplitterModel.value = 99;
  }
  else{
    sideBarSplitterModel.value = 60;
  }
}

const isFullscreen = ref(false);

const handleFullScreen = () => {
  q.fullscreen.toggle();
  isFullscreen.value = !isFullscreen.value;
}

watch(() => q.fullscreen.isActive, (val) => {
  isFullscreen.value = val;
  emits('update:fullscreen', val);
});

defineExpose({
  tab,
  validateInputs,
  pipelineEditorRef,
  pipelineObj,
  step,
  splitterModel,
  collapseFieldList,
  collapseFields,
  expandState,
  standardOutput,
  streamFields,
  getStreamFields,
  selectedStreamName,
  streamOptions,
  filteredStreams,
  streams,
  selectedStreamType,
  handleSidebarEvent,
  dateTime,
  updateDateChange,
  getColumns,
  rows,
  sideBarSplitterModel,
  handleBuildQuery,
  handleSetVariables,
  previewPromqlQueryRef,
});

</script>

<style lang="scss" scoped>
.scheduled-pipeline-tabs {
  padding: 0px !important;
  height: 28px !important;
  border: 1px solid $primary;
  width: 200px;
  border-radius: 4px;
  overflow: hidden;
}




</style>
<style lang="scss">

.scheduled-pipeline-container{
  .q-splitter__before {
    overflow: hidden;
  }
  .q-splitter__after {
    overflow: hidden;
  }

  .q-table__control {
    width: 100%;
  }
  .q-table__top{
    padding: 0px !important;
  }
.scheduled-pipeline-tabs {
  padding: 0px !important;

  .q-tab--active {
    background-color: $primary;
    color: $white;
  }

  .q-tab__indicator {
    display: none;
  }

  .q-tab {
    min-height: 28px;
    height: 32px;
  }
}
.scheduled-pipelines {
  .monaco-editor {
  
    width: 100%;
  }
  .query-editor-container {
    width: 97% !important;
    border: 1px solid $border-color;
  }

  .q-btn {
    &.icon-dark {
      filter: none !important;
    }
  }
}
.field-list-collapse-btn {
      z-index: 11;
      position: absolute;
      top: -6px !important;
      font-size: 12px !important;
    }
    .stream-routing-title {
  font-size: 20px;
  padding-top: 12px;
}
}

.search-button-pipeline {
  height: 32px !important;
    min-width: 77px;
    line-height: 29px;
    font-weight: bold;
    text-transform: initial;
    font-size: 11px;
    color: white;

    .q-btn__content {
      background: $secondary;
      border-radius: 3px 3px 3px 3px;
      padding: 0px 5px;

      .q-icon {
        font-size: 15px;
        color: #ffffff;
      }
    }
  }

</style>
