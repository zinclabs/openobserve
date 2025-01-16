<!-- Copyright 2023 OpenObserve Inc.

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

     http:www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License. 
-->

<!-- eslint-disable vue/no-unused-components -->
<template>
  <div
    style="padding: 0px 10px; min-width: 60%"
    class="scroll o2-input"
    data-test="dashboard-drilldown-popup"
  >
    <div
      class="flex justify-between items-center q-pa-md"
      style="border-bottom: 2px solid gray; margin-bottom: 5px"
    >
      <div class="flex items-center q-table__title q-mr-md">
        <span data-test="dashboard-drilldown-title" v-if="isEditMode"
          >Edit Join
        </span>
        <span data-test="dashboard-drilldown-title" v-else>Add Join</span>
      </div>
    </div>

    <q-card-actions class="confirmActions">
      <q-btn
        unelevated
        no-caps
        class="q-mr-sm"
        @click="$emit('close')"
        data-test="cancel-button"
      >
        {{ t("confirmDialog.cancel") }}
      </q-btn>
      <q-btn
        unelevated
        no-caps
        class="no-border"
        color="primary"
        style="min-width: 60px"
        data-test="confirm-button"
        :label="isEditMode ? 'Update' : 'Add'"
      />
      <!-- :disable="isFormValid" -->
      <!-- @click="saveDrilldown" -->
    </q-card-actions>
  </div>
</template>

<script lang="ts">
import { inject } from "vue";
import { defineComponent } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
export default defineComponent({
  name: "AddJoinPopUp",
  components: {},
  props: {
    isEditMode: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["close"],
  setup(props, { emit }) {
    const { t } = useI18n();
    const store = useStore();

    return {
      t,
    };
  },
});
</script>

<style lang="scss" scoped>
.selected {
  background-color: var(--q-primary) !important;
  font-weight: bold;
  color: white;
}
.dropdownDiv {
  display: flex;
  align-items: center;
  margin: 10px 0px;
  width: 100%;
}

.dropdownLabel {
  width: 150px;
}

.dropdown {
  min-width: 100%;
}

:deep(.no-case .q-field__native > :first-child) {
  text-transform: none !important;
}
</style>
