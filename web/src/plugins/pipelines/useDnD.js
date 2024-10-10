
/* Copyright 2023 Zinc Labs Inc.

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
*/

import { b64EncodeUnicode, getUUID } from "@/utils/zincutils";
import { useVueFlow  } from "@vue-flow/core";
import { watch, reactive } from "vue";

const dialogObj = {
  show: false,
  name: "",
  title: "",
  message: "",
  data: null,
};

const defaultPipelineObj = {
  name: "",
  description: "",
  source: {
    source_type: "realtime",
  },
  nodes: [],
  edges: [],
  org: "",
  
};

const defaultObject = {
  pipelineDirectionTopBottom: false,
  dirtyFlag: false,
  isEditPipeline: false,
  isEditNode: false,
  nodesChange:false,
  edgesChange:false,
  draggedNode: null,
  isDragOver: false,
  isDragging: false,
  hasInputNode: false,
  currentSelectedNodeID: "",
  currentSelectedNodeData: {
    stream_type: "logs",
    stream_name: "",
    data: {},
    type:"",
  },
  dialog: dialogObj,
  nodeTypes: null,
  currentSelectedPipeline: defaultPipelineObj,
  pipelineWithoutChange: defaultPipelineObj,
  functions: {},
};

const pipelineObj = reactive(Object.assign({}, defaultObject));

