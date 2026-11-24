import { afterEach, describe, expect, it } from "vitest";

import type { DiscoveredInstance, InstanceStatus } from "../types/launcher";
import { readCachedListPrefs } from "../utils/instanceListCache";
import {
  compareVersionsDesc,
  externalRunningPort,
  filterInstanceRows,
  sortInstanceRows,
  type ExternalInstanceRow,
  type InstanceRow,
} from "./useInstanceList";

const launcherBase: InstanceStatus = {
  id: "launcher-row",
  name: "Instance",
  state: "stopped",
  port: 6185,
  version: "3.9.0",
  dashboard_enabled: true,
  dashboard_credentials_configured: true,
  pid_tracker_not_available: false,
  configured_host: "127.0.0.1",
  configured_port: 6185,
  check_update_enabled: true,
  auto_restart_enabled: false,
  auto_restart_max_retries: 3,
  auto_restart_notify: false,
  last_exit_reason: "",
  restart_attempts: 0,
  restarts_completed: 0,
  started_at_ms: null,
};

function launcherRow(overrides: Partial<InstanceStatus> = {}): InstanceRow {
  return { ...launcherBase, ...overrides };
}

function externalRow(
  overrides: Partial<DiscoveredInstance> = {},
): ExternalInstanceRow {
  return {
    source: "docker",
    id: "external-row",
    name: "External",
    webui_url: "http://127.0.0.1:6200/",
    state: "running",
    capabilities: { read_only: true },
    detail: { kind: "docker", host: "127.0.0.1", port: 6200 },
    ...overrides,
    external: true,
  };
}

function names(rows: readonly InstanceRow[]): string[] {
  return rows.map((row) => row.name);
}

describe("readCachedListPrefs", () => {
  // Storage key from src/main/utils/instanceListCache.ts.
  const cacheKey = "launcher.instances.listPrefs";

  afterEach(() => {
    localStorage.removeItem(cacheKey);
  });

  it("defaults hidden columns for caches written before the field existed", () => {
    localStorage.setItem(
      cacheKey,
      JSON.stringify({ viewMode: "cards", sortKey: "nameAsc" }),
    );
    expect(readCachedListPrefs()).toEqual({
      viewMode: "cards",
      sortKey: "nameAsc",
      hiddenColumns: [],
    });
  });

  it("keeps valid hidden columns", () => {
    localStorage.setItem(
      cacheKey,
      JSON.stringify({
        viewMode: "table",
        sortKey: "nameAsc",
        hiddenColumns: ["port"],
      }),
    );
    expect(readCachedListPrefs().hiddenColumns).toEqual(["port"]);
  });

  it("falls back to defaults for unknown hidden column names", () => {
    localStorage.setItem(
      cacheKey,
      JSON.stringify({
        viewMode: "table",
        sortKey: "nameAsc",
        hiddenColumns: ["bogus"],
      }),
    );
    expect(readCachedListPrefs()).toEqual({
      viewMode: "table",
      sortKey: "nameAsc",
      hiddenColumns: [],
    });
  });
});

describe("sortInstanceRows", () => {
  it("sorts names ascending case-insensitively", () => {
    const rows = [
      launcherRow({ name: "banana" }),
      launcherRow({ name: "cherry" }),
      launcherRow({ name: "Apple" }),
    ];
    expect(names(sortInstanceRows(rows, "nameAsc"))).toEqual([
      "Apple",
      "banana",
      "cherry",
    ]);
  });

  it("sorts names descending case-insensitively", () => {
    const rows = [
      launcherRow({ name: "banana" }),
      launcherRow({ name: "cherry" }),
      launcherRow({ name: "Apple" }),
    ];
    expect(names(sortInstanceRows(rows, "nameDesc"))).toEqual([
      "cherry",
      "banana",
      "Apple",
    ]);
  });

  it("orders states running, starting, stopping, stopped", () => {
    const rows = [
      launcherRow({ name: "stopped", state: "stopped" }),
      launcherRow({ name: "stopping", state: "stopping" }),
      launcherRow({ name: "starting", state: "starting" }),
      launcherRow({ name: "running", state: "running" }),
    ];
    expect(names(sortInstanceRows(rows, "state"))).toEqual([
      "running",
      "starting",
      "stopping",
      "stopped",
    ]);
  });

  it("sorts versions descending and keeps versionless rows last", () => {
    const rows = [
      launcherRow({ name: "old", version: "3.9.0" }),
      externalRow({ name: "external" }),
      launcherRow({ name: "patch", version: "3.9.5" }),
      launcherRow({ name: "new", version: "v3.10.0" }),
    ];
    expect(names(sortInstanceRows(rows, "versionDesc"))).toEqual([
      "new",
      "patch",
      "old",
      "external",
    ]);
  });

  it("falls back to a descending string compare for non-numeric versions", () => {
    const rows = [
      launcherRow({ name: "alpha", version: "alpha" }),
      launcherRow({ name: "beta", version: "beta" }),
    ];
    expect(names(sortInstanceRows(rows, "versionDesc"))).toEqual([
      "beta",
      "alpha",
    ]);
  });

  it("sorts ports ascending and keeps rows without a port last", () => {
    const rows = [
      launcherRow({ name: "high", configured_port: 6200 }),
      externalRow({
        name: "docker-port",
        detail: { kind: "docker", host: "127.0.0.1", port: 6150 },
      }),
      launcherRow({ name: "low", configured_port: 6100 }),
      externalRow({
        name: "no-port",
        webui_url: null,
        detail: { kind: "directory", path: "D:/instances/scan" },
      }),
    ];
    expect(names(sortInstanceRows(rows, "portAsc"))).toEqual([
      "low",
      "docker-port",
      "high",
      "no-port",
    ]);
  });

  it("keeps ties on the current row order", () => {
    const rows = [
      launcherRow({ name: "Same", configured_port: 6100 }),
      launcherRow({ name: "Same", configured_port: 6100 }),
      externalRow({ name: "Same", detail: null, webui_url: null }),
    ];
    expect(names(sortInstanceRows(rows, "portAsc"))).toEqual([
      "Same",
      "Same",
      "Same",
    ]);
    // The two launcher rows keep their input order ahead of the external one.
    const sorted = sortInstanceRows(rows, "portAsc");
    expect(sorted[0]).toBe(rows[0]);
    expect(sorted[1]).toBe(rows[1]);
    expect(sorted[2]).toBe(rows[2]);
  });
});

