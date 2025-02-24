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
  <div class="flex justify-between items-center q-pb-sm">
    <div class="flex items-center">
      <div
        data-test="add-pipeline-back-btn"
        class="flex justify-center items-center q-mr-md cursor-pointer"
        style="
          border: 1.5px solid;
          border-radius: 50%;
          width: 22px;
          height: 22px;
        "
        title="Go Back"
        @click="openCancelDialog"
      >
        <q-icon name="arrow_back_ios_new" size="14px" />
      </div>
      <div class="text-h6" v-if="pipelineObj.isEditPipeline == true">
        {{ pipelineObj.currentSelectedPipeline.name }}
      </div>
      <div class="text-h6" v-if="pipelineObj.isEditPipeline == false">
        <q-input
          v-model="pipelineObj.currentSelectedPipeline.name"
          :label="t('pipeline.pipelineName')"
          style="border: 1px solid #eaeaea"
          filled
          dense
        />
      </div>
    </div>

    <div class="flex justify-end">
      <q-btn
        data-test="add-pipeline-cancel-btn"
        label="Cancel"
        class="text-bold border q-ml-md"
        padding="sm xl"
        no-caps
        @click="openCancelDialog"
      />

      <q-btn
        data-test="add-pipeline-save-btn"
        label="Save"
        class="text-bold no-border q-ml-md"
        color="secondary"
        padding="sm xl"
        no-caps
        @click="savePipeline"
      />
    </div>
  </div>

  <q-separator class="q-mb-sm" />

  <div class="flex q-mt-md">
    <div class="nodes-drag-container q-pr-md">
      <div
        data-test="pipeline-editor-nodes-list-title"
        class="text-bold q-mb-sm q-mx-sm"
      >
        {{ t("pipeline.nodes") }}
      </div>

      <q-separator class="q-mb-md" />

      <div class="flex q-mt-sm">
        <NodeSidebar :nodeTypes="nodeTypes" />
      </div>
    </div>
    <div
      id="pipelineChartContainer"
      ref="chartContainerRef"
      class="relative-position pipeline-chart-container o2vf_node"
      :class="store.state.theme === 'dark' ? '' : 'bg-grey-2'"
    >
      <PipelineFlow />
    </div>
  </div>

  <q-dialog
    v-model="pipelineObj.dialog.show"
    position="right"
    full-height
    maximized
  >
    <div
      data-test="pipeline-nodes-list-dragable"
      class="stream-routing-dialog-container full-height"
    >
      <QueryForm
        v-if="pipelineObj.dialog.name === 'query'"
        :stream-name="pipeline.stream_name"
        :stream-type="pipeline.stream_type"
        :stream-routes="streamRoutes"
        @cancel:hideform="resetDialog"
      />

      <ConditionForm
        v-if="pipelineObj.dialog.name === 'condition'"
        @cancel:hideform="resetDialog"
      />

      <AssociateFunction
        v-if="pipelineObj.dialog.name === 'function'"
        :functions="functionOptions"
        :associated-functions="associatedFunctions"
        @cancel:hideform="resetDialog"
        @add:function="refreshFunctionList"
      />

      <StreamNode
        v-if="pipelineObj.dialog.name === 'stream'"
        @cancel:hideform="resetDialog"
      />
      <ExternalDestination
        v-if="pipelineObj.dialog.name === 'remote_stream'"
        @cancel:hideform="resetDialog"
       />
      
    </div>
  </q-dialog>
  <confirm-dialog
    :title="confirmDialogMeta.title"
    :message="confirmDialogMeta.message"
    @update:ok="confirmDialogMeta.onConfirm()"
    @update:cancel="resetConfirmDialog"
    v-model="confirmDialogMeta.show"
  />
  <ConfirmDialog
    title="Save Pipeline"
    message="Are you sure you want to save this Pipeline, as it does not have any impact?"
    @update:ok="confirmSaveBasicPipeline"
    @update:cancel="resetBasicDialog"
    v-model="confirmDialogBasicPipeline"
  />
