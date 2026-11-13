import { createApp, nextTick } from "vue";
import { createVuetify } from "vuetify";
import { aliases, mdi } from "vuetify/iconsets/mdi";
import { en, ja, ru, zhHans } from "vuetify/locale";
import "@fontsource-variable/outfit";
import "@mdi/font/css/materialdesignicons.css";
import "vuetify/styles";
import "./scss/style.scss";
import "./styles/pages.css";
import "./styles/controls.css";
import "./styles/platform-effects.css";
import App from "./App.vue";
import { i18n, initialLocale } from "./i18n";
import { PurpleTheme } from "./theme/LightTheme";
import { PurpleThemeDark } from "./theme/DarkTheme";
import { router } from "./router";
import { applyThemeColors, readStoredThemeColors } from "./theme/customColors";
import { readCachedTheme } from "./utils/themeCache";
import { showMainWindow } from "./services/windowControls";

type DesktopPlatform = "macos" | "windows" | "web";

// Outside the Tauri webview there is no native window material, so the page
// keeps its opaque styling. Inside Tauri the UA picks the native platform.
function detectPlatform(): DesktopPlatform {
  if (!("__TAURI_INTERNALS__" in window)) return "web";
  if (/Macintosh|Mac OS X/.test(navigator.userAgent)) return "macos";
  if (/Windows/.test(navigator.userAgent)) return "windows";
  return "web";
}

// Set synchronously before createApp so the first paint already has the
// platform effects applied.
document.documentElement.dataset.platform = detectPlatform();

document.documentElement.lang = initialLocale;

const vuetify = createVuetify({
  icons: {
    defaultSet: "mdi",
    aliases,
    sets: {
      mdi,
    },
  },
  locale: {
    locale: initialLocale,
    fallback: "zh-CN",
    messages: {
      "zh-CN": zhHans,
      "en-US": en,
      "ru-RU": ru,
      "ja-JP": ja,
    },
  },
  theme: {
    defaultTheme: readCachedTheme(),
    themes: {
      light: PurpleTheme,
      dark: PurpleThemeDark,
    },
  },
});

const storedThemeColors = readStoredThemeColors();
if (storedThemeColors) {
  applyThemeColors(vuetify.theme, storedThemeColors);
}

createApp(App).use(i18n).use(vuetify).use(router).mount("#app");

void revealMainWindowAfterFirstPaint();

async function revealMainWindowAfterFirstPaint(): Promise<void> {
  await nextTick();
  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });

  try {
    await showMainWindow();
  } catch (error: unknown) {
    console.error("显示主窗口失败：", error);
  }
}
