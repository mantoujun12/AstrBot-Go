import { describe, expect, it } from "vitest";

import { renderMarkdown } from "./markdown";

describe("renderMarkdown", () => {
  it("returns an empty string for blank input", () => {
    expect(renderMarkdown("   ")).toBe("");
  });

  it("never emits live tags or unsafe link hrefs", () => {
    const html = renderMarkdown(
      'hello <script>alert(1)</script> [click](javascript:alert(1))',
    );
    expect(html.toLowerCase()).not.toContain("<script");
    expect(html.toLowerCase()).not.toContain('href="javascript:');
  });
});