export default function useDragAndDrop() {
  const { screenToFlowCoordinate, onNodesInitialized, updateNode } =
    useVueFlow();

    watch(
      () => pipelineObj.isDragging,
      (dragging) => {
        document.body.style.userSelect = dragging ? "none" : "";
      }
    );

  function hasInputNodeFn() {
    pipelineObj.hasInputNode = pipelineObj.currentSelectedPipeline.nodes.some(
      (node) => node.io_type === "input",
    );
  }

  function onDragStart(event, node) {
    if (event.dataTransfer) {
      event.dataTransfer.setData("application/vueflow", node.io_type);
      event.dataTransfer.effectAllowed = "move";
    }

    pipelineObj.draggedNode = node;
    pipelineObj.isDragging = true;
    pipelineObj.currentSelectedNodeData = null;

    document.addEventListener("drop", onDragEnd);
  }

  /**
   * Handles the drag over event.
   *
   * @param {DragEvent} event
   */
  function onDragOver(event) {
    event.preventDefault();

    if (pipelineObj.draggedNode.io_type) {
      pipelineObj.isDragOver = true;

      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "move";
      }
    }
  }

  function onDragLeave() {
    pipelineObj.isDragOver = false;
  }

  function onDragEnd() {
    pipelineObj.isDragging = false;
    pipelineObj.isDragOver = false;
    document.removeEventListener("drop", onDragEnd);
  }

  /**
   * Handles the drop event.
   *
   * @param {DragEvent} event
   */
  function onDrop(event) {
    const position = screenToFlowCoordinate({
      x: event.clientX,
      y: event.clientY,
    });

    const nodeId = getUUID();

    const newNode = {
      id: nodeId,
      type: pipelineObj.draggedNode.io_type || "default",
      io_type: pipelineObj.draggedNode.io_type || "default",
      position,
      data: { label: nodeId, node_type: pipelineObj.draggedNode.subtype },
    };

    /**
     * Align node position after drop, so it's centered to the mouse
     *
     * We can hook into events even in a callback, and we can remove the event listener after it's been called.
     */
    const { off } = onNodesInitialized(() => {
      updateNode(nodeId, (node) => ({
        position: {
          x: node.position.x - node.dimensions.width / 2,
          y: node.position.y - node.dimensions.height / 2,
        },
      }));

      off();
    });

    pipelineObj.currentSelectedNodeData = newNode;
    pipelineObj.dialog.name = newNode.data.node_type;
    pipelineObj.dialog.show = true;
    pipelineObj.isEditNode = false;
  }

  function onNodeChange(changes) {

    console.log("Node change", changes);
  }

  function onNodesChange(changes) {
    hasInputNodeFn();

  }

  function onEdgesChange(changes) {
    pipelineObj.dirtyFlag = true;
    if(changes.length > 0){
      pipelineObj.edgesChange = true;
    }
    console.log("Edges change", changes);
  }

  function onConnect(connection) {
    // Add new connection (edge) to edges array
    const newEdge = {
      id: `e${connection.source}-${connection.target}`,
      source: connection.source,
      target: connection.target,
      markerEnd: { type: 'arrowclosed' }, // Add arrow marker


    };

    pipelineObj.currentSelectedPipeline.edges = [
      ...pipelineObj.currentSelectedPipeline.edges,
      newEdge,
      console.log(pipelineObj.currentSelectedPipeline.edges,"edges new"),
    ]; // Update edges state
  }

  function validateConnection({ source, target, sourceHandle, targetHandle }) {
    // Example validation rules
    const sourceNode = pipelineObj.currentSelectedPipeline.nodes.find(
      (node) => node.id === source,
    );
    const targetNode = pipelineObj.currentSelectedPipeline.nodes.find(
      (node) => node.id === target,
    );

    // Input-only node (cannot be the source of a connection)
    if (sourceNode.type === "input") {
      return false;
    }

    // Output-only node (cannot be the target of a connection)
    if (targetNode.type === "output") {
      return false;
    }

    return true; // Allow connection for 'both' nodes
  }

  function addNode(newNode) {
    if(pipelineObj.isEditPipeline == true ){
      pipelineObj.dirtyFlag = true;
      pipelineObj.nodesChange = true;
    }
    let currentSelectedNode = pipelineObj.currentSelectedNodeData;
    if (pipelineObj.isEditNode == true && currentSelectedNode.id != "") {
      if (currentSelectedNode) {
        currentSelectedNode.data = { ...currentSelectedNode.data, ...newNode };

        //find the index from pipelineObj.currentSelectedPipeline.nodes based on id
        const index = pipelineObj.currentSelectedPipeline.nodes.findIndex(
          (node) => node.id === currentSelectedNode.id,
        );

        pipelineObj.currentSelectedPipeline.nodes[index] = currentSelectedNode;
      }
    } else {
      if (currentSelectedNode) {
        currentSelectedNode.data = { ...currentSelectedNode.data, ...newNode };
        pipelineObj.currentSelectedPipeline.nodes = [
          ...pipelineObj.currentSelectedPipeline.nodes,
          currentSelectedNode,
        ];
      }
    }
    pipelineObj.isEditNode = false;
    pipelineObj.currentSelectedNodeData = dialogObj;
  }

  function editNode(updatedNode) {
    const index = pipelineObj.currentSelectedPipeline.nodes.findIndex(
      (node) => node.id === updatedNode.id,
    );
    if (index !== -1) {
      pipelineObj.currentSelectedPipeline.nodes[index] = {
        ...pipelineObj.currentSelectedPipeline.nodes[index],
        ...updatedNode,
      };
    }
  }
  const comparePipelinesById = (pipeline1, pipeline2) => {
    const compareIds = (items1, items2) => {
      const extractAndSortIds = (items) =>
        items.map(item => item.id).sort();

      const ids1 = extractAndSortIds(items1);
      const ids2 = extractAndSortIds(items2);
      console.log(ids1,ids2)
  
      return JSON.stringify(ids1) === JSON.stringify(ids2);
    };
    const nodesEqual = compareIds(pipeline1.nodes, pipeline2.nodes);
  
    return nodesEqual;
  };

  // delete the node from pipelineObj.currentSelectedPipeline.nodes and pipelineObj.currentSelectedPipeline.edges all reference associated with target and source
  // also empty pipelineObj.currentSelectedNodeData
  function deletePipelineNode(nodeId) {

    pipelineObj.currentSelectedPipeline.nodes =
      pipelineObj.currentSelectedPipeline.nodes.filter(
        (node) => node.id !== nodeId,
      );
    pipelineObj.currentSelectedPipeline.edges =
      pipelineObj.currentSelectedPipeline.edges.filter(
        (edge) => edge.source !== nodeId && edge.target !== nodeId,
      );
    pipelineObj.currentSelectedNodeData = null;
    hasInputNodeFn();
    console.log(pipelineObj.currentSelectedPipeline,"current")
    console.log(pipelineObj.pipelineWithoutChange,"past")

    const arePipelinesEqualById = comparePipelinesById(
      pipelineObj.currentSelectedPipeline,
      pipelineObj.pipelineWithoutChange
    );
    console.log(pipelineObj.edgesChange,"edges")
    if(arePipelinesEqualById == true && pipelineObj.edgesChange == false ){
      pipelineObj.dirtyFlag = false;
    }
    if(arePipelinesEqualById == false){
      pipelineObj.dirtyFlag = true;
    }
    
    
  }

  const resetPipelineData = () => {
    pipelineObj.currentSelectedPipeline = JSON.parse(JSON.stringify(defaultPipelineObj));
    pipelineObj.currentSelectedNodeData = JSON.parse(JSON.stringify(dialogObj));
    pipelineObj.isEditPipeline = false;
    pipelineObj.isEditNode = false;
    pipelineObj.dirtyFlag = false;
    pipelineObj.hasInputNode = false;
    pipelineObj.draggedNode = null;
  };
 

  return {
    pipelineObj,
    onDragStart,
    onDragLeave,
    onDragOver,
    onDrop,
    onNodeChange,
    onNodesChange,
    onEdgesChange,
    onConnect,
    validateConnection,
    addNode,
    editNode,
    deletePipelineNode,
    resetPipelineData,
    comparePipelinesById,
  };
}
