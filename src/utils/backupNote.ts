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
