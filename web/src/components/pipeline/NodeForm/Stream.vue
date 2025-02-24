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
    data-test="add-stream-input-stream-routing-section"
    class=" full-height"
    style="width: 40vw;"
    :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
  >
    <div class="stream-routing-title q-pb-sm q-pl-md">
      {{ t("pipeline.streamTitle") }}
    </div>
    <q-separator />

    <div   class="stream-routing-container full-width q-pa-md">

      <q-toggle
      v-if="selectedNodeType == 'output'"
        data-test="create-stream-toggle"
        class="q-mb-sm"
        :label="isUpdating ? 'Edit Stream' : 'Create new Stream'"
        v-model="createNewStream"
      />
      <q-form   @submit="saveStream">

      <div v-if="!createNewStream">
        <div class="flex justify-start items-center" style="padding-top: 0px">
          <div
            data-test="input-node-stream-type-select"
            class="alert-stream-type o2-input q-mr-sm full-width"
            style="padding-top: 0"
          >

            <q-select
              v-model="stream_type"
              :options="filteredStreamTypes"
              :label="t('alerts.streamType') + ' *'"
              :popup-content-style="{ textTransform: 'none' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop no-case full-width"
              stack-label
              outlined
              filled
              dense
              @update:model-value="updateStreams()"
              :rules="[(val: any) => !!val || 'Field is required!']"
            />
          </div>
          <div
            data-test="input-node-stream-type-select"
            class="alert-stream-type o2-input q-mr-sm full-width"
            style="padding-top: 0"
          >
          <q-select
            v-model="stream_name"
            :options="filteredStreams"
             option-label="label"
              option-value="value"
            :label="t('alerts.stream_name') + ' *'"
            :loading="isFetchingStreams"
            :popup-content-style="{ textTransform: 'lowercase' }"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm showLabelOnTop no-case full-width"
            filled
            stack-label
            dense
            use-input
            hide-selected
            fill-input
            @filter="filterStreams"
            behavior="menu"
            @input-debounce="100"
            :rules="[(val: any) => !!val || 'Field is required!']"
            :option-disable="(option : any)  => option.isDisable"
            @input-value="handleDynamicStreamName"
            />


            <q-toggle
            v-if="stream_type == 'enrichment_tables' && selectedNodeType == 'output'"
              class="col-12 q-py-md text-grey-8 text-bold"
              v-model="appendData"
              :label="t('function.appendData')"
            />






          </div>
          <div v-if="selectedNodeType == 'output'" style="font-size: 14px;" class="note-message" >
            <span class="tw-flex tw-items-center"> <q-icon name="info" class="q-pr-xs"</q-icon>
              Enrichment_tables as destination stream is only available for scheduled pipelines
             </span>

          <span class="tw-flex"> <q-icon name="info" class="q-pr-xs q-pt-xs"</q-icon> Use curly braces '{}' to configure stream name dynamically. e.g. static_text_{fieldname}_postfix. Static text before/after {} is optional</span>
            </div>

        </div>

        <div
          class="flex justify-start full-width"
          :class="store.state.theme === 'dark' ? 'bg-dark' : 'bg-white'"
        >
          <q-btn
            data-test="input-node-stream-cancel-btn"
            class="text-bold"
            :label="t('alerts.cancel')"
            text-color="light-text"
            padding="sm md"
            no-caps
            @click="openCancelDialog"
          />
          <q-btn
            data-test="input-node-stream-save-btn"
            :label="t('alerts.save')"
            class="text-bold no-border q-ml-md"
            color="secondary"
            padding="sm xl"
            no-caps
            type="submit"
          />
          <q-btn
            v-if="pipelineObj.isEditNode"
            data-test="input-node-stream-delete-btn"
            :label="t('pipeline.deleteNode')"
            class="text-bold no-border q-ml-md"
            color="negative"
            padding="sm xl"
            no-caps
            @click="openDeleteDialog"
          />
        </div>
      </div>
      <div v-else class="pipeline-add-stream ">
        <AddStream
        ref="addStreamRef"
        @added:stream-aded="getLogStream"
        :is-in-pipeline = "true"
         />
      </div>
      </q-form>
    </div>

    
  </div>
  <confirm-dialog
    v-model="dialog.show"
    :title="dialog.title"
    :message="dialog.message"
    @update:ok="dialog.okCallback"
    @update:cancel="dialog.show = false"
  />
