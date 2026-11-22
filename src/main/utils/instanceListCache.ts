export type InstanceViewMode = "table" | "cards";

export type InstanceSortKey =
  | "nameAsc"
  | "nameDesc"
  | "state"
  | "versionDesc"
  | "portAsc";

/** Persisted instance list preferences; search and filters stay ephemeral. */
export interface InstanceListPrefs {
  viewMode: InstanceViewMode;
  sortKey: InstanceSortKey;
}

const INSTANCE_LIST_CACHE_KEY = "launcher.instances.listPrefs";

const DEFAULT_PREFS: InstanceListPrefs = {
  viewMode: "table",
  sortKey: "nameAsc",
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

function isInstanceListPrefs(value: unknown): value is InstanceListPrefs {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as { viewMode?: unknown; sortKey?: unknown };
  return (
    isInstanceViewMode(candidate.viewMode) &&
    isInstanceSortKey(candidate.sortKey)
  );
}

export function readCachedListPrefs(): InstanceListPrefs {
  try {
    const cached = localStorage.getItem(INSTANCE_LIST_CACHE_KEY);
    if (cached === null) {
      return { ...DEFAULT_PREFS };
    }
    const parsed: unknown = JSON.parse(cached);
    return isInstanceListPrefs(parsed) ? parsed : { ...DEFAULT_PREFS };
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
