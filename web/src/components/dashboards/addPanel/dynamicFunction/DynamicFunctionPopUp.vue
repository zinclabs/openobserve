<template>
  <div style="width: 500px">
    <q-tabs
      v-model="fields.type"
      dense
      class="text-grey"
      active-color="primary"
      indicator-color="primary"
      narrow-indicator
    >
      <q-tab name="build" label="Build" />
      <q-tab name="raw" label="Raw" />
    </q-tabs>

    <q-separator />

    <q-tab-panels v-model="fields.type" animated>
      <q-tab-panel name="build">
        <SelectFunction />
      </q-tab-panel>
      <q-tab-panel name="raw">
        <RawQueryBuilder />
      </q-tab-panel>
    </q-tab-panels>
  </div>
</template>

<script lang="ts">
import { ref } from "vue";
//   import useDashboardPanelData from "@/composables/useDashboardPanel";
import RawQueryBuilder from "./RawQueryBuilder.vue";
import SelectFunction from "./SelectFunction.vue";

export default {
  name: "DynamicFunctionPopUp",
  components: { RawQueryBuilder, SelectFunction },
  setup() {
    //   const dashboardPanelDataPageKey = inject(
    //     "dashboardPanelDataPageKey",
    //     "dashboard",
    //   );
    //   const { dashboardPanelData } = useDashboardPanelData(
    //     dashboardPanelDataPageKey
    //   );

    const fields = ref({
      type: "build",
      functionName: "histogram",
      args: [
        {
          type: "field",
          fieldName: "_timestamp",
        },
        {
          type: "histogramInterval",
          value: "5 min",
        },
      ],
    });

    return {
      fields,
    };
  },
};
</script>
<style scoped></style>
