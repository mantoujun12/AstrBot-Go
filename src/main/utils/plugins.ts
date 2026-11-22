import type { InstancePlugin } from "../types/plugins";

interface MarketLike {
  name: string;
  repository_url?: string | null;
}

function normalizeRepoUrl(url: string): string {
  return url.trim().replace(/\/+$/, "").replace(/\.git$/, "").toLowerCase();
}

/**
 * Matches an installed plugin to a market record the way upstream does:
 * by normalized repository URL first, falling back to the plugin name.
 * The market `plugin_id` is publisher-based and does not equal
 * `metadata.yaml` author/name, so it is never used for matching.
 */
export function isSamePlugin(installed: InstancePlugin, market: MarketLike): boolean {
  const installedRepo = installed.repo ? normalizeRepoUrl(installed.repo) : "";
  const marketRepo = market.repository_url ? normalizeRepoUrl(market.repository_url) : "";
  if (installedRepo && marketRepo) {
    return installedRepo === marketRepo;
  }
  return installed.name.trim().toLowerCase() === market.name.trim().toLowerCase();
}

// In-source tests, stripped from the production bundle by the
// `import.meta.vitest` define in vite.config.ts.
if (import.meta.vitest) {
  const { describe, expect, it } = import.meta.vitest;

  const installed = (overrides: Partial<InstancePlugin> = {}): InstancePlugin => ({
    dir_name: "d",
    name: "meme_manager",
    author: "anka",
    version: null,
    desc: null,
    repo: "https://github.com/anka-afk/astrbot_plugin_meme_manager",
    astrbot_version: null,
    enabled: true,
    has_metadata: true,
    ...overrides,
  });

  describe("isSamePlugin", () => {
    it("matches by normalized repository url", () => {
      expect(
        isSamePlugin(installed(), {
          name: "whatever",
          repository_url: "https://github.com/anka-afk/astrbot_plugin_meme_manager/",
        }),
      ).toBe(true);
      expect(
        isSamePlugin(installed(), {
          name: "whatever",
          repository_url: "https://github.com/anka-afk/astrbot_plugin_meme_manager.git",
        }),
      ).toBe(true);
      expect(
        isSamePlugin(installed(), {
          name: "whatever",
          repository_url: "https://github.com/other/repo",
        }),
      ).toBe(false);
    });

    it("falls back to the name when a repo is missing", () => {
      expect(isSamePlugin(installed({ repo: null }), { name: "Meme_Manager" })).toBe(true);
      expect(isSamePlugin(installed({ repo: null }), { name: "other" })).toBe(false);
      expect(
        isSamePlugin(installed(), { name: "meme_manager", repository_url: null }),
      ).toBe(true);
    });
  });
}
