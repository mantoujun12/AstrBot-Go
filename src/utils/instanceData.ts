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
