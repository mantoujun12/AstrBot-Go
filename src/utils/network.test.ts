import { describe, expect, it } from "vitest";

import { formatLatency, formatSpeed } from "./network";

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
