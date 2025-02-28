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
  <div v-if="!promqlMode && dashboardPanelData.data.type == 'geomap'">
    <!-- latitude container -->
    <div style="display: flex; flex-direction: row" class="q-pl-md">
      <div class="layout-name">
        {{ t("panel.latitude") }}
        <q-icon name="info_outline" class="q-ml-xs">
          <q-tooltip>
            {{ Hint }}
          </q-tooltip>
        </q-icon>
      </div>
      <span class="layout-separator">:</span>
      <div
        class="axis-container droppable scroll"
        :class="{
          'drop-target': dashboardPanelData.meta.dragAndDrop.dragging,
          'drop-entered':
            dashboardPanelData.meta.dragAndDrop.dragging &&
            dashboardPanelData.meta.dragAndDrop.currentDragArea == 'latitude',
        }"
        @dragend="onDragEnd()"
        @dragover="onDragOver($event, 'latitude')"
        @drop="onDrop($event, 'latitude')"
        @dragenter="onDragEnter($event, 'latitude', null)"
        data-test="dashboard-latitude-layout"
      >
        <q-btn-group
          class="axis-field q-mr-sm q-my-xs"
          v-if="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields?.latitude
          "
          :draggable="true"
          @dragstart="
            onFieldDragStart(
              $event,
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields?.latitude.column,
              'latitude',
            )
          "
        >
          <div>
            <q-icon
              name="drag_indicator"
              color="grey-13"
              size="13px"
              class="'cursor-grab q-my-xs'"
            />
            <q-btn
              square
              icon-right="arrow_drop_down"
              no-caps
              color="primary"
              dense
              size="sm"
              :label="latitudeLabel"
              class="q-pl-sm"
              :data-test="`dashboard-latitude-item-${latitudeLabel}`"
            >
              <q-menu
                class="q-pa-md"
                :data-test="`dashboard-latitude-item-${latitudeLabel}-menu`"
              >
                <div>
                  <DynamicFunctionPopUp
                    v-model="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields.latitude
                    "
                  />
                  <q-input
                    dense
                    filled
                    data-test="dashboard-latitude-item-input"
                    :label="t('common.label')"
                    v-model="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields.latitude.label
                    "
                    :rules="[(val: any) => val > 0 || 'Required']"
                  />
                  <div
                    v-if="
                      !dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery &&
                      dashboardPanelData.data.queryType == 'sql'
                    "
                  >
                    <SortByBtnGrp
                      :fieldObj="
                        dashboardPanelData.data.queries[
                          dashboardPanelData.layout.currentQueryIndex
                        ].fields.latitude
                      "
                    />
                  </div>
                </div>
              </q-menu>
            </q-btn>
            <q-btn
              style="height: 100%"
              size="xs"
              dense
              :data-test="`dashboard-latitude-item-${latitudeLabel}-remove`"
              @click="removeLatitude()"
              icon="close"
            />
          </div>
        </q-btn-group>
        <div
          class="text-caption text-weight-bold text-center q-py-xs"
          v-if="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.latitude == null
          "
        >
          <div class="q-mt-xs">{{ Hint }}</div>
        </div>
      </div>
    </div>
    <q-separator />
    <!-- longitude container -->
    <div style="display: flex; flex-direction: row" class="q-pl-md">
      <div class="layout-name">
        {{ t("panel.longitude") }}
        <q-icon name="info_outline" class="q-ml-xs">
          <q-tooltip>
            {{ Hint }}
          </q-tooltip>
        </q-icon>
      </div>
      <span class="layout-separator">:</span>
      <div
        class="axis-container droppable scroll"
        :class="{
          'drop-target': dashboardPanelData.meta.dragAndDrop.dragging,
          'drop-entered':
            dashboardPanelData.meta.dragAndDrop.dragging &&
            dashboardPanelData.meta.dragAndDrop.currentDragArea == 'longitude',
        }"
        @dragend="onDragEnd()"
        @dragover="onDragOver($event, 'longitude')"
        @drop="onDrop($event, 'longitude')"
        @dragenter="onDragEnter($event, 'longitude', null)"
        data-test="dashboard-longitude-layout"
      >
        <q-btn-group
          class="axis-field q-mr-sm q-my-xs"
          v-if="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields?.longitude
          "
          :draggable="true"
          @dragstart="
            onFieldDragStart(
              $event,
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields?.longitude.column,
              'longitude',
            )
          "
        >
          <div>
            <q-icon
              name="drag_indicator"
              color="grey-13"
              size="13px"
              class="'cursor-grab q-my-xs'"
            />
            <q-btn
              square
              icon-right="arrow_drop_down"
              no-caps
              dense
              color="primary"
              size="sm"
              :label="longitudeLabel"
              :data-test="`dashboard-longitude-item-${longitudeLabel}`"
              class="q-pl-sm"
            >
              <q-menu
                class="q-pa-md"
                :data-test="`dashboard-longitude-item-${longitudeLabel}-menu`"
              >
                <div>
                  <DynamicFunctionPopUp
                    v-model="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields.longitude
                    "
                  />
                  <q-input
                    dense
                    filled
                    label="Label"
                    data-test="dashboard-longitude-item-input"
                    v-model="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields.longitude.label
                    "
                    :rules="[(val: any) => val > 0 || 'Required']"
                  />
                  <div
                    v-if="
                      !dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery &&
                      dashboardPanelData.data.queryType == 'sql'
                    "
                  >
                    <SortByBtnGrp
                      :fieldObj="
                        dashboardPanelData.data.queries[
                          dashboardPanelData.layout.currentQueryIndex
                        ].fields.longitude
                      "
                    />
                  </div>
                </div>
              </q-menu>
            </q-btn>
            <q-btn
              style="height: 100%"
              size="xs"
              dense
              :data-test="`dashboard-longitude-item-${longitudeLabel}-remove`"
              @click="removeLongitude()"
              icon="close"
            />
          </div>
        </q-btn-group>
        <div
          class="text-caption text-weight-bold text-center q-py-xs"
          v-if="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.longitude == null
          "
        >
          <div class="q-mt-xs">{{ Hint }}</div>
        </div>
      </div>
    </div>
    <q-separator />
    <!-- weight container -->
    <div style="display: flex; flex-direction: row" class="q-pl-md">
      <div class="layout-name">
        {{ t("panel.weight") }}
        <q-icon name="info_outline" class="q-ml-xs">
          <q-tooltip>
            {{ WeightHint }}
          </q-tooltip>
        </q-icon>
      </div>
      <span class="layout-separator">:</span>
      <div
        class="axis-container droppable scroll"
        :class="{
          'drop-target': dashboardPanelData.meta.dragAndDrop.dragging,
          'drop-entered':
            dashboardPanelData.meta.dragAndDrop.dragging &&
            dashboardPanelData.meta.dragAndDrop.currentDragArea == 'weight',
        }"
        @dragend="onDragEnd()"
        @dragover="onDragOver($event, 'weight')"
        @drop="onDrop($event, 'weight')"
        @dragenter="onDragEnter($event, 'weight', null)"
        data-test="dashboard-weight-layout"
      >
        <q-btn-group
          class="axis-field q-mr-sm q-my-xs"
          v-if="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields?.weight
          "
          :draggable="true"
          @dragstart="
            onFieldDragStart(
              $event,
              dashboardPanelData.data.queries[
                dashboardPanelData.layout.currentQueryIndex
              ].fields?.weight.column,
              'weight',
            )
          "
        >
          <div>
            <q-icon
              name="drag_indicator"
              color="grey-13"
              size="13px"
              class="'cursor-grab q-my-xs'"
            />
            <q-btn
              square
              icon-right="arrow_drop_down"
              no-caps
              dense
              color="primary"
              size="sm"
              :label="weightLabel"
              :data-test="`dashboard-weight-item-${weightLabel}`"
              class="q-pl-sm"
            >
              <q-menu
                class="q-pa-md"
                :data-test="`dashboard-weight-item-${weightLabel}-menu`"
              >
                <div>
                  <div class="row q-mb-sm" style="align-items: center">
                    <div
                      v-if="
                        !dashboardPanelData.data.queries[
                          dashboardPanelData.layout.currentQueryIndex
                        ].customQuery
                      "
                      class="q-mr-xs"
                      style="width: 160px"
                    >
                      <DynamicFunctionPopUp
                        v-model="
                          dashboardPanelData.data.queries[
                            dashboardPanelData.layout.currentQueryIndex
                          ].fields.weight
                        "
                      />
                      <!-- <q-select
                        v-model="
                          dashboardPanelData.data.queries[
                            dashboardPanelData.layout.currentQueryIndex
                          ].fields.weight.aggregationFunction
                        "
                        :options="triggerOperators"
                        dense
                        filled
                        emit-value
                        map-options
                        :label="t('common.aggregation')"
                        data-test="dashboard-weight-item-dropdown"
                      >
                        <template v-slot:append>
                          <q-icon
                            name="close"
                            size="small"
                            @click.stop.prevent="
                              dashboardPanelData.data.queries[
                                dashboardPanelData.layout.currentQueryIndex
                              ].fields.weight.aggregationFunction = null
                            "
                            class="cursor-pointer"
                          />
                        </template>
                      </q-select> -->
                    </div>
                  </div>
                  <q-input
                    dense
                    filled
                    :label="t('common.label')"
                    data-test="dashboard-weight-item-input"
                    v-model="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields.weight.label
                    "
                    :rules="[(val: any) => val > 0 || 'Required']"
                  />
                  <div
                    v-if="
                      !dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].customQuery &&
                      dashboardPanelData.data.queryType == 'sql'
                    "
                  >
                    <SortByBtnGrp
                      :fieldObj="
                        dashboardPanelData.data.queries[
                          dashboardPanelData.layout.currentQueryIndex
                        ].fields.weight
                      "
                    />
                  </div>
                </div>
              </q-menu>
            </q-btn>
            <q-btn
              style="height: 100%"
              size="xs"
              dense
              :data-test="`dashboard-weight-item-${weightLabel}-remove`"
              @click="removeWeight()"
              icon="close"
            />
          </div>
        </q-btn-group>
        <div
          class="text-caption text-weight-bold text-center q-py-xs"
          v-if="
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.weight == null
          "
        >
          <div class="q-mt-xs">{{ WeightHint }}</div>
        </div>
      </div>
    </div>
    <q-separator />
    <!-- filters container -->
    <DashboardFiltersOption
      :dashboardData="dashboardData"
    ></DashboardFiltersOption>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, reactive, watch, computed } from "vue";
