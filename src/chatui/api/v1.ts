// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island glue: trimmed port of dashboard/src/api/v1.ts. Only the objects and
// methods the chat island actually calls exist; each one keeps the upstream
// call signature and response envelope so ported code paths are unmodified.

import { apiV1Client } from "./http";
import { getChatInstance } from "./adapter";
import { fetchChatMedia } from "../../main/services/chat";

export interface ApiEnvelope<T> {
  status: "ok" | "error";
  message?: string | null;
  data: T;
}

type AnyResponse = { data: { status: string; message?: string | null; data: any } };

export const chatApi = {
  listSessions(params?: { page?: number; page_size?: number; username?: string }) {
    return apiV1Client.get<AnyResponse["data"]>("/chat/sessions", { params });
  },
  getSession(sessionId: string, params?: { page?: number; page_size?: number }) {
    return apiV1Client.get<AnyResponse["data"]>(
      `/chat/sessions/${encodeURIComponent(sessionId)}`,
      { params },
    );
  },
  updateSession(sessionId: string, payload: Record<string, unknown>) {
    return apiV1Client.patch<AnyResponse["data"]>(
      `/chat/sessions/${encodeURIComponent(sessionId)}`,
      payload,
    );
  },
  deleteSession(sessionId: string) {
    return apiV1Client.delete<AnyResponse["data"]>(
      `/chat/sessions/${encodeURIComponent(sessionId)}`,
    );
  },
  stopSession(sessionId: string) {
    return apiV1Client.post<AnyResponse["data"]>(
      `/chat/sessions/${encodeURIComponent(sessionId)}/stop`,
    );
  },
  updateMessage(
    sessionId: string,
    messageId: string | number,
    payload: Record<string, unknown>,
  ) {
    return apiV1Client.patch<AnyResponse["data"]>(
      `/chat/sessions/${encodeURIComponent(sessionId)}/messages/${encodeURIComponent(String(messageId))}`,
      payload,
    );
  },
  createThread(payload: Record<string, unknown>) {
    return apiV1Client.post<AnyResponse["data"]>("/chat/threads", payload);
  },
  getThread(threadId: string) {
    return apiV1Client.get<AnyResponse["data"]>(
      `/chat/threads/${encodeURIComponent(threadId)}`,
    );
  },
  // shortcut: the thread panel is gated off (`CHATUI_FEATURES.threads`) and
  // its send path still uses the browser fetch helper, so this URL only keeps
  // the ported panel type-complete until threads get a Tauri stream.
  sendThreadMessageUrl(threadId: string) {
    return `/api/v1/chat/threads/${encodeURIComponent(threadId)}/messages`;
  },
  deleteThread(threadId: string) {
    return apiV1Client.delete<AnyResponse["data"]>(
      `/chat/threads/${encodeURIComponent(threadId)}`,
    );
  },
};

export const providerApi = {
  list(params?: Record<string, unknown>) {
    return apiV1Client.get<AnyResponse["data"]>("/providers", { params });
  },
  async listByProviderType(providerType: string) {
    const response = await providerApi.list({ capability: "chat", type: providerType });
    return {
      ...response,
      data: {
        ...response.data,
        // Upstream flattens provider rows into `data` with metadata alongside.
        data: response.data.data?.providers || [],
        model_metadata: response.data.data?.model_metadata || {},
      },
    };
  },
  schema() {
    return apiV1Client.get<AnyResponse["data"]>("/providers/schema");
  },
  test(providerId: string) {
    return apiV1Client.post<AnyResponse["data"]>("/providers/test", {
      provider_id: providerId,
    });
  },
};

export const commandApi = {
  list(configId?: string) {
    return apiV1Client.get<AnyResponse["data"]>("/commands", {
      params: configId ? { config_id: configId } : undefined,
    });
  },
};

export const configProfileApi = {
  list() {
    return apiV1Client.get<AnyResponse["data"]>("/config-profiles");
  },
  get(configId: string) {
    return apiV1Client.get<AnyResponse["data"]>(
      `/config-profiles/${encodeURIComponent(configId)}`,
    );
  },
};

export const configRouteApi = {
  list() {
    return apiV1Client.get<AnyResponse["data"]>("/config-routes");
  },
  upsert(umo: string, payload: Record<string, unknown>) {
    return apiV1Client.post<AnyResponse["data"]>(
      `/config-routes/${encodeURIComponent(umo)}`,
      payload,
    );
  },
};

export const fileApi = {
  // Media bytes cannot ride the JSON proxy, so both loaders return blobs
  // fetched through the dedicated media command.
  loadContent(attachmentId: string) {
    return loadMedia(`/api/v1/files/${encodeURIComponent(attachmentId)}/content`);
  },
  loadByName(filename: string) {
    return loadMedia(
      `/api/v1/files/content?filename=${encodeURIComponent(filename)}`,
    );
  },
};

async function loadMedia(path: string): Promise<Blob> {
  const media = await fetchChatMedia(getChatInstance(), path);
  return base64ToBlob(media.base64, media.mime);
}

function base64ToBlob(base64: string, mime: string): Blob {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return new Blob([bytes], { type: mime });
}
