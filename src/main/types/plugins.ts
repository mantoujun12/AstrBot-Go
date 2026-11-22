/**
 * Frontend contract mirroring the Rust IPC types in
 * `src-tauri/src/instance/plugins.rs`. Field names stay snake_case because
 * serde serializes the backend structs without a rename rule.
 */

export interface PluginMarketCategory {
  name: string;
  icon: string;
}

export interface PluginMarketItem {
  plugin_id: string;
  author: string;
  name: string;
  display_name: string | null;
  description: string | null;
  repository_url: string | null;
  download_url: string | null;
  logo_url: string | null;
  tags: string[];
  category: string | null;
  astrbot_version: string | null;
  official: boolean;
  pinned: boolean;
  downloads: number;
  github_stars: number;
  latest_version: string | null;
  updated_at: string | null;
}

export interface PluginMarketPage {
  categories: PluginMarketCategory[];
  category_counts: Record<string, number>;
  items: PluginMarketItem[];
  page: number;
  page_size: number;
  total: number;
}

export interface InstancePlugin {
  dir_name: string;
  name: string;
  author: string | null;
  version: string | null;
  desc: string | null;
  repo: string | null;
  astrbot_version: string | null;
  plugin_id: string | null;
  enabled: boolean;
  has_metadata: boolean;
}

export interface PluginInstallRequest {
  plugin_id: string;
  download_url: string;
}
