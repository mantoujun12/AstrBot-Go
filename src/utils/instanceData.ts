/**
 * Pure helpers for staging instance-data edits in the management dialog.
 */

export interface RecordDiff<T> {
  upserts: T[];
  deletes: string[];
}

/**
 * Splits staged records against the loaded snapshot into upserts (added or
 * changed records) and deletes (loaded ids missing from the staged view).
 * Records added and then removed before saving produce neither.
 */
export function diffRecords<T>(
  original: readonly T[],
  current: readonly T[],
  idOf: (item: T) => string,
): RecordDiff<T> {
  const originalById = new Map(original.map((item) => [idOf(item), item]));
  const upserts = current.filter((item) => {
    const before = originalById.get(idOf(item));
    return before === undefined || JSON.stringify(before) !== JSON.stringify(item);
  });
  const currentIds = new Set(current.map(idOf));
  const deletes = [...originalById.keys()].filter((id) => !currentIds.has(id));
  return { upserts, deletes };
}

/**
 * Parses a comma/newline separated field into ids. Empty input means `null`
 * ("all"); explicit entries become a list.
 */
export function parseListField(raw: string): string[] | null {
  const entries = raw
    .split(/[,\n]/)
    .map((entry) => entry.trim())
    .filter((entry) => entry !== "");
  return entries.length > 0 ? entries : null;
}

/** One entry per line, empty lines dropped. */
export function parseLinesField(raw: string): string[] {
  return raw
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

function stableStringify(value: unknown): string {
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(",")}]`;
  }
  if (typeof value === "object" && value !== null) {
    const entries = Object.keys(value)
      .sort()
      .map(
        (key) =>
          `${JSON.stringify(key)}:${stableStringify(
            (value as Record<string, unknown>)[key],
          )}`,
      );
    return `{${entries.join(",")}}`;
  }
  return JSON.stringify(value);
}

/**
 * Content-level comparison for staged config files: JSON files compare parsed
 * values (whitespace and key order ignored, matching AstrBot's config editor
 * where reverting a change clears the dirty state), everything else compares
 * raw text. Unparseable JSON falls back to text equality.
 */
export function isSameConfigContent(
  name: string,
  original: string,
  staged: string,
): boolean {
  if (original === staged) {
    return true;
  }
  if (!name.endsWith(".json")) {
    return false;
  }
  try {
    const parsedOriginal: unknown = JSON.parse(original);
    const parsedStaged: unknown = JSON.parse(staged);
    return stableStringify(parsedOriginal) === stableStringify(parsedStaged);
  } catch {
    return false;
  }
}

// In-source tests, stripped from the production bundle by the
// `import.meta.vitest` define in vite.config.ts.
if (import.meta.vitest) {
  const { describe, expect, it } = import.meta.vitest;

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
}
