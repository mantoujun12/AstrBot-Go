export type InstanceViewMode = "table" | "cards";

export type InstanceSortKey =
  | "nameAsc"
  | "nameDesc"
  | "state"
  | "versionDesc"
  | "portAsc";

export type InstanceColumnName = "version" | "port";

/** Persisted instance list preferences; search and filters stay ephemeral. */
export interface InstanceListPrefs {
  viewMode: InstanceViewMode;
  sortKey: InstanceSortKey;
  /** Hidden table columns; empty means every optional column is visible. */
  hiddenColumns: InstanceColumnName[];
}

const INSTANCE_LIST_CACHE_KEY = "launcher.instances.listPrefs";

const DEFAULT_PREFS: InstanceListPrefs = {
  viewMode: "table",
  sortKey: "nameAsc",
  hiddenColumns: [],
};

function isInstanceViewMode(value: unknown): value is InstanceViewMode {
  return value === "table" || value === "cards";
}

function isInstanceSortKey(value: unknown): value is InstanceSortKey {
  return (
    value === "nameAsc" ||
    value === "nameDesc" ||
    value === "state" ||
    value === "versionDesc" ||
    value === "portAsc"
  );
}

function isInstanceColumnName(value: unknown): value is InstanceColumnName {
  return value === "version" || value === "port";
}

// `hiddenColumns` is optional here so cached prefs written before the field
// existed still validate and fall back to the default.
type CachedInstanceListPrefs = Omit<InstanceListPrefs, "hiddenColumns"> & {
  hiddenColumns?: InstanceColumnName[];
};

function isInstanceListPrefs(value: unknown): value is CachedInstanceListPrefs {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as {
    viewMode?: unknown;
    sortKey?: unknown;
    hiddenColumns?: unknown;
  };
  return (
    isInstanceViewMode(candidate.viewMode) &&
    isInstanceSortKey(candidate.sortKey) &&
    (candidate.hiddenColumns === undefined ||
      (Array.isArray(candidate.hiddenColumns) &&
        candidate.hiddenColumns.every(isInstanceColumnName)))
  );
}

export function readCachedListPrefs(): InstanceListPrefs {
  try {
    const cached = localStorage.getItem(INSTANCE_LIST_CACHE_KEY);
    if (cached === null) {
      return { ...DEFAULT_PREFS };
    }
    const parsed: unknown = JSON.parse(cached);
    return isInstanceListPrefs(parsed)
      ? { ...parsed, hiddenColumns: parsed.hiddenColumns ?? [] }
      : { ...DEFAULT_PREFS };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

export function cacheListPrefs(prefs: InstanceListPrefs): void {
  try {
    localStorage.setItem(INSTANCE_LIST_CACHE_KEY, JSON.stringify(prefs));
  } catch {
    // Storage may be unavailable in restricted webviews; the defaults apply
    // on the next visit.
  }
}
