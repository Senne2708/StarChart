import { getDefaultStore } from "jotai";
import {
  activitiesAtom,
  connectionErrorAtom,
  connectionStatusAtom,
  studentsAtom,
  teachersAtom,
  tokenAtom,
} from "../atoms";
import type { ClientMsg, ServerMsg, ServerMsgType } from "./protocol";

const store = getDefaultStore();

let socket: WebSocket | null = null;
let nextId = 1;

// Requests waiting for a reply, keyed by message id
const pending = new Map<
  number,
  { resolve: (msg: ServerMsgType) => void; reject: (err: Error) => void }
>();

// Add an item to a list, or replace it if one with the same id exists.
// Needed because when you add something, you get both the reply AND the broadcast.
function upsert<T extends { id: unknown }>(list: T[], item: T): T[] {
  return list.some((x) => x.id === item.id)
    ? list.map((x) => (x.id === item.id ? item : x))
    : [...list, item];
}

// Write whatever the server sent into the right atom
function applyToAtoms(msg: ServerMsgType) {
  switch (msg.type) {
    case "students":
      store.set(studentsAtom, msg.data);
      break;
    case "teachers":
      store.set(teachersAtom, msg.data);
      break;
    case "activities":
      store.set(activitiesAtom, msg.data);
      break;
    case "student_added":
      store.set(studentsAtom, (prev) => upsert(prev, msg.data));
      break;
    case "activity_added":
      store.set(activitiesAtom, (prev) => upsert(prev, msg.data.activity));
      store.set(studentsAtom, (prev) => upsert(prev, msg.data.student)); // updated star total
      break;
    case "error":
      console.error(`Server error (${msg.data.code}): ${msg.data.message}`);
      break;
  }
}

export function connect(token: string) {
  disconnect();

  const protocol = location.protocol === "https:" ? "wss" : "ws";
  const ws = new WebSocket(`${protocol}://${location.host}/ws?token=${encodeURIComponent(token)}`);
  socket = ws;
  let opened = false;

  store.set(connectionStatusAtom, "connecting");
  store.set(connectionErrorAtom, null);

  ws.onopen = () => {
    opened = true;
    store.set(connectionStatusAtom, "connected");
    // Load everything once connected
    request({ type: "get_students" }).catch(console.error);
    request({ type: "get_teachers" }).catch(console.error);
    request({ type: "get_activities" }).catch(console.error);
  };

  ws.onmessage = (event) => {
    const msg: ServerMsg = JSON.parse(event.data);
    applyToAtoms(msg.msg);

    // If this is a reply to one of our requests, finish that request
    if (msg.id !== null) {
      const waiting = pending.get(msg.id);
      if (waiting) {
        pending.delete(msg.id);
        if (msg.msg.type === "error") waiting.reject(new Error(msg.msg.data.message));
        else waiting.resolve(msg.msg);
      }
    }
  };

  ws.onclose = () => {
    if (ws !== socket) return; // an old socket we closed on purpose
    socket = null;
    store.set(connectionStatusAtom, "disconnected");
    pending.forEach((p) => p.reject(new Error("Connection closed")));
    pending.clear();

    if (!opened) {
      // Rejected straight away: token expired/invalid, or server not running
      store.set(connectionErrorAtom, "Couldn't connect. Is the server running? Please log in again.");
      store.set(tokenAtom, null);
    }
  };
}

export function disconnect() {
  const ws = socket;
  socket = null; // set first so onclose ignores this socket
  ws?.close();
}

// Send a message and wait for the server's reply to it
export function request(msg: ClientMsg): Promise<ServerMsgType> {
  return new Promise((resolve, reject) => {
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      reject(new Error("Not connected to the server"));
      return;
    }
    const id = nextId++;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, ...msg }));
  });
}