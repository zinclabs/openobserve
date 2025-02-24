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

<script setup>
import { Handle } from "@vue-flow/core";
import useDragAndDrop from "./useDnD";
import { defineEmits, ref } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { getImageURL } from "@/utils/zincutils";

import config from "@/aws-exports";

const streamImage = getImageURL("images/pipeline/stream.svg");

const functionImage = getImageURL("images/pipeline/function.svg");
const streamOutputImage = getImageURL("images/pipeline/outputStream.svg");
const streamRouteImage = getImageURL("images/pipeline/route.svg");
const conditionImage = getImageURL("images/pipeline/condition.svg");
const queryImage = getImageURL("images/pipeline/query.svg");
const externalOutputImage = getImageURL("images/pipeline/externalOutput.svg");



const props = defineProps({
  id: {
    type: String,
  },
  data: {
    type: Object,
  },
  io_type: {
    type: String,
  },
});

const emit = defineEmits(["delete:node"]);
const { pipelineObj, deletePipelineNode,onDragStart,onDrop } = useDragAndDrop();
const menu = ref(false)

const hanldeMouseOver = () => {
  console.log("mouse over")
}


const onFunctionClick = (data,event,id) =>{
  console.log(data,"data")
  console.log(id,"id")
  console.log(event,"event")
  pipelineObj.userSelectedNode = data;
  const dataToOpen  =   {
    label: "Function",
    subtype: "function",
    io_type: "default",
    icon: "img:" + functionImage,
    tooltip: "Function Node",
    isSectionHeader: false,
  };
  pipelineObj.userClickedNode = id;
  onDragStart(event,dataToOpen)
  onDrop(event,{x:100,y:100});
  menu.value = false
}

const onConditionClick = (data,event,id) =>{
  data.label = id;
  console.log(data,"data")
  pipelineObj.userSelectedNode = data;

  const dataToOpen  =   {
    label: "Condition",
    subtype: "condition",
    io_type: "default",
    icon: "img:" + conditionImage,
    tooltip: "Condition Node",
    isSectionHeader: false,
  }
  pipelineObj.userClickedNode = id
  onDragStart(event,dataToOpen)
  onDrop(event,{x:100,y:100});
  menu.value = false
}

const onStreamOutputClick = (data,event,id) =>{
  pipelineObj.userSelectedNode = data;

  if(!id){
    pipelineObj.userClickedNode = data.label;
  }
  else{
    pipelineObj.userClickedNode = id;
  }
  const dataToOpen  =    
  {
    label: "Stream",
    subtype: "stream",
    io_type: "output",
    icon: "img:" + streamOutputImage,
    tooltip: "Destination: Stream Node",
    isSectionHeader: false,
  }
  // pipelineObj.userClickedNode = id
  onDragStart(event,dataToOpen)
  onDrop(event,{x:100,y:100});
  menu.value = false
}
const onExternalDestinationClick = (data,event,id) =>{
  pipelineObj.userSelectedNode = data;

  if(!id){
    pipelineObj.userClickedNode = data.label;
  }
  else{
    pipelineObj.userClickedNode = id;
  }
  const dataToOpen  =    
  {
    label: "Remote",
    subtype: "remote_stream",
    io_type: "output",
    icon: "img:" + externalOutputImage,
    tooltip: "Destination: Remote Node",
    isSectionHeader: false,
  }
  // pipelineObj.userClickedNode = id
  onDragStart(event,dataToOpen)
  onDrop(event,{x:100,y:100});
  menu.value = false
}

const { t } = useI18n();


const editNode = (id) => {
  //from id find the node from pipelineObj.currentSelectedPipelineData.nodes
  const fullNode = pipelineObj.currentSelectedPipeline.nodes.find(
    (node) => node.id === id
  );
  console.log(fullNode,'full node')
  pipelineObj.isEditNode = true;
  pipelineObj.currentSelectedNodeData = fullNode;
  pipelineObj.currentSelectedNodeID = id;
  pipelineObj.dialog.name = fullNode.data.node_type;
  pipelineObj.dialog.show = true;
};

const deleteNode = (id) => {
  openCancelDialog(id);
};
const  functionInfo = (data) =>  {

      return pipelineObj.functions[data.name] || null;
  }

const confirmDialogMeta = ref({
  show: false,
  title: "",
  message: "",
  data: null,
  onConfirm: () => {},
});

