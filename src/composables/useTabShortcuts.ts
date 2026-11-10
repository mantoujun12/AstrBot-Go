import { onBeforeUnmount, onMounted } from "vue";
import { HOME_TAB, useTabsStore } from "../stores/tabs";

/**
 * Message marker used by the all-frames bridge script defined in
 * `src-tauri/src/webview_shortcuts.rs`. Keydown events inside the instance
 * WebUI iframe never reach this window, so the bridge forwards them.
 */
const BRIDGE_SOURCE = "astrbot-launcher:tab-shortcut";

interface ShortcutInput {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  defaultPrevented: boolean;
  preventDefault?: () => void;
}

interface BridgeMessage {
  source: typeof BRIDGE_SOURCE;
  key: string;
  shiftKey: boolean;
}

function isBridgeMessage(value: unknown): value is BridgeMessage {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const message = value as Record<string, unknown>;
  return (
    message.source === BRIDGE_SOURCE &&
    typeof message.key === "string" &&
    typeof message.shiftKey === "boolean"
  );
}

/**
 * Browser-style tab shortcuts:
 * - Ctrl+Tab / Ctrl+PageDown: next tab
 * - Ctrl+Shift+Tab / Ctrl+PageUp: previous tab
 * - Ctrl+1…8: nth tab, Ctrl+9: last tab
 * - Ctrl+W / Ctrl+F4: close the active tab (the home tab stays open)
 * - Ctrl+Shift+PageUp/PageDown: move the active tab one position
 */
export function useTabShortcuts(): void {
  const {
    instanceIds,
    activeTab,
    activateInstanceTab,
    closeInstanceTab,
    activateHome,
    reorderInstanceTabs,
  } = useTabsStore();

  function tabIds(): string[] {
    return [HOME_TAB, ...instanceIds.value];
  }

  function activate(id: string): void {
    if (id === HOME_TAB) {
      activateHome();
    } else {
      activateInstanceTab(id);
    }
  }

  function switchBy(offset: number): void {
    const ids = tabIds();
    const index = ids.indexOf(activeTab.value);
    if (index === -1) {
      activateHome();
      return;
    }
    activate(ids[(index + offset + ids.length) % ids.length]);
  }

  function selectByIndex(index: number): void {
    const ids = tabIds();
    if (index >= 0 && index < ids.length) {
      activate(ids[index]);
    }
  }

  /**
   * Ctrl+Shift+PageUp/PageDown: move the active tab one position left or
   * right, like common browser behaviour. The home tab never moves and the
   * active tab stays active.
   */
  function moveActiveTab(offset: number): void {
    const current = activeTab.value;
    if (current === HOME_TAB) {
      return;
    }
    const ids = instanceIds.value;
    const from = ids.indexOf(current);
    if (from === -1) {
      return;
    }
    const to = from + offset;
    if (to < 0 || to >= ids.length) {
      return;
    }
    const next = ids.slice();
    next.splice(from, 1);
    next.splice(to, 0, current);
    reorderInstanceTabs(next);
  }

  function handleShortcut(event: ShortcutInput): void {
    if (
      event.defaultPrevented ||
      !event.ctrlKey ||
      event.altKey ||
      event.metaKey
    ) {
      return;
    }

    switch (event.key) {
      case "Tab":
        event.preventDefault?.();
        switchBy(event.shiftKey ? -1 : 1);
        return;
      case "PageDown":
        event.preventDefault?.();
        if (event.shiftKey) {
          moveActiveTab(1);
        } else {
          switchBy(1);
        }
        return;
      case "PageUp":
        event.preventDefault?.();
        if (event.shiftKey) {
          moveActiveTab(-1);
        } else {
          switchBy(-1);
        }
        return;
      case "w":
      case "W":
      case "F4":
        event.preventDefault?.();
        if (activeTab.value !== HOME_TAB) {
          closeInstanceTab(activeTab.value);
        }
        return;
      default:
        break;
    }

    if (!/^[1-9]$/.test(event.key)) {
      return;
    }

    // Ctrl+9 jumps to the last tab, matching common browser behaviour.
    const ids = tabIds();
    const index = event.key === "9" ? ids.length - 1 : Number(event.key) - 1;
    if (index >= 0 && index < ids.length) {
      event.preventDefault?.();
      selectByIndex(index);
    }
  }

  function onKeydown(event: KeyboardEvent): void {
    handleShortcut(event);
  }

  function onMessage(event: MessageEvent<unknown>): void {
    const data = event.data;
    if (!isBridgeMessage(data)) {
      return;
    }
    handleShortcut({
      key: data.key,
      shiftKey: data.shiftKey,
      ctrlKey: true,
      altKey: false,
      metaKey: false,
      defaultPrevented: false,
    });
  }

  onMounted(() => {
    window.addEventListener("keydown", onKeydown);
    window.addEventListener("message", onMessage);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("keydown", onKeydown);
    window.removeEventListener("message", onMessage);
  });
}