</template>

<script setup lang="ts">
import {
  computed,
  defineAsyncComponent,
  onBeforeMount,
  onMounted,
  onUnmounted,
  watch,
  ref,
  type Ref,
} from "vue";
import { getImageURL } from "@/utils/zincutils";
import AssociateFunction from "@/components/pipeline/NodeForm/AssociateFunction.vue";
import functionsService from "@/services/jstransform";

import { useStore } from "vuex";
import pipelineService from "@/services/pipelines";
import { onBeforeRouteLeave, useRouter } from "vue-router";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { useI18n } from "vue-i18n";
import { useQuasar } from "quasar";
import jstransform from "@/services/jstransform";
import NodeSidebar from "@/components/pipeline/NodeSidebar.vue";
import useDragAndDrop from "@/plugins/pipelines/useDnD";
import StreamNode from "@/components/pipeline/NodeForm/Stream.vue";
import QueryForm from "@/components/pipeline/NodeForm/Query.vue";
import ConditionForm from "@/components/pipeline/NodeForm/Condition.vue";
import { MarkerType } from "@vue-flow/core";
import ExternalDestination from "./NodeForm/ExternalDestination.vue";

const functionImage = getImageURL("images/pipeline/function.svg");
const streamImage = getImageURL("images/pipeline/stream.svg");
const streamOutputImage = getImageURL("images/pipeline/outputStream.svg");
const externalOutputImage = getImageURL("images/pipeline/externalOutput.svg");
const streamRouteImage = getImageURL("images/pipeline/route.svg");
const conditionImage = getImageURL("images/pipeline/condition.svg");
const queryImage = getImageURL("images/pipeline/query.svg");

import config from "@/aws-exports";

const PipelineFlow = defineAsyncComponent(
  () => import("@/plugins/pipelines/PipelineFlow.vue"),
);

interface Routing {
  [key: string]: RouteCondition[];
}

interface RouteCondition {
  column: string;
  operator: string;
  value: string;
}

interface Function {
  name: string;
  description: string;
  stream: string;
  order: number;
}

interface Pipeline {
  pipeline_id: string;
  name: string;
  description: string;
  stream_name: string;
  stream_type: string;
  routing: Routing;
  functions: Function[];
  derived_streams: any[];
}

interface Node {
  name: string;
  x: number;
  y: number;
  type: string;
  fixed: boolean;
  order?: number;
  stream?: string;
}

interface NodeLink {
  from: string[];
  to: string[];
}

const plotChart: any = ref({
  options: {
    tooltip: {},
    series: [
      {
        type: "graph",
        layout: "none",
        symbolSize: "60",
        roam: false,
        label: {
          show: true,
          position: "bottom",
        },
        draggable: true,
        edgeSymbol: ["arrow"],
        edgeSymbolSize: [10, 10],
        edgeLabel: {
          fontSize: 20,
        },
        data: [],
        links: [],
        lineStyle: {
          opacity: 0.9,
          width: 2,
          curveness: 0,
        },
      },
    ],
  },
});

const pipeline = ref<Pipeline>({
  pipeline_id: "",
  name: "",
  stream_type: "",
  description: "",
  stream_name: "",
  routing: {},
  functions: [],
  derived_streams: [],
});

const router = useRouter();
const store = useStore();

const confirmDialogMeta: any = ref({
  show: false,
  title: "",
  message: "",
  data: null,
  onConfirm: () => {},
});

