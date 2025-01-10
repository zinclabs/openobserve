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
  <template v-for="(span, index) in spans as any[]" :key="span.spanId">
    <div
      :style="{
        position: 'relative',
        width: '100%',
        overflow: 'visible',
        flexWrap: 'nowrap',
      }"
      class="flex"
    >
      <div
         class="border-right" :style="{ width: leftWidth + 'px' }"  >
        <div
          :style="{
            height: '100%',
            margin: `0 0px 0 ${
              span.hasChildSpans
                ? span.style.left
                : parseInt(span.style.left) +
                  spanDimensions.collapseWidth +
                  'px'
            }`,
          }"
          class="flex items-start justify-start ellipsis"
          :title="span.operationName"
        >
          <div
            class="flex no-wrap q-pt-sm full-width relative-position operation-name-container"
            :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
            :style="{ height: '30px' }"
            @mouseover="() => (spanHoveredIndex = index)"
            @mouseout="() => (spanHoveredIndex = -1)"
          >
            <div
              class="absolute view-logs-container"
              :class="spanHoveredIndex === index ? 'show' : ''"
            >
              <q-btn
                class="q-mx-xs view-span-logs"
                :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
                size="10px"
                icon="search"
                dense
                no-caps
                :title="t('traces.viewLogs')"
                @click.stop="viewSpanLogs(span)"
              >
                <!-- <span class="text view-logs-btn-text">View Logs</span> -->
              </q-btn>
            </div>
            <div
              v-if="span.hasChildSpans"
              :style="{
                width: spanDimensions.collapseWidth + 'px',
                height: spanDimensions.collapseHeight + 'px',
              }"
              class="q-pt-xs flex justify-center items-center collapse-container cursor-pointer"
              @click.stop="toggleSpanCollapse(span.spanId)"
            >
              <q-icon
                dense
                round
                flat
                name="expand_more"
                class="collapse-btn"
                :style="{
                  rotate: collapseMapping[span.spanId] ? '0deg' : '270deg',
                }"
              />
            </div>
            <div
              class="ellipsis q-pl-xs cursor-pointer"
              :style="{
                paddingLeft: '4px',
                borderLeft: `3px solid ${span.style.color}`,
              }"
              @click="selectSpan(span.spanId)"
            >
              <q-icon
                v-if="span.spanStatus === 'ERROR'"
                name="error"
                class="text-red-6 q-mr-xs"
                title="Error Span"
              />
              <span class="text-subtitle2 text-bold q-mr-sm"  
                  :class="{
                  highlighted: isHighlighted(index),
                  'current-match': currentSelectedValue === index, // Current match class
              }">
                {{ span.serviceName }}
              </span>
              <span
                class="text-body2"
                :class="
                  store.state.theme === 'dark'
                    ? 'text-grey-5'
                    : 'text-blue-grey-9'
                "
                >{{ span.operationName }}</span
              >
            </div>
          </div>
          <div
            :style="{
              backgroundColor: span.style.backgroundColor,
              height: `calc(100% - 30px)`,
              borderLeft: `3px solid ${span.style.color}`,
              marginLeft: span.hasChildSpans ? '14px' : '0',
              width: '100%',
            }"
          ></div>
        </div>
      </div>
      <span-block
        :span="span"
        :depth="depth"
        :baseTracePosition="baseTracePosition"
        :styleObj="{
          position: 'absolute',
          top: span.style.top,
          left: span.style.left,
          height: '60px',
        }"
        :style="{
          width: `calc(100% - ${leftWidth}px)`,
        }"
        :spanDimensions="spanDimensions"
        :isCollapsed="collapseMapping[span.spanId]"
        :spanData="spanMap[span.spanId]"
        @toggle-collapse="toggleSpanCollapse"
        @select-span="selectSpan"
        @mouseover="() => (spanHoveredIndex = index)"
        @mouseout="() => (spanHoveredIndex = -1)"
        @view-logs="viewSpanLogs(span)"
      />
    </div>
  </template>
</template>

<script lang="ts">
import { defineComponent, onBeforeMount, nextTick, ref,watch ,defineExpose, computed} from "vue";
import { getImageURL } from "@/utils/zincutils";
import useTraces from "@/composables/useTraces";
import { useStore } from "vuex";
import SpanBlock from "./SpanBlock.vue";
import type { Ref } from "vue";
import { useI18n } from "vue-i18n";
import { b64EncodeStandard } from "@/utils/zincutils";
import { useRouter } from "vue-router";