const openCancelDialog = (id) => {
  confirmDialogMeta.value.show = true;
  confirmDialogMeta.value.title = t("common.delete");
  confirmDialogMeta.value.message = "Are you sure you want to delete node?";
  confirmDialogMeta.value.onConfirm = () => {
    deletePipelineNode(id);
  };
};

const resetConfirmDialog = () => {
  confirmDialogMeta.value.show = false;
  confirmDialogMeta.value.title = "";
  confirmDialogMeta.value.message = "";
  confirmDialogMeta.value.onConfirm = () => {};
};

function getIcon(data, ioType) {
  const searchTerm = data.node_type;
  const node = pipelineObj.nodeTypes.find(
    (node) => node.subtype === searchTerm && node.io_type === ioType,
  );
  return node ? node.icon : undefined;
}
</script>

<template>
  <!-- Input Handle (Target) -->
  <div class="o2vf_node">
    <Handle
      v-if="io_type == 'output' || io_type === 'default'"
      id="input"
      type="target"
      :position="'top'"
      class="node_handle_custom"
     
    />

    <div
      v-if="data.node_type == 'function'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
      padding: 5px 2px;
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
      "
      @mouseover="menu = true"
      @mouseleave="menu = false"
    >
      <q-menu      @mouseover="menu = true" @mouseleave="menu = false"  v-model="menu" name="myMenu" class="menu-list" anchor="top right" self="top left">
    <q-list >
    
      <q-item clickable @click="(event) => onFunctionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="functionImage" alt="Function" style="width: 30px; height: 30px;">
          <q-tooltip  anchor="top middle" self="bottom right">Function</q-tooltip>

        </q-item-section>

      </q-item>
      <q-item clickable @click="(event) => onConditionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="conditionImage" alt="Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Condition</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item clickable @click="(event) => onStreamOutputClick(data, event,id)">
        <q-item-section avatar>
          <img :src="streamOutputImage" alt="Output Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Output</q-tooltip>

        </q-item-section>
      </q-item>
          <q-item
              v-if="config.isEnterprise == 'true'"
            clickable
            @click="(event) => onExternalDestinationClick(data, event, id)"
          >
        <q-item-section avatar>
          <img :src="externalOutputImage" alt="Remote Destination" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Remote</q-tooltip>

        </q-item-section>
      </q-item>
      <!-- Add more items similarly for other images -->
    </q-list>
  </q-menu>

    <q-tooltip :style="{ maxWidth: '300px', whiteSpace: 'pre-wrap' }">
  <div>
    <strong>Name:</strong> {{ functionInfo(data).name }}<br />
    <strong>Definition:</strong><br />
    <div style="border: 1px solid lightgray; padding: 4px; border-radius: 1px ;">
      {{ functionInfo(data).function }}
    </div>
  </div>
