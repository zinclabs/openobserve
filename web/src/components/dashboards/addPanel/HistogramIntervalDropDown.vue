<template>
  <!-- histogram interval for sql queries -->
  <q-select
    v-model="histogramIntervalModel"
    label="Histogram interval"
    :options="histogramIntervalOptions"
    behavior="menu"
    filled
    borderless
    data-test="histogram-interval-dropdown"
    dense
    @update:model-value="$emit('update:modelValue', $event)"
    :display-value="`${histogramIntervalModel?.label ?? 'Auto'}`"
  >
  </q-select>
</template>

<script lang="ts">
import { defineComponent, ref, watch } from "vue";

export default defineComponent({
  name: "HistogramIntervalDropDown",
  props: {
    modelValue: {
      type: Object,
      required: true,
    },
  },
  emits: ["update:modelValue"],
  setup(props) {
    const histogramIntervalOptions = [
      {
        label: "Auto",
        value: null,
      },
      {
        label: "1 second",
        value: "1 second",
      },
      {
        label: "5 seconds",
        value: "5 seconds",
      },
      {
        label: "10 seconds",
        value: "10 seconds",
      },
      {
        label: "30 seconds",
        value: "30 seconds",
      },
      {
        label: "1 minute",
        value: "1 minute",
      },
      {
        label: "5 minutes",
        value: "5 minutes",
      },
      {
        label: "10 minutes",
        value: "10 minutes",
      },
      {
        label: "15 minutes",
        value: "15 minutes",
      },
      {
        label: "30 minutes",
        value: "30 minutes",
      },
      {
        label: "45 minutes",
        value: "45 minutes",
      },
      {
        label: "1 hour",
        value: "1 hour",
      },
      {
        label: "2 hours",
        value: "2 hours",
      },
      {
        label: "3 hours",
        value: "3 hours",
      },
      {
        label: "6 hours",
        value: "6 hours",
      },
      {
        label: "8 hours",
        value: "8 hours",
      },
      {
        label: "12 hours",
        value: "12 hours",
      },
      {
        label: "1 day",
        value: "1 day",
      },
      {
        label: "2 days",
        value: "2 days",
      },
      {
        label: "3 days",
        value: "3 days",
      },
      {
        label: "5 days",
        value: "5 days",
      },
      {
        label: "7 days",
        value: "7 days",
      },
      {
        label: "30 days",
        value: "30 days",
      },
    ];

    const histogramIntervalModel: any = ref(
      histogramIntervalOptions.find(
        (v: any) => v.value == props.modelValue,
      ) ?? { label: "Auto", value: null },
    );

    // on modelvalue change, update the model
    watch(
      () => props.modelValue,
      () => {
        histogramIntervalModel.value = histogramIntervalOptions.find(
          (v: any) => v.value == props.modelValue,
        );
      },
    );

    return {
      histogramIntervalModel,
      histogramIntervalOptions,
    };
  },
});
</script>

<style lang="scss" scoped></style>
