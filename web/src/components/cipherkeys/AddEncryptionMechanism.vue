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
  <div class="cipher-keys-add-encryption-mechanism">
    <q-select
      data-test="add-cipher-key-auth-method-input"
      v-model="frmData.key.mechanism.type"
      :label="t('cipherKey.providerType') + ' *'"
      color="input-border q-w-lg"
      bg-color="input-bg"
      class="showLabelOnTop full-width"
      stack-label
      outlined
      filled
      dense
      :options="providerTypeOptions"
      option-value="value"
      option-label="label"
      map-options
      emit-value
      :rules="[(val: any) => !!val || 'Provider type is required']"
      tabindex="0"
    />

    <q-select
      v-if="frmData.key.mechanism.type === 'simple'"
      data-test="add-cipher-algorithm-input"
      v-model="frmData.key.mechanism.simple_algorithm"
      :label="t('cipherKey.algorithm') + ' *'"
      color="input-border q-w-lg"
      bg-color="input-bg"
      class="showLabelOnTop full-width"
      stack-label
      outlined
      filled
      dense
      :options="plainAlgorithmOptions"
      option-value="value"
      option-label="label"
      map-options
      emit-value
      :rules="[(val: any) => !!val || 'Algorithm is required']"
      tabindex="1"
    />

  </div>
</template>

<script lang="ts">
import { ref, defineComponent } from "vue";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "PageAddEncryptionMechanism",
  props: {
    formData: Object,
  },
  setup(props: any) {
    const { t } = useI18n();
    const frmData = ref(props.formData || {});

    const providerTypeOptions = ref([
      { value: "simple", label: "Simple" },
      { value: "tink_keyset", label: "Tink KeySet" },
    ]);

    const plainAlgorithmOptions = ref([
      { value: "aes-256-siv", label: "AES 256 SIV" },
    ]);

    return {
      t,
      frmData,
      providerTypeOptions,
      plainAlgorithmOptions,
    };
  },
});
</script>

<style lang="scss">
.cipher-keys-add-encryption-mechanism {
  .q-field--labeled.showLabelOnTop .q-field__bottom {
    padding: 0px;
  }  
}
</style>