const nodeTypes: any = [
  {
    label: "Source",
    icon: "input",
    isSectionHeader: true,
  },
  {
    label: "Stream",
    subtype: "stream",
    io_type: "input",
    icon: "img:" + streamImage,
    tooltip: "Source: Stream Node",
    isSectionHeader: false,
  },
  {
    label: "Query",
    subtype: "query",
    io_type: "input",
    icon: "img:" + queryImage,
    tooltip: "Source: Query Node",
    isSectionHeader: false,
  },
  {
    label: "Transform",
    icon: "processing",
    isSectionHeader: true,
  },
  {
    label: "Function",
    subtype: "function",
    io_type: "default",
    icon: "img:" + functionImage,
    tooltip: "Function Node",
    isSectionHeader: false,
  },
  {
    label: "Condition",
    subtype: "condition",
    io_type: "default",
    icon: "img:" + conditionImage,
    tooltip: "Condition Node",
    isSectionHeader: false,
  },
  {
    label: "Destination",
    icon: "input",
    isSectionHeader: true,
  },
  {
    label: "Stream",
    subtype: "stream",
    io_type: "output",
    icon: "img:" + streamOutputImage,
    tooltip: "Destination: Stream Node",
    isSectionHeader: false,
  }
];
const functions = ref<{ [key: string]: Function }>({});

const { pipelineObj, resetPipelineData } = useDragAndDrop();
pipelineObj.nodeTypes = nodeTypes;
pipelineObj.functions = functions;

const nodes: Ref<Node[]> = ref([]);

const hasInputType = computed(() => {
  return pipelineObj.currentSelectedPipeline.nodes.some(
    (node: any) => node.io_type === "input",
  );
});

const nodeLinks = ref<{ [key: string]: NodeLink }>({});

const refreshFunctionList = () => {
  getFunctions();
};
const functionOptions = ref<string[]>([]);

const streamRoutes = ref<{ [key: string]: any }>({});

const editingStreamRouteName = ref<string>("");

const editingFunctionName = ref<string>("");

const isFetchingFunctions = ref(false);

const chartContainerRef = ref(null);

const nodeRows = ref<(string | null)[]>([]);

const q = useQuasar();

const confirmDialogBasicPipeline = ref(false);
const associatedFunctions: Ref<string[]> = ref([]);

const { t } = useI18n();

const dialog = ref({
  name: "streamRouting",
  show: false,
  title: "Stream Routing",
  message: "",
  okCallback: () => {},
});

onBeforeMount(() => {
  if (config.isEnterprise == "true") {
    nodeTypes.push({
      label: "Remote",
      subtype: "remote_stream",
      io_type: "output",
      icon: "img:" + externalOutputImage,
      tooltip: "Destination: External Destination Node",
      isSectionHeader: false,
    });
  }
  const route = router.currentRoute.value;
  if (route.name == "pipelineEditor" && route.query.id) {
    getPipeline();
    pipelineObj.isEditPipeline = true;
  } else {
    pipelineObj.isEditPipeline = false;
    resetPipelineData();
  }
  getFunctions();
});

onMounted(() => {
  window.addEventListener("beforeunload", beforeUnloadHandler);
  const { path, query } = router.currentRoute.value; 
    if (path.includes("edit") && !query.id) {
      router.push({
        name:"pipelines",
        query:{
          org_identifier: store.state.selectedOrganization.identifier
        }
      })
    }
});

onUnmounted(() => {
  window.removeEventListener("beforeunload", beforeUnloadHandler);
});


let forceSkipBeforeUnloadListener = false;

onBeforeRouteLeave((to, from, next) => {
  // check if it is a force navigation, then allow
  if (forceSkipBeforeUnloadListener) {
    next();
    return;
  }
  // else continue to warn user
  if (
    (from.path === "/pipeline/pipelines/edit" && pipelineObj.dirtyFlag) ||
    (from.path === "/pipeline/pipelines/add" &&
      pipelineObj.currentSelectedPipeline.nodes.length)
  ) {
    const confirmMessage = t("pipeline.unsavedMessage");
    if (window.confirm(confirmMessage)) {
      // User confirmed, allow navigation
      next();
    } else {
      // User canceled, prevent navigation
      next(false);
    }
  } else {
    // No unsaved changes or not leaving the edit route, allow navigation
    next();
  }
});

