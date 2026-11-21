import DOMPurify from "dompurify";
import MarkdownIt from "markdown-it";

const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  typographer: false,
});

markdown.enable(["table", "strikethrough"]);

export function renderMarkdown(source: string): string {
  if (source.trim() === "") {
    return "";
  }

  const rendered = markdown.render(source);
  return DOMPurify.sanitize(rendered, {
    USE_PROFILES: { html: true },
  });
}

// In-source tests, stripped from the production bundle by the
// `import.meta.vitest` define in vite.config.ts.
if (import.meta.vitest) {
  const { describe, expect, it } = import.meta.vitest;

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
}
