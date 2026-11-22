import { computed, ref } from "vue";

import type { DiscoveredInstance } from "../types/launcher";

export const HOME_TAB = "home";

const instanceIds = ref<string[]>([]);
const activeTab = ref<string>(HOME_TAB);

/**
 * Registry for tabs backed by discovered (external) instances instead of
 * launcher-managed ones: display name plus the WebUI origin to embed. Only
 * instances with a `webui_url` can open a tab.
 */
const externalTabs = ref<
  Map<string, { name: string; url: string; state: DiscoveredInstance["state"] }>
>(new Map());

const isHomeActive = computed(() => activeTab.value === HOME_TAB);

export function openInstanceTab(instanceId: string): void {
  if (!instanceIds.value.includes(instanceId)) {
    instanceIds.value = [...instanceIds.value, instanceId];
  }
  activeTab.value = instanceId;
}

export function openExternalInstanceTab(
  instance: Pick<DiscoveredInstance, "id" | "name" | "webui_url" | "state">,
): void {
  if (instance.webui_url === null) {
    return;
  }
  externalTabs.value.set(instance.id, {
    name: instance.name,
    url: instance.webui_url,
    state: instance.state,
  });
  openInstanceTab(instance.id);
}

export function externalTabOf(
  instanceId: string,
): { name: string; url: string; state: DiscoveredInstance["state"] } | null {
  return externalTabs.value.get(instanceId) ?? null;
}

export function activateInstanceTab(instanceId: string): void {
  if (instanceIds.value.includes(instanceId)) {
    activeTab.value = instanceId;
  }
}

export function closeInstanceTab(instanceId: string): void {
  const index = instanceIds.value.indexOf(instanceId);
  if (index === -1) {
    return;
  }

  externalTabs.value.delete(instanceId);

  const next = instanceIds.value.filter((id) => id !== instanceId);
  instanceIds.value = next;

  if (activeTab.value !== instanceId) {
    return;
  }

  activeTab.value = next[index] ?? next[index - 1] ?? HOME_TAB;
}

export function activateHome(): void {
  activeTab.value = HOME_TAB;
}

/**
 * Reorders open instance tabs to the given sequence. Unknown ids, duplicates
 * and no-op changes are ignored; missing tabs are appended so every open tab
 * stays represented. The active tab is never changed by a reorder.
 */
export function reorderInstanceTabs(ids: string[]): void {
  if (instanceIds.value.length === 0) {
    return;
  }

  const seen = new Set<string>();
  const next: string[] = [];
  for (const id of ids) {
    if (!seen.has(id) && instanceIds.value.includes(id)) {
      seen.add(id);
      next.push(id);
    }
  }
  for (const id of instanceIds.value) {
    if (!seen.has(id)) {
      next.push(id);
    }
  }

  if (next.length !== instanceIds.value.length) {
    return;
  }
  if (next.every((id, index) => id === instanceIds.value[index])) {
    return;
  }

  instanceIds.value = next;
  if (activeTab.value !== HOME_TAB && !next.includes(activeTab.value)) {
    activeTab.value = HOME_TAB;
  }
}

/**
 * Removes tabs whose instances no longer exist. Called whenever the snapshot
 * changes so deleting an instance also closes its WebUI tab.
 */
export function pruneTabs(validIds: string[]): void {
  const valid = new Set(validIds);
  const next = instanceIds.value.filter(
    (id) => valid.has(id) || externalTabs.value.has(id),
  );
  if (next.length === instanceIds.value.length) {
    return;
  }

  // Dropped internal tabs lose their registration as well.
  for (const id of instanceIds.value) {
    if (!next.includes(id)) {
      externalTabs.value.delete(id);
    }
  }

  instanceIds.value = next;
  if (
    activeTab.value !== HOME_TAB &&
    !next.includes(activeTab.value)
  ) {
    activeTab.value = next[0] ?? HOME_TAB;
  }
}

export function useTabsStore() {
  return {
    instanceIds,
    activeTab,
    isHomeActive,
    openInstanceTab,
    openExternalInstanceTab,
    externalTabOf,
    activateInstanceTab,
    closeInstanceTab,
    activateHome,
    pruneTabs,
    reorderInstanceTabs,
  };
}
