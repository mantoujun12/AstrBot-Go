import { invoke } from "@tauri-apps/api/core";
import type {
  LanguagePreference,
  ReduceMotionPreference,
  ThemePreference,
} from "../types/launcher";

export interface ProxySettings {
  proxyUrl: string;
  proxyPort: string;
  proxyUsername: string;
  proxyPassword: string;
}

export function saveGithubProxy(githubProxy: string): Promise<void> {
  return invoke("save_github_proxy", { githubProxy });
}

export function saveProxy(input: ProxySettings): Promise<void> {
  return invoke("save_proxy", {
    proxyUrl: input.proxyUrl,
    proxyPort: input.proxyPort,
    proxyUsername: input.proxyUsername,
    proxyPassword: input.proxyPassword,
  });
}

export function savePypiMirror(pypiMirror: string): Promise<void> {
  return invoke("save_pypi_mirror", { pypiMirror });
}

export function saveNodejsMirror(nodejsMirror: string): Promise<void> {
  return invoke("save_nodejs_mirror", { nodejsMirror });
}

export function saveNpmRegistry(npmRegistry: string): Promise<void> {
  return invoke("save_npm_registry", { npmRegistry });
}

export function saveUseUvForDeps(useUvForDeps: boolean): Promise<void> {
  return invoke("save_use_uv_for_deps", { useUvForDeps });
}

export function saveCloseToTray(closeToTray: boolean): Promise<void> {
  return invoke("save_close_to_tray", { closeToTray });
}

export function saveAutostartMinimizeToTray(
  autostartMinimizeToTray: boolean,
): Promise<void> {
  return invoke("save_autostart_minimize_to_tray", { autostartMinimizeToTray });
}

export function saveCheckInstanceUpdate(
  checkInstanceUpdate: boolean,
): Promise<void> {
  return invoke("save_check_instance_update", { checkInstanceUpdate });
}

export function savePersistInstanceState(
  persistInstanceState: boolean,
): Promise<void> {
  return invoke("save_persist_instance_state", { persistInstanceState });
}

export function saveIgnoreExternalPath(ignoreExternalPath: boolean): Promise<void> {
  return invoke("save_ignore_external_path", { ignoreExternalPath });
}

export function saveUseShortInstanceId(useShortInstanceId: boolean): Promise<void> {
  return invoke("save_use_short_instance_id", { useShortInstanceId });
}

export function saveLockCheckExtensionWhitelist(
  lockCheckExtensionWhitelist: boolean,
): Promise<void> {
  return invoke("save_lock_check_extension_whitelist", {
    lockCheckExtensionWhitelist,
  });
}

export function saveThemePreference(
  themePreference: ThemePreference,
): Promise<void> {
  return invoke("save_theme_preference", { themePreference });
}

export function saveLanguagePreference(
  languagePreference: LanguagePreference,
): Promise<void> {
  return invoke("save_language_preference", { languagePreference });
}

export function saveReduceMotionPreference(
  reduceMotionPreference: ReduceMotionPreference,
): Promise<void> {
  return invoke("save_reduce_motion_preference", { reduceMotionPreference });
}

export function savePythonMirror(pythonMirror: string): Promise<void> {
  return invoke("save_python_mirror", { pythonMirror });
}

export function saveAllowLocalDnsProbe(
  allowLocalDnsProbe: boolean,
): Promise<void> {
  return invoke("save_allow_local_dns_probe", { allowLocalDnsProbe });
}

export function saveUvMirror(uvMirror: string): Promise<void> {
  return invoke("save_uv_mirror", { uvMirror });
}

export function compareVersions(a: string, b: string): Promise<number> {
  return invoke<number>("compare_versions", { a, b });
}

export function isMacos(): Promise<boolean> {
  return invoke<boolean>("is_macos");
}
