export function formatLatency(milliseconds: number): string {
  if (!Number.isFinite(milliseconds)) {
    return "—";
  }
  return `${Math.max(0, Math.round(milliseconds))} ms`;
}

export function formatSpeed(bytesPerSecond: number | null): string {
  if (
    bytesPerSecond === null ||
    !Number.isFinite(bytesPerSecond) ||
    bytesPerSecond <= 0
  ) {
    return "—";
  }
  if (bytesPerSecond >= 1024 * 1024) {
    return `${(bytesPerSecond / (1024 * 1024)).toFixed(1)} MB/s`;
  }
  if (bytesPerSecond >= 1024) {
    return `${Math.round(bytesPerSecond / 1024)} KB/s`;
  }
  return `${Math.round(bytesPerSecond)} B/s`;
}

// In-source tests, stripped from the production bundle by the
// `import.meta.vitest` define in vite.config.ts.
if (import.meta.vitest) {
  const { describe, expect, it } = import.meta.vitest;

  describe("formatLatency", () => {
    it("rounds finite values and dashes non-finite ones", () => {
      expect(formatLatency(12.4)).toBe("12 ms");
      expect(formatLatency(Number.NaN)).toBe("—");
    });
  });

  describe("formatSpeed", () => {
    it("formats the unit thresholds and dashes invalid input", () => {
      expect(formatSpeed(512)).toBe("512 B/s");
      expect(formatSpeed(2048)).toBe("2 KB/s");
      expect(formatSpeed(1.5 * 1024 * 1024)).toBe("1.5 MB/s");
      expect(formatSpeed(null)).toBe("—");
    });
  });
}
