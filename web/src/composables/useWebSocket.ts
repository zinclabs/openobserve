// useWebSocket.ts

import { onBeforeUnmount } from "vue";

type MessageHandler = (event: MessageEvent) => void;
type OpenHandler = (event: Event) => void;
type CloseHandler = (event: CloseEvent) => void;
type ErrorHandler = (event: Event) => void;

type WebSocketHandler =
  | MessageHandler
  | OpenHandler
  | CloseHandler
  | ErrorHandler;
type HandlerMap = Record<string, WebSocketHandler[]>;

// Store WebSocket instances and their handlers by socketId
const sockets: Record<string, WebSocket> = {};
const messageHandlers: Record<string, MessageHandler[]> = {};
const openHandlers: Record<string, OpenHandler[]> = {};
const closeHandlers: Record<string, CloseHandler[]> = {};
const errorHandlers: Record<string, ErrorHandler[]> = {};

const pingIntervals: Record<string, any> = {};

const connect = (socketId: string, url: string) => {
  if (!socketId?.trim()) {
    throw new Error("Invalid socketId");
  }

  if (!url?.trim() || !url.match(/^wss?:\/\/.+/)) {
    throw new Error("Invalid WebSocket URL");
  }

  if (sockets[socketId]) return; // Prevent duplicate connections

  const createSocket = () => {
    try {
      const socket = new WebSocket(url);
      sockets[socketId] = socket;

      socket.addEventListener("open", (event) => onOpen(socketId, event));
      socket.addEventListener("message", (event) => onMessage(socketId, event));
      socket.addEventListener("close", (event) => onClose(socketId, event));
    } catch (error) {
      console.error("Error in creating WebSocket", error);
    }
  };

  createSocket();
};

const onOpen = (socketId: string, event: Event) => {
  openHandlers[socketId]?.forEach((handler) => handler(event));
};

const onMessage = (socketId: string, event: MessageEvent) => {
  const data = JSON.parse(event.data);

  if (data.type === "error") {
    errorHandlers[socketId]?.forEach((handler) => handler(data));
    return;
  }

  messageHandlers[socketId]?.forEach((handler) => handler(data));
};

const onClose = (socketId: string, event: CloseEvent) => {
  console.log("onClose", socketId, event.code, event.reason);
  clearInterval(pingIntervals[socketId]);
  delete pingIntervals[socketId];
  delete sockets[socketId];

  closeHandlers[socketId]?.forEach((handler) => handler(event));
};

const onError = (socketId: string, event: Event) => {
  errorHandlers[socketId]?.forEach((handler) => handler(event));
};

// Message and handler functions
const sendMessage = (socketId: string, message: string) => {
  const socket = sockets[socketId];
  if (socket && socket.readyState === WebSocket.OPEN) {
    socket.send(message);
  } else {
    console.error(
      `WebSocket ${socketId} is not open. Ready state is:`,
      socket?.readyState,
    );
  }
};

const addHandler = (
  handlersMap: HandlerMap,
  socketId: string,
  handler: WebSocketHandler,
) => {
  if (typeof handler !== "function") {
    throw new Error("Handler must be a function");
  }

  if (!handlersMap[socketId]) handlersMap[socketId] = [];
  handlersMap[socketId].push(handler);
};

const removeHandler = (
  handlersMap: HandlerMap,
  socketId: string,
  handler: WebSocketHandler,
) => {
  if (typeof handler !== "function") {
    throw new Error("Handler must be a function");
  }

  const handlers = handlersMap[socketId];
  if (handlers) {
    const index = handlers.indexOf(handler);
    if (index > -1) handlers.splice(index, 1);
  }
};

const closeSocket = (socketId: string) => {
  const socket = sockets[socketId];
  if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
      // sendMessage(socketId, JSON.stringify({ type: "close" }));
      socket.close(1000, "search cancelled");
  }
};

// Composable
const useWebSocket = () => {
  onBeforeUnmount(() => {
    for (const socketId in sockets) {
      const socket = sockets[socketId];
      if (socket) {
        cleanupSocket(socketId, socket);
      }
    }
  });

  const cleanupSocket = (socketId: string, socket: WebSocket) => {
    // Remove all event listeners
    socket.onopen = null;
    socket.onmessage = null;
    socket.onclose = null;
    socket.onerror = null;

    // Close connection if still open
    closeSocket(socketId);

    // Clear intervals
    clearInterval(pingIntervals[socketId]);

    // Clear references
    delete sockets[socketId];
    delete pingIntervals[socketId];
    if (messageHandlers[socketId]?.length) messageHandlers[socketId].length = 0;
    if (openHandlers[socketId]?.length) openHandlers[socketId].length = 0;
    if (closeHandlers[socketId]?.length) closeHandlers[socketId].length = 0;
    if (errorHandlers[socketId]?.length) errorHandlers[socketId].length = 0;
  };

  const getWebSocketBasedOnSocketId = (socketId: string) => {
    return sockets[socketId];
  };

  return {
    connect,
    sendMessage,
    closeSocket,
    getWebSocketBasedOnSocketId,
    addMessageHandler: (socketId: string, handler: MessageHandler) =>
      addHandler(messageHandlers, socketId, handler),
    removeMessageHandler: (socketId: string, handler: MessageHandler) =>
      removeHandler(messageHandlers, socketId, handler),
    addOpenHandler: (socketId: string, handler: OpenHandler) =>
      addHandler(openHandlers, socketId, handler),
    removeOpenHandler: (socketId: string, handler: OpenHandler) =>
      removeHandler(openHandlers, socketId, handler),
    addCloseHandler: (socketId: string, handler: CloseHandler) =>
      addHandler(closeHandlers, socketId, handler),
    removeCloseHandler: (socketId: string, handler: CloseHandler) =>
      removeHandler(closeHandlers, socketId, handler),
    addErrorHandler: (socketId: string, handler: ErrorHandler) =>
      addHandler(errorHandlers, socketId, handler),
    removeErrorHandler: (socketId: string, handler: ErrorHandler) =>
      removeHandler(errorHandlers, socketId, handler),
  };
};

export default useWebSocket;
