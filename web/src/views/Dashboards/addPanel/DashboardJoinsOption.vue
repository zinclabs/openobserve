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
          v-for="(itemY, index) in dashboardPanelData.data.queries[
            dashboardPanelData.layout.currentQueryIndex
          ].fields?.y"
          :key="index"
        >
          <div
            v-if="
              dashboardPanelData.meta.dragAndDrop.targetDragIndex == index &&
              dashboardPanelData.meta.dragAndDrop.currentDragArea == 'y'
            "
            class="dragItem"
          >
            &nbsp;
          </div>
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
                :data-test="`dashboard-y-item-${itemY?.column}`"
                class="q-pl-sm"
              >
              </q-btn>
              <q-btn
                style="height: 100%"
                size="xs"
                dense
                :data-test="`dashboard-y-item-${itemY?.column}-remove`"
                icon="close"
              />
              <!-- @click="removeYAxisItem(itemY?.column)" -->
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
          @click="showAddJoinPopUp = !showAddJoinPopUp"
        />
      </div>
    </div>
    <q-dialog v-model="showAddJoinPopUp">
      <AddJoinPopUp
        :class="store.state.theme == 'dark' ? 'dark-mode' : 'bg-white'"
        v-model="addJoinModel"
        :isEditMode="false"
        mainStream="default"
        @close="showAddJoinPopUp = false"
      />
    </q-dialog>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, inject } from "vue";
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

    const showAddJoinPopUp = ref(false);

    const addJoinModel = ref({
      stream: "default1",
      joinType: "inner",
      conditions: [
        {
          field1: "",
          field2: "",
          logicalOperator: "",
          operation: "",
        },
      ],
    });

    return {
      t,
      store,
      dashboardPanelData,
      removeFilterItem,
      loadFilterItem,
      showAddJoinPopUp,
      addJoinModel,
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
