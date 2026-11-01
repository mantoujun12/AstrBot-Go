import { onBeforeUnmount, onMounted } from "vue";
import { HOME_TAB, useTabsStore } from "../stores/tabs";

/**
 * Browser-style tab shortcuts:
 * - Ctrl+Tab / Ctrl+PageDown: next tab
 * - Ctrl+Shift+Tab / Ctrl+PageUp: previous tab
 * - Ctrl+1…8: nth tab, Ctrl+9: last tab
 * - Ctrl+W / Ctrl+F4: close the active tab (the home tab stays open)
 */
export function useTabShortcuts(): void {
  const {
    instanceIds,
    activeTab,
    activateInstanceTab,
    closeInstanceTab,
    activateHome,
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

  function onKeydown(event: KeyboardEvent): void {
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
        event.preventDefault();
        switchBy(event.shiftKey ? -1 : 1);
        return;
      case "PageDown":
        event.preventDefault();
        switchBy(1);
        return;
      case "PageUp":
        event.preventDefault();
        switchBy(-1);
        return;
      case "w":
      case "W":
      case "F4":
        event.preventDefault();
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
      event.preventDefault();
      selectByIndex(index);
    }
  }

  onMounted(() => {
    window.addEventListener("keydown", onKeydown);
  });

  onBeforeUnmount(() => {
    window.removeEventListener("keydown", onKeydown);
  });
}