const getPipeline = () => {
  const route = router.currentRoute.value;

  pipelineService
    .getPipelines(store.state.selectedOrganization.identifier)
    .then(async (response) => {
      const _pipeline = response.data.list.find(
        (pipeline: Pipeline) => pipeline.pipeline_id === route.query.id,
      );

      _pipeline.edges.forEach((edge: any) => {
        edge.markerEnd = {
          type: MarkerType.ArrowClosed,
          width: 20, // Increase arrow width
          height: 20, // Increase arrow height
        };
        edge.style = {
          ...edge.style, // Preserve existing styles
          strokeWidth: 2,
        };
        edge.type = "custom";
        edge.animated = true;
      });

      _pipeline.nodes.forEach((node: any) => {
        node.type = node.io_type;
      });

      _pipeline.nodes.forEach((node: any) => {
        node.type = node.io_type;
      });

      if (!_pipeline) {
        q.notify({
          message: "Pipeline not found",
          color: "negative",
          position: "bottom",
          timeout: 3000,
        });
        router.replace({
          name: "pipelines",
          query: {
            org_identifier: store.state.selectedOrganization.identifier,
          },
        });
        return;
      }

      pipelineObj.currentSelectedPipeline = _pipeline;
      pipelineObj.pipelineWithoutChange = JSON.parse(JSON.stringify(_pipeline));
    });
};

const getFunctions = () => {
  // if (Object.keys(functions.value).length) return;
  isFetchingFunctions.value = true;
  return functionsService
    .list(
      1,
      100000,
      "name",
      false,
      "",
      store.state.selectedOrganization.identifier,
    )
    .then((res) => {
      functions.value = {};
      functionOptions.value = [];
      res.data.list.forEach((func: Function) => {
        functions.value[func.name] = func;
        functionOptions.value.push(func.name);
      });
    })
    .finally(() => {
      isFetchingFunctions.value = false;
    });
};

const resetDialog = () => {
  pipelineObj.dialog.show = false;
  pipelineObj.dialog.name = "";
  editingFunctionName.value = "";
  editingStreamRouteName.value = "";
};

const savePipeline = async () => {
  forceSkipBeforeUnloadListener = true;
  if (pipelineObj.currentSelectedPipeline.name === "") {
    q.notify({
      message: "Pipeline name is required",
      color: "negative",
      position: "bottom",
      timeout: 3000,
    });
    return;
  }
  // Find the input node
  const inputNodeIndex = pipelineObj.currentSelectedPipeline.nodes.findIndex(
    (node: any) =>
      node?.io_type === "input" &&
      (node.data?.node_type === "stream" || node.data?.node_type === "query"),
  );

  const outputNodeIndex = pipelineObj.currentSelectedPipeline.nodes.findIndex(
    (node: any) => node?.io_type === "output",
  );

  if (inputNodeIndex === -1) {
    q.notify({
      message: "Source node is required",
      color: "negative",
      position: "bottom",
      timeout: 3000,
    });
    return;
  } else if (outputNodeIndex === -1) {
    q.notify({
      message: "Destination node is required",
      color: "negative",
      position: "bottom",
      timeout: 3000,
    });
    return;
  } else {
    pipelineObj.currentSelectedPipeline.nodes.map((node: any) => {
      if (
        node.data.node_type === "stream" &&
        node.data.stream_name &&
        node.data.stream_name.hasOwnProperty("value")
      ) {
        node.data.stream_name = node.data.stream_name.value;
      }
    });
    const nodes = pipelineObj.currentSelectedPipeline.nodes as any[];

    const inputNode: any = nodes.splice(inputNodeIndex, 1)[0];
    nodes.unshift(inputNode);
    if (inputNode.data.node_type === "stream") {
      pipelineObj.currentSelectedPipeline.source.source_type = "realtime";
    } else {
      pipelineObj.currentSelectedPipeline.source.source_type = "scheduled";
    }
  }

  pipelineObj.currentSelectedPipeline.org =
    store.state.selectedOrganization.identifier;
  if (findMissingEdges()) {
    q.notify({
      message: "Please connect all nodes before saving",
      color: "negative",
      position: "bottom",
      timeout: 3000,
    });
    return;
  }

  const isValid = isValidNodes(pipelineObj.currentSelectedPipeline.nodes);
  if (!isValid) {
    confirmDialogBasicPipeline.value = true;
    return;
  }

  await onSubmitPipeline();
};

