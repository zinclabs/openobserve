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
  <div class="cipher-keys-add-akeyless-type">
    <!-- Add input filed for base URL, access id with URL validation from q-input-->
    <q-input
      data-test="add-cipher-key-akeyless-baseurl-input"
      v-model="formData.key.store.akeyless.base_url"
      :label="t('cipherKey.baseURL') + ' *'"
      color="input-border"
      bg-color="input-bg"
      class="showLabelOnTop q-w-lg q-pb-xs"
      stack-label
      outlined
      filled
      dense
      :rules="[
        (val: any) => !!val || 'Base URL is required',
        (val: any) => validateUrl(val) || 'Please provide correct URL.',
      ]"
    />
    <!-- Add input filed for access id with URL validation from q-input -->
    <q-input
      data-test="add-cipher-key-akeyless-access-id-input"
      v-model="formData.key.store.akeyless.access_id"
      :label="t('cipherKey.accessId') + ' *'"
      color="input-border"
      bg-color="input-bg"
      class="showLabelOnTop q-w-lg"
      stack-label
      outlined
      filled
      dense
      :rules="[
        (val: any) => !!val || 'Access ID is required',
        (val: string) =>
          /^[a-zA-Z0-9-]*$/.test(val) || 'Access ID should be alphanumeric',
      ]"
    />
    <q-select
      data-test="add-cipher-key-auth-method-input"
      v-model="formData.key.store.akeyless.auth.type"
      :label="t('cipherKey.authenticationType') + ' *'"
      color="input-border q-w-lg"
      bg-color="input-bg"
      class="showLabelOnTop full-width"
      stack-label
      outlined
      filled
      dense
      :options="authenticationTypeOptions"
      option-value="value"
      option-label="label"
      map-options
      emit-value
      :rules="[(val: any) => !!val || 'Authentication type is required']"
      tabindex="0"
    />
    <fieldset
      class="q-fieldset q-pa-md q-w-lg"
      v-if="formData.key.store.akeyless.auth.type != ''"
    >
      <legend class="q-caption bg-white q-px-sm">
        {{ getAuthenticationTypeLabel(formData.key.store.akeyless.auth.type) }}
        Configuration
      </legend>
      <div v-if="formData.key.store.akeyless.auth.type === 'access_key'">
        <q-input
          data-test="add-cipher-key-akeyless-access-key-input"
          v-model="formData.key.store.akeyless.auth.access_key"
          :label="t('cipherKey.accessKey') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-w-lg"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val || 'Access Key is required']"
        />
      </div>
      <div v-if="formData.key.store.akeyless.auth.type === 'ldap'">
        <q-input
          data-test="add-cipher-key-akeyless-ldap-username-input"
          v-model="formData.key.store.akeyless.auth.ldap.username"
          :label="t('cipherKey.ldapUsername') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-w-lg"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val || 'LDAP Username is required']"
        />
        <q-input
          data-test="add-cipher-key-akeyless-ldap-password-input"
          v-model="formData.key.store.akeyless.auth.ldap.password"
          :label="t('cipherKey.ldapPassword') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-w-lg"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val || 'LDAP Password is required']"
        />
      </div>
    </fieldset>
    <q-select
      data-test="add-cipher-key-secret-type-input"
      v-model="formData.key.store.akeyless.store.type"
      :label="t('cipherKey.secretType') + ' *'"
      color="input-border q-w-lg"
      bg-color="input-bg"
      class="showLabelOnTop full-width"
      stack-label
      outlined
      filled
      dense
      :options="secretTypeOptions"
      option-value="value"
      option-label="label"
      map-options
      emit-value
      :rules="[(val: any) => !!val || 'Secret type is required']"
      tabindex="0"
    />
    <fieldset
      class="q-fieldset q-pa-md q-w-lg"
      v-if="formData.key.store.akeyless.store.type != ''"
    >
      <legend class="q-caption bg-white q-px-sm">
        {{ getSecretOptionLabel(formData.key.store.akeyless.store.type) }}
        Configuration
      </legend>
      <div v-if="formData.key.store.akeyless.store.type === 'static_secret'">
        <q-input
          data-test="add-cipher-key-akeyless-static-secret-name-input"
          v-model="formData.key.store.akeyless.store.static_secret"
          :label="t('cipherKey.staticSecretName') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-w-lg"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val || 'Static Secret Name is required']"
        />
      </div>
      <div v-if="formData.key.store.akeyless.store.type === 'dfc'">
        <q-input
          data-test="add-cipher-key-akeyless-dfc-name-input"
          v-model="formData.key.store.akeyless.store.dfc.name"
          :label="t('cipherKey.dfcName') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-w-lg"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val || 'DFC Name is required']"
        />
        <q-input
          data-test="add-cipher-key-akeyless-dfc-iv-input"
          v-model="formData.key.store.akeyless.store.dfc.iv"
          :label="t('cipherKey.dfcIV') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-w-lg"
          stack-label
          outlined
          filled
          dense
          :rules="[(val: any) => !!val || 'DFC IV is required']"
        />
        <q-input
          data-test="add-cipher-key-akeyless-dfc-encrypted-data-input"
          v-model="formData.key.store.akeyless.store.dfc.encrypted_data"
          :label="t('cipherKey.dfcEncryptedData') + ' *'"
          color="input-border"
          bg-color="input-bg"
          class="showLabelOnTop q-w-lg"
          stack-label
          type="textarea"
          outlined
          filled
          dense
          :rules="[(val: any) => !!val || 'DFC Encrypted Data is required']"
        />
      </div>
    </fieldset>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import { validateUrl } from "@/utils/zincutils";

