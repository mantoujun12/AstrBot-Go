// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island glue: maps the upstream axios call surface onto Tauri commands.
// All traffic goes through Rust (the API key never enters the webview), so
// this adapter fabricates axios-shaped responses from `chat_list_providers`
// and `chat_list_sessions` and rejects every endpoint we have no command for.

import { AxiosError, type AxiosAdapter, type AxiosResponse, type InternalAxiosRequestConfig } from "axios";
import {
  listChatProviders,
  listChatSessions,
  type AstrBotProvider,
  type AstrBotSession,
} from "../../services/chat";

/** Instance the adapter currently serves; set by the host on every change. */
let currentInstanceId = "";

export function setChatInstance(instanceId: string): void {
  currentInstanceId = instanceId;
}

/** Instance the adapter currently serves. */
export function getChatInstance(): string {
  return currentInstanceId;
}

interface Envelope {
  status: "ok";
  message: string | null;
  data: unknown;
  [key: string]: unknown;
}

function envelope(data: unknown, extra: Record<string, unknown> = {}): Envelope {
  return { status: "ok", message: null, data, ...extra };
}

function axiosResponse(config: InternalAxiosRequestConfig, body: Envelope): AxiosResponse<Envelope> {
  return {
    data: body,
    status: 200,
    statusText: "OK",
    headers: {},
    config,
  };
}

function rejectAdapter(config: InternalAxiosRequestConfig, message: string): never {
  const response: AxiosResponse<Envelope> = {
    data: { status: "error", message, data: null },
    status: 501,
    statusText: "Not Implemented",
    headers: {},
    config,
  };
  throw new AxiosError(message, "ERR_UNSUPPORTED", config, undefined, response);
}

type ProviderRow = {
  id: string;
  name: string;
  enable: boolean;
  model?: string;
};

function toProviderRows(providers: AstrBotProvider[]): ProviderRow[] {
  return providers.map((provider) => ({
    id: provider.id,
    name: provider.name,
    enable: provider.enabled,
    model: provider.models[0],
  }));
}

const EPOCH = "1970-01-01T00:00:00";

function toSessionRows(sessions: AstrBotSession[]): Record<string, unknown>[] {
  return sessions.map((session) => ({
    session_id: session.id,
    display_name: session.title === session.id ? null : session.title,
    updated_at: EPOCH,
    created_at: EPOCH,
    platform_id: "webchat",
    creator: "astrbot-go",
    is_group: 0,
  }));
}

function sessionDetail(sessionId: string): Record<string, unknown> {
  return {
    session_id: sessionId,
    display_name: null,
    updated_at: EPOCH,
    created_at: EPOCH,
    platform_id: "webchat",
    creator: "astrbot-go",
    is_group: 0,
  };
}

/**
 * Route one upstream request onto our Tauri commands. Reads with a matching
 * command are reshaped into their envelopes; everything else is rejected so
 * the UI's own catch paths (toasts, console) handle it.
 */
export const chatuiAdapter: AxiosAdapter = async (config) => {
  const method = (config.method || "get").toLowerCase();
  // axios hands the adapter the relative url plus baseURL separately.
  const relative = (config.url || "").split("?")[0];
  const url = `${(config.baseURL || "").replace(/\/+$/, "")}${relative}`;
  const instance = currentInstanceId;
  if (instance === "") {
    return rejectAdapter(config, "No chat instance selected.");
  }

  if (method === "get" && url === "/api/v1/providers") {
    const providers = await listChatProviders(instance);
    return axiosResponse(
      config,
      envelope({ providers: toProviderRows(providers), model_metadata: {} }),
    );
  }

  if (method === "get" && url === "/api/v1/chat/sessions") {
    const sessions = await listChatSessions(instance);
    const rows = toSessionRows(sessions);
    return axiosResponse(
      config,
      envelope(
        { sessions: rows, page: 1, page_size: Math.max(30, rows.length), total: rows.length },
      ),
    );
  }

  const sessionMatch = url.match(/^\/api\/v1\/chat\/sessions\/([^/]+)$/);
  if (method === "get" && sessionMatch) {
    // shortcut: no history command exists yet, so transcripts load as empty.
    return axiosResponse(
      config,
      envelope({
        history: [],
        threads: [],
        active_runs: [],
        page: 1,
        page_size: 50,
        total: 0,
        has_more: false,
        session: sessionDetail(decodeURIComponent(sessionMatch[1])),
      }),
    );
  }

  if (method === "get" && url === "/api/v1/commands") {
    return axiosResponse(config, envelope({ items: [], wake_prefix: ["/"], summary: {} }));
  }

  if (method === "get" && url === "/api/v1/config-profiles") {
    return axiosResponse(config, envelope({ info_list: [] }));
  }

  const profileMatch = url.match(/^\/api\/v1\/config-profiles\/([^/]+)$/);
  if (method === "get" && profileMatch) {
    return axiosResponse(config, envelope({ config: {} }));
  }

  if (method === "get" && url === "/api/v1/config-routes") {
    return axiosResponse(config, envelope({ routing: {} }));
  }

  return rejectAdapter(config, "Not available in this window.");
};
