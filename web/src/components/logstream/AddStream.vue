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
    class="q-pt-md"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <div class="add-stream-header row items-center no-wrap q-px-md">
      <div class="col">
        <div class="text-body1 text-bold" data-test="add-stream-title">
          {{ t("logStream.add") }}
        </div>
      </div>
      <div class="col-auto">
        <q-btn
          data-test="add-stream-close-btn"
          v-close-popup="true"
          round
          flat
          icon="close"
        />
      </div>
    </div>
    <q-separator class="q-my-md" />
    <div class="q-px-md o2-input add-stream-inputs">
      <q-form @submit="saveStream">
        <div data-test="add-stream-name-input">
          <q-input
            v-model="streamInputs.name"
            :label="t('common.name') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            :rules="[(val: any) => !!val.trim() || 'Field is required!']"
            tabindex="0"
            style="min-width: 480px"
          />
        </div>

        <div data-test="add-stream-type-input">
          <q-select
            v-model="streamInputs.stream_type"
            :options="filteredStreamTypes"
            :label="t('alerts.streamType') + ' *'"
            :popup-content-style="{ textTransform: 'capitalize' }"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop no-case"
            map-options
            stack-label
            emit-value
            outlined
            filled
            dense
            :rules="[(val: any) => !!val || 'Field is required!']"
            style="min-width: 220px"
          />
        </div>

        <div data-test="add-stream-data-retention-input">
          <q-input
            v-model="streamInputs.dataRetentionDays"
            :label="t('logStream.dataRetention') + ' *'"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            type="number"
            :rules="[(val: any) => val > 0 || 'Field is required!']"
            style="min-width: 480px"
          />
        </div>

        <StreamFieldInputs
          :fields="fields"
          @add="addField"
          @remove="removeField"
        />

        <div class="flex justify-end q-mt-sm">
          <q-btn
            v-close-popup="true"
            data-test="add-stream-cancel-btn"
            :label="t('logStream.cancel')"
            class="q-my-sm text-bold q-mr-md"
            text-color="light-text"
            padding="sm md"
            no-caps
          />
          <q-btn
            data-test="save-stream-btn"
            :label="t('logStream.createStream')"
            class="q-my-sm text-bold no-border"
            color="secondary"
            padding="sm xl"
            type="submit"
            no-caps
          />
        </div>
      </q-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import StreamFieldInputs from "./StreamFieldInputs.vue";
import type { Ref } from "vue";
import streamService from "@/services/stream";
import { useStore } from "vuex";
import { computed } from "vue";
import { useQuasar } from "quasar";
import useStreams from "@/composables/useStreams";

const { t } = useI18n();

const streamTypes = [
  { label: "Logs", value: "logs" },
  { label: "Metrics", value: "metrics" },
  { label: "Traces", value: "traces" },
];

const emits = defineEmits(["streamAdded", "close","added:stream-added"]);
const props = defineProps<{
  isInPipeline: boolean;
}>();


const { addStream, getStream, getUpdatedSettings } = useStreams();

const fields: Ref<any[]> = ref([]);

const store = useStore();

const q = useQuasar();

const streamInputs = ref({
  name: "",
  stream_type: "",
  index_type: [],
  dataRetentionDays: 14,
});

const getDefaultField = () => {
  return {
    name: "",
    type: "",
    index_type: [],
  };
};


const isSchemaEvolutionEnabled = computed(() => {
  return store.state.zoConfig.user_defined_schemas_enabled;
});

const filteredStreamTypes = computed(() => {
  //here we can filter out based on isInPipeline prop
  //but for testing purpose we are returning all streamTypes
  return streamTypes;
});


const showDataRetention = computed(
  () =>
    !!(store.state.zoConfig.data_retention_days || false) &&
    streamInputs.value.stream_type !== "enrichment_tables"
);

const saveStream = async () => {
  let isStreamPresent = false;

  await getStream(
    streamInputs.value.name,
    streamInputs.value.stream_type,
    false
  )
    .then(() => {
      q.notify({
        color: "negative",
        message: `Stream "${streamInputs.value.name}" of type "${streamInputs.value.stream_type}" is already present.`,
        timeout: 4000,
      });
      isStreamPresent = true;
    })
    .catch(() => {});

  if (isStreamPresent) return;

  const payload = getStreamPayload();
  streamService
    .createSettings(
      store.state.selectedOrganization.identifier,
      streamInputs.value.name,
      streamInputs.value.stream_type,
      payload
    )
    .then(() => {
      q.notify({
        color: "positive",
        message: "Stream created successfully",
        timeout: 4000,
      });

      streamService
        .schema(
          store.state.selectedOrganization.identifier,
          streamInputs.value.name,
          streamInputs.value.stream_type
        )
        .then((streamRes: any) => {
          addStream(streamRes.data);
          emits("streamAdded");
          emits("close");
          emits("added:stream-added", streamInputs.value);
        });
    })
    .catch((err) => {
      if(err.response.status != 403){
        q.notify({
        color: "negative",
        message: err.response?.data?.message || "Failed to create stream",
        timeout: 4000,
      });
      }
    });

};

const getStreamPayload = () => {
  let settings: {
    partition_keys: any[];
    index_fields: any[];
    full_text_search_keys: any[];
    bloom_filter_fields: any[];
    data_retention?: number;
    defined_schema_fields: any[];
  } = {
    partition_keys: [],
    index_fields: [],
    full_text_search_keys: [],
    bloom_filter_fields: [],
    defined_schema_fields: [],
  };

  if (showDataRetention.value && streamInputs.value.dataRetentionDays < 1) {
    q.notify({
      color: "negative",
      message:
        "Invalid Data Retention Period: Retention period must be at least 1 day.",
      timeout: 4000,
    });
    return;
  }

  if (showDataRetention.value) {
    settings["data_retention"] = Number(streamInputs.value.dataRetentionDays);
  }

  fields.value.forEach((field) => {
    field.name = field.name
      .trim()
      .toLowerCase()
      .replace(/ /g, "_")
      .replace(/-/g, "_");

    field.index_type?.forEach((index: string) => {
      if (index === "fullTextSearchKey") {
        settings.full_text_search_keys.push(field.name);
      }

      if (index === "secondaryIndexKey") {
        settings.index_fields.push(field.name);
      }

      if (index === "keyPartition") {
        settings.partition_keys.push({
          field: field.name,
          types: "value",
        });
      }
      if (index === "prefixPartition") {
        settings.partition_keys.push({
          field: field.name,
          types: "prefix",
        });
      }

      if (index.includes("hashPartition")) {
        const [, buckets] = index.split("_");
        settings.partition_keys.push({
          field: field.name,
          types: {
            hash: Number(buckets),
          },
        });
      }

      if (index === "bloomFilterKey") {
        settings.bloom_filter_fields.push(field.name);
      }
    });

    if (isSchemaEvolutionEnabled.value)
      settings.defined_schema_fields.push(field.name);
  });

  return settings;
};

const addField = () => {
  fields.value.push(getDefaultField());
};

const removeField = (field: any, index: number) => {
  fields.value.splice(index, 1);
};
</script>

<style lang="scss">
.add-stream-inputs {
  .q-field__label {
    font-weight: normal !important;
    font-size: 13px;
    transform: translate(-0.75rem, -155%);
    color: #3a3a3a;
    top: 12px !important;
  }
}
</style>
