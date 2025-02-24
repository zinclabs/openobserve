<template>
  <BaseEdge
    :id="id"
    :style="style"
    :path="path[0]"
    :marker-end="markerEnd"
    type="smoothstep"
  />
  <EdgeLabelRenderer v-if="!isInView">
    <div
      :style="{
        position: 'absolute',
        top: `${midY}px`,
        left: `${midX}px`,
        transform: 'translate(-50%, -50%)',
        pointerEvents: 'auto', /* Ensure the button receives events */
        zIndex: 10, /* Bring it to the front */
      }"
    >
      <button
        @click="handleClick"
        class="remove-edge-button"
        aria-label="Remove edge"
      >
      <q-icon name="close"
      flat
      dense
      size="12px"
      class="red-btn"
       />
      </button>
    </div>
  </EdgeLabelRenderer>
</template>

<script setup>
import { BaseEdge, getBezierPath,EdgeLabelRenderer,getSmoothStepPath,useVueFlow } from '@vue-flow/core'
import { computed, onMounted } from 'vue'


const props = defineProps({
  id: {
    type: String,
    required: true,
  },
  sourceX: {
    type: Number,
    required: true,
  },
  sourceY: {
    type: Number,
    required: true,
  },
  targetX: {
    type: Number,
    required: true,
  },
  targetY: {
    type: Number,
    required: true,
  },
  sourcePosition: {
    type: String,
    required: true,
  },
  targetPosition: {
    type: String,
    required: true,
  },
  data: {
    type: Object,
    required: false,
  },
  markerEnd: {
    type: String,
    required: false,
  },
  style: {
    type: Object,
    required: false,
  },
  isInView: {
    type: Boolean,
    required: false,
    default: false,
  },
})
const midX = computed(() => (props.sourceX + props.targetX) / 2)
const midY = computed(() => (props.sourceY + props.targetY) / 2)
const { removeEdges } = useVueFlow()


onMounted(() => {
 
})


const path = computed(() => getBezierPath(props))
function handleClick(event) {
  event.stopPropagation() // Prevent edge selection
  removeEdges(props.id)
}
</script>

<script>
export default {
  inheritAttrs: false,
}
</script>


<style scoped>
.remove-edge-button {
  background: red;
  color: white;
  border: none;
  border-radius: 100%;
  cursor: pointer;
  padding: 0px 6px !important;
}
.red-btn{
  padding: 0px !important;
  bottom: 1px;
}
</style>