export default defineComponent({
  name: "TraceTree",
  props: {
    spans: {
      type: Array,
      default: () => [],
    },
    isCollapsed: {
      type: Boolean,
      default: false,
    },
    collapseMapping: {
      type: Object,
      default: () => {},
    },
    baseTracePosition: {
      type: Object,
      default: () => null,
    },
    depth: {
      type: Number,
      default: 0,
    },
    spanDimensions: {
      type: Object,
      default: () => {},
    },
    spanMap: {
      type: Object,
      default: () => ({}),
    },
    leftWidth: {
      type: Number,
      default: 0,
    },
    searchQuery: {
      type: String,
      default: '',
    },
    spanList: {
      type: Array,
      default: () => [],
    },
  },
  emits: ["toggleCollapse", "selectSpan","update-current-index","search-result"],
  setup(props, { emit }) {
    const { searchObj, buildQueryDetails, navigateToLogs } = useTraces();
    const store = useStore();

    const { t } = useI18n();

    const spanHoveredIndex = ref(-1);

    const router = useRouter();

    function toggleSpanCollapse(spanId: number | string) {
      emit("toggleCollapse", spanId);
    }
    const selectSpan = (spanId: string) => {
      emit("selectSpan", spanId);
    };

    // Main function to view span logs
    const viewSpanLogs = (span: any) => {
      const queryDetails = buildQueryDetails(span);
      navigateToLogs(queryDetails);
    };
    const searchResults = ref<any[]>([]);
    const currentIndex = ref<number | null>(null);
    const currentSelectedValue = computed(() => {
      if (currentIndex.value === -1 || searchResults.value.length === 0) {
        return null;
      }
      return searchResults.value[currentIndex.value ?? 0];
    });

    const findMatches = (spanList:any, searchQuery:any) => {
      const query = searchQuery.toLowerCase().trim();
      return spanList
        .map((span:any, index:any) => {
          // Check if any span value matches the query
          const matches = Object.values(span).some((value) => {
            if (typeof value === "string" || typeof value === "number") {
              return String(value).toLowerCase().includes(query);
            }
            return false; // Skip non-string/non-number values
          });
          // Return the index if a match is found, otherwise return -1
          return matches ? index : -1;
        })
        .filter((index:any) => index !== -1);
    };
    const updateSearch = () => {
      if (props.searchQuery?.trim()) {
        searchResults.value = findMatches(props.spanList, props.searchQuery);
        currentIndex.value = 0; // Reset to first match
        nextTick(() => {
          scrollToMatch(); // Wait for DOM updates before scrolling
        });
      } else {
        searchResults.value = [];
        currentIndex.value = null;
      }
    };

    const isHighlighted = (path: any) => {
      // If the path is an array, join it and compare with resultPath joined
      if (Array.isArray(path)) {
        return searchResults.value.some((resultPath: any) =>
          resultPath.join(',') === path.join(',')
        );
      }

      // If path is a single value (index), compare it directly
      return searchResults.value.includes(path);
    };

    const scrollToMatch = () => {
      if (searchResults.value.length > 0) {
        const matchElement = document.querySelector(`.current-match`);
        if (matchElement) {
          matchElement.scrollIntoView({
            behavior: 'smooth',
            block: 'center',
          });
        }
      }
    };

    const nextMatch = () => {
     if (currentIndex.value !== null && currentIndex.value < searchResults.value.length - 1) {
        currentIndex.value++;
        nextTick(() => {
          scrollToMatch(); // Wait for DOM updates before scrolling
        });
      }
    };

    const prevMatch = () => {
      if (currentIndex.value !== null && currentIndex.value > 0) {
        currentIndex.value--;
        nextTick(() => {
          scrollToMatch(); // Wait for DOM updates before scrolling
        });
      }
    };
    defineExpose({
      nextMatch,
      prevMatch
    });

    watch(() => props.searchQuery, (newValue) => {
      updateSearch();
    });
    watch(currentIndex, (newValue) => {
      emit('update-current-index', newValue);
    });
    watch(searchResults, (newValue) => {
      emit('search-result', newValue.length);
    });

    return {
      toggleSpanCollapse,
      getImageURL,
      selectSpan,
      store,
      viewSpanLogs,
      t,
      spanHoveredIndex,
      searchResults,
      currentIndex,
      updateSearch,
      nextMatch,
      prevMatch,
      isHighlighted,
      currentSelectedValue
    };
  },
  components: { SpanBlock },
});
</script>

<style scoped lang="scss">
.view-logs-container {
  top: 7px;
  right: 0;
}
.spans-container {
  position: relative;
}

.collapse-btn {
  width: 14px;
  height: auto;
  opacity: 0.6;
}

.operation-name-container {
  .view-logs-container {
    visibility: hidden;
  }
  .view-logs-container {
    &.show {
      visibility: visible !important;
    }
  }
}
</style>
<style lang="scss">
.view-logs-btn-text {
  visibility: visible;
}
.view-span-logs {
  background-color: inherit;
  .view-logs-btn-text {
    visibility: hidden;
    width: 0px;
    transition: width 0.3s ease-in;
  }
  &:hover .view-logs-btn-text {
    visibility: visible;
    width: auto;
  }
}
.border-right{
  border-right: 1px solid rgb(236, 236, 236);
}
.highlighted {
  background-color: yellow;
  font-weight: bold;
}
.current-match {
  background-color: yellow;
  color: red;
  font-weight: bold;
}
</style>
