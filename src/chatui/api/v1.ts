// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island glue: trimmed port of dashboard/src/api/v1.ts. Only the objects and
// methods the chat island actually calls exist; each one keeps the upstream
// call signature and response envelope so ported code paths are unmodified.

import { apiV1Client } from "./http";

export interface ApiEnvelope<T> {
  status: "ok" | "error";
  message?: string | null;
  data: T;
}

type AnyResponse = { data: { status: string; message?: string | null; data: any } };

export const chatApi = {
  sendStreamUrl() {
    return "/api/v1/chat";
  },
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
  regenerateMessage(
    sessionId: string,
    messageId: string | number,
    payload?: Record<string, unknown>,
  ) {
    return apiV1Client.post<AnyResponse["data"]>(
      `/chat/sessions/${encodeURIComponent(sessionId)}/messages/${encodeURIComponent(String(messageId))}/regenerate`,
      payload,
    );
  },
  createThread(payload: Record<string, unknown>) {
    return apiV1Client.post<AnyResponse["data"]>("/chat/threads", payload);
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
  // Attachment transport is stubbed; URLs are never dereferenced in practice.
  contentUrl(_attachmentId: string) {
    return "";
  },
  byNameUrl(_filename: string) {
    return "";
  },
  getByName(_filename: string) {
    return Promise.reject(new Error("Not available in this window."));
  },
};