</template>
<script lang="ts" setup>
import { ref, type Ref, defineEmits, onMounted, watch, defineAsyncComponent, computed } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "vuex";
import ConfirmDialog from "../../ConfirmDialog.vue";
import useDragAndDrop from "@/plugins/pipelines/useDnD";
import useStreams from "@/composables/useStreams";
import pipelineService from "@/services/pipelines";
import AddStream from "@/components/logstream/AddStream.vue";

import { useQuasar } from "quasar";

import { outlinedInfo } from "@quasar/extras/material-icons-outlined";

const emit = defineEmits(["cancel:hideform"]);


const $q = useQuasar();

const { t } = useI18n();

const store = useStore();

const { addNode, pipelineObj , deletePipelineNode} = useDragAndDrop();

const { getStreams } = useStreams();

const filteredStreams: Ref<string[]> = ref([]);
const addStreamRef = ref(null);
const createNewStream = ref(false);
const isUpdating = ref(false);
const isFetchingStreams = ref(false);
const indexOptions = ref([]);
const schemaList = ref([]);
const streams: any = ref({});
const usedStreams: any = ref([]);
const streamTypes = ["logs", "metrics", "traces"];
//for testing purpose but remove metrics and traces as discuessedf
const outputStreamTypes = ["logs", "metrics", "traces","enrichment_tables"];
const stream_name = ref((pipelineObj.currentSelectedNodeData?.data as { stream_name?: string })?.stream_name || {label: "", value: "", isDisable: false});
const dynamic_stream_name = ref((pipelineObj.currentSelectedNodeData?.data as { stream_name?: string })?.stream_name || {label: "", value: "", isDisable: false});

const appendData = ref((pipelineObj.currentSelectedNodeData?.data as { meta?: { append_data?: boolean } })?.meta?.append_data || false);

const stream_type = ref((pipelineObj.currentSelectedNodeData?.data as { stream_type?: string })?.stream_type || "logs");
const selectedNodeType = ref((pipelineObj.currentSelectedNodeData as { io_type?: string })?.io_type || "");
onMounted(async () => {
    if(pipelineObj.userSelectedNode){
      pipelineObj.userSelectedNode = {};
    }
  await getUsedStreamsList();
  await getStreamList();
});

watch(stream_type, (newValue:any) => {
  if(newValue){
    stream_name.value = {label: "", value: "", isDisable: false};
    
  }
  getStreamList();
});

watch(() => dynamic_stream_name.value,
()=>{
  if(  dynamic_stream_name.value !== null && dynamic_stream_name.value !== "" && selectedNodeType.value === 'output'){
    const regex = /^[a-zA-Z0-9_]*\{[a-zA-Z0-9_]+\}[a-zA-Z0-9_]*$/;
    // Check if there is any value between {{ stream_name }}
      if ( typeof dynamic_stream_name.value == 'object' &&  dynamic_stream_name.value.hasOwnProperty('value') && regex.test(dynamic_stream_name.value.value)) {
        saveDynamicStream();
      }
   
  }
})
async function getUsedStreamsList() {
    const org_identifier = store.state.selectedOrganization.identifier;
  try {
    const res = await pipelineService.getPipelineStreams(org_identifier);
    usedStreams.value = res.data.list;
  } catch (error:any) {
    usedStreams.value = [];
    if(error.response.status != 403){
      $q.notify({
      message: error.response?.data?.message || "Error fetching used streams",
      color: "negative",
      position: "bottom",
      timeout: 2000,
    });
    }
   
  }
}
async function getStreamList() {
  const streamType = pipelineObj.currentSelectedNodeData.data.hasOwnProperty("stream_type")
    ? pipelineObj.currentSelectedNodeData.data.stream_type
    : "logs";
  
  
  isFetchingStreams.value = true;
  
  try {
    const res : any = await getStreams(stream_type.value, false);
    
    if (res.list.length > 0 && pipelineObj.currentSelectedNodeData.hasOwnProperty("type") && pipelineObj.currentSelectedNodeData.type === "input") {
      res.list.forEach((stream : any) => {
        stream.isDisable = usedStreams.value.some(
          (usedStream : any) => 
            (usedStream.stream_name === stream.name  && usedStream.stream_type === stream.stream_type)
          
        );
      });
    }
    streams.value[streamType] = res.list;
    schemaList.value = res.list;
    indexOptions.value = res.list.map((data : any) => data.name);
  } finally {
    isFetchingStreams.value = false;
  }
}
const updateStreams = () => {
  getStreamList();
  
};



const handleDynamicStreamName = (val:any) =>{
  val = val.replace(/-/g, '_');
  dynamic_stream_name.value = {label: val, value: val, isDisable: false};
}

