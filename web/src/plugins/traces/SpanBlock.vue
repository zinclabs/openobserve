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
    class="flex wrap justify-start items-center"
    :class="[
      defocusSpan ? 'defocus' : '',
      store.state.theme === 'dark' ? 'bg-dark' : 'bg-white',
    ]"
    :style="{
      zIndex: 2,
      borderBottom:
        (isSpanSelected && `2px solid ${span.style.color}`) || 'none',
    }"
    :id="span.spanId"
  >
    <div
      class="flex justify-between items-end cursor-pointer span-block relative-position"
      :class="[store.state.theme === 'dark' ? 'bg-dark' : 'bg-white']"
      :style="{
        height: spanDimensions.height + 'px',
        width: '100%',
        paddingBottom: '6px',
      }"
      ref="spanBlock"
      @click="selectSpan(span.spanId)"
      @mouseover="onSpanHover"
    >
      <div
        :style="{
          width: '100%',
          overflow: 'hidden',
        }"
        class="cursor-pointer flex items-center no-wrap position-relative"
        :class="defocusSpan ? 'defocus' : ''"
        @click="selectSpan(span.spanId)"
      >
        <div
          :style="{
            height: spanDimensions.barHeight + 'px',
            width: spanWidth + '%',
            left: leftPosition + '%',
            position: 'relative',
          }"
          class="flex justify-start items-center no-wrap"
          ref="spanMarkerRef"
        >
          <div
            :style="{
              width: 'calc(100% - 6px)',
              height: '100%',
              borderRadius: '2px',
              backgroundColor: span.style?.color || '#58508d',
            }"
          />
        </div>
        <div
          :style="{
            position: 'absolute',
            ...durationStyle,
            transition: 'all 0.5s ease',
            zIndex: 1,
          }"
          class="text-caption flex items-center"
        >
          <div>
            {{ formatTimeWithSuffix(span.durationUs) }}
          </div>
        </div>
        <q-resize-observer debounce="300" @resize="onResize" />
      </div>
    </div>
    <template v-if="isSpanSelected">
      <span-details
        :style="{
          borderTop: `2px solid ${span.style.color}`,
        }"
        :span="span"
        :spanData="spanData"
        :baseTracePosition="baseTracePosition"
        @view-logs="viewSpanLogs"
        @select-span="selectSpan"
      />
    </template>
  </div>
</template>

<script lang="ts">
import {
  defineComponent,
  computed,
  ref,
  onMounted,
  nextTick,
  watch,
  onActivated,
} from "vue";
import useTraces from "@/composables/useTraces";
import { getImageURL, formatTimeWithSuffix } from "@/utils/zincutils";
import SpanDetails from "./SpanDetails.vue";
import { useStore } from "vuex";
import { useI18n } from "vue-i18n";
import { b64EncodeStandard } from "@/utils/zincutils";
import { useRouter } from "vue-router";