import { useI18n } from "vue-i18n";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import { getImageURL } from "../../../utils/zincutils";
import SortByBtnGrp from "@/components/dashboards/addPanel/SortByBtnGrp.vue";
import CommonAutoComplete from "@/components/dashboards/addPanel/CommonAutoComplete.vue";
import SanitizedHtmlRenderer from "@/components/SanitizedHtmlRenderer.vue";
import { inject } from "vue";
import useNotifications from "@/composables/useNotifications";
import DashboardFiltersOption from "@/views/Dashboards/addPanel/DashboardFiltersOption.vue";
import DynamicFunctionPopUp from "@/components/dashboards/addPanel/dynamicFunction/DynamicFunctionPopUp.vue";
import { buildSQLQueryFromInput } from "@/utils/dashboard/convertDataIntoUnitValue";

export default defineComponent({
  name: "DashboardGeoMapsQueryBuilder",
  components: {
    SortByBtnGrp,
    CommonAutoComplete,
    SanitizedHtmlRenderer,
    DashboardFiltersOption,
    DynamicFunctionPopUp,
  },
  props: ["dashboardData"],
  setup(props) {
    const { t } = useI18n();
    const { showErrorNotification } = useNotifications();
    const expansionItems = reactive({
      latitude: true,
      longitude: true,
      weight: true,
      filter: false,
    });
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );
    const {
      dashboardPanelData,
      addLatitude,
      addLongitude,
      addWeight,
      removeLatitude,
      removeLongitude,
      removeWeight,
      addFilteredItem,
      promqlMode,
      cleanupDraggingFields,
      selectedStreamFieldsBasedOnUserDefinedSchema,
    } = useDashboardPanelData(dashboardPanelDataPageKey);
    const triggerOperators = [
      { label: t("dashboard.count"), value: "count" },
      { label: t("dashboard.countDistinct"), value: "count-distinct" },
      { label: t("dashboard.sum"), value: "sum" },
      { label: t("dashboard.avg"), value: "avg" },
      { label: t("dashboard.min"), value: "min" },
      { label: t("dashboard.max"), value: "max" },
      {
        label: t("dashboard.p50"),
        value: "p50",
      },
      {
        label: t("dashboard.p90"),
        value: "p90",
      },
      {
        label: t("dashboard.p95"),
        value: "p95",
      },
      {
        label: t("dashboard.p99"),
        value: "p99",
      },
    ];

    watch(
      () => dashboardPanelData.meta.dragAndDrop.dragging,
      (newVal: boolean, oldVal: boolean) => {
        if (oldVal == false && newVal == true) {
          expansionItems.latitude = true;
          expansionItems.longitude = true;
          expansionItems.weight = true;
          expansionItems.filter = true;
        }
      },
    );

    const onDrop = (e: any, targetAxis: string) => {
      // move the items  between axis or from the field list
      // check if the source is from axis or field list
      if (dashboardPanelData.meta.dragAndDrop.dragSource === "fieldList") {
        // add the item to the target list
        const dragElement = dashboardPanelData.meta.dragAndDrop.dragElement;
        if (!dragElement) {
          return;
        }

        switch (targetAxis) {
          case "latitude":
            addLatitude(dragElement);
            break;
          case "longitude":
            addLongitude(dragElement);
            break;
          case "weight":
            addWeight(dragElement);
            break;
          case "f":
            addFilteredItem(dragElement?.name);
            break;
        }
      } else {
        // move the item from field list to axis
        const dragElement = dashboardPanelData.meta.dragAndDrop.dragElement;

        const dragName =
          selectedStreamFieldsBasedOnUserDefinedSchema.value.find(
            (item: any) => item?.name === dragElement,
          );
        const customDragName =
          dashboardPanelData.meta.stream.customQueryFields.find(
            (item: any) => item?.name === dragElement,
          );

        if (dragName || customDragName) {
          const currentQueryField =
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields;
          if (targetAxis !== "f") {
            if (
              (targetAxis === "latitude" && currentQueryField.latitude) ||
              (targetAxis === "longitude" && currentQueryField.longitude) ||
              (targetAxis === "weight" && currentQueryField.weight)
            ) {
              const maxAllowedAxisFields = 1;

              const errorMessage = `Max ${maxAllowedAxisFields} field in ${targetAxis.toUpperCase()} is allowed.`;

              showErrorNotification(errorMessage);
              cleanupDraggingFields();
              return;
            }

            // Remove from the original axis
            const dragSource = dashboardPanelData.meta.dragAndDrop.dragSource;
            if (dragSource === "latitude") {
              removeLatitude();
            } else if (dragSource === "longitude") {
              removeLongitude();
            } else if (dragSource === "weight") {
              removeWeight();
            }
          }
        }
        if (targetAxis === "f") {
          return;
        }

        // Add to the new axis
        if (targetAxis === "latitude") {
          addLatitude(dragName || customDragName);
        } else if (targetAxis === "longitude") {
          addLongitude(dragName || customDragName);
        } else if (targetAxis === "weight") {
          addWeight(dragName || customDragName);
        }
      }

      cleanupDraggingFields();
    };

    const onDragStart = (e: any, item: any) => {
      e.preventDefault();
    };

    const onDragOver = (e: any, area: string) => {
      e.preventDefault();
    };
    const onDragEnter = (e: any, area: string, index: any) => {
      if (
        dashboardPanelData.meta.dragAndDrop.dragSource != "fieldList" &&
        area === "f"
      ) {
        e.preventDefault();
        return;
      }
      dashboardPanelData.meta.dragAndDrop.targetDragIndex =
        index != null && index >= 0
          ? index
          : dashboardPanelData.meta.dragAndDrop.targetDragIndex;
      dashboardPanelData.meta.dragAndDrop.currentDragArea = area;
      e.preventDefault();
    };

    const onFieldDragStart = (e: any, item: any, axis: string) => {
      dashboardPanelData.meta.dragAndDrop.dragging = true;
      dashboardPanelData.meta.dragAndDrop.dragElement = item;
      dashboardPanelData.meta.dragAndDrop.dragSource = axis;
    };

    const onDragEnd = () => {
      cleanupDraggingFields();
    };
    const Hint = computed((e: any) => {
      switch (dashboardPanelData.data.type) {
        case "geomap":
          return t("dashboard.oneFieldMessage");
        default:
          return t("dashboard.maxtwofieldMessage");
      }
    });

    const WeightHint = computed((e: any) => {
      switch (dashboardPanelData.data.type) {
        case "geomap":
          return t("dashboard.oneFieldConfigMessage");
        default:
          return t("dashboard.maxtwofieldMessage");
      }
    });

    const commonBtnLabel = (field: any) => {
      if (
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].customQuery
      ) {
        // HERE NEED CHANGES
        return field.column;
      }
      return buildSQLQueryFromInput(
        field,
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields?.stream,
      );
    };

    const latitudeLabel = computed(() => {
      const latitudeField =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.latitude;
      return commonBtnLabel(latitudeField);
    });

    const longitudeLabel = computed(() => {
      const longitudeField =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.longitude;
      return commonBtnLabel(longitudeField);
    });

    const weightLabel = computed(() => {
      const weightField =
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].fields.weight;
      return commonBtnLabel(weightField);
    });

    return {
      t,
      dashboardPanelData,
      removeLatitude,
      removeLongitude,
      removeWeight,
      triggerOperators,
      pagination: ref({
        rowsPerPage: 0,
      }),
      model: ref([]),
      tab: ref("General"),
      getImageURL,
      onDrop,
      onDragEnter,
      onDragStart,
      onDragEnd,
      onDragOver,
      expansionItems,
      Hint,
      WeightHint,
      promqlMode,
      latitudeLabel,
      longitudeLabel,
      weightLabel,
      onFieldDragStart,
      options: [
        "=",
        "<>",
        ">=",
        "<=",
        ">",
        "<",
        "IN",
        "Contains",
        "Not Contains",
        "Is Null",
        "Is Not Null",
      ],
    };
  },
});
</script>

