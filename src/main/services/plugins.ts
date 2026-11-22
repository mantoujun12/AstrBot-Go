import { invoke } from "@tauri-apps/api/core";
import type {
  InstancePlugin,
  PluginInstallRequest,
  PluginMarketPage,
} from "../types/plugins";

export function fetchPluginMarket(
  page: number,
  pageSize: number,
  search?: string,
  category?: string,
): Promise<PluginMarketPage> {
  return invoke<PluginMarketPage>("fetch_plugin_market", {
    page,
    pageSize,
    search: search ?? null,
    category: category ?? null,
  });
}

export function listInstancePlugins(instanceId: string): Promise<InstancePlugin[]> {
  return invoke<InstancePlugin[]>("list_instance_plugins", { instanceId });
}

export function installInstancePlugin(
  instanceId: string,
  request: PluginInstallRequest,
): Promise<InstancePlugin> {
  return invoke<InstancePlugin>("install_instance_plugin", {
    instanceId,
    request,
  });
}

export function setInstancePluginEnabled(
  instanceId: string,
  dirName: string,
  enabled: boolean,
): Promise<void> {
  return invoke("set_instance_plugin_enabled", {
    instanceId,
    dirName,
    enabled,
  });
}

export function uninstallInstancePlugin(
  instanceId: string,
  dirName: string,
  deleteConfig: boolean,
  deleteData: boolean,
): Promise<void> {
  return invoke("uninstall_instance_plugin", {
    instanceId,
    dirName,
    deleteConfig,
    deleteData,
  });
}
