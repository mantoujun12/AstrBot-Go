// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Tests for the island's pass-through axios adapter.

import { AxiosError } from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { dashboardApiRequest } from "../../services/chat";
import { chatuiAdapter, setChatInstance } from "./adapter";

vi.mock("../../services/chat", () => ({
  dashboardApiRequest: vi.fn(),
}));

const mockedRequest = vi.mocked(dashboardApiRequest);

/** Minimal axios config shaped like what axios hands a custom adapter. */
function config(
  method: string,
  url: string,
  data?: unknown,
  params?: Record<string, unknown>,
) {
  return { method, url, baseURL: "/api/v1", headers: {}, data, params } as never;
}

beforeEach(() => {
  setChatInstance("instance-1");
  mockedRequest.mockReset();
  mockedRequest.mockResolvedValue({ status: "ok", message: null, data: {} });
});

describe("chatuiAdapter", () => {
  it("forwards the joined path and uppercase method", async () => {
    await chatuiAdapter(config("get", "/chat/sessions"));
    expect(mockedRequest).toHaveBeenCalledWith(
      "instance-1",
      "GET",
      "/api/v1/chat/sessions",
      undefined,
    );
  });

  it("rebuilds the query string from axios params", async () => {
    await chatuiAdapter(
      config("get", "/chat/sessions", undefined, { page: 1, page_size: 30 }),
    );
    expect(mockedRequest).toHaveBeenCalledWith(
      "instance-1",
      "GET",
      "/api/v1/chat/sessions?page=1&page_size=30",
      undefined,
    );
  });

  it("skips empty params and repeats array params", async () => {
    await chatuiAdapter(
      config("get", "/providers", undefined, {
        capability: "chat",
        type: null,
        ids: ["a", "b"],
      }),
    );
    expect(mockedRequest).toHaveBeenCalledWith(
      "instance-1",
      "GET",
      "/api/v1/providers?capability=chat&ids=a&ids=b",
      undefined,
    );
  });

  it("keeps an existing query string when appending params", async () => {
    await chatuiAdapter(config("post", "/chat/threads?x=1", undefined, { y: 2 }));
    expect(mockedRequest).toHaveBeenCalledWith(
      "instance-1",
      "POST",
      "/api/v1/chat/threads?x=1&y=2",
      undefined,
    );
  });

  it("parses the serialized json body back into a value", async () => {
    await chatuiAdapter(
      config("patch", "/chat/sessions/s1", JSON.stringify({ display_name: "New" })),
    );
    expect(mockedRequest).toHaveBeenCalledWith(
      "instance-1",
      "PATCH",
      "/api/v1/chat/sessions/s1",
      { display_name: "New" },
    );
  });

  it("passes a non-json body through as raw text", async () => {
    await chatuiAdapter(config("post", "/chat/threads", "not json"));
    expect(mockedRequest).toHaveBeenCalledWith(
      "instance-1",
      "POST",
      "/api/v1/chat/threads",
      "not json",
    );
  });

  it("returns the raw dashboard envelope with a 200", async () => {
    mockedRequest.mockResolvedValue({
      status: "error",
      message: "session not found",
      data: null,
    });
    const response = await chatuiAdapter(config("get", "/chat/sessions/s1"));
    expect(response.status).toBe(200);
    expect(response.data).toEqual({
      status: "error",
      message: "session not found",
      data: null,
    });
  });

  it("maps a backend failure to an axios error with the message", async () => {
    mockedRequest.mockRejectedValue("dashboard credentials missing");
    const error = await chatuiAdapter(config("get", "/providers")).catch(
      (cause: unknown) => cause,
    );
    expect(error).toBeInstanceOf(AxiosError);
    expect((error as AxiosError).response?.status).toBe(502);
    expect((error as AxiosError).response?.data).toMatchObject({
      status: "error",
      message: "dashboard credentials missing",
    });
  });

  it("falls back to a readable message for a non-string failure", async () => {
    mockedRequest.mockRejectedValue({});
    const error = await chatuiAdapter(config("get", "/providers")).catch(
      (cause: unknown) => cause,
    );
    expect(String((error as AxiosError).message)).toBe("Dashboard request failed.");
  });

  it("rejects before an instance is selected", async () => {
    setChatInstance("");
    const error = await chatuiAdapter(config("get", "/providers")).catch(
      (cause: unknown) => cause,
    );
    expect(error).toBeInstanceOf(AxiosError);
    expect((error as AxiosError).response?.status).toBe(400);
    expect(mockedRequest).not.toHaveBeenCalled();
  });
});
