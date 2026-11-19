// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Tests for the island's Tauri-backed axios adapter (pure reshape logic).

import { AxiosError } from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  listChatProviders,
  listChatSessions,
} from "../../services/chat";
import { chatuiAdapter, setChatInstance } from "./adapter";

vi.mock("../../services/chat", () => ({
  listChatProviders: vi.fn(),
  listChatSessions: vi.fn(),
}));

const mockedProviders = vi.mocked(listChatProviders);
const mockedSessions = vi.mocked(listChatSessions);

/** Minimal axios config shaped like what axios hands a custom adapter. */
function config(method: string, url: string) {
  return { method, url, baseURL: "/api/v1", headers: {} } as never;
}

beforeEach(() => {
  setChatInstance("instance-1");
  mockedProviders.mockResolvedValue([
    { id: "openai/gpt-4o", name: "gpt-4o", enabled: true, models: ["gpt-4o"] },
    { id: "off", name: "off", enabled: false, models: [] },
  ]);
  mockedSessions.mockResolvedValue([
    { id: "s1", title: "First chat" },
    { id: "s2", title: "s2" },
  ]);
});

describe("chatuiAdapter", () => {
  it("resolves the baseURL-joined provider url and reshapes rows", async () => {
    const response = await chatuiAdapter(config("get", "/providers"));
    expect(response.status).toBe(200);
    const body = response.data as {
      status: string;
      data: { providers: Array<Record<string, unknown>> };
    };
    expect(body.status).toBe("ok");
    const providers = body.data.providers;
    expect(providers).toHaveLength(2);
    expect(providers[0]).toMatchObject({
      id: "openai/gpt-4o",
      enable: true,
      model: "gpt-4o",
    });
    expect(providers[1]).toMatchObject({ enable: false, model: undefined });
  });

  it("resolves the sessions url with upstream row fields", async () => {
    const response = await chatuiAdapter(config("get", "/chat/sessions"));
    const body = response.data as {
      data: {
        sessions: Array<Record<string, unknown>>;
        page_size: number;
        total: number;
      };
    };
    const sessions = body.data.sessions;
    expect(sessions[0]).toMatchObject({
      session_id: "s1",
      display_name: "First chat",
      platform_id: "webchat",
      is_group: 0,
    });
    // A title that only mirrors the id is reported as unnamed, like upstream.
    expect(sessions[1].display_name).toBeNull();
    // One page covers the whole list so upstream pagination stops immediately.
    expect(body.data.page_size).toBeGreaterThanOrEqual(30);
    expect(body.data.total).toBe(2);
  });

  it("rejects endpoints without a matching command", async () => {
    await expect(chatuiAdapter(config("post", "/chat/sessions"))).rejects
      .toMatchObject({
        isAxiosError: true,
        response: { status: 501 },
      });
  });

  it("rejects reads before an instance is selected", async () => {
    setChatInstance("");
    const error = await chatuiAdapter(config("get", "/providers")).catch(
      (cause: unknown) => cause,
    );
    expect(error).toBeInstanceOf(AxiosError);
    expect((error as AxiosError).response?.status).toBe(501);
    expect(String((error as AxiosError).message)).not.toBe("");
  });
});
