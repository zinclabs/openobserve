<template>
  <div class="q-pa-md">
    <div class="q-gutter-md">
      <q-select
        filled
        v-model="internalModel"
        :options="options"
        label="Select a Field"
        option-label="name"
        option-value="name"
        :display-value="internalModel?.field"
        map-options
      >
        <template v-slot:option="scope">
          <q-expansion-item
            expand-separator
            group="streamGroup"
            :default-opened="scope.index === 0 ? true : false"
            header-class="text-weight-bold"
            :label="scope.opt.label"
          >
            <template v-for="child in scope.opt.children" :key="child.name">
              <q-item
                clickable
                v-ripple
                v-close-popup
                @click="selectField(child)"
              >
                <q-item-section>
                  <q-item-label v-html="child.name"></q-item-label>
                </q-item-section>
              </q-item>
            </template>
          </q-expansion-item>
        </template>
      </q-select>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, watch, PropType, inject } from "vue";
import useDashboardPanelData from "@/composables/useDashboardPanel";
import useStreams from "@/composables/useStreams";

interface OptionChild {
  label: string;
  value: string;
}

interface Option {
  label: string;
  children: OptionChild[];
}

export default defineComponent({
  name: "StreamFieldSelect",

  props: {
    streams: {
      type: Array,
      required: true,
    },
    modelValue: {
      type: Object,
      default: {},
    },
  },

  emits: ["update:modelValue"],

  setup(props, { emit }) {
    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "dashboard",
    );

    const internalModel = ref(props.modelValue);

    const options = ref<Option[]>([]);

    const { getStream } = useStreams();
    const { dashboardPanelData } = useDashboardPanelData(
      dashboardPanelDataPageKey,
    );

    async function loadStreamFields(streamName: string) {
      try {
        if (streamName != "") {
          return await getStream(
            streamName,
            dashboardPanelData.data.queries[
              dashboardPanelData.layout.currentQueryIndex
            ].fields.stream_type ?? "logs",
            true,
          ).then((res) => {
            return res;
          });
        } else {
        }
        return;
      } catch (e: any) {
        console.log("Error while loading stream fields");
      }
    }

    async function fetchFieldsForStreams() {
      if (!props.streams || props.streams.length === 0) {
        options.value = [];
        return;
      }

      // Fetch schema for each stream and build options
      options.value = await Promise.all(
        props.streams.map(async (stream: any) => {
          const streamSchemaObj = await loadStreamFields(stream.stream);

          return {
            label: stream.streamAlias
              ? `${stream.stream}(${stream.streamAlias})`
              : `${stream.stream}`,
            children: streamSchemaObj?.schema?.map((field: any) => ({
              ...field,
              stream: stream,
            })),
          };
        }),
      );
    }

    function selectField(field: any) {
      internalModel.value = {
        streamAlias: field?.stream?.streamAlias,
        field: field.name,
      };
    }

    // Watch for v-model changes
    watch(internalModel, (newValue) => {
      emit("update:modelValue", newValue);
    });

    // Watch for streams changes
    watch(() => props.streams, fetchFieldsForStreams, { immediate: true });

    return {
      internalModel,
      options,
      selectField,
    };
  },
});
</script>
