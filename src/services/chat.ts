/**
 * Chat window service: typed wrappers for the pinned backend contract and the
 * window plumbing (`open_chat_window`, `chat_window_target`).
 *
 * All payloads coming from the backend are validated and normalized into the
 * explicit domain types below; nothing is cast through `any`.
 */

import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type ChatStreamEvent =
  | { type: "session"; session_id: string }
  | {
      type: "delta";
      text: string;
      /** Set for reasoning / tool-call output; absent for normal reply text. */
      chain?: string | null;
    }
  | { type: "done" }
  | { type: "error"; message: string };

export interface AstrBotProvider {
  id: string;
  name: string;
  enabled: boolean;
  models: string[];
}

export interface AstrBotSession {
  id: string;
  title: string;
}

/** One rendered bubble in the chat transcript. */
export interface ChatMessage {
  id: number;
  role: "user" | "bot";
  content: string;
  /** True while stream deltas may still arrive for this bubble. */
  streaming: boolean;
  /** True when the reply ended in a stream error; content holds the reason. */
  failed: boolean;
}

export interface ChatSendInput {
  instanceId: string;
  sessionId: string | null;
  message: string;
  /** Owner provider of the selected model; AstrBot resolves both together. */
  provider: string | null;
  model: string | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function toChatStreamEvent(value: unknown): ChatStreamEvent | null {
  if (!isRecord(value)) {
    return null;
  }
  switch (value.type) {
    case "session":
      return typeof value.session_id === "string" && value.session_id !== ""
        ? { type: "session", session_id: value.session_id }
        : null;
    case "delta":
      return {
        type: "delta",
        text: readString(value.text),
        chain: typeof value.chain === "string" && value.chain !== ""
          ? value.chain
          : null,
      };
    case "done":
      return { type: "done" };
    case "error":
      return {
        type: "error",
        message: readString(value.message),
      };
    default:
      return null;
  }
}

function toProvider(value: unknown): AstrBotProvider | null {
  if (!isRecord(value) || typeof value.id !== "string") {
    return null;
  }
  const models = Array.isArray(value.models)
    ? value.models.filter((model): model is string => typeof model === "string")
    : [];
  return {
    id: value.id,
    name: readString(value.name) || value.id,
    enabled: value.enabled === true,
    models,
  };
}

function toSession(value: unknown): AstrBotSession | null {
  if (!isRecord(value) || typeof value.id !== "string") {
    return null;
  }
  return {
    id: value.id,
    title: readString(value.title) || value.id,
  };
}

export async function listChatProviders(
  instanceId: string,
): Promise<AstrBotProvider[]> {
  const raw = await invoke<unknown>("chat_list_providers", { instanceId });
  if (!Array.isArray(raw)) {
    return [];
  }
  return raw
    .map(toProvider)
    .filter((provider): provider is AstrBotProvider => provider !== null);
}

export async function listChatSessions(
  instanceId: string,
): Promise<AstrBotSession[]> {
  const raw = await invoke<unknown>("chat_list_sessions", { instanceId });
  if (!Array.isArray(raw)) {
    return [];
  }
  return raw
    .map(toSession)
    .filter((session): session is AstrBotSession => session !== null);
}

/**
 * Sends one message and resolves when the stream completes. Streamed events
 * are delivered through a Tauri IPC channel; malformed events are dropped.
 */
export function sendChatMessage(
  input: ChatSendInput,
  onEvent: (event: ChatStreamEvent) => void,
): Promise<void> {
  const channel = new Channel<unknown>();
  channel.onmessage = (message) => {
    const event = toChatStreamEvent(message);
    if (event) {
      onEvent(event);
    }
  };
  return invoke("chat_send_message", {
    instanceId: input.instanceId,
    sessionId: input.sessionId,
    message: input.message,
    provider: input.provider,
    model: input.model,
    onEvent: channel,
  });
}

/**
 * Opens (or focuses) the chat window. An `instanceId` switches the window to
 * that instance; omitting it keeps the current conversation.
 */
export function openChatWindow(instanceId: string | null = null): Promise<void> {
  return invoke("open_chat_window", { instanceId });
}

export function getChatWindowTarget(): Promise<string> {
  return invoke<string>("chat_window_target");
}

/** The window target is emitted with `emit_to`, so only the chat window sees it. */
export function listenChatTargetChanged(
  handler: (instanceId: string) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("chat-target-changed", (event) => {
    if (typeof event.payload === "string" && event.payload !== "") {
      handler(event.payload);
    }
  });
}

/** Whether this webview is the standalone chat window. */
export function isChatWindow(): boolean {
  try {
    return getCurrentWindow().label === "chat";
  } catch {
    // Outside Tauri (browser dev preview) there is no window label.
    return false;
  }
}