export default defineComponent({
  name: "SpanBlock",
  props: {
    span: {
      type: Object,
      default: () => null,
    },
    baseTracePosition: {
      type: Object,
      default: () => null,
    },
    depth: {
      type: Number,
      default: 0,
    },
    styleObj: {
      type: Object,
      default: () => {},
    },
    showCollapse: {
      type: Boolean,
      default: true,
    },
    isCollapsed: {
      type: Boolean,
      default: false,
    },
    spanDimensions: {
      type: Object,
      default: () => {},
    },
    spanData: {
      type: Object,
      default: () => ({}),
    },
  },
  emits: ["toggleCollapse", "selectSpan", "hover", "view-logs"],
  components: { SpanDetails },
  setup(props, { emit }) {
    const store = useStore();
    const { searchObj } = useTraces();
    const spanBlock: any = ref(null);
    const spanBlockWidth = ref(0);
    const onePixelPercent = ref(0);
    const defocusSpan = computed(() => {
      if (!searchObj.data.traceDetails.selectedSpanId) return false;
      return searchObj.data.traceDetails.selectedSpanId !== props.span.spanId;
    });
    const durationStyle = ref({});
    const router = useRouter();
    const { t } = useI18n();

    const leftPosition = ref(0);

    const spanWidth = ref(0);

    const selectSpan = (spanId: string) => {
      emit("selectSpan", spanId);
    };
    const toggleSpanCollapse = () => {
      emit("toggleCollapse", props.span.spanId);
    };

    const isSpanSelected = computed(() => {
      return searchObj.data.traceDetails.expandedSpans.includes(
        props.span.spanId
      );
    });

    const spanMarkerRef = ref(null);

    const getLeftPosition = () => {
      const left =
        props.span.startTimeMs - props.baseTracePosition["startTimeMs"];

      return (left / props.baseTracePosition?.durationMs) * 100;
    };

    const getSpanWidth = () => {
      return Number(
        (
          (props.span?.durationMs / props.baseTracePosition?.durationMs) *
          100
        ).toFixed(2)
      );
    };

    onMounted(async () => {
      durationStyle.value = getDurationStyle();
      const params = router.currentRoute.value.query;
      const spanId = Array.isArray(params.span_id) ? params.span_id[0] : params.span_id; // Ensure it's a single string

      if (spanId) {
        const element = document.getElementById(spanId);
        if (element) {
          element.scrollIntoView({
            behavior: "smooth", // Smooth scrolling
            block: "center",    // Attempt to align the element at the center of the screen
            inline: "nearest"   // Keep horizontal alignment as close as possible
          });

        }
      }
    });

    watch(
      () => props.span.startTimeMs,
      () => {
        leftPosition.value = getLeftPosition();
      },
      {
        immediate: true,
      }
    );

    watch(
      () => props.baseTracePosition,
      () => {
        leftPosition.value = getLeftPosition();
      },
      {
        immediate: true,
        deep: true,
      }
    );

    watch(
      () => props.span?.durationMs + props.baseTracePosition?.durationMs,
      () => {
        spanWidth.value = getSpanWidth();
      }
    );

    watch(
      () => spanBlockWidth.value + leftPosition.value + spanWidth.value,
      (val) => {
        durationStyle.value = getDurationStyle();
      }
    );

    const getDurationStyle = () => {
      const style: any = {
        top: "10px",
      };

      const onePercent = Number((spanBlockWidth.value / 100).toFixed(2));
      const labelWidth = 60;
      if (
        (leftPosition.value + spanWidth.value) * onePercent + labelWidth >
        spanBlockWidth.value
      ) {
        style.right = 0;
        style.top = "-5px";
      } else if (leftPosition.value > 50) {
        style.left = leftPosition.value * onePercent - labelWidth + "px";
      } else {
        const left =
          leftPosition.value +
          (Math.floor(spanWidth.value) ? spanWidth.value : 1);

        style.left =
          (left * onePercent - leftPosition.value * onePercent < 19
            ? leftPosition.value * onePercent + 19
            : left * onePercent) + "px";
      }

      return style;
    };

    const getSpanStartTime = computed(() => {
      return props.span.startTimeMs - props.baseTracePosition["startTimeMs"];
    });

    const onResize = () => {
      spanBlockWidth.value = spanBlock.value.clientWidth;
    };

    const toggleSpanDetails = () => {
      if (!isSpanSelected.value) {
        searchObj.data.traceDetails.expandedSpans.push(props.span.spanId);
      } else {
        searchObj.data.traceDetails.expandedSpans =
          searchObj.data.traceDetails.expandedSpans.filter(
            (val) => props.span.spanId !== val
          );
      }
    };

    const viewSpanLogs = () => {
      emit("view-logs");
    };

    const onSpanHover = () => {
      emit("hover");
    };

    return {
      t,
      formatTimeWithSuffix,
      selectSpan,
      toggleSpanCollapse,
      getImageURL,
      leftPosition,
      spanWidth,
      getDurationStyle,
      spanBlock,
      onResize,
      onePixelPercent,
      getSpanStartTime,
      spanMarkerRef,
      toggleSpanDetails,
      defocusSpan,
      isSpanSelected,
      store,
      viewSpanLogs,
      onSpanHover,
      durationStyle,
    };
  },
});
</script>

<style scoped lang="scss">
.defocus {
  opacity: 0.3;
}

.collapse-btn {
  opacity: 0.6;
}

.collapse-container {
  border: 1px solid #ececec;
}

.light-grey {
  background-color: #ececec;
}

.view-span-logs {
  visibility: hidden;
}

.span-block-overlay {
  &:hover {
    .view-span-logs {
      visibility: visible;
    }
  }
}
</style>
