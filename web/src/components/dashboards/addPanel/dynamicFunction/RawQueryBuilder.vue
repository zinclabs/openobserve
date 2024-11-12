<template>
  <div class="row">RAW QUERY</div>
</template>

<script lang="ts">
import { ref } from "vue";
//   import useDashboardPanelData from "@/composables/useDashboardPanel";

export default {
  name: "RawQueryBuilder",
  setup() {
    //   const dashboardPanelDataPageKey = inject(
    //     "dashboardPanelDataPageKey",
    //     "dashboard",
    //   );
    //   const { dashboardPanelData } = useDashboardPanelData(
    //     dashboardPanelDataPageKey
    //   );

    const fields = ref({
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
