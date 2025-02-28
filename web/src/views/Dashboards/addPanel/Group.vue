<template>
  <!-- <q-select
    v-if="groupIndex !== 0"
    v-model="group.logicalOperator"
    dense
    options-dense
    filled
    :options="filterOptions"
    @update:model-value="emitLogicalOperatorChange"
    class="condition-logical-operator"
    data-test="dashboard-add-group-logical-operator"
  /> -->
  <div class="group" :style="`--group-index: ${groupNestedIndex}`">
    <div class="group-conditions">
      <div
        v-for="(condition, index) in group.conditions"
        :key="index"
        class="condition-group"
      >
        <Group
          v-if="condition.filterType === 'group'"
          :group="condition"
          :group-nested-index="groupNestedIndex + 1"
          :group-index="index"
          :dashboard-variables-filter-items="dashboardVariablesFilterItems"
          :schema-options="schemaOptions"
          :load-filter-item="loadFilterItem"
          :dashboard-panel-data="dashboardPanelData"
          @add-condition="addConditionToGroup"
          @add-group="addGroupToGroup"
          @remove-group="removeGroupFromNested(index)"
          @logical-operator-change="emitLogicalOperatorChange"
        />
        <AddCondition
          v-else-if="condition.filterType === 'condition'"
          :condition="condition"
          :dashboard-variables-filter-items="dashboardVariablesFilterItems"
          :schema-options="schemaOptions"
          :load-filter-item="loadFilterItem"
          :dashboard-panel-data="dashboardPanelData"
          @remove-condition="removeConditionFromGroup(index)"
          @logical-operator-change="emitLogicalOperatorChange"
          :condition-index="index"
        />
      </div>
      <q-btn
        icon="add"
        color="primary"
        size="xs"
        round
        class="add-btn"
        data-test="dashboard-add-condition-add"
      >
        <q-menu v-model="showAddMenu">
          <q-list>
            <q-item clickable @click="emitAddCondition">
              <q-item-section data-test="dashboard-add-group-add-condition">{{ t("common.addCondition") }}</q-item-section>
            </q-item>
            <q-item clickable @click="emitAddGroup">
              <q-item-section data-test="dashboard-add-group-add-group">{{ t("common.addGroup") }}</q-item-section>
            </q-item>
          </q-list>
        </q-menu>
      </q-btn>
    </div>
    <div v-if="groupNestedIndex !== 0" class="group-remove">
      <q-btn
        flat
        size="xs"
        dense
        @click="$emit('remove-group')"
        icon="close"
        data-test="dashboard-add-group-remove"
      />
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import AddCondition from "./AddCondition.vue";

export default defineComponent({
  name: "Group",
  components: { AddCondition },
  props: {
    group: {
      type: Object,
      required: true,
    },
    // For identifying the index of that group in the current conditions array
    groupIndex: {
      type: Number,
      required: true,
    },
    // For identifying the index of the nested position of that group
    groupNestedIndex: {
      type: Number,
      required: true,
    },
    dashboardVariablesFilterItems: {
      type: Array,
      required: true,
    },
    schemaOptions: {
      type: Array,
      required: true,
    },
    loadFilterItem: {
      type: Function,
      required: true,
    },
    dashboardPanelData: {
      type: Object,
      required: true,
    },
  },
  emits: [
    "add-condition",
    "add-group",
    "remove-group",
    "logical-operator-change",
  ],
  setup(props, { emit }) {
    const { t } = useI18n();
    const showAddMenu = ref(false);
    const filterOptions = ["AND", "OR"];

    const emitAddCondition = () => {
      emit("add-condition", props.group);
      showAddMenu.value = false;
    };

    const emitAddGroup = () => {
      emit("add-group", props.group);
      showAddMenu.value = false;
    };

    const removeConditionFromGroup = (index: number) => {
      props.group.conditions.splice(index, 1);
    };

    const removeGroupFromNested = (groupIndex: number) => {
      const removeGroup = (conditions: any[], index: number) => {
        const nestedGroup = conditions[index];
        if (nestedGroup && nestedGroup.filterType === "group") {
          // Create a copy of the conditions array to avoid modifying it while iterating
          const nestedConditions = [...nestedGroup.conditions];
          nestedConditions.forEach((condition: any, idx: number) => {
            if (condition.filterType === "group") {
              removeGroup(nestedGroup.conditions, idx);
            }
          });
        }
        conditions.splice(index, 1);
      };

      removeGroup(props.group.conditions, groupIndex);
    };

    const addConditionToGroup = (group: any) => {
      emit("add-condition", group);
    };

    const addGroupToGroup = (group: any) => {
      emit("add-group", group);
    };
    const emitLogicalOperatorChange = (newOperator: string) => {
      emit("logical-operator-change", newOperator);
    };
    return {
      t,
      showAddMenu,
      emitAddCondition,
      emitAddGroup,
      removeConditionFromGroup,
      removeGroupFromNested,
      addConditionToGroup,
      addGroupToGroup,
      emitLogicalOperatorChange,
      filterOptions,
    };
  },
});
</script>

<style lang="scss" scoped>
.group {
  display: flex;
  padding: 0px 0px 0px 5px;
  background-color: rgba(89, 96, 178, calc(0.12 * var(--group-index)));
  border-radius: 5px;
}

.group-remove {
  border-left: 1px solid $grey-2;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.group-conditions {
  display: flex;
  flex-direction: row;
  // flex-wrap: wrap;
  overflow-x: auto;
  align-items: center;
}

.condition-group {
  display: inline-flex;
  align-items: center;
  margin-right: 10px;
  padding: 0px 0px 0px 0px;
  min-height: 35px;
  gap: 8px;
}

.add-btn {
  height: 20px;
  width: 20px;
  margin-right: 5px;
}

.condition-logical-operator {
  width: 55px;
}

:deep(.condition-logical-operator .q-field__control) {
  min-height: 23px !important;
  height: 23px !important;
  padding: 0px 0px 0px 5px !important;
}

:deep(.condition-logical-operator .q-field__native) {
  min-height: 23px !important;
  height: 23px !important;
  padding: 0px 0px 0px 0px !important;
}

:deep(.condition-logical-operator .q-field__append) {
  min-height: 23px !important;
  height: 23px !important;
  padding: 0px 0px 0px 0px !important;
}
</style>