const saveDynamicStream = () =>{
  if(typeof dynamic_stream_name.value == 'object' && dynamic_stream_name.value.hasOwnProperty('value') && dynamic_stream_name.value.hasOwnProperty('label')){
    const{label,value} = dynamic_stream_name.value;
    stream_name.value = {label: label, value:value, isDisable: false}; 
  }
  //this condition will never be true but we are keeping it for future reference
  else{
    stream_name.value = dynamic_stream_name.value;
  }
}

const filteredStreamTypes = computed(() => {
      return selectedNodeType.value === 'output' ? outputStreamTypes : streamTypes;
    });

const getLogStream = (data: any) =>{

  data.name = data.name.replace(/-/g, '_');

  stream_name.value = {label: data.name, value: data.name, isDisable: false};
  stream_type.value = data.stream_type;
  if(createNewStream.value){
    createNewStream.value = false;
    return;
  }
}



const dialog = ref({
  show: false,
  title: "",
  message: "",
  okCallback: () => {},
});

const openCancelDialog = () => {
  dialog.value.show = true;
  dialog.value.title = "Discard Changes";
  dialog.value.message = "Are you sure you want to cancel changes?";
  dialog.value.okCallback = () => emit("cancel:hideform");
  pipelineObj.userClickedNode = {};
  pipelineObj.userSelectedNode = {};
};

const openDeleteDialog = () => {
  dialog.value.show = true;
  dialog.value.title = "Delete Node";
  dialog.value.message =
    "Are you sure you want to delete stream association?";
  dialog.value.okCallback = deleteNode;
};

const deleteNode = () => {
  deletePipelineNode(pipelineObj.currentSelectedNodeID);
  emit("cancel:hideform");
};



const saveStream = () => {
  // Validate pipeline configuration

  const streamNodeData: any = {
    stream_type: stream_type,
    stream_name: stream_name,
    org_id: store.state.selectedOrganization.identifier,
    node_type: "stream",
  };

  if(stream_type.value == 'enrichment_tables'){
    streamNodeData.meta = { append_data: appendData.value };
  }

  if( typeof stream_name.value === 'object' && stream_name.value !== null && stream_name.value.hasOwnProperty('value') && stream_name.value.value === ""){
    $q.notify({
      message: "Please select Stream from the list",
      color: "negative",
      position: "bottom",
      timeout: 2000,
    });
    return;
  }
  addNode(streamNodeData);
  emit("cancel:hideform");
};

const filterStreams = (val: string, update: any) => {
  const streamType = pipelineObj.currentSelectedNodeData.data.stream_type || 'logs';
  if( pipelineObj.currentSelectedNodeData.hasOwnProperty("type") &&  pipelineObj.currentSelectedNodeData.type === 'input') {
    const filtered = streams.value[streamType]?.filter((stream :any) => {
    return stream.name.toLowerCase().includes(val.toLowerCase());
  }).map((stream : any) => ({
    label: stream.name,
    value: stream.name,  // Use a unique identifier if needed
    isDisable: stream.isDisable
  }));
  filteredStreams.value = filtered;
  }
  else{
    const filtered = streams.value[streamType].filter((stream : any) => {
    return stream.name.toLowerCase().includes(val.toLowerCase());
  }).map((stream : any) => ({
    label: stream.name,
    value: stream.name,  // Use a unique identifier if needed
    isDisable: false
  }));
  filteredStreams.value = filtered;

  }

  
  update();
};


const filterColumns = (options: any[], val: String, update: Function) => {
  let filteredOptions: any[] = [];
  if (val === "") {
    update(() => {
      filteredOptions = [...options];
    });
    return filteredOptions;
  }
  update(() => {
    const value = val.toLowerCase();
    filteredOptions = options.filter(
      (column: any) => column.toLowerCase().indexOf(value) > -1,
    );
  });
  return filteredOptions;
};
</script>

<style >
.stream-routing-title {
  font-size: 20px;
  padding-top: 16px;
}
.pipeline-add-stream {
  .add-stream-header.row {
    display: none ;
  }
  .add-stream-inputs :nth-child(5){
    /* background-color: red; */
    justify-content: flex-start;
  }



  .q-separator {
    display: none !important;
  }
}
.q-field--labeled.showLabelOnTop.q-select .q-field__control-container .q-field__native > :first-child {
  text-transform: none !important;
  font-size: 0.875rem; /* Keep the font size and weight as needed */
  font-weight: 600;
}

.note-message{
  background-color: #F9F290 ;
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid #F5A623;
  color: #865300;
  width: 100%;
  margin-bottom: 20px;
}


</style>
