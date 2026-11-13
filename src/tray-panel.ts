import { createApp } from "vue";
import { createVuetify } from "vuetify";
import { aliases, mdi } from "vuetify/iconsets/mdi";
import { en, ja, ru, zhHans } from "vuetify/locale";
import "@fontsource-variable/outfit";
import "@mdi/font/css/materialdesignicons.css";
import "vuetify/styles";
import "./scss/_radius-tokens.scss";
import TrayPanel from "./components/tray/TrayPanel.vue";
import { i18n, initialLocale } from "./i18n";
import { applyThemeColors, readStoredThemeColors } from "./theme/customColors";
import { PurpleTheme } from "./theme/LightTheme";
import { PurpleThemeDark } from "./theme/DarkTheme";
import { readCachedTheme } from "./utils/themeCache";

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

createApp(TrayPanel).use(i18n).use(vuetify).mount("#tray-panel");
