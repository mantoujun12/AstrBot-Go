/**
 * Chat window service: typed wrappers for the pinned backend contract and the
 * window plumbing (`open_chat_window`, `chat_window_target`).
 *
 * Stream events from the backend are validated and normalized here; dashboard
 * API traffic is forwarded raw through `dashboard_api_request`.
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
  | { type: "agent_stats"; data: unknown }
  | { type: "complete"; data: unknown }
  | { type: "done" }
  | { type: "error"; message: string };

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
  /** Null starts a new session; the backend creates it and emits `session`. */
  sessionId: string | null;
  message: string;
  /** Owner provider of the selected model; AstrBot resolves both together. */
  provider: string | null;
  model: string | null;
  /**
   * `send` (default) posts a new message; `regenerate` re-runs the turn of
   * `targetMessageId` inside `sessionId` without creating a new session.
   */
  mode?: "send" | "regenerate";
  /** History message the regenerate mode re-runs; ignored by `send`. */
  targetMessageId?: string | number | null;
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
    case "agent_stats":
      return { type: "agent_stats", data: value.data };
    case "complete":
      return { type: "complete", data: value.data };
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

/**
 * Forwards one authenticated dashboard API request through Rust. `path` is
 * rooted at the instance dashboard, e.g. `/api/v1/chat/sessions`; the backend
 * enforces the allowlist. Resolves with the raw JSON envelope
 * `{ status, message, data }`.
 */
export function dashboardApiRequest(
  instanceId: string,
  method: string,
  path: string,
  body?: unknown,
): Promise<unknown> {
  return invoke<unknown>("dashboard_api_request", { instanceId, method, path, body });
}

/** One media file fetched through the authenticated dashboard proxy. */
export interface ChatMediaPayload {
  mime: string;
  base64: string;
}

/**
 * Fetches one chat attachment as base64. The dashboard origin and JWT stay in
 * Rust; the island turns the payload into a blob URL.
 */
export function fetchChatMedia(
  instanceId: string,
  path: string,
): Promise<ChatMediaPayload> {
  return invoke<ChatMediaPayload>("chat_fetch_media", { instanceId, path });
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
    mode: input.mode ?? null,
    targetMessageId: input.targetMessageId == null ? null : String(input.targetMessageId),
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