<style lang="scss" scoped>
.axis-field {
  overflow: hidden;
}
:deep(.axis-field .q-btn--rectangle) {
  border-radius: 0%;
}
:deep(.axis-field .q-btn:before) {
  border: 0px solid transparent;
}

.axis-container {
  flex: 1;
  width: 100%;
  // white-space: nowrap;
  overflow-x: auto;
}

.layout-separator {
  display: flex;
  align-items: center;
  margin-left: 2px;
  margin-right: 2px;
}

.layout-name {
  white-space: nowrap;
  min-width: 130px;
  display: flex;
  align-items: center;
}

.droppable {
  border-color: transparent;
  border-style: dashed;
  border-width: 2px;
}

.drop-target {
  background-color: rgba(0, 0, 0, 0.042);
  border-color: white;
  border-style: dotted;
}

.drop-entered {
  transition: all;
  transition-duration: 200ms;
  background-color: #cbcbcb;
}

.color-input-wrapper {
  height: 1.5em;
  width: 1.5em;
  overflow: hidden;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  position: relative;
}

.color-input-wrapper input[type="color"] {
  position: absolute;
  height: 4em;
  width: 4em;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  overflow: hidden;
  border: none;
  margin: 0;
  padding: 0;
}

.q-menu {
  box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
  transform: translateY(0.5rem);
  border-radius: 0px;

  .q-virtual-scroll__content {
    padding: 0.5rem;
  }
}

