// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Pinia-free island port of stores/customizer.ts. `uiTheme` mirrors the host
// app's Vuetify theme (synced by ChatUIHost), so upstream `isDark` checks keep
// working; theme changes from the island's own settings dialog are forwarded
// to the host handler.

import { reactive } from "vue";
import { chatPreferences } from "../utils/chatPreferences";

type ThemeMode = "light" | "dark" | "system";

const store = reactive({
  fontTheme: "Noto Sans SC",
  uiTheme: "PurpleTheme",
  themeMode: "light" as ThemeMode,
  chatNavigationInSidebar: false,
  chatSidebarOpen: false,
  chatSidebarCollapsed: false,
  SET_THEME_MODE(mode: ThemeMode) {
    themeChangeHandler?.(mode);
  },
  SET_CHAT_NAVIGATION_IN_SIDEBAR(payload: boolean) {
    store.chatNavigationInSidebar = payload;
    chatPreferences((preferences) => {
      preferences.global.navigationInSidebar = payload;
    });
  },
  TOGGLE_CHAT_SIDEBAR() {
    store.chatSidebarOpen = !store.chatSidebarOpen;
  },
  SET_CHAT_SIDEBAR(payload: boolean) {
    store.chatSidebarOpen = payload;
  },
  SET_CHAT_SIDEBAR_COLLAPSED(payload: boolean) {
    store.chatSidebarCollapsed = payload;
  },
});

/** The reactive state itself, for host-side reads (sidebar width). */
export const customizerState = store;

let themeChangeHandler: ((mode: ThemeMode) => void) | null = null;

/** ChatUIHost registers the handler that applies theme mode app-wide. */
export function registerThemeChange(handler: (mode: ThemeMode) => void) {
  themeChangeHandler = handler;
}

/** ChatUIHost pushes the host Vuetify theme into the island. */
export function setIsDark(dark: boolean) {
  store.uiTheme = dark ? "PurpleThemeDark" : "PurpleTheme";
  store.themeMode = dark ? "dark" : "light";
}

export function useCustomizerStore() {
  return store;
}
