<template>
  <div>
    <div
      v-if="
        !(
          dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].customQuery && dashboardPanelData.data.queryType == 'sql'
        )
      "
      style="display: flex; flex-direction: row"
      class="q-pl-md"
    >
      <div class="layout-name">{{ t("panel.joins") }}</div>
      <span class="layout-separator">:</span>
      <div
        class="axis-container droppable scroll row"
        data-test="dashboard-filter-layout"
      >
        <div
          class="row q-mr-sm q-my-xs"
          v-for="(joinObj, index) in dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].joins"
          :key="index"
        >
          <q-btn-group class="axis-field">
            <div>
              <!-- <q-icon
                name="drag_indicator"
                color="grey-13"
                size="13px"
                class="cursor-grab q-my-xs"
              /> -->
              <!-- icon-right="arrow_drop_down" -->
              <q-btn
                no-caps
                dense
                color="primary"
                square
                :no-wrap="true"
                size="sm"
                :label="index"
                :data-test="`dashboard-join-item-${index}`"
                class="q-pl-sm"
              >
                <q-menu
                  class="q-pa-md"
                  :data-test="`dashboard-join-menu-${index}`"
                >
                  <AddJoinPopUp
                    :class="
                      store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'
                    "
                    v-model="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].joins[index]
                    "
                    :joinIndex="index"
                    :mainStream="
                      dashboardPanelData.data.queries[
                        dashboardPanelData.layout.currentQueryIndex
                      ].fields.stream
                    "
                  />
                </q-menu>
              </q-btn>
              <q-btn
                style="height: 100%"
                size="xs"
                dense
                :data-test="`dashboard-join-item-${index}-remove`"
                icon="close"
                @click="removeJoin(index)"
              />
            </div>
          </q-btn-group>
        </div>
        <q-btn
          icon="add"
          color="primary"
          size="xs"
          round
          class="add-btn"
          data-test="dashboard-add-join-btn"
          @click="addJoin"
        />
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, inject, onMounted, watch } from "vue";
import useDashboardPanelData from "../../../composables/useDashboardPanel";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import AddJoinPopUp from "./AddJoinPopUp.vue";

export default defineComponent({
  name: "DashboardJoinOption",
  components: {
    AddJoinPopUp,
  },
  props: [],

  setup(props) {
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );

    const { dashboardPanelData, removeFilterItem, loadFilterItem } =
      useDashboardPanelData(dashboardPanelDataPageKey);

    const { t } = useI18n();
    const store = useStore();

    const initializeJoinObj = () => {
      dashboardPanelData?.data?.queries?.forEach((queryObj: any) => {
        if (!queryObj?.joins) {
          queryObj.joins = [];
        }
      });
    };

    const addJoin = () => {
      const initialValue = {
        stream: "",
        streamAlias: "",
        joinType: "inner",
        conditions: [
          {
            leftField: {
              streamAlias: "",
              field: "",
            },
            rightField: {
              streamAlias: "",
              field: "",
            },
            logicalOperator: "AND",
            operation: "=",
          },
        ],
      };

      // initialize join array is available for old version as well
      initializeJoinObj();

      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ]?.joins?.push(initialValue);
    };

    const removeJoin = (index: number) => {
      dashboardPanelData.data.queries[
        dashboardPanelData.layout.currentQueryIndex
      ].joins.splice(index, 1);
    };

    onMounted(() => {
      // initialize join array is available for old version as well
      initializeJoinObj();
    });

    watch(
      () =>
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ].joins,
      () => {
        // generate alias for each join stream
        // make sure that alias is unique
        // if stream is duplicate then add _1, _2, _3 etc
        dashboardPanelData.data.queries[
          dashboardPanelData.layout.currentQueryIndex
        ]?.joins?.forEach((join: any, index: number) => {
          if (join?.stream) {
            join.streamAlias = `stream_${index}`;
          }
        });
      },
      {
        deep: true,
      },
    );

    return {
      t,
      store,
      dashboardPanelData,
      removeFilterItem,
      loadFilterItem,
      addJoin,
      removeJoin,
    };
  },
});
</script>

<style lang="scss" scoped>
.layout-name {
  font-size: 14px;
  white-space: nowrap;
  min-width: 130px;
  display: flex;
  align-items: center;
}

.layout-separator {
  display: flex;
  align-items: center;
  margin-left: 2px;
  margin-right: 2px;
}

.axis-container {
  margin: 5px;
}
</style>