.index-menu {
  width: 100%;

  .q-field {
    &__control {
      height: 35px;
      padding: 0px 5px;
      min-height: auto !important;

      &-container {
        padding-top: 0px !important;
      }
    }

    &__native :first-of-type {
      padding-top: 0.25rem;
    }
  }

  .q-select {
    text-transform: capitalize;
  }

  .index-table {
    width: 100%;

    .q-table {
      display: block;
    }

    tr {
      margin-bottom: 1px;
    }

    tbody,
    tr,
    td {
      width: 100%;
      display: block;
      height: 25px;
    }

    .q-table__top {
      padding: 0px;
    }

    .q-table__control,
    label.q-field {
      width: 100%;
    }

    .q-table thead tr,
    .q-table tbody td {
      height: auto;
    }

    .q-table__top {
      border-bottom: unset;
    }
  }

  .field-table {
    width: 100%;
  }

  .field_list {
    padding: 0px;
    margin-bottom: 0.125rem;
    position: relative;
    overflow: visible;
    cursor: default;

    .field_overlay {
      justify-content: space-between;
      background-color: transparent;
      transition: all 0.3s ease;
      padding: 0px 10px;
      align-items: center;
      position: absolute;
      overflow: hidden;
      inset: 0;
      display: flex;
      z-index: 1;
      width: 100%;
      border-radius: 0px;
      height: 25px;

      .field_icons {
        padding: 0 0.625rem 0 0.25rem;
        transition: all 0.3s ease;
        background-color: white;
        position: absolute;
        z-index: 3;
        opacity: 0;
        right: 0;

        .q-icon {
          cursor: pointer;
        }
      }

      .field_label {
        pointer-events: none;
        font-size: 0.825rem;
        position: relative;
        display: inline;
        z-index: 2;
        left: 0;
      }
    }

    &.selected {
      .field_overlay {
        background-color: rgba(89, 96, 178, 0.3);

        .field_icons {
          opacity: 0;
        }
      }

      &:hover {
        .field_overlay {
          box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);
          background-color: white;

          .field_icons {
            background-color: white;
          }
        }
      }
    }

    &:hover {
      .field_overlay {
        box-shadow: 0px 4px 15px rgba(0, 0, 0, 0.17);

        .field_icons {
          background-color: white;
          opacity: 1;
        }
      }
    }
  }
}

.q-item {
  min-height: 1.3rem;
  padding: 5px 10px;

  &__label {
    font-size: 0.75rem;
  }

  &.q-manual-focusable--focused > .q-focus-helper {
    background: none !important;
    opacity: 0.3 !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &--active {
    background-color: $selected-list-bg !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &:hover,
  &--active {
    color: $primary;
  }
}

.q-field--dense .q-field__before,
.q-field--dense .q-field__prepend {
  padding: 0px 0px 0px 0px;
  height: auto;
  line-height: auto;
}

.q-field__native,
.q-field__input {
  padding: 0px 0px 0px 0px;
}

.q-field--dense .q-field__label {
  top: 5px;
}

.q-field--dense .q-field__control,
.q-field--dense .q-field__marginal {
  height: 34px;
}
</style>
