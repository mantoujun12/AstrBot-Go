// Tests for the shiki language whitelist used by the ChatUI island.

import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import {
  createHighlighter,
  LIMITED_SHIKI_SUPPORTED_LANGUAGES,
  loadShikiLanguages,
  normalizeLimitedShikiLanguage,
} from "./shikiLimitedBundle";

describe("normalizeLimitedShikiLanguage", () => {
  it("keeps whitelisted languages and their aliases", () => {
    expect(normalizeLimitedShikiLanguage("python")).toBe("python");
    expect(normalizeLimitedShikiLanguage("PY")).toBe("python");
    expect(normalizeLimitedShikiLanguage("bash")).toBe("shellscript");
    expect(normalizeLimitedShikiLanguage("c++")).toBe("cpp");
    expect(normalizeLimitedShikiLanguage("cxx")).toBe("cpp");
    expect(normalizeLimitedShikiLanguage("hxx")).toBe("cpp");
    expect(normalizeLimitedShikiLanguage("ts")).toBe("typescript");
    expect(normalizeLimitedShikiLanguage("python linenums")).toBe("python");
  });

  it("falls back to plain text for unknown languages", () => {
    expect(normalizeLimitedShikiLanguage("emacs-lisp")).toBe("text");
    expect(normalizeLimitedShikiLanguage("wolfram")).toBe("text");
    expect(normalizeLimitedShikiLanguage("")).toBe("text");
    expect(normalizeLimitedShikiLanguage(undefined)).toBe("text");
  });

  it("ships only the whitelisted grammars", () => {
    expect(LIMITED_SHIKI_SUPPORTED_LANGUAGES.size).toBeLessThan(30);
    expect([...LIMITED_SHIKI_SUPPORTED_LANGUAGES]).not.toContain("emacs-lisp");
  });

  // The dynamic imports are what Vite turns into language chunks, so scanning
  // them covers the emitted bundle without building the whole app in a test.
  it("imports only whitelisted grammar modules", () => {
    const source = readFileSync("src/chatui/utils/shikiLimitedBundle.js", "utf8");
    const imported = [
      ...source.matchAll(/import\("shiki\/langs\/([^"]+)\.mjs"\)/g),
    ].map((match) => match[1]);

    expect(imported.length).toBeGreaterThan(0);
    for (const language of imported) {
      expect(LIMITED_SHIKI_SUPPORTED_LANGUAGES.has(language)).toBe(true);
    }
    expect(source).not.toMatch(/from\s+["']shiki\/langs["']/);
  });
});

describe("loadShikiLanguages", () => {
  it("loads whitelisted grammars and renders unknown languages as text", async () => {
    const highlighter = await createHighlighter();
    await loadShikiLanguages(highlighter, ["python", "bash", "emacs-lisp"]);

    const loaded = highlighter.getLoadedLanguages();
    expect(loaded).toContain("python");
    expect(loaded).toContain("shellscript");
    expect(loaded).not.toContain("emacs-lisp");

    const html = highlighter.codeToHtml("print(1)", {
      lang: "text",
      theme: "github-dark",
    });
    expect(html).toContain("print(1)");
  });

  it("loads whitelisted lazy embedded grammars", async () => {
    const highlighter = await createHighlighter();
    await loadShikiLanguages(highlighter, ["markdown"]);

    const loaded = highlighter.getLoadedLanguages();
    expect(loaded).toContain("markdown");
    expect(loaded).toContain("python");
    expect(loaded).toContain("css");
    // Embedded languages outside the whitelist stay unloaded.
    expect(loaded).not.toContain("lua");
  });
});
