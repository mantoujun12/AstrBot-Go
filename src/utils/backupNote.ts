/** Longest accepted backup note, matching the Rust backend limit. */
export const BACKUP_NOTE_MAX_LENGTH = 200;

/**
 * Count a backup note in Unicode scalar values so supplementary-plane
 * characters such as emoji count as one, matching Rust's `chars()`.
 */
export function countBackupNoteChars(
  value: string | null | undefined,
): number {
  return value ? Array.from(value).length : 0;
}

// In-source tests, stripped from the production bundle by the
// `import.meta.vitest` define in vite.config.ts.
if (import.meta.vitest) {
  const { describe, expect, it } = import.meta.vitest;

  describe("countBackupNoteChars", () => {
    it("counts emoji as one and empty input as zero", () => {
      expect(countBackupNoteChars("😀")).toBe(1);
      expect(countBackupNoteChars("")).toBe(0);
    });
  });
}
