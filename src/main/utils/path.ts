/**
 * Returns the parent directory for both Windows and POSIX style paths.
 */
export function dirname(path: string): string {
  const normalized = path.replace(/[\\/]+$/, "");
  const separatorIndex = Math.max(
    normalized.lastIndexOf("/"),
    normalized.lastIndexOf("\\"),
  );

  if (separatorIndex <= 0) {
    return normalized;
  }

  return normalized.slice(0, separatorIndex);
}

// In-source tests, stripped from the production bundle by the
// `import.meta.vitest` define in vite.config.ts.
if (import.meta.vitest) {
  const { describe, expect, it } = import.meta.vitest;

  describe("dirname", () => {
    it("splits POSIX and Windows paths", () => {
      expect(dirname("a/b/c")).toBe("a/b");
      expect(dirname("C:\\dir\\file.txt")).toBe("C:\\dir");
    });

    it("returns the input unchanged without a parent", () => {
      expect(dirname("file.txt")).toBe("file.txt");
    });
  });
}
