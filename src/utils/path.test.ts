import { describe, expect, it } from "vitest";

import { dirname } from "./path";

describe("dirname", () => {
  it("splits POSIX and Windows paths", () => {
    expect(dirname("a/b/c")).toBe("a/b");
    expect(dirname("C:\\dir\\file.txt")).toBe("C:\\dir");
  });

  it("returns the input unchanged without a parent", () => {
    expect(dirname("file.txt")).toBe("file.txt");
  });
});
