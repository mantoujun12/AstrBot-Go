import { describe, expect, it } from "vitest";

import {
  diffRecords,
  isSameConfigContent,
  parseLinesField,
  parseListField,
} from "./instanceData";

describe("diffRecords", () => {
  it("marks added and changed records as upserts", () => {
    const original = [{ id: "a", v: 1 }];
    const current = [
      { id: "a", v: 2 },
      { id: "b", v: 1 },
    ];
    expect(diffRecords(original, current, (item) => item.id)).toEqual({
      upserts: [
        { id: "a", v: 2 },
        { id: "b", v: 1 },
      ],
      deletes: [],
    });
  });

  it("reports removed records as deletes", () => {
    const original = [
      { id: "a", v: 1 },
      { id: "b", v: 1 },
    ];
    const diff = diffRecords(original, [{ id: "a", v: 1 }], (item) => item.id);
    expect(diff).toEqual({ upserts: [], deletes: ["b"] });
  });

  it("drops records added and deleted before saving", () => {
    const original = [{ id: "a", v: 1 }];
    const diff = diffRecords(original, [], (item) => item.id);
    // "a" removed; an added-then-removed "c" leaves no trace.
    expect(diff).toEqual({ upserts: [], deletes: ["a"] });
    const withTemp = [
      { id: "a", v: 1 },
      { id: "c", v: 1 },
    ];
    expect(
      diffRecords(original, withTemp.slice(0, 1), (item) => item.id),
    ).toEqual({ upserts: [], deletes: [] });
  });

  it("returns nothing when the staged view matches the snapshot", () => {
    const original = [{ id: "a", v: 1 }];
    expect(diffRecords(original, [{ id: "a", v: 1 }], (item) => item.id)).toEqual(
      { upserts: [], deletes: [] },
    );
  });
});

describe("parseListField", () => {
  it("returns null for empty input meaning all", () => {
    expect(parseListField("")).toBe(null);
    expect(parseListField("  \n ")).toBe(null);
  });

  it("splits on commas and newlines and trims entries", () => {
    expect(parseListField("a, b\nc ")).toEqual(["a", "b", "c"]);
  });
});

describe("parseLinesField", () => {
  it("drops empty lines", () => {
    expect(parseLinesField("a\n\n  b \n")).toEqual(["a", "b"]);
  });
});

describe("isSameConfigContent", () => {
  it("treats re-serialized but semantically equal JSON as unchanged", () => {
    expect(
      isSameConfigContent(
        "mcp_server.json",
        '{"mcpServers":{"a":{"command":"uv","active":true}}}',
        '{\n  "mcpServers": {\n    "a": {\n      "active": true,\n      "command": "uv"\n    }\n  }\n}',
      ),
    ).toBe(true);
  });

  it("detects semantic changes in JSON", () => {
    expect(
      isSameConfigContent(
        "mcp_server.json",
        '{"mcpServers":{"a":{"command":"uv"}}}',
        '{"mcpServers":{"a":{"command":"uvx"}}}',
      ),
    ).toBe(false);
    expect(
      isSameConfigContent(
        "mcp_server.json",
        '{"mcpServers":{}}',
        '{"mcpServers":{"a":{"command":"uv"}}}',
      ),
    ).toBe(false);
  });

  it("compares non-JSON files as raw text", () => {
    expect(isSameConfigContent("config/abconf_a.yaml", "a: 1\n", "a: 1\n")).toBe(
      true,
    );
    expect(isSameConfigContent("config/abconf_a.yaml", "a: 1\n", "a: 2\n")).toBe(
      false,
    );
  });

  it("falls back to text comparison for unparseable JSON", () => {
    expect(isSameConfigContent("a.json", "{", "{")).toBe(true);
    expect(isSameConfigContent("a.json", "{", "}")).toBe(false);
  });
});
