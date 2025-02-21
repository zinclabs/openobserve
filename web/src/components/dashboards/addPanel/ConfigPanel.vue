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
    v-if="dashboardPanelData.data.type == 'custom_chart'"
    style="padding-bottom: 30px">
    <div class="" style="max-width: 300px">
      <div class="q-mb-sm" style="font-weight: 600">
        {{ t("dashboard.description") }}
      </div>
      <q-input
        outlined
        v-model="dashboardPanelData.data.description"
        filled
        autogrow
        class="showLabelOnTop"
        data-test="dashboard-config-description"
      />
    </div>
  </div>
  <div v-else style="padding-bottom: 30px">
    <div class="" style="max-width: 300px">
      <div class="q-mb-sm" style="font-weight: 600">
        {{ t("dashboard.description") }}
      </div>
      <q-input
        outlined
        v-model="dashboardPanelData.data.description"
        filled
        autogrow
        class="showLabelOnTop"
        data-test="dashboard-config-description"
      />
    </div>

    <div class="space"></div>

    <q-input
      v-if="promqlMode"
      v-model="dashboardPanelData.data.config.step_value"
      :value="0"
      :min="0"
      color="input-border"
      bg-color="input-bg"
      class="q-py-sm showLabelOnTop"
      stack-label
      outlined
      filled
      dense
      label-slot
      placeholder="Default: 0"
      data-test="dashboard-config-step-value"
    >
      <template v-slot:label>
        <div class="row items-center all-pointer-events">
          Step Value
          <div>
            <q-icon
              class="q-ml-xs"
              size="20px"
              name="info"
              data-test="dashboard-config-top_results-info"
            />
            <q-tooltip
              class="bg-grey-8"
              anchor="top middle"
              self="bottom middle"
              max-width="250px"
            >
              <b>Step - </b>
              The interval between datapoints, which must be returned from the
              range query.
              <br />
              Eg: 10s, 1h
            </q-tooltip>
          </div>
        </div>
      </template>
    </q-input>

    <div class="space"></div>

    <div v-if="showTrellisConfig" class="q-mb-sm">
      <q-select
        :label="t('dashboard.trellisLayout')"
        data-test="dashboard-trellis-chart"
        outlined
        v-model="dashboardPanelData.data.config.trellis.layout"
        :options="trellisOptions"
        dense
        class="showLabelOnTop"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.config.trellis?.layout ?? 'None'
        }`"
        :disable="isBreakdownFieldEmpty || hasTimeShifts"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">
            {{ t("dashboard.trellisLayout") }}
            <div>
              <q-icon
                class="q-ml-xs"
                size="20px"
                name="info"
                data-test="dashboard-config-top_results-info"
              />
              <q-tooltip
                class="bg-grey-8"
                anchor="top middle"
                self="bottom middle"
                max-width="250px"
              >
                <b>
                  {{
                    hasTimeShifts
                      ? t("dashboard.trellisTimeShiftTooltip")
                      : t("dashboard.trellisTooltip")
                  }}</b
                >
              </q-tooltip>
            </div>
          </div>
        </template>
      </q-select>

      <template
        v-if="dashboardPanelData.data.config.trellis?.layout === 'custom'"
      >
        <q-input
          outlined
          v-model.number="dashboardPanelData.data.config.trellis.num_of_columns"
          color="input-border"
          :label="t('dashboard.numOfColumns')"
          bg-color="input-bg"
          class="q-mr-sm showLabelOnTop"
          stack-label
          filled
          dense
          :type="'number'"
          placeholder="Auto"
          data-test="trellis-chart-num-of-columns"
          :disable="isBreakdownFieldEmpty || hasTimeShifts"
          :min="1"
          :max="16"
          @update:model-value="
            (value: any) =>
              dashboardPanelData.data.config.trellis.num_of_columns > 16
                ? (dashboardPanelData.data.config.trellis.num_of_columns = 16)
                : value
          "
        >
          <template v-slot:label>
            <div class="row items-center all-pointer-events">
              {{ t("dashboard.numOfColumns") }}
              <div>
                <q-icon
                  class="q-ml-xs"
                  size="20px"
                  name="info"
                  data-test="dashboard-config-top_results-info"
                />
                <q-tooltip
                  class="bg-grey-8"
                  anchor="top middle"
                  self="bottom middle"
                  max-width="250px"
                >
                  <b>
                    {{
                      hasTimeShifts
                        ? t("dashboard.trellisTimeShiftTooltip")
                        : t("dashboard.trellisTooltip")
                    }}</b
                  >
                </q-tooltip>
              </div>
            </div>
          </template>
        </q-input>
      </template>
    </div>

    <q-toggle
      v-if="
        dashboardPanelData.data.type != 'table' &&
        dashboardPanelData.data.type != 'heatmap' &&
        dashboardPanelData.data.type != 'metric' &&
        dashboardPanelData.data.type != 'gauge' &&
        dashboardPanelData.data.type != 'geomap' &&
        dashboardPanelData.data.type != 'sankey' &&
        dashboardPanelData.data.type != 'maps'
      "
      v-model="dashboardPanelData.data.config.show_legends"
      :label="t('dashboard.showLegendsLabel')"
      data-test="dashboard-config-show-legend"
    />

    <div class="space"></div>

    <q-toggle
      v-if="dashboardPanelData.data.type == 'table'"
      v-model="dashboardPanelData.data.config.wrap_table_cells"
      :label="t('dashboard.wraptext')"
      data-test="dashboard-config-wrap-table-cells"
    />

    <div class="space"></div>

    <q-toggle
      v-if="dashboardPanelData.data.type == 'table'"
      v-model="dashboardPanelData.data.config.table_transpose"
      :label="t('dashboard.tableTranspose')"
      data-test="dashboard-config-table_transpose"
    />

    <div class="space"></div>

    <q-toggle
      v-if="dashboardPanelData.data.type == 'table'"
      v-model="dashboardPanelData.data.config.table_dynamic_columns"
      :label="t('dashboard.tableDynamicColumns')"
      data-test="dashboard-config-table_dynamic_columns"
    />

    <div class="space"></div>

    <div class="o2-input">
      <q-select
        v-if="
          dashboardPanelData.data.type != 'table' &&
          dashboardPanelData.data.type != 'heatmap' &&
          dashboardPanelData.data.type != 'metric' &&
          dashboardPanelData.data.type != 'gauge' &&
          dashboardPanelData.data.type != 'geomap' &&
          dashboardPanelData.data.config.show_legends &&
          dashboardPanelData.data.type != 'sankey' &&
          dashboardPanelData.data.type != 'maps'
        "
        outlined
        v-model="dashboardPanelData.data.config.legends_position"
        :options="legendsPositionOptions"
        dense
        :label="t('dashboard.legendsPositionLabel')"
        class="showLabelOnTop"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.config.legends_position ?? 'Auto'
        }`"
        data-test="dashboard-config-legend-position"
      >
      </q-select>

      <div class="space"></div>

      <div class="input-container">
        <q-input
          v-if="
            dashboardPanelData.data.type != 'table' &&
            dashboardPanelData.data.type != 'heatmap' &&
            dashboardPanelData.data.type != 'metric' &&
            dashboardPanelData.data.type != 'gauge' &&
            dashboardPanelData.data.type != 'geomap' &&
            dashboardPanelData.data.config.show_legends &&
            dashboardPanelData.data.config.legends_position == 'right' &&
            dashboardPanelData.data.type != 'sankey'
          "
          v-model.number="legendWidthValue"
          :label="t('common.legendWidth')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop q-mr-sm"
          stack-label
          outlined
          filled
          dense
          label-slot
          :type="'number'"
          placeholder="Auto"
          data-test="dashboard-config-legend-width"
        ></q-input>
        <div
          class="unit-container"
          v-if="
            dashboardPanelData.data.type != 'table' &&
            dashboardPanelData.data.type != 'heatmap' &&
            dashboardPanelData.data.type != 'metric' &&
            dashboardPanelData.data.type != 'gauge' &&
            dashboardPanelData.data.type != 'geomap' &&
            dashboardPanelData.data.config.show_legends &&
            dashboardPanelData.data.config.legends_position == 'right' &&
            dashboardPanelData.data.type != 'sankey'
          "
        >
          <button
            @click="setUnit('px')"
            :class="{
              active:
                dashboardPanelData?.data?.config.legend_width?.unit === null ||
                dashboardPanelData?.data?.config?.legend_width?.unit === 'px',
            }"
            style="height: 100%; width: 100%; font-size: 14px"
            :data-test="`dashboard-config-legend-width-unit-${
              dashboardPanelData?.data?.config?.legend_width?.unit === 'px'
                ? 'active'
                : 'inactive'
            }`"
          >
            px
          </button>
          <button
            @click="setUnit('%')"
            :class="{
              active:
                dashboardPanelData?.data?.config?.legend_width?.unit === '%',
            }"
            style="height: 100%; width: 100%; font-size: 14px"
            :data-test="`dashboard-config-legend-width-unit-${
              dashboardPanelData?.data?.config?.legend_width?.unit === '%'
                ? 'active'
                : 'inactive'
            }`"
          >
            %
          </button>
        </div>
      </div>

      <div class="space"></div>

      <q-select
        outlined
        v-model="dashboardPanelData.data.config.unit"
        :options="unitOptions"
        dense
        :label="t('dashboard.unitLabel')"
        class="showLabelOnTop selectedLabel"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.config.unit
            ? unitOptions.find(
                (it) => it.value == dashboardPanelData.data.config.unit,
              )?.label
            : 'Default'
        }`"
        data-test="dashboard-config-unit"
      >
      </q-select>
      <!-- :rules="[(val: any) => !!val || 'Field is required!']" -->
      <q-input
        v-if="dashboardPanelData.data.config.unit == 'custom'"
        v-model="dashboardPanelData.data.config.unit_custom"
        :label="t('dashboard.customunitLabel')"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        filled
        dense
        label-slot
        data-test="dashboard-config-custom-unit"
      />
      <div class="space"></div>
      <q-input
        type="number"
        v-model.number="dashboardPanelData.data.config.decimals"
        value="2"
        min="0"
        max="100"
        @update:model-value="
          (value: any) =>
            (dashboardPanelData.data.config.decimals =
              typeof value == 'number' && value >= 0 ? value : 2)
        "
        :rules="[
          (val: any) =>
            (val >= 0 && val <= 100) || 'Decimals must be between 0 and 100',
        ]"
        label="Decimals"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        filled
        dense
        label-slot
        data-test="dashboard-config-decimals"
      />

      <div class="space"></div>

      <q-select
        v-if="dashboardPanelData.data.type == 'maps'"
        outlined
        v-model="dashboardPanelData.data.config.map_type.type"
        :options="mapTypeOptions"
        dense
        :label="t('dashboard.mapTypeLabel')"
        class="showLabelOnTop"
        stack-label
        emit-value
        data-test="dashboard-config-map-type"
      >
      </q-select>

      <div class="space"></div>

      <q-select
        v-if="dashboardPanelData.data.type == 'geomap'"
        outlined
        v-model="dashboardPanelData.data.config.base_map.type"
        :options="basemapTypeOptions"
        dense
        :label="t('dashboard.basemapLabel')"
        class="showLabelOnTop"
        stack-label
        emit-value
        :display-value="'OpenStreetMap'"
        data-test="dashboard-config-basemap"
      >
      </q-select>

      <div class="space"></div>

      <div v-if="dashboardPanelData.data.type == 'geomap'">
        <span>Initial View:</span>
        <div class="row">
          <q-input
            v-model.number="dashboardPanelData.data.config.map_view.lat"
            :value="0"
            @blur="
              handleBlur(dashboardPanelData.data.config.map_view, 0, 'lat')
            "
            :label="t('dashboard.latitudeLabel')"
            color="input-border"
            bg-color="input-bg"
            class="col-6 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            label-slot
            :type="'number'"
            data-test="dashboard-config-latitude"
          >
          </q-input>
          <q-input
            v-model.number="dashboardPanelData.data.config.map_view.lng"
            :value="0"
            @blur="
              handleBlur(dashboardPanelData.data.config.map_view, 0, 'lng')
            "
            :label="t('dashboard.longitudeLabel')"
            color="input-border"
            bg-color="input-bg"
            class="col-6 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            label-slot
            :type="'number'"
            data-test="dashboard-config-longitude"
          >
          </q-input>
        </div>
        <q-input
          v-model.number="dashboardPanelData.data.config.map_view.zoom"
          :value="1"
          @blur="handleBlur(dashboardPanelData.data.config.map_view, 1, 'zoom')"
          :label="t('dashboard.zoomLabel')"
          color="input-border"
          bg-color="input-bg"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          label-slot
          :type="'number'"
          data-test="dashboard-config-zoom"
        >
        </q-input>

        <!-- symbol size -->
        <q-select
          v-model="dashboardPanelData.data.config.map_symbol_style.size"
          :label="t('dashboard.symbolsize')"
          outlined
          :options="symbolOptions"
          dense
          class="showLabelOnTop"
          stack-label
          emit-value
          :display-value="`${dashboardPanelData.data.config.map_symbol_style.size}`"
          data-test="dashboard-config-symbol"
        >
        </q-select>

        <div class="space"></div>

        <div class="row">
          <q-input
            v-if="
              dashboardPanelData.data.config.map_symbol_style.size ===
              'by Value'
            "
            v-model.number="
              dashboardPanelData.data.config.map_symbol_style.size_by_value.min
            "
            :value="1"
            @blur="
              handleBlur(
                dashboardPanelData.data.config.map_symbol_style.size_by_value,
                1,
                'min',
              )
            "
            :label="t('dashboard.minimum')"
            color="input-border"
            bg-color="input-bg"
            class="col-6 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            label-slot
            :type="'number'"
            data-test="dashboard-config-map-symbol-min"
            :min="0"
          >
          </q-input>

          <q-input
            v-if="
              dashboardPanelData.data.config.map_symbol_style.size ===
              'by Value'
            "
            v-model.number="
              dashboardPanelData.data.config.map_symbol_style.size_by_value.max
            "
            :value="100"
            @blur="
              handleBlur(
                dashboardPanelData.data.config.map_symbol_style.size_by_value,
                100,
                'max',
              )
            "
            :label="t('dashboard.maximum')"
            color="input-border"
            bg-color="input-bg"
            class="col-6 q-py-md showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            label-slot
            :type="'number'"
            data-test="dashboard-config-map-symbol-max"
            :min="0"
          >
          </q-input>
        </div>
        <q-input
          v-if="
            dashboardPanelData.data.config.map_symbol_style.size === 'fixed'
          "
          v-model.number="
            dashboardPanelData.data.config.map_symbol_style.size_fixed
          "
          :value="2"
          @blur="
            handleBlur(
              dashboardPanelData.data.config.map_symbol_style,
              2,
              'size_fixed',
            )
          "
          :label="t('dashboard.fixedValue')"
          color="input-border"
          bg-color="input-bg"
          class="col-6 q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          label-slot
          :type="'number'"
          data-test="dashboard-config-map-symbol-fixed"
        >
        </q-input>
      </div>

      <div class="space"></div>

      <!-- <q-input v-if="promqlMode" v-model="dashboardPanelData.data.config.promql_legend" label="Legend" color="input-border"
      bg-color="input-bg" class="q-py-md showLabelOnTop" stack-label outlined filled dense label-slot> -->
      <div
        v-if="promqlMode || dashboardPanelData.data.type == 'geomap'"
        class="q-py-md showLabelOnTop"
        style="font-weight: 600"
      >
        Query
        <q-tabs
          v-model="dashboardPanelData.layout.currentQueryIndex"
          narrow-indicator
          dense
          inline-label
          outside-arrows
          mobile-arrows
          data-test="dashboard-config-query-tab"
        >
          <q-tab
            no-caps
            v-for="(tab, index) in dashboardPanelData.data.queries"
            :key="index"
            :name="index"
            :label="'Query ' + (index + 1)"
            :data-test="`dashboard-config-query-tab-${index}`"
          >
          </q-tab>
        </q-tabs>
      </div>
      <!-- </q-input> -->
      <div class="space"></div>

      <!-- for auto sql query limit -->
      <!-- it should not be promql and custom query -->
      <q-input
        v-if="
          !promqlMode &&
          !dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery
        "
        v-model.number="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.limit
        "
        :value="0"
        :min="0"
        @update:model-value="
          (value: any) =>
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].config.limit = value ? value : 0)
        "
        label="Limit"
        color="input-border"
        bg-color="input-bg"
        class="q-py-sm showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        placeholder="0"
        :type="'number'"
        data-test="dashboard-config-limit"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">
            Query Limit
            <div>
              <q-icon
                class="q-ml-xs"
                size="20px"
                name="info"
                data-test="dashboard-config-limit-info"
              />
              <q-tooltip
                class="bg-grey-8"
                anchor="top middle"
                self="bottom middle"
              >
                Limit for the query result
              </q-tooltip>
            </div>
          </div>
        </template>
      </q-input>

      <div class="space"></div>
      <q-input
        v-if="
          [
            'area',
            'bar',
            'line',
            'h-bar',
            'h-stacked',
            'scatter',
            'area-stacked',
            'stacked',
          ].includes(dashboardPanelData.data.type) && !promqlMode
        "
        v-model.number="dashboardPanelData.data.config.top_results"
        :min="0"
        @update:model-value="
          (value: any) =>
            (dashboardPanelData.data.config.top_results = value ? value : null)
        "
        label="Top Results"
        color="input-border"
        bg-color="input-bg"
        class="q-py-sm showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        placeholder="ALL"
        :type="'number'"
        :disable="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ]?.fields?.breakdown?.length == 0
        "
        data-test="dashboard-config-top_results"
        ><template v-slot:label>
          <div class="row items-center all-pointer-events">
            Show Top N values
            <div>
              <q-icon
                class="q-ml-xs"
                size="20px"
                name="info"
                data-test="dashboard-config-top_results-info"
              />
              <q-tooltip
                class="bg-grey-8"
                anchor="top middle"
                self="bottom middle"
                max-width="250px"
              >
                <b>This is only applicable when breakdown field is available</b>
                <br />
                <br />
                Specify the number of Top N values to show when breakdown field
                is available.
              </q-tooltip>
            </div>
          </div>
        </template></q-input
      >

      <div
        class="row items-center"
        v-if="
          [
            'area',
            'bar',
            'line',
            'h-bar',
            'h-stacked',
            'scatter',
            'area-stacked',
            'stacked',
          ].includes(dashboardPanelData.data.type) && !promqlMode
        "
      >
        <q-toggle
          v-model="dashboardPanelData.data.config.top_results_others"
          label="Add 'others' series"
          data-test="dashboard-config-top_results_others"
          :disable="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields?.breakdown?.length == 0
          "
        />

        <q-icon
          class="q-ml-xs"
          size="20px"
          name="info"
          data-test="dashboard-config-top_results-others-info"
        >
          <q-tooltip
            class="bg-grey-8"
            anchor="top middle"
            self="bottom middle"
            max-width="250px"
          >
            Include an 'others' series for values outside the top results when
            using breakdown fields.
          </q-tooltip>
        </q-icon>
      </div>

      <div class="space"></div>

      <CommonAutoComplete
        v-if="promqlMode"
        :label="t('common.legend')"
        v-model="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.promql_legend
        "
        :items="dashboardSelectfieldPromQlList"
        searchRegex="(?:{([^}]*)(?:{.*})*$|([a-zA-Z-_]+)$)"
        color="input-border"
        bg-color="input-bg"
        class="showLabelOnTop"
        stack-label
        outlined
        label-slot
        style="
          top: none !important;
          margin-top: none !important;
          padding-top: 3px !important;
          width: auto !important;
        "
        :value-replace-fn="selectPromQlNameOption"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">
            {{ t("dashboard.legendLabel") }}
            <div>
              <q-icon
                class="q-ml-xs"
                size="20px"
                name="info"
                data-test="dashboard-config-promql-legend-info"
              />
              <q-tooltip
                class="bg-grey-8"
                anchor="top middle"
                self="bottom middle"
              >
                {{ t("dashboard.overrideMessage") }}
              </q-tooltip>
            </div>
          </div>
        </template>
      </CommonAutoComplete>

      <div class="space"></div>

      <q-toggle
        v-if="
          ['area', 'line', 'area-stacked'].includes(
            dashboardPanelData.data.type,
          )
        "
        v-model="dashboardPanelData.data.config.connect_nulls"
        label="Connect null values"
        data-test="dashboard-config-connect-null-values"
      />

      <div class="space"></div>

      <q-input
        v-if="
          ['area', 'line', 'area-stacked', 'bar', 'stacked'].includes(
            dashboardPanelData.data.type,
          )
        "
        v-model="dashboardPanelData.data.config.no_value_replacement"
        label="No Value Replacement"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        filled
        dense
        label-slot
        placeholder="-"
        data-test="dashboard-config-no-value-replacement"
        ><template v-slot:label>
          <div class="row items-center all-pointer-events">
            No Value Replacement
            <div>
              <q-icon
                class="q-ml-xs"
                size="20px"
                name="info"
                data-test="dashboard-config-limit-info"
              />
              <q-tooltip
                class="bg-grey-8"
                anchor="top middle"
                self="bottom middle"
              >
                What to display when a value is missing for time series
              </q-tooltip>
            </div>
          </div>
        </template></q-input
      >

      <div class="space"></div>
      <q-select
        v-if="dashboardPanelData.data.type == 'geomap'"
        outlined
        v-model="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.layer_type
        "
        :options="layerTypeOptions"
        dense
        :label="t('dashboard.layerType')"
        class="showLabelOnTop"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.layer_type
        }`"
        data-test="dashboard-config-layer-type"
      >
      </q-select>

      <div class="space"></div>

      <q-input
        v-if="dashboardPanelData.data.type == 'geomap' && !isWeightFieldPresent"
        v-model.number="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.weight_fixed
        "
        :value="1"
        @blur="
          handleBlur(
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].config,
            1,
            'weight_fixed',
          )
        "
        :label="t('common.weight')"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        :type="'number'"
        data-test="dashboard-config-weight"
      >
      </q-input>

      <q-input
        v-if="dashboardPanelData.data.type === 'gauge'"
        v-model.number="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.min
        "
        :value="0"
        @update:model-value="
          (value: any) =>
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].config.min = value ? value : 0)
        "
        label="Gauge Min Value"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        :type="'number'"
        data-test="dashboard-config-gauge-min"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">Gauge Min Value</div>
        </template>
      </q-input>
      <q-input
        v-if="dashboardPanelData.data.type === 'gauge'"
        v-model.number="
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].config.max
        "
        :value="100"
        @update:model-value="
          (value: any) =>
            (dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].config.max = value ? value : 100)
        "
        label="Gauge Max Value"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        placeholder="100"
        :type="'number'"
        data-test="dashboard-config-gauge-max"
      >
        <template v-slot:label>
          <div class="row items-center all-pointer-events">Gauge Max Value</div>
        </template>
      </q-input>

      <q-input
        v-if="
          dashboardPanelData.data.type != 'gauge' &&
          dashboardPanelData.data.type != 'metric' &&
          dashboardPanelData.data.type != 'geomap' &&
          dashboardPanelData.data.type != 'table' &&
          dashboardPanelData.data.type != 'pie' &&
          dashboardPanelData.data.type != 'donut' &&
          dashboardPanelData.data.type != 'sankey' &&
          dashboardPanelData.data.type != 'maps'
        "
        v-model.number="dashboardPanelData.data.config.axis_width"
        :label="t('common.axisWidth')"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        :type="'number'"
        placeholder="Auto"
        @update:model-value="
          (value: any) =>
            (dashboardPanelData.data.config.axis_width =
              value !== '' ? value : null)
        "
        data-test="dashboard-config-axis-width"
      >
      </q-input>

      <div class="space"></div>

      <q-toggle
        v-if="
          dashboardPanelData.data.type != 'gauge' &&
          dashboardPanelData.data.type != 'metric' &&
          dashboardPanelData.data.type != 'geomap' &&
          dashboardPanelData.data.type != 'table' &&
          dashboardPanelData.data.type != 'pie' &&
          dashboardPanelData.data.type != 'donut' &&
          dashboardPanelData.data.type != 'sankey' &&
          dashboardPanelData.data.type != 'maps'
        "
        v-model="dashboardPanelData.data.config.axis_border_show"
        :label="t('dashboard.showBorder')"
        data-test="dashboard-config-axis-border"
      />

      <div class="space"></div>

      <div style="width: 100%; display: flex; gap: 16px">
        <q-input
          v-model.number="dashboardPanelData.data.config.y_axis_min"
          color="input-border"
          bg-color="input-bg"
          style="width: 50%"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          label-slot
          :type="'number'"
          placeholder="Auto"
          @update:model-value="
            (value: any) =>
              (dashboardPanelData.data.config.y_axis_min =
                value !== '' ? value : null)
          "
          data-test="dashboard-config-y_axis_min"
          ><template v-slot:label>
            <div class="row items-center all-pointer-events">
              {{ t("common.yAxisMin") }}
              <div>
                <q-icon
                  class="q-ml-xs"
                  size="20px"
                  name="info"
                  data-test="dashboard-config-y_axis_min-info"
                />
                <q-tooltip
                  class="bg-grey-8"
                  anchor="top middle"
                  self="bottom middle"
                >
                  Set the minimum value for the Y-axis.
                  <br />
                  This determines the lowest point displayed on the chart.
                </q-tooltip>
              </div>
            </div>
          </template>
        </q-input>
        <q-input
          v-model.number="dashboardPanelData.data.config.y_axis_max"
          color="input-border"
          bg-color="input-bg"
          style="width: 50%"
          class="q-py-md showLabelOnTop"
          stack-label
          outlined
          filled
          dense
          label-slot
          :type="'number'"
          placeholder="Auto"
          @update:model-value="
            (value: any) =>
              (dashboardPanelData.data.config.y_axis_max =
                value !== '' ? value : null)
          "
          data-test="dashboard-config-y_axis_max"
          ><template v-slot:label>
            <div class="row items-center all-pointer-events">
              {{ t("common.yAxisMax") }}
              <div>
                <q-icon
                  class="q-ml-xs"
                  size="20px"
                  name="info"
                  data-test="dashboard-config-y_axis_max-info"
                />
                <q-tooltip
                  class="bg-grey-8"
                  anchor="top middle"
                  self="bottom middle"
                >
                  Set the maximum value for the Y-axis.
                  <br />
                  This determines the highest point displayed on the chart.
                </q-tooltip>
              </div>
            </div>
          </template>
        </q-input>
      </div>
      <div class="space"></div>

      <q-select
        v-if="
          [
            'area',
            'area-stacked',
            'bar',
            'h-bar',
            'line',
            'scatter',
            'stacked',
            'h-stacked',
          ].includes(dashboardPanelData.data.type)
        "
        outlined
        v-model="dashboardPanelData.data.config.label_option.position"
        :options="labelPositionOptions"
        dense
        :label="t('dashboard.labelPosition')"
        class="showLabelOnTop selectedLabel"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.config.label_option.position
            ? labelPositionOptions.find(
                (it) =>
                  it.value ==
                  dashboardPanelData.data.config.label_option.position,
              )?.label
            : 'None'
        }`"
        data-test="dashboard-config-label-position"
      >
      </q-select>

      <div class="space"></div>

      <q-input
        v-if="
          [
            'area',
            'area-stacked',
            'bar',
            'h-bar',
            'line',
            'scatter',
            'stacked',
            'h-stacked',
          ].includes(dashboardPanelData.data.type)
        "
        v-model.number="dashboardPanelData.data.config.label_option.rotate"
        :label="t('dashboard.labelRotate')"
        color="input-border"
        bg-color="input-bg"
        class="q-py-md showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        :type="'number'"
        placeholder="0"
        @update:model-value="
          (value: any) =>
            (dashboardPanelData.data.config.label_option.rotate =
              value !== '' ? value : 0)
        "
        data-test="dashboard-config-label-rotate"
      >
      </q-input>

      <div class="space"></div>

      <q-select
        v-if="
          ['area', 'area-stacked', 'line'].includes(
            dashboardPanelData.data.type,
          )
        "
        outlined
        v-model="dashboardPanelData.data.config.show_symbol"
        :options="showSymbol"
        dense
        :label="t('dashboard.showSymbol')"
        class="showLabelOnTop selectedLabel"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.config.show_symbol
            ? showSymbol.find(
                (it: any) =>
                  it.value == dashboardPanelData.data.config.show_symbol,
              )?.label
            : 'No'
        }`"
        data-test="dashboard-config-show_symbol"
      >
        <template v-slot:option="scope">
          <q-item v-bind="scope.itemProps">
            <q-item-section avatar style="height: 20px">
              <q-icon>
                <component :is="scope.opt.iconComponent"></component>
              </q-icon>
            </q-item-section>
            <q-item-section>
              <q-item-label>{{ scope.opt.label }}</q-item-label>
            </q-item-section>
          </q-item>
        </template>
      </q-select>

      <div class="space"></div>

      <q-select
        v-if="
          ['area', 'area-stacked', 'line'].includes(
            dashboardPanelData.data.type,
          )
        "
        outlined
        v-model="dashboardPanelData.data.config.line_interpolation"
        :options="lineInterpolationOptions"
        dense
        :label="t('dashboard.lineInterpolation')"
        class="showLabelOnTop selectedLabel"
        stack-label
        emit-value
        :display-value="`${
          dashboardPanelData.data.config.line_interpolation
            ? lineInterpolationOptions.find(
                (it: any) =>
                  it.value == dashboardPanelData.data.config.line_interpolation,
              )?.label
            : 'smooth'
        }`"
        data-test="dashboard-config-line_interpolation"
      >
        <template v-slot:option="scope">
          <q-item v-bind="scope.itemProps">
            <q-item-section avatar>
              <q-icon>
                <component :is="scope.opt.iconComponent"></component>
              </q-icon>
            </q-item-section>
            <q-item-section>
              <q-item-label>{{ scope.opt.label }}</q-item-label>
            </q-item-section>
          </q-item>
        </template>
      </q-select>
      <div class="space"></div>

      <q-input
        v-if="
          !promqlMode &&
          !dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery &&
          ['area', 'area-stacked', 'line'].includes(
            dashboardPanelData.data.type,
          )
        "
        v-model.number="dashboardPanelData.data.config.line_thickness"
        :value="1.5"
        :min="0"
        @update:model-value="
          (value: any) =>
            (dashboardPanelData.data.config.line_thickness =
              typeof value == 'number' && value >= 0 ? value : 1.5)
        "
        label="Line Thickness"
        color="input-border"
        bg-color="input-bg"
        class="q-py-sm showLabelOnTop"
        stack-label
        outlined
        filled
        dense
        label-slot
        placeholder="Default: 1.5"
        :type="'number'"
        data-test="dashboard-config-line_thickness"
      >
      </q-input>

      <div class="space"></div>

      <Drilldown
        v-if="
          !['html', 'markdown', 'geomap', 'maps'].includes(
            dashboardPanelData.data.type,
          )
        "
        :variablesData="variablesData"
      />

      <div class="space"></div>
      <ValueMapping v-if="dashboardPanelData.data.type == 'table'" />

      <div class="space"></div>
      <MarkLineConfig
        v-if="
          [
            'area',
            'area-stacked',
            'bar',
            'h-bar',
            'line',
            'scatter',
            'stacked',
            'h-stacked',
          ].includes(dashboardPanelData.data.type)
        "
      />
    </div>

    <div
      v-if="
        [
          'area',
          'bar',
          'line',
          'h-bar',
          'h-stacked',
          'scatter',
          'area-stacked',
          'stacked',
        ].includes(dashboardPanelData.data.type) && !promqlMode
      "
    >
      <div class="flex items-center q-mr-sm">
        <div
          data-test="scheduled-dashboard-period-title"
          class="text-bold q-py-md flex items-center"
          style="width: 190px"
        >
          Comparison Against
          <q-btn
            no-caps
            padding="xs"
            size="sm"
            flat
            icon="info_outline"
            data-test="dashboard-addpanel-config-time-shift-info"
          >
            <q-tooltip
              anchor="bottom middle"
              self="top middle"
              style="font-size: 10px"
              max-width="250px"
            >
              <span>
                This feature allows you to compare data points from multiple
                queries over a selected time range. By adjusting the date or
                time, the system will retrieve corresponding data from different
                queries, enabling you to observe changes or differences between
                the selected time periods.
              </span>
            </q-tooltip>
          </q-btn>
        </div>
      </div>
      <CustomDateTimePicker
        modelValue="0m"
        :isFirstEntry="true"
        :disable="true"
        class="q-mb-md"
      />
      <div
        v-for="(picker, index) in dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift"
        :key="index"
        class="q-mb-md"
      >
        <div class="flex items-center">
          <CustomDateTimePicker
            v-model="picker.offSet"
            :picker="picker"
            :isFirstEntry="false"
          />
          <q-icon
            class="q-mr-xs q-ml-sm"
            size="15px"
            name="close"
            style="cursor: pointer"
            @click="removeTimeShift(index)"
            :data-test="`dashboard-addpanel-config-time-shift-remove-${index}`"
          />
        </div>
      </div>

      <q-btn
        @click="addTimeShift"
        style="cursor: pointer; padding: 0px 5px"
        label="+ Add"
        no-caps
        data-test="dashboard-addpanel-config-time-shift-add-btn"
      />
    </div>

    <div class="space"></div>
    <ColorPaletteDropDown v-if="showColorPalette" />
    <div class="space"></div>

    <div class="space"></div>
    <OverrideConfig
      v-if="dashboardPanelData.data.type == 'table'"
      :dashboardPanelData="dashboardPanelData"
    />
    <div class="space"></div>
  </div>
</template>

<script lang="ts">
import useDashboardPanelData from "@/composables/useDashboardPanel";
import { computed, defineComponent, inject, onBeforeMount } from "vue";
import { useI18n } from "vue-i18n";
import Drilldown from "./Drilldown.vue";
import ValueMapping from "./ValueMapping.vue";
import MarkLineConfig from "./MarkLineConfig.vue";
import CommonAutoComplete from "@/components/dashboards/addPanel/CommonAutoComplete.vue";
import CustomDateTimePicker from "@/components/CustomDateTimePicker.vue";
import ColorPaletteDropDown from "./ColorPaletteDropDown.vue";
import OverrideConfig from "./OverrideConfig.vue";
import LinearIcon from "@/components/icons/dashboards/LinearIcon.vue";
import NoSymbol from "@/components/icons/dashboards/NoSymbol.vue";
import Smooth from "@/components/icons/dashboards/Smooth.vue";
import StepBefore from "@/components/icons/dashboards/StepBefore.vue";
import StepAfter from "@/components/icons/dashboards/StepAfter.vue";
import StepMiddle from "@/components/icons/dashboards/StepMiddle.vue";
import { markRaw } from "vue";

export default defineComponent({
  components: {
    Drilldown,
    ValueMapping,
    CommonAutoComplete,
    MarkLineConfig,
    CustomDateTimePicker,
    ColorPaletteDropDown,
    OverrideConfig,
    LinearIcon,
    NoSymbol,
    Smooth,
    StepBefore,
    StepAfter,
    StepMiddle,
  },
  props: ["dashboardPanelData", "variablesData"],
  setup(props) {
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );
    const { dashboardPanelData, promqlMode } = useDashboardPanelData(
      dashboardPanelDataPageKey,
    );
    const { t } = useI18n();

    const basemapTypeOptions = [
      {
        label: t("dashboard.openStreetMap"),
        value: "osm",
      },
    ];

    const mapTypeOptions = [
      {
        label: t("dashboard.world"),
        value: "world",
      },
    ];
    onBeforeMount(() => {
      // Ensure that the nested structure is initialized
      if (!dashboardPanelData.data.config.legend_width) {
        dashboardPanelData.data.config.legend_width = {
          value: null,
          unit: "px",
        };
      }

      if (!dashboardPanelData.data.config.trellis) {
        dashboardPanelData.data.config.trellis = {
          layout: null,
          num_of_columns: 1,
        };
      }

      if (!dashboardPanelData.data.config.axis_border_show) {
        dashboardPanelData.data.config.axis_border_show = false;
      }

      // by default, use top_results_others as false
      if (!dashboardPanelData.data.config.top_results_others) {
        dashboardPanelData.data.config.top_results_others = false;
      }

      // Ensure that the nested structure is initialized
      if (!dashboardPanelData.data.config.map_symbol_style) {
        dashboardPanelData.data.config.map_symbol_style = {
          size: "by Value",
          size_by_value: {
            min: 1,
            max: 100,
          },
          size_fixed: 2,
        };
      }

      // by default, use connect_nulls as false
      if (!dashboardPanelData.data.config.connect_nulls) {
        dashboardPanelData.data.config.connect_nulls = false;
      }

      // by default, use no_value_replacement as empty string
      if (!dashboardPanelData.data.config.no_value_replacement) {
        dashboardPanelData.data.config.no_value_replacement = "";
      }

      // by default, use wrap_table_cells as false
      if (!dashboardPanelData.data.config.wrap_table_cells) {
        dashboardPanelData.data.config.wrap_table_cells = false;
      }

      // by default, use table_transpose as false
      if (!dashboardPanelData.data.config.table_transpose) {
        dashboardPanelData.data.config.table_transpose = false;
      }

      // by default, use table_dynamic_columns  as false
      if (!dashboardPanelData.data.config.table_dynamic_columns) {
        dashboardPanelData.data.config.table_dynamic_columns = false;
      }

      // by default, use label_option position as null and rotate as 0
      if (!dashboardPanelData.data.config.label_option) {
        dashboardPanelData.data.config.label_option = {
          position: null,
          rotate: 0,
        };
      }

      // by default, set show_symbol as false
      if (!dashboardPanelData.data.config.show_symbol) {
        dashboardPanelData.data.config.show_symbol = false;
      }

      // by default, set line interpolation as smooth
      if (!dashboardPanelData.data.config.line_interpolation) {
        dashboardPanelData.data.config.line_interpolation = "smooth";
      }

      // Initialize map_type configuration
      if (!dashboardPanelData.data.config.map_type) {
        dashboardPanelData.data.config.map_type = { type: "world" };
      }

      // If no step value is set, set it to 0
      if (!dashboardPanelData.data.config.step_value) {
        dashboardPanelData.data.config.step_value = "0";
      }
    });

    const legendWidthValue = computed({
      get() {
        return dashboardPanelData.data.config?.legend_width?.value;
      },
      set(value) {
        // Ensure that the nested structure is initialized
        if (!dashboardPanelData.data.config.legend_width) {
          dashboardPanelData.data.config.legend_width = {
            value: null,
            unit: "px",
          };
        }

        // Set the value
        dashboardPanelData.data.config.legend_width.value =
          value !== "" ? value : null;
      },
    });

    const setUnit = (unit: any) => {
      // Ensure that the nested structure is initialized
      if (!dashboardPanelData.data.config.legend_width) {
        dashboardPanelData.data.config.legend_width = {
          value: null,
          unit: null,
        };
      }

      // Set the unit
      dashboardPanelData.data.config.legend_width.unit = unit;
    };

    const layerTypeOptions = [
      {
        label: t("dashboard.scatter"),
        value: "scatter",
      },
      {
        label: t("dashboard.heatmap"),
        value: "heatmap",
      },
    ];

    //options for symbol
    const symbolOptions = [
      {
        label: "Fixed",
        value: "fixed",
      },
      {
        label: "By Value",
        value: "by Value",
      },
    ];

    const trellisOptions = [
      {
        label: t("common.none"),
        value: null,
      },
      {
        label: t("common.auto"),
        value: "auto",
      },
      {
        label: t("common.vertical"),
        value: "vertical",
      },
      {
        label: t("common.custom"),
        value: "custom",
      },
    ];

    // options for legends position
    const legendsPositionOptions = [
      {
        label: t("dashboard.auto"),
        value: null,
      },
      {
        label: t("dashboard.right"),
        value: "right",
      },
      {
        label: t("dashboard.bottom"),
        value: "bottom",
      },
    ];
    const unitOptions = [
      {
        label: t("dashboard.default"),
        value: null,
      },
      {
        label: t("dashboard.numbers"),
        value: "numbers",
      },
      {
        label: t("dashboard.bytes"),
        value: "bytes",
      },
      {
        label: t("dashboard.kilobytes"),
        value: "kilobytes",
      },
      {
        label: t("dashboard.megabytes"),
        value: "megabytes",
      },
      {
        label: t("dashboard.bytesPerSecond"),
        value: "bps",
      },
      {
        label: t("dashboard.seconds"),
        value: "seconds",
      },
      {
        label: t("dashboard.milliseconds"),
        value: "milliseconds",
      },
      {
        label: t("dashboard.microseconds"),
        value: "microseconds",
      },
      {
        label: t("dashboard.nanoseconds"),
        value: "nanoseconds",
      },
      {
        label: t("dashboard.percent1"),
        value: "percent-1",
      },
      {
        label: t("dashboard.percent"),
        value: "percent",
      },
      {
        label: t("dashboard.currencyDollar"),
        value: "currency-dollar",
      },
      {
        label: t("dashboard.currencyEuro"),
        value: "currency-euro",
      },
      {
        label: t("dashboard.currencyPound"),
        value: "currency-pound",
      },
      {
        label: t("dashboard.currencyYen"),
        value: "currency-yen",
      },
      {
        label: t("dashboard.currencyRupees"),
        value: "currency-rupee",
      },

      {
        label: t("dashboard.custom"),
        value: "custom",
      },
    ];

    const labelPositionOptions = [
      {
        label: t("dashboard.none"),
        value: null,
      },
      {
        label: t("dashboard.top"),
        value: "top",
      },
      {
        label: t("dashboard.inside"),
        value: "inside",
      },
      {
        label: t("dashboard.insideTop"),
        value: "insideTop",
      },
      {
        label: t("dashboard.insideBottom"),
        value: "insideBottom",
      },
    ];

    const showSymbol = [
      {
        label: t("dashboard.no"),
        value: false,
        iconComponent: markRaw(NoSymbol),
      },
      {
        label: t("dashboard.yes"),
        value: true,
        iconComponent: markRaw(LinearIcon),
      },
    ];

    const lineInterpolationOptions = [
      {
        label: t("dashboard.smooth"),
        value: "smooth",
        iconComponent: markRaw(Smooth),
      },
      {
        label: t("dashboard.linear"),
        value: "linear",
        iconComponent: markRaw(LinearIcon),
      },
      {
        label: t("dashboard.stepBefore"),
        value: "step-start",
        iconComponent: markRaw(StepBefore),
      },
      {
        label: t("dashboard.stepAfter"),
        value: "step-end",
        iconComponent: markRaw(StepAfter),
      },
      {
        label: t("dashboard.stepMiddle"),
        value: "step-middle",
        iconComponent: markRaw(StepMiddle),
      },
    ];
    const isWeightFieldPresent = computed(() => {
      const layoutFields =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields;
      return !!layoutFields?.weight;
    });

    const handleBlur = (field: any, key: any, value: any) => {
      if (!field[value]) {
        field[value] = key;
      }
    };

    const selectPromQlNameOption = (option: any) => {
      const inputValue =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.promql_legend;

      // Find the index of the last opening brace '{'
      const openingBraceIndex = inputValue.lastIndexOf("{");

      //if { is not present add it at the start and than return

      if (openingBraceIndex === -1) {
        const newValue =
          "{" + inputValue.slice(0, openingBraceIndex + 1) + option + "}";
        return newValue;
      } else {
        const newValue =
          inputValue.slice(0, openingBraceIndex + 1) + option + "}";
        return newValue;
      }
    };

    const dashboardSelectfieldPromQlList = computed(() =>
      props.dashboardPanelData.meta.stream.selectedStreamFields.map(
        (it: any) => {
          return {
            label: it.name,
            value: it.name,
          };
        },
      ),
    );

    const timeShifts = [];

    const addTimeShift = () => {
      if (dashboardPanelData.data.config.trellis.layout)
        dashboardPanelData.data.config.trellis.layout = null;

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

      timeShifts.push(newTimeShift);
      if (
        !dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift
      ) {
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift = [];
      }
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].config.time_shift.push({
        offSet: newTimeShift.offSet,
      });
    };

    const removeTimeShift = (index: any) => {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].config.time_shift.splice(index, 1);
    };

    const showColorPalette = computed(() => {
      return [
        "area",
        "area-stacked",
        "bar",
        "h-bar",
        "line",
        "scatter",
        "stacked",
        "h-stacked",
        "pie",
        "donut",
        "gauge",
      ].includes(dashboardPanelData.data.type);
    });

    const showTrellisConfig = computed(() => {
      const supportedTypes = ["area", "bar", "h-bar", "line", "scatter"];
      return supportedTypes.includes(dashboardPanelData.data.type);
    });

    const isBreakdownFieldEmpty = computed(() => {
      return (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.fields?.breakdown?.length == 0
      );
    });

    const hasTimeShifts = computed(() => {
      return (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].config.time_shift?.length > 0
      );
    });

    return {
      t,
      dashboardPanelData,
      promqlMode,
      basemapTypeOptions,
      mapTypeOptions,
      layerTypeOptions,
      symbolOptions,
      legendsPositionOptions,
      unitOptions,
      labelPositionOptions,
      showSymbol,
      lineInterpolationOptions,
      isWeightFieldPresent,
      setUnit,
      handleBlur,
      legendWidthValue,
      dashboardSelectfieldPromQlList,
      selectPromQlNameOption,
      addTimeShift,
      removeTimeShift,
      showColorPalette,
      trellisOptions,
      showTrellisConfig,
      isBreakdownFieldEmpty,
      hasTimeShifts,
    };
  },
});
</script>

<style lang="scss" scoped>
:deep(.selectedLabel span) {
  text-transform: none !important;
}

.space {
  margin-top: 10px;
  margin-bottom: 10px;
}

.input-container {
  display: flex;
  align-items: center;
}

.input-container button-group {
  border: 1px solid gray !important;
  border-radius: 9px;
}

.input-container button {
  display: block;
  cursor: pointer;
  background-color: #f0eaea;
  border: none;
  font-size: 16px;
  padding: 3px 10px;
}

.input-container button-left {
  border-top-left-radius: 8px;
  border-bottom-left-radius: 8px;
}

.input-container button-right {
  border-top-right-radius: 8px;
  border-bottom-right-radius: 8px;
}

.input-container button.active {
  background-color: var(--q-primary) !important;
  font-weight: bold;
  color: white;
}
.unit-container {
  display: flex;
  height: 36px;
  margin-top: 9px;
  width: 100px;
}
</style>
