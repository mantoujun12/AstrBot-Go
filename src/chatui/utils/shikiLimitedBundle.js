// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import githubDark from "shiki/themes/github-dark.mjs";
import githubLight from "shiki/themes/github-light.mjs";

// Import grammars directly: the `shiki/langs` barrel exposes all ~200 languages
// as dynamic imports, so bundling it emits every language chunk.
const languageLoaders = {
  c: () => import("shiki/langs/c.mjs"),
  cpp: () => import("shiki/langs/cpp.mjs"),
  csharp: () => import("shiki/langs/csharp.mjs"),
  css: () => import("shiki/langs/css.mjs"),
  go: () => import("shiki/langs/go.mjs"),
  html: () => import("shiki/langs/html.mjs"),
  java: () => import("shiki/langs/java.mjs"),
  javascript: () => import("shiki/langs/javascript.mjs"),
  json: () => import("shiki/langs/json.mjs"),
  jsx: () => import("shiki/langs/jsx.mjs"),
  markdown: () => import("shiki/langs/markdown.mjs"),
  python: () => import("shiki/langs/python.mjs"),
  rust: () => import("shiki/langs/rust.mjs"),
  shellscript: () => import("shiki/langs/shellscript.mjs"),
  sql: () => import("shiki/langs/sql.mjs"),
  toml: () => import("shiki/langs/toml.mjs"),
  tsx: () => import("shiki/langs/tsx.mjs"),
  typescript: () => import("shiki/langs/typescript.mjs"),
  yaml: () => import("shiki/langs/yaml.mjs"),
};

const BUILT_IN_LANGUAGES = ["text", "plaintext", "plain"];

export const LIMITED_SHIKI_LANGUAGE_ALIASES = {
  bash: "shellscript",
  "c#": "csharp",
  "c++": "cpp",
  cc: "cpp",
  cjs: "javascript",
  console: "shellscript",
  cs: "csharp",
  cts: "typescript",
  cxx: "cpp",
  golang: "go",
  h: "c",
  "h++": "cpp",
  hh: "cpp",
  hpp: "cpp",
  htm: "html",
  hxx: "cpp",
  js: "javascript",
  md: "markdown",
  mjs: "javascript",
  mts: "typescript",
  plain: "text",
  plaintext: "text",
  py: "python",
  rs: "rust",
  sh: "shellscript",
  shell: "shellscript",
  text: "text",
  ts: "typescript",
  txt: "text",
  xhtml: "html",
  yml: "yaml",
  zsh: "shellscript",
};

export const LIMITED_SHIKI_SUPPORTED_LANGUAGES = new Set([
  ...BUILT_IN_LANGUAGES,
  ...Object.keys(languageLoaders),
]);

export function normalizeLimitedShikiLanguage(language) {
  const normalized = String(language || "text")
    .trim()
    .split(/\s+/, 1)[0]
    .split(":", 1)[0]
    .toLowerCase();

  if (!normalized) return "text";
  if (normalized in LIMITED_SHIKI_LANGUAGE_ALIASES) {
    return LIMITED_SHIKI_LANGUAGE_ALIASES[normalized];
  }

  return LIMITED_SHIKI_SUPPORTED_LANGUAGES.has(normalized) ? normalized : "text";
}

export function createHighlighter() {
  return createHighlighterCore({
    engine: createJavaScriptRegexEngine(),
    themes: [githubLight, githubDark],
    langs: [],
  });
}

export async function loadShikiLanguages(highlighter, languages) {
  const loaded = new Set(highlighter.getLoadedLanguages());
  const queue = languages.map(normalizeLimitedShikiLanguage);
  const seen = new Set();
  const registrations = [];

  // A grammar can lazily embed other grammars (markdown embeds css, python,
  // ...) whose modules are not part of the requested import. Follow the
  // embedded lists and register every whitelisted one too.
  while (queue.length) {
    const language = queue.shift();
    if (language === "text" || seen.has(language) || loaded.has(language)) continue;
    seen.add(language);

    const load = languageLoaders[language];
    if (!load) continue;

    const { default: embedded } = await load();
    registrations.push(...embedded);
    for (const registration of embedded) {
      queue.push(
        ...(registration.embeddedLangs ?? []),
        ...(registration.embeddedLangsLazy ?? []),
      );
    }
  }

  if (registrations.length) highlighter.loadLanguageSync(registrations);
}
