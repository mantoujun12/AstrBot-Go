import { computed, ref, watch } from "vue";
import type {
  DiscoveredInstance,
  InstanceState,
  InstanceStatus,
} from "../types/launcher";
import {
  cacheListPrefs,
  readCachedListPrefs,
  type InstanceSortKey,
  type InstanceViewMode,
} from "../utils/instanceListCache";

export type { InstanceSortKey, InstanceViewMode };

export type InstanceStateFilter = "all" | "running" | "stopped";
export type InstanceSourceFilter = "all" | "launcher" | "external";

export type ExternalInstanceRow = DiscoveredInstance & { external: true };

export type InstanceRow =
  | (InstanceStatus & { external?: false })
  | ExternalInstanceRow;

/** Port of an external row's WebUI, when the row is running and exposes one. */
export function externalRunningPort(row: DiscoveredInstance): number | null {
  if (row.detail?.kind === "docker" || row.detail?.kind === "uv_tool") {
    return row.detail.port;
  }
  try {
    const port = new URL(row.webui_url ?? "").port;
    return port === "" ? null : Number(port);
  } catch {
    return null;
  }
}

/** Running instances first, then transients, then stopped ones. */
const STATE_ORDER: Record<InstanceState, number> = {
  running: 0,
  starting: 1,
  stopping: 2,
  stopped: 3,
};

function parseVersionSegments(version: string): number[] | null {
  const segments = version.replace(/^v/i, "").split(".");
  const numbers: number[] = [];
  for (const segment of segments) {
    if (!/^\d+$/.test(segment)) {
      return null;
    }
    numbers.push(Number(segment));
  }
  return numbers;
}

/**
 * Descending, semantic-ish version compare: dot-separated numeric segments
 * win, a non-numeric version falls back to a plain string compare, and a
 * missing version always sorts last.
 */
export function compareVersionsDesc(
  a: string | null,
  b: string | null,
): number {
  if (a === null || b === null) {
    if (a === b) {
      return 0;
    }
    return a === null ? 1 : -1;
  }
  const as = parseVersionSegments(a);
  const bs = parseVersionSegments(b);
  if (as !== null && bs !== null) {
    const length = Math.max(as.length, bs.length);
    for (let index = 0; index < length; index += 1) {
      const delta = (as[index] ?? 0) - (bs[index] ?? 0);
      if (delta !== 0) {
        return delta > 0 ? -1 : 1;
      }
    }
    return 0;
  }
  return b.localeCompare(a);
}

function compareNumbersNullLast(a: number | null, b: number | null): number {
  if (a === null || b === null) {
    if (a === b) {
      return 0;
    }
    return a === null ? 1 : -1;
  }
  return a - b;
}

/** Compare two rows under a sort key; `0` means the caller keeps the row order. */
export function compareInstanceRows(
  a: InstanceRow,
  b: InstanceRow,
  sortKey: InstanceSortKey,
): number {
  switch (sortKey) {
    case "nameAsc":
      return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
    case "nameDesc":
      return b.name.toLowerCase().localeCompare(a.name.toLowerCase());
    case "state":
      return STATE_ORDER[a.state] - STATE_ORDER[b.state];
    case "versionDesc":
      return compareVersionsDesc(
        a.external ? null : a.version,
        b.external ? null : b.version,
      );
    case "portAsc":
      return compareNumbersNullLast(
        a.external ? externalRunningPort(a) : a.configured_port,
        b.external ? externalRunningPort(b) : b.configured_port,
      );
  }
}

/**
 * Stable sort; ties fall back to the current row order, which keeps launcher
 * rows ahead of external ones in the merged list.
 */
export function sortInstanceRows(
  rows: readonly InstanceRow[],
  sortKey: InstanceSortKey,
): InstanceRow[] {
  return rows
    .map((row, index) => ({ row, index }))
    .sort(
      (a, b) => compareInstanceRows(a.row, b.row, sortKey) || a.index - b.index,
    )
    .map((entry) => entry.row);
}

/**
 * Transient states stay visible under the filter they are transitioning into,
 * so a row does not vanish while its start or stop is still settling.
 */
export function matchesStateFilter(
  row: InstanceRow,
  stateFilter: InstanceStateFilter,
): boolean {
  switch (stateFilter) {
    case "running":
      return row.state === "running" || row.state === "starting";
    case "stopped":
      return row.state === "stopped" || row.state === "stopping";
    default:
      return true;
  }
}

export function matchesSourceFilter(
  row: InstanceRow,
  sourceFilter: InstanceSourceFilter,
): boolean {
  switch (sourceFilter) {
    case "launcher":
      return !row.external;
    case "external":
      return row.external === true;
    default:
      return true;
  }
}

export interface InstanceListFilters {
  search: string;
  stateFilter: InstanceStateFilter;
  sourceFilter: InstanceSourceFilter;
}

export function filterInstanceRows(
  rows: readonly InstanceRow[],
  filters: InstanceListFilters,
): InstanceRow[] {
  const query = filters.search.trim().toLowerCase();
  return rows.filter(
    (row) =>
      (query === "" || row.name.toLowerCase().includes(query)) &&
      matchesStateFilter(row, filters.stateFilter) &&
      matchesSourceFilter(row, filters.sourceFilter),
  );
}

/**
 * Sorting, filtering, and view state for the instance list. `viewMode` and
 * `sortKey` persist to localStorage; `search` and both filters are ephemeral
 * (fresh refs per page mount, so they reset when the page is left).
 */
export function useInstanceList(getRows: () => readonly InstanceRow[]) {
  const search = ref("");
  const stateFilter = ref<InstanceStateFilter>("all");
  const sourceFilter = ref<InstanceSourceFilter>("all");
  const { viewMode: cachedViewMode, sortKey: cachedSortKey } =
    readCachedListPrefs();
  const sortKey = ref<InstanceSortKey>(cachedSortKey);
  const viewMode = ref<InstanceViewMode>(cachedViewMode);

  const filteredRows = computed(() =>
    sortInstanceRows(
      filterInstanceRows(getRows(), {
        search: search.value,
        stateFilter: stateFilter.value,
        sourceFilter: sourceFilter.value,
      }),
      sortKey.value,
    ),
  );

  watch([viewMode, sortKey], () => {
    cacheListPrefs({
      viewMode: viewMode.value,
      sortKey: sortKey.value,
    });
  });

  function resetFilters(): void {
    search.value = "";
    stateFilter.value = "all";
    sourceFilter.value = "all";
  }

  return {
    search,
    stateFilter,
    sourceFilter,
    sortKey,
    viewMode,
    filteredRows,
    resetFilters,
  };
}
