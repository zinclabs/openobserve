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
  <div class="column index-menu">
    <div class="index-table q-mt-xs">
      <q-table
        data-test="log-search-index-list-fields-table"
        :visible-columns="['name']"
        :rows="fields"
        row-key="name"
        :filter="filterFieldValue"
        :filter-method="filterFieldFn"
        :pagination="{ rowsPerPage: 10000 }"
        hide-header
        hide-bottom
        class="traces-field-table"
        id="tracesFieldList"
      >
        <template #body-cell-name="props">
          <q-tr :props="props">
            <q-td :props="props" class="field_list">
              <!-- TODO OK : Repeated code make separate component to display field  -->
              <div
                v-if="props.row.ftsKey || !props.row.showValues"
                class="field-container flex content-center ellipsis q-pl-lg q-pr-sm"
                :title="props.row.name"
              >
                <div class="field_label ellipsis">
                  {{ props.row.name }}
                </div>
                <div
                  class="field_overlay"
                  :style="{
                    background:
                      store.state.theme === 'dark' ? '#414345' : '#d9d9d9',
                  }"
                >
                  <q-btn
                    :icon="outlinedAdd"
                    :data-test="`log-search-index-list-filter-${props.row.name}-field-btn`"
                    style="margin-right: 0.375rem"
                    size="0.4rem"
                    class="q-mr-sm"
                    @click.stop="addSearchTerm(`${props.row.name}=''`)"
                    round
                  />
                </div>
              </div>
              <q-expansion-item
                v-else
                dense
                switch-toggle-side
                :label="props.row.name"
                expand-icon-class="field-expansion-icon"
                expand-icon="
                     expand_more
                  "
                @before-show="(event: any) => openFilterCreator(event, props.row)"
              >
                <template v-slot:header>
                  <div
                    class="flex content-center ellipsis"
                    :title="props.row.name"
                  >
                    <div class="field_label ellipsis">
                      {{ props.row.name }}
                    </div>
                    <div class="field_overlay">
                      <q-btn
                        :data-test="`log-search-index-list-filter-${props.row.name}-field-btn`"
                        :icon="outlinedAdd"
                        style="margin-right: 0.375rem"
                        size="0.4rem"
                        class="q-mr-sm"
                        @click.stop="addSearchTerm(`${props.row.name}=''`)"
                        round
                      />
                    </div>
                  </div>
                </template>
                <q-card>
                  <q-card-section class="q-pl-md q-pr-xs q-py-xs">
                    <div class="filter-values-container">
                      <div
                        v-show="fieldValues[props.row.name]?.isLoading"
                        class="q-pl-md q-py-xs"
                        style="height: 60px"
                      >
                        <q-inner-loading
                          size="xs"
                          :showing="fieldValues[props.row.name]?.isLoading"
                          label="Fetching values..."
                          label-style="font-size: 1.1em"
                        />
                      </div>
                      <div
                        v-show="
                          !fieldValues[props.row.name]?.values?.length &&
                          !fieldValues[props.row.name]?.isLoading
                        "
                        class="q-pl-md q-py-xs text-subtitle2"
                      >
                        No values found
                      </div>
                      <div
                        v-for="value in fieldValues[props.row.name]?.values ||
                        []"
                        :key="value.key"
                      >
                        <q-list  dense>
                          <q-item tag="label" class="q-pr-none">
                            <div
                              class="flex row wrap justify-between"
                              style="width: calc(100% - 46px)"
                            >
                              <div
                                :title="value.key"
                                class="ellipsis q-pr-xs"
                                style="width: calc(100% - 50px)"
                              >
                                {{ value.key }}
                              </div>
                              <div
                                :title="value.count"
                                class="ellipsis text-right q-pr-sm"
                                style="width: 50px"
                              >
                                {{ value.count }}
                              </div>
                            </div>
                            <div
                            v-if="!hideIncludeExlcude"
                              class="flex row"
                              :class="
                                store.state.theme === 'dark'
                                  ? 'text-white'
                                  : 'text-black'
                              "
                            >
                              <q-btn
                                class="q-mr-xs"
                                size="6px"
                                title="Include Term"
                                round
                                @click="
                                  addSearchTerm(
                                    `${props.row.name}='${value.key}'`
                                  )
                                "
                              >
                                <q-icon>
                                  <EqualIcon></EqualIcon>
                                </q-icon>
                              </q-btn>
                              <q-btn
                                class="q-mr-xs"
                                size="6px"
                                title="Include Term"
                                round
                                @click="
                                  addSearchTerm(
                                    `${props.row.name}!='${value.key}'`
                                  )
                                "
                              >
                                <q-icon>
                                  <NotEqualIcon></NotEqualIcon>
                                </q-icon>
                              </q-btn>
                            </div>
                            <div
                            v-if="!hideCopyValue"
                              class="flex row"
                              :class="
                                store.state.theme === 'dark'
                                  ? 'text-white'
                                  : 'text-black'
                              "
                            >
                              <q-btn
                                class="q-ml-sm"
                                size="8px"
                                title="Copy Value"
                                round
                                dense 
                                flat
                                @click="
                                 copyContentValue(value.key)
                                "
                              >
                                <q-icon  name="content_copy"></q-icon>
   
                              </q-btn>
                            </div>
                          </q-item>
                        </q-list>
                      </div>
                    </div>
                  </q-card-section>
                </q-card>
              </q-expansion-item>
            </q-td>
          </q-tr>
        </template>
        <template #top-right>
          <q-input
            data-test="log-search-index-list-field-search-input"
            v-model="filterFieldValue"
            data-cy="index-field-search-input"
            filled
            borderless
            dense
            clearable
            debounce="1"
            :placeholder="t('search.searchField')"
          >
            <template #prepend>
              <q-icon name="search" />
            </template>
          </q-input>
        </template>
      </q-table>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import { useQuasar } from "quasar";