</q-tooltip>

      <div class="icon-container " style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
      </div>

      <!-- Separator -->
      <q-separator vertical class="q-mr-sm" />

      <!-- Label -->
      <div class="container">
        <div
          class="row"
          align="left"
          style="
            text-align: left;
            text-wrap: wrap;
            width: auto;
            text-overflow: ellipsis;
          "
        >
          {{ data.name }}
        </div>
        <div class="row">
          <div style="text-transform: capitalize">
            Run
            {{
              data.after_flatten ? "After Flattening" : "Before Flattening"
            }}
          </div>
        </div>
      </div>
      <div class="float-right tw-pl-2">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>

    <div
      v-if="data.node_type == 'stream'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
        padding: 5px 2px;

      "
       @mouseover="menu = true"
      @mouseleave="menu = false"
    >

       <q-menu v-if="io_type == 'input'" @mouseover="menu = true" @mouseleave="menu = false"   v-model="menu" name="myMenu" class="menu-list" anchor="top right" self="top left">
    <q-list >
      <q-item clickable  @click="(event) => onFunctionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="functionImage" alt="Function" style="width: 30px; height: 30px;">
          <q-tooltip  anchor="top middle" self="bottom right">Function</q-tooltip>
        </q-item-section>
      

      </q-item>
      <q-item clickable @click="(event) => onConditionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="conditionImage" alt="Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Condition</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item clickable  @click="(event) => onStreamOutputClick(data, event,id)">
        <q-item-section avatar>
          <img :src="streamOutputImage" alt="Output Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Output</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item
            v-if="config.isEnterprise == 'true'"
            clickable
            @click="(event) => onExternalDestinationClick(data, event, id)"
          >
        <q-item-section avatar>
          <img :src="externalOutputImage" alt="Remote Destination" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Remote</q-tooltip>

        </q-item-section>
      </q-item>
      <!-- Add more items similarly for other images -->
    </q-list>
  </q-menu>

  

      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
      </div>

      <!-- Separator -->
      <q-separator vertical class="q-mr-sm" />

      <!-- Label -->
      <div class="container">
        <div
        v-if=" data.stream_name &&  data.stream_name.hasOwnProperty('label')"
          class="row"
          style="
            text-align: left;
            text-wrap: wrap;
            width: auto;
            text-overflow: ellipsis;
          "
        >
          {{ data.stream_type }} - {{   data.stream_name.label }}
        </div>
        <div
        v-else
          class="row"
          style="
            text-align: left;
            text-wrap: wrap;
            width: auto;
            text-overflow: ellipsis;
          "
        >
          {{ data.stream_type }} - {{    data.stream_name }}
        </div>
      </div>
      <div class="float-right tw-pl-2">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>
    <div
      v-if="data.node_type == 'remote_stream'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
        padding: 5px 2px;

      "
       @mouseover="menu = true"
      @mouseleave="menu = false"
    >

       <q-menu v-if="io_type == 'input'" @mouseover="menu = true" @mouseleave="menu = false"   v-model="menu" name="myMenu" class="menu-list" anchor="top right" self="top left">
    <q-list >
      <q-item clickable  @click="(event) => onFunctionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="functionImage" alt="Function" style="width: 30px; height: 30px;">
          <q-tooltip  anchor="top middle" self="bottom right">Function</q-tooltip>
        </q-item-section>
      

      </q-item>
      <q-item clickable @click="(event) => onConditionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="conditionImage" alt="Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Condition</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item clickable  @click="(event) => onStreamOutputClick(data, event,id)">
        <q-item-section avatar>
          <img :src="streamOutputImage" alt="Output Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Output</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item
            v-if="config.isEnterprise == 'true'"
            clickable
            @click="(event) => onExternalDestinationClick(data, event, id)"
          >
        <q-item-section avatar>
          <img :src="externalOutputImage" alt="Remote Destination" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Remote</q-tooltip>

        </q-item-section>
      </q-item>
      <!-- Add more items similarly for other images -->
    </q-list>
  </q-menu>

  

      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
      </div>

      <!-- Separator -->
      <q-separator vertical class="q-mr-sm" />

      <!-- Label -->
      <div class="container">
        <div
          class="row"
          style="
            text-align: left;
            text-wrap: wrap;
            width: auto;
            text-overflow: ellipsis;
          "
        >
          {{ data.destination_name }}
        </div>
      </div>
      <div class="float-right tw-pl-2">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>

    <div
      v-if="data.node_type == 'query'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
        padding: 5px 2px;

      "
       @mouseover="menu = true"
      @mouseleave="menu = false"
    >
    <q-menu  v-model="menu" @mouseover="menu = true" @mouseleave="menu = false"  name="myMenu" class="menu-list" anchor="top right" self="top left">
    <q-list >
      <q-item clickable  @click="(event) => onFunctionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="functionImage" alt="Function" style="width: 30px; height: 30px;">
          <q-tooltip  anchor="top middle" self="bottom right">Function</q-tooltip>

        </q-item-section>

      </q-item>
      <q-item clickable @click="(event) => onConditionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="conditionImage" alt="Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Condition</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item clickable  @click="(event) => onStreamOutputClick(data, event,id)">
        <q-item-section avatar>
          <img :src="streamOutputImage" alt="Output Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Output</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item 
            v-if="config.isEnterprise == 'true'"
            clickable
            @click="(event) => onExternalDestinationClick(data, event, id)"
          >
        <q-item-section avatar>
          <img :src="externalOutputImage" alt="Remote Destination" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Remote</q-tooltip>

        </q-item-section>
      </q-item>
      <!-- Add more items similarly for other images -->
    </q-list>
  </q-menu>

    <q-tooltip :style="{ maxWidth: '300px', whiteSpace: 'pre-wrap' }">
  <div>
    <strong>{{  data.query_condition.type == 'sql' ? 'SQL' : 'PromQL' }}:</strong> <pre style="max-width: 200px ; text-wrap: wrap;">{{  data.query_condition.type == 'sql' ? data.query_condition.sql : data.query_condition.promql }}</pre><br />
    <strong>Period:</strong> {{ data.trigger_condition.period }}<br />
    <strong>Frequency:</strong> {{ data.trigger_condition.frequency }} {{ data.trigger_condition.frequency_type }}<br />
    <strong>Operator:</strong> {{ data.trigger_condition.operator }}<br />
    <strong>Threshold:</strong> {{ data.trigger_condition.threshold }}<br />
    <strong>Cron:</strong> {{ data.trigger_condition.cron || 'None' }}<br />
    <strong>Silence:</strong> {{ data.trigger_condition.silence }}
  </div>
