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