export interface AkeylessStore {
  store: {
    akeyless: {
      access_id: string;
      base_url: string;
      auth: {
        type: string;
        access_key: string;
        ldap: {
          username: string;
          password: string;
        };
      };
      store: {
        type: string;
        static_secret: string;
        dfc: {
          name: string;
          iv: string;
          encrypted_data: string;
        };
      };
    };
  };
}

export interface FormData {
  name: string;
  key: AkeylessStore;
  provider: Object;
}
export default defineComponent({
  name: "PageAddAkeylessType",
  props: {
    formData: {
      type: Object,
      required: true,
      default: () => ({
        key: {
          store: {
            type: "local",
            akeyless: {
              base_url: "",
              access_id: "",
              auth: {
                type: "access_key",
                access_key: "",
                ldap: {
                  username: "",
                  password: "",
                },
              },
              store: {
                type: "static_secret",
                static_secret: "",
                dfc: {
                  name: "",
                  iv: "",
                  encrypted_data: "",
                },
              },
            },
            local: "",
          },
          mechanism: {
            type: "simple",
            simple_algorithm: "aes-256-siv",
          },
        },
      }),
    },
  },
  setup(props: any, { emit }) {
    const { t } = useI18n();

    const authenticationTypeOptions = ref([
      { label: "Access Key", value: "access_key" },
      { label: "LDAP", value: "ldap" },
    ]);
    const secretTypeOptions = ref([
      { label: "Static Secret", value: "static_secret" },
      { label: "DFC", value: "dfc" },
    ]);

    const getSecretOptionLabel = (value: string) => {
      const option = secretTypeOptions.value.find(
        (option) => option.value === value,
      );
      return option ? option.label : "";
    };

    const getAuthenticationTypeLabel = (value: string) => {
      const option = authenticationTypeOptions.value.find(
        (option) => option.value === value,
      );
      return option ? option.label : "";
    };

    return {
      t,
      authenticationTypeOptions,
      secretTypeOptions,
      getSecretOptionLabel,
      getAuthenticationTypeLabel,
      validateUrl,
    };
  },
});
</script>

<style lang="scss">
.cipher-keys-add-akeyless-type {
  .q-field--labeled.showLabelOnTop .q-field__bottom {
    padding: 0px;
  }

  .q-fieldset {
    border: 1px solid lightgray;
    border-radius: 4px;
    position: relative;
  }

  legend {
    font-size: 12px;
    color: var(--q-color-dark);
    margin-left: 8px;
    padding: 0 4px;
  }
}
</style>