const confirmSaveBasicPipeline = async () => {
  confirmDialogBasicPipeline.value = false;
  await onSubmitPipeline();
};
const validatePipeline = () => {
  // Find input node
  const inputNode = pipelineObj.currentSelectedPipeline.nodes?.find((node: any) => node.type === 'input');

  const outputNode = pipelineObj.currentSelectedPipeline.nodes?.find((node: any) => node.type === 'output');
  

  // If trying to use enrichment_tables with stream input, return false
  if ( inputNode.data?.node_type === 'stream' && outputNode.data?.node_type === 'stream' && outputNode.data?.stream_type === 'enrichment_tables') {
    q.notify({
      message: "Enrichment tables as destination stream is only available for scheduled pipelines",
      color: "negative",
      position: "bottom",
      timeout: 2000,
    });
    return false;
  }

  return true;
};

const onSubmitPipeline = async () => {
  if(!validatePipeline()){
    return;
  }
  const dismiss = q.notify({
    message: "Saving pipeline...",
    position: "bottom",
    spinner: true,
  });

  const saveOperation = pipelineObj.isEditPipeline
    ? pipelineService.updatePipeline({
        data: pipelineObj.currentSelectedPipeline,
        org_identifier: store.state.selectedOrganization.identifier,
      })
    : pipelineService.createPipeline({
        data: pipelineObj.currentSelectedPipeline,
        org_identifier: store.state.selectedOrganization.identifier,
      });

  saveOperation
    .then(() => {
      if (pipelineObj.isEditPipeline) {
        pipelineObj.isEditPipeline = false;
      }
      q.notify({
        message: "Pipeline saved successfully",
        color: "positive",
        position: "bottom",
        timeout: 3000,
      });
      router.push({
        name: "pipelines",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    })
    .catch((error) => {
      if (pipelineObj.isEditPipeline) {
        pipelineObj.isEditPipeline = true;
      }

      if (
        error.response?.data?.message === "Invalid Pipeline: empty edges list"
      ) {
        q.notify({
          message: "Please connect all nodes",
          color: "negative",
          position: "bottom",
          timeout: 3000,
        });
      } else {
        if (error.response.status != 403) {
          q.notify({
            message:
              error.response?.data?.message || "Error while saving pipeline",
            color: "negative",
            position: "bottom",
            timeout: 3000,
          });
        }
      }
    })
    .finally(() => {
      dismiss();
    });
};

const openCancelDialog = () => {
  if (
    pipelineObj.dirtyFlag ||
    (!pipelineObj.isEditPipeline &&
      pipelineObj.currentSelectedPipeline.nodes.length > 1)
  ) {
    confirmDialogMeta.value.show = true;
    confirmDialogMeta.value.title = t("common.cancelChanges");
    confirmDialogMeta.value.message =
      "Are you sure you want to cancel changes?";
    confirmDialogMeta.value.onConfirm = () => {
      resetPipelineData();
      router.push({
        name: "pipelines",
        query: {
          org_identifier: store.state.selectedOrganization.identifier,
        },
      });
    };
  } else {
    router.push({
      name: "pipelines",
      query: {
        org_identifier: store.state.selectedOrganization.identifier,
      },
    });
  }
};
const resetConfirmDialog = () => {
  confirmDialogMeta.value.show = false;
  confirmDialogMeta.value.title = "";
  confirmDialogMeta.value.message = "";
  confirmDialogMeta.value.onConfirm = () => {};
  confirmDialogMeta.value.data = null;
};

const resetBasicDialog = () => {
  confirmDialogBasicPipeline.value = false;
  router.push({
    name: "pipelines",
    query: {
      org_identifier: store.state.selectedOrganization.identifier,
    },
  });
};

const findMissingEdges = () => {
  const nodes = pipelineObj.currentSelectedPipeline.nodes;
  const edges = pipelineObj.currentSelectedPipeline.edges;

  // Collect node IDs that are part of edges (either source or target)
  const outgoingConnections = new Set(edges.map((edge: any) => edge.source));
  const incomingConnections = new Set(edges.map((edge: any) => edge.target));

  // Find nodes that are not connected properly
  const unconnectedNodes = nodes.filter((node: any) => {
    if (node.type === "default") {
      // Check for both incoming and outgoing edges
      return (
        !incomingConnections.has(node.id) || !outgoingConnections.has(node.id)
      );
    } else {
      // Check for at least one connection (incoming or outgoing)
      return (
        !incomingConnections.has(node.id) && !outgoingConnections.has(node.id)
      );
    }
  });

  if (unconnectedNodes.length > 0) {
    console.log(unconnectedNodes, "unconnectedNodes");
    return true; // There are unconnected nodes
  }

  return false; // All nodes are properly connected
};
const isValidNodes = (nodes: any) => {
  if (nodes.length > 2) {
    return true;
  }
  const inputNode = nodes.find((node: any) => node.io_type === "input");
  const outputNode = nodes.find((node: any) => node.io_type === "output");

  if (inputNode.data.node_type !== "stream") {
    return true;
  }
  if (
    inputNode.data.node_type === "stream" &&
    outputNode.data.node_type === "stream" &&
    inputNode.data.stream_name === outputNode.data.stream_name &&
    inputNode.data.stream_type === outputNode.data.stream_type
  ) {
    return false;
  }
  return true;
};

// Drag n Drop methods

const onNodeDragStart = (event: any, data: any) => {
  event.dataTransfer.setData("text", data);
};

const onNodeDrop = (event: any) => {
  event.preventDefault();
  const nodeType = event.dataTransfer.getData("text");
};

const onNodeDragOver = (event: any) => {
  event.preventDefault();
};

const updateNewFunction = (_function: Function) => {
  if (!functions.value[_function.name]) {
    functions.value[_function.name] = _function;
    functionOptions.value.push(_function.name);
  }
};

const beforeUnloadHandler = (e: any) => {
  //check is data updated or not
  if (
    pipelineObj.dirtyFlag ||
    (pipelineObj.currentSelectedPipeline.nodes.length > 1 &&
      !pipelineObj.isEditPipeline)
  ) {
    // Display a confirmation message
    const confirmMessage = t("pipeline.unsavedMessage"); // Some browsers require a return statement to display the message
    e.returnValue = confirmMessage;
    return confirmMessage;
  }
};
</script>

<style scoped lang="scss">
.node-actions {
  top: 227px;
  left: 119px;
}
.diagram-container {
  border: 1px solid #000;
}

.pipeline-chart-container {
  height: 80vh;
  border-radius: 12px;
  width: calc(100% - 200px);
}

.nodes-drag-container {
  width: 200px;
}

.node-type-row {
  cursor: grab;
  padding: 5px;
  border: 1px solid #e4e4e4;
  border-radius: 8px;
  user-select: none;
}
</style>

<style lang="scss">
.stream-routing-dialog-container {
  min-width: 540px !important;
}

.o2vf_node {
  width: auto;

  .vue-flow__node {
    padding: 0px;
    width: auto;
  }

  .o2vf_node_input,
  .vue-flow__node-input {
    background-color: #c8d6f5;
    border-color: 1px solid #2c6b2f;
    color: black;
  }

  .o2vf_node_output,
  .vue-flow__node-output {
    background-color: #8fd4b8;
    border-color: 1px solid #3b6f3f;
    color: black;
  }

  .o2vf_node_default,
  .vue-flow__node-default {
    background-color: #efefef;
    border-color: 1px solid #171e25;
    color: black;
  }
}
</style>