import { useRouter } from "vue-router";
import { formatLargeNumber, getImageURL } from "@/utils/zincutils";
import streamService from "@/services/stream";
import { outlinedAdd } from "@quasar/extras/material-icons-outlined";
import EqualIcon from "@/components/icons/EqualIcon.vue";
import NotEqualIcon from "@/components/icons/NotEqualIcon.vue";

export default defineComponent({
  name: "IndexList",
  components: {
    EqualIcon,
    NotEqualIcon,
  },
  props: {
    fields: {
      type: Array,
      default: () => [],
    },
    streamName: {
      type: String,
      default: "",
    },
    timeStamp: {
      type: Object,
      default: () => {
        return {
          endTime: "",
          startTime: "",
        };
      },
    },
    streamType: {
      type: String,
      default: "logs",
    },
    hideIncludeExlcude:{
      type: Boolean,
      default: false,
    },
    hideCopyValue:{
      type: Boolean,
      default: true,
    }
  },
  emits: ["event-emitted"],
  setup(props, { emit }) {
    const filterFieldValue = ref("");
    const store = useStore();
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const fieldValues: Ref<{
      [key: string | number]: {
        isLoading: boolean;
        values: { key: string; count: string }[];
      };
    }> = ref({});

    const filterFieldFn = (rows: any, terms: any) => {
      var filtered = [];
      if (terms != "") {
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (rows[i]["name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
      }
      return filtered;
    };

    const openFilterCreator = (
      event: any,
      { name, ftsKey, stream_name }: any
    ) => {
      if (ftsKey) {
        event.stopPropagation();
        event.preventDefault();
        return;
      }

      fieldValues.value[name] = {
        isLoading: true,
        values: [],
      };
      streamService
        .fieldValues({
          org_identifier: store.state.selectedOrganization.identifier,
          stream_name: stream_name ? stream_name : props.streamName,
          start_time: props.timeStamp.startTime,
          end_time: props.timeStamp.endTime,
          fields: [name],
          size: 10,
          type: props.streamType,
        })
        .then((res: any) => {
          if (res.data.hits.length) {
            fieldValues.value[name]["values"] = res.data.hits
              .find((field: any) => field.field === name)
              .values.map((value: any) => {
                return {
                  key: value.zo_sql_key ? value.zo_sql_key : "null",
                  count: formatLargeNumber(value.zo_sql_num),
                };
              });
          }
        })
        .catch(() => {
          $q.notify({
            type: "negative",
            message: `Error while fetching values for ${name}`,
          });
        })
        .finally(() => {
          fieldValues.value[name]["isLoading"] = false;
        });
    };

    const addSearchTerm = (term: string) => {
      emit("event-emitted", "add-field", term);
    };

    const copyContentValue = (value: string) => {
      navigator.clipboard.writeText(value);
      $q.notify({
        type: "positive",
        message: "Value copied to clipboard",
      });
    };

    return {
      t,
      store,
      router,
      filterFieldFn,
      getImageURL,
      openFilterCreator,
      addSearchTerm,
      fieldValues,
      outlinedAdd,
      filterFieldValue,
      copyContentValue,
    };
  },
});
</script>

<style lang="scss" scoped>
.traces-field-table {
  height: calc(100vh - 190px) !important;
}
.q-menu {
  box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
  transform: translateY(0.5rem);
  border-radius: 0px;

  .q-virtual-scroll__content {
    padding: 0.5rem;
  }
}
.index-menu {
  width: 100%;

  .q-field {
    &__control {
      height: 35px;
      padding: 0px 5px;
      min-height: auto !important;

      &-container {
        padding-top: 0px !important;
      }
    }
    &__native :first-of-type {
      padding-top: 0.25rem;
    }
  }

  .index-table {
    width: 100%;
    // border: 1px solid rgba(0, 0, 0, 0.02);

    .q-table {
      display: table;
      table-layout: fixed !important;
    }
    tr {
      margin-bottom: 1px;
    }
    tbody,
    tr,
    td {
      width: 100%;
      display: block;
      height: fit-content;
      overflow: hidden;
    }

    .q-table__control,
    label.q-field {
      width: 100%;
    }
    .q-table thead tr,
    .q-table tbody td {
      height: auto;
    }

    .q-table__top {
      border-bottom: unset;
    }
  }
  .traces-field-table {
    width: 100%;
  }

  .field_list {
    padding: 0px;
    margin-bottom: 0.125rem;
    position: relative;
    overflow: visible;
    cursor: default;

    .field_label {
      pointer-events: none;
      font-size: 0.825rem;
      position: relative;
      display: inline;
      z-index: 2;
      left: 0;
      // text-transform: capitalize;
    }

    .field-container {
      height: 25px;
    }

    .field_overlay {
      position: absolute;
      height: 100%;
      right: 0;
      top: 0;
      z-index: 5;
      padding: 0 6px;
      visibility: hidden;
      display: flex;
      align-items: center;

      .q-icon {
        cursor: pointer;
        opacity: 0;
        margin: 0 1px;
      }
    }

    &.selected {
      .field_overlay {
        background-color: rgba(89, 96, 178, 0.3);

        .field_icons {
          opacity: 0;
        }
      }
    }
    &:hover {
      .field-container {
        background-color: color-mix(in srgb, currentColor 15%, transparent);
      }
    }
  }
}

.q-item {
  // color: $dark-page;
  min-height: 1.3rem;
  padding: 5px 10px;

  &__label {
    font-size: 0.75rem;
  }

  &.q-manual-focusable--focused > .q-focus-helper {
    background: none !important;
    opacity: 0.3 !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &--active {
    background-color: $selected-list-bg !important;
  }

  &.q-manual-focusable--focused > .q-focus-helper,
  &:hover,
  &--active {
    color: $primary;
  }
}
.q-field--dense .q-field__before,
.q-field--dense .q-field__prepend {
  padding: 0px 0px 0px 0px;
  height: auto;
  line-height: auto;
}
.q-field__native,
.q-field__input {
  padding: 0px 0px 0px 0px;
}

.q-field--dense .q-field__label {
  top: 5px;
}
.q-field--dense .q-field__control,
.q-field--dense .q-field__marginal {
  height: 34px;
}
</style>

<style lang="scss">
.index-table {
  .q-table {
    width: 100%;
    table-layout: fixed;

    .q-expansion-item {
      .q-item {
        display: flex;
        align-items: center;
        padding: 0;
        height: 25px !important;
        min-height: 25px !important;
      }
      .q-item__section--avatar {
        min-width: 12px;
        max-width: 12px;
        margin-right: 8px;
      }

      .filter-values-container {
        .q-item {
          padding-left: 4px;

          .q-focus-helper {
            background: none !important;
          }
        }
      }
      .q-item-type {
        &:hover {
          .field_overlay {
            visibility: visible;

            .q-icon {
              opacity: 1;
            }
          }
        }
      }
      .field-expansion-icon {
        img {
          width: 12px;
          height: 12px;
        }
      }
    }

    .field-container {
      &:hover {
        .field_overlay {
          visibility: visible;

          .q-icon {
            opacity: 1;
          }
        }
      }
    }

    .field_list {
      &.selected {
        .q-expansion-item {
          background-color: rgba(89, 96, 178, 0.3);
        }

        .field_overlay {
          // background-color: #ffffff;
        }
      }
    }
  }
}
</style>
