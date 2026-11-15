import { describe, expect, it } from "vitest";

import { countBackupNoteChars } from "./backupNote";

describe("countBackupNoteChars", () => {
  it("counts emoji as one and empty input as zero", () => {
    expect(countBackupNoteChars("😀")).toBe(1);
    expect(countBackupNoteChars("")).toBe(0);
  });
});
