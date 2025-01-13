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
  <div class="q-mx-md q-my-md">
    <div class="row items-center no-wrap">
      <div class="col">
        <div v-if="isUpdating" class="text-h6">
          {{ t("function.updateEnrichmentTable") }}
        </div>
        <div v-else class="text-h6">{{ t("function.addEnrichmentTable") }}</div>
      </div>
    </div>

    <q-separator />
    <div>
      <q-form ref="addJSTransformForm" @submit="onSubmit">
        <div class="row">
          <q-input
            v-model="formData.name"
            :label="t('function.name')"
            color="input-border"
            bg-color="input-bg"
            class="col-12 q-py-md showLabelOnTop text-grey-8 text-bold"
            stack-label
            outlined
            filled
            dense
            v-bind:readonly="isUpdating"
            v-bind:disable="isUpdating"
            :rules="[(val: any) => !!val || 'Field is required!']"
            tabindex="0"
          />
          <!-- Add the toggle button to switch between File Upload and URL Input -->
          <div>
            <q-toggle
            v-model="useUrlInput"
            :label="t('function.useUrlInput')"
            left-label
          />
          </div>
          <q-file
            v-if="!useUrlInput"
            color="lime-11"
            filled
            v-model="formData.file"
            :label="t('function.uploadCSVFile')"
            bg-color="input-bg"
            class="col-12 q-py-md showLabelOnTop lookup-table-file-uploader"
            stack-label
            outlined
            dense
            :rules="[(val: any) => !!val || 'CSV File is required!']"
            hint="Note: Only CSV files are allowed. Maximum file size: 10 MB." 
            >
            <template v-slot:prepend>
              <q-icon name="attachment" />
            </template>
          </q-file>
          <div v-if="isUpdating && !useUrlInput">
            <q-toggle
              class="col-12 q-py-md text-grey-8 text-bold"
              v-model="formData.append"
              :label="t('function.appendData')"
            />
          </div>
          <q-input
            v-if="useUrlInput"
            v-model="formData.file_link"
            :label="t('function.fileLink')"
            color="input-border"
            bg-color="input-bg"
            class="col-12 q-py-md showLabelOnTop text-grey-8 text-bold"
            stack-label
            filled
            outlined
            dense
            :rules="[(val: any) => !!val || 'URL is required!']"
          />
        </div>

        <pre class="q-py-md showLabelOnTop text-bold text-h7">{{
          compilationErr
        }}</pre>

        <div class="flex q-mt-md">
          <q-btn
            v-close-popup="true"
            class="q-mb-md text-bold"
            :label="t('function.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            @click="$emit('cancel:hideform')"
          />
          <q-btn
            :label="t('function.save')"
            class="q-mb-md text-bold no-border q-ml-md"
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

<script lang="ts">
import { defineComponent, ref, computed } from "vue";
import jsTransformService from "../../services/jstransform";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import segment from "../../services/segment_analytics";

const defaultValue: any = () => {
  return {
    name: "",
    file: "",
    append: false,
  };
};

export default defineComponent({
  name: "AddEnrichmentTable",
  props: {
    modelValue: {
      type: Object,
      default: () => defaultValue(),
    },
    isUpdating: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["update:list", "cancel:hideform"],
  setup(props, { emit }) {
    const store: any = useStore();
    const addJSTransformForm: any = ref(null);
    const disableColor: any = ref("");
    const formData: any = ref(defaultValue());
    const indexOptions = ref([]);
    const { t } = useI18n();
    const q = useQuasar();
    const editorRef: any = ref(null);
    let editorobj: any = null;
    const isFetchingStreams = ref(false);
    const useUrlInput = ref(false); 
    let compilationErr = ref("");

    const editorUpdate = (e: any) => {
      formData.value.function = e.target.value;
    };
    // Helper to handle success
    const handleSuccess = (res, dismiss) => {
      formData.value = defaultValue(); // Clear form data
      emit("update:list"); // Emit event to update the list
      dismiss(); // Dismiss the loading notification
      q.notify({
        type: "positive",
        message: res.data.message,
      });
    };

    // Helper to handle errors
    const handleError = (err, dismiss) => {
      compilationErr.value = err.response?.data?.message || "Unknown error";
      if (err.response?.status !== 403) {
        q.notify({
          type: "negative",
          message:
            JSON.stringify(err.response?.data?.error) ||
            "Enrichment Table creation failed",
        });
      }
      dismiss(); // Dismiss the loading notification
    };
    const onSubmit = () => {
      const dismiss = q.notify({
        spinner: true,
        message: "Please wait...",
        timeout: 2000,
      });

      const organizationId = store.state.selectedOrganization.identifier;

      if (useUrlInput.value) {
        // Handle URL input submission
        jsTransformService
          .create_enrichment_table(
            organizationId,
            formData.value.name,
            formData.value.file_link,
            formData.value.append,
            true
          )
          .then((res) => handleSuccess(res, dismiss))
          .catch((err) => handleError(err, dismiss));
      } else {
        // Handle file upload submission
        const reqFormData = new FormData();
        reqFormData.append("file", formData.value.file);

        jsTransformService
          .create_enrichment_table(
            organizationId,
            formData.value.name,
            reqFormData,
            formData.value.append,
            false
          )
          .then((res) => handleSuccess(res, dismiss))
          .catch((err) => handleError(err, dismiss));
      }
  };

    return {
      t,
      q,
      disableColor,
      formData,
      addJSTransformForm,
      store,
      compilationErr,
      indexOptions,
      editorRef,
      editorobj,
      editorUpdate,
      isFetchingStreams,
      useUrlInput,
      onSubmit,
    };
  },
  created() {
    this.formData = { ...defaultValue(), ...this.modelValue };

    if (this.isUpdating) {
      this.disableColor = "grey-5";
      this.formData = this.modelValue;
      if (this.formData.append == undefined) this.formData.append = false;
    }
  },
});
</script>

<style scoped>
#editor {
  width: 100%;
  min-height: 15rem;
  padding-bottom: 14px;
  resize: both;
}
</style>
<style lang="scss">
.no-case .q-field__native span {
  text-transform: none !important;
}

.lookup-table-file-uploader {
  .q-field__label {
    left: -30px;
  }
}
</style>
