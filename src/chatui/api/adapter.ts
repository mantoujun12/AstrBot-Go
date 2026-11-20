// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island glue: routes every ported axios request through the Rust dashboard
// proxy, which holds the instance's JWT and enforces the path allowlist. The
// adapter only rebuilds the wire request and returns the server's raw JSON
// envelope; dashboard failures arrive inside it with `status: "error"`.

import { AxiosError, type AxiosAdapter, type AxiosResponse, type InternalAxiosRequestConfig } from "axios";
import { dashboardApiRequest } from "../../services/chat";

/** Instance the adapter currently serves; set by the host on every change. */
let currentInstanceId = "";

export function setChatInstance(instanceId: string): void {
  currentInstanceId = instanceId;
}

/** Instance the adapter currently serves. */
export function getChatInstance(): string {
  return currentInstanceId;
}

function axiosResponse(config: InternalAxiosRequestConfig, body: unknown): AxiosResponse<unknown> {
  return {
    data: body,
    status: 200,
    statusText: "OK",
    headers: {},
    config,
  };
}

function rejectAdapter(
  config: InternalAxiosRequestConfig,
  message: string,
  status: number,
  statusText: string,
  code: string,
): never {
  const response: AxiosResponse = {
    data: { status: "error", message, data: null },
    status,
    statusText,
    headers: {},
    config,
  };
  throw new AxiosError(message, code, config, undefined, response);
}

/** Backend command failures arrive as strings; keep a readable message. */
function failureText(cause: unknown): string {
  if (typeof cause === "string" && cause !== "") return cause;
  if (cause instanceof Error && cause.message !== "") return cause.message;
  return "Dashboard request failed.";
}

/** axios hands the adapter the url and params separately; rebuild the query. */
function buildPath(config: InternalAxiosRequestConfig): string {
  const base = `${(config.baseURL || "").replace(/\/+$/, "")}${config.url || ""}`;
  const params = config.params as Record<string, unknown> | undefined | null;
  if (!params) return base;
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    for (const item of Array.isArray(value) ? value : [value]) {
      if (item === undefined || item === null) continue;
      search.append(key, String(item));
    }
  }
  const query = search.toString();
  if (query === "") return base;
  return `${base}${base.includes("?") ? "&" : "?"}${query}`;
}

/**
 * axios already serialized a JSON body into a string; the proxy forwards a
 * JSON value, so parse it back. Anything unparseable travels as raw text.
 */
function toBody(data: unknown): unknown {
  if (data === undefined || data === null || data === "") return undefined;
  if (typeof data !== "string") return data;
  try {
    return JSON.parse(data) as unknown;
  } catch {
    return data;
  }
}

/**
 * Pass-through transport: one dashboard request per axios call, restricted by
 * the backend allowlist (chat, providers, configs, personas).
 */
export const chatuiAdapter: AxiosAdapter = async (config) => {
  const instance = currentInstanceId;
  if (instance === "") {
    return rejectAdapter(config, "No chat instance selected.", 400, "Bad Request", "ERR_BAD_REQUEST");
  }

  const method = (config.method || "get").toUpperCase();
  let body: unknown;
  try {
    body = await dashboardApiRequest(instance, method, buildPath(config), toBody(config.data));
  } catch (cause) {
    return rejectAdapter(config, failureText(cause), 502, "Bad Gateway", "ERR_BAD_RESPONSE");
  }
  return axiosResponse(config, body);
};