</q-tooltip>

      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
      </div>

      <!-- Separator -->
      <q-separator vertical class="q-mr-sm" />

      <!-- Label -->
      <div class="container">
        <div
          class="row"
          style="
            text-align: left;
            text-wrap: wrap;
            width: auto;
            text-overflow: ellipsis;
          "
        >
          {{ data.stream_type }} - {{ data.stream_name }}
        </div>
      </div>

      <div class="float-right">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
          style="margin-right: 5px"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>

    <div
      v-if="data.node_type == 'condition'"
      :class="`o2vf_node_${io_type}`"
      class="custom-btn q-pa-none btn-fixed-width"
      style="
        width: fit-content;
        display: flex;
        align-items: center;
        border: none;
        cursor: pointer;
      "
       @mouseover="menu = true"
      @mouseleave="menu = false"
    >
    <q-menu   v-model="menu" @mouseover="menu = true" @mouseleave="menu = false"  name="myMenu" class="menu-list" anchor="top right" self="top left">
    <q-list >

      <q-item clickable  @click="(event) => onFunctionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="functionImage" alt="Function" style="width: 30px; height: 30px;">
          <q-tooltip  anchor="top middle" self="bottom right">Function</q-tooltip>
        </q-item-section>
      </q-item>
      <q-item clickable @click="(event) => onConditionClick(data, event,id)">
        <q-item-section avatar>
          <img :src="conditionImage" alt="Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Condition</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item class="q-item-output" clickable  @click="(event) => onStreamOutputClick(data, event,id)">
        <q-item-section avatar>
          <img :src="streamOutputImage" alt="Output Stream" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Output</q-tooltip>

        </q-item-section>
      </q-item>
      <q-item 
            v-if="config.isEnterprise == 'true'"
            clickable
            @click="(event) => onExternalDestinationClick(data, event, id)"
          >
        <q-item-section avatar>
          <img :src="externalOutputImage" alt="Remote Destination" style="width: 30px; height: 30px;">
          <q-tooltip anchor="top middle" self="bottom right">Remote</q-tooltip>

        </q-item-section>
      </q-item>
      <!-- Add more items similarly for other images -->
    </q-list>
  </q-menu>

      <div class="icon-container" style="display: flex; align-items: center">
        <!-- Icon -->
        <q-icon
          :name="getIcon(data, io_type)"
          size="1em"
          class="q-ma-sm"
        />
      </div>

      <!-- Separator -->
      <q-separator vertical class="q-mr-sm" />

      <!-- Label -->
      <div class="container">
        <div
    style="
      text-align: left;
      text-wrap: wrap;
      width: auto;
      text-overflow: ellipsis;
    "
  >
    <div class="column" v-for="(condition, index) in data.conditions" :key="index" style="margin-bottom: 1px;">
      {{ condition.column }} {{ condition.operator }} {{ condition.value }}
    </div>
  </div>
      </div>

      <div class="float-right">
        <q-btn
          flat
          round
          dense
          icon="edit"
          size="0.8em"
          @click="editNode(id)"
          style="margin-right: 5px"
        />
        <q-btn
          flat
          round
          dense
          icon="delete"
          size="0.8em"
          @click="deleteNode(id)"
        />
      </div>
    </div>
    <Handle
      v-if="io_type === 'input' || io_type === 'default'"
      id="output"
      type="source"
      :position="'bottom'"
      class="node_handle_custom"
      
    />
  </div>

  <confirm-dialog
    :title="confirmDialogMeta.title"
    :message="confirmDialogMeta.message"
    @update:ok="confirmDialogMeta.onConfirm()"
    @update:cancel="resetConfirmDialog"
    v-model="confirmDialogMeta.show"
  />
</template>

<style lang="scss">
.node_handle_custom{
  background-color:#9a9698;
    height:10px;
    width:30px;
    border-radius:4px;
  filter: invert(100%);
}
.vue-flow__node-custom {
  padding: 10px;
  border-radius: 3px;
  width: 150px;
  font-size: 12px;
  text-align: center;
  border-width: 1px;
  border-style: solid;
  color: var(--vf-node-text);
  background-color: var(--vf-node-bg);
  border-color: var(--vf-node-color);
}
.menu-list{
  margin: 0px 10px;
  background-color: white;
}

</style>
