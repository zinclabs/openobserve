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
  <div>
    <q-input
      data-test="add-cipher-key-openobserve-secret-input"
      v-model="formData.key.store.local"
      :label="t('cipherKey.secret') + ' *'"
      color="input-border"
      bg-color="input-bg"
      class="showLabelOnTop q-w-lg q-pb-xs"
      type="textarea"
      stack-label
      outlined
      filled
      dense
      :rules="[(val: any) => !!val || 'Secret is required']"
    />
  </div>
</template>

<script lang="ts">
import { ref, defineComponent } from "vue";
import { useI18n } from "vue-i18n";

export default defineComponent({
  name: "AddOpenobserveType",
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
  setup(props) {
    const { t } = useI18n();
    return {
      t,
    };
  },
});
</script>