describe("filterInstanceRows", () => {
  const rows = [
    launcherRow({ id: "l-running", name: "Alpha", state: "running" }),
    launcherRow({ id: "l-stopped", name: "Beta", state: "stopped" }),
    launcherRow({ id: "l-starting", name: "Gamma", state: "starting" }),
    externalRow({ id: "e-running", name: "Delta" }),
    externalRow({ id: "e-stopped", name: "Epsilon", state: "stopped" }),
    launcherRow({ id: "l-stopping", name: "Zeta", state: "stopping" }),
  ];

  it("matches search case-insensitively on the name", () => {
    const hit = filterInstanceRows(rows, {
      search: "  aLPh ",
      stateFilter: "all",
      sourceFilter: "all",
    });
    expect(hit.map((row) => row.id)).toEqual(["l-running"]);
  });

  it("keeps every row when the search is blank", () => {
    expect(filterInstanceRows(rows, {
      search: "   ",
      stateFilter: "all",
      sourceFilter: "all",
    })).toHaveLength(rows.length);
  });

  it("keeps transient states visible under their target filter", () => {
    const ids = (stateFilter: "all" | "running" | "stopped") =>
      filterInstanceRows(rows, {
        search: "",
        stateFilter,
        sourceFilter: "all",
      }).map((row) => row.id);

    expect(ids("running")).toEqual(["l-running", "l-starting", "e-running"]);
    expect(ids("stopped")).toEqual(["l-stopped", "e-stopped", "l-stopping"]);
    expect(ids("all")).toEqual([
      "l-running",
      "l-stopped",
      "l-starting",
      "e-running",
      "e-stopped",
      "l-stopping",
    ]);
  });

  it("filters by source", () => {
    const ids = (sourceFilter: "all" | "launcher" | "external") =>
      filterInstanceRows(rows, {
        search: "",
        stateFilter: "all",
        sourceFilter,
      }).map((row) => row.id);

    expect(ids("launcher")).toEqual([
      "l-running",
      "l-stopped",
      "l-starting",
      "l-stopping",
    ]);
    expect(ids("external")).toEqual(["e-running", "e-stopped"]);
    expect(ids("all")).toHaveLength(rows.length);
  });

  it("combines search, state, and source filters", () => {
    expect(
      filterInstanceRows(rows, {
        search: "a",
        stateFilter: "running",
        sourceFilter: "external",
      }).map((row) => row.id),
    ).toEqual(["e-running"]);

    expect(
      filterInstanceRows(rows, {
        search: "zzz",
        stateFilter: "all",
        sourceFilter: "all",
      }),
    ).toEqual([]);
  });
});

describe("compareVersionsDesc", () => {
  it("compares dot-separated numeric segments", () => {
    expect(compareVersionsDesc("3.10.0", "3.9.0")).toBeLessThan(0);
    expect(compareVersionsDesc("3.9.0", "3.10.0")).toBeGreaterThan(0);
    expect(compareVersionsDesc("3.9", "3.9.0")).toBe(0);
  });

  it("sorts missing versions last in either direction", () => {
    expect(compareVersionsDesc(null, "3.9.0")).toBeGreaterThan(0);
    expect(compareVersionsDesc("3.9.0", null)).toBeLessThan(0);
    expect(compareVersionsDesc(null, null)).toBe(0);
  });
});

describe("externalRunningPort", () => {
  it("uses the detail port for docker and uv tool rows", () => {
    expect(
      externalRunningPort({
        ...externalRow(),
        detail: { kind: "docker", host: "127.0.0.1", port: 6300 },
      }),
    ).toBe(6300);
    expect(
      externalRunningPort({
        ...externalRow(),
        detail: { kind: "uv_tool", version: "0.5.1", port: 6301 },
      }),
    ).toBe(6301);
  });

  it("falls back to the WebUI URL port", () => {
    expect(
      externalRunningPort({
        ...externalRow(),
        detail: null,
        webui_url: "http://127.0.0.1:6302/",
      }),
    ).toBe(6302);
  });

  it("returns null without a usable port", () => {
    expect(
      externalRunningPort({ ...externalRow(), detail: null, webui_url: null }),
    ).toBeNull();
    expect(
      externalRunningPort({
        ...externalRow(),
        detail: null,
        webui_url: "not-a-url",
      }),
    ).toBeNull();
  });
});
