import { isTauri } from "@tauri-apps/api/core";
import { createApp } from "vue";
import { createVuetify } from "vuetify";
import { aliases, mdi } from "vuetify/iconsets/mdi";
import "@fontsource-variable/outfit";
import "@mdi/font/css/materialdesignicons.css";
import "vuetify/styles";
import "./scss/style.scss";
import App from "./App.vue";
import { PurpleTheme } from "./theme/LightTheme";
import { PurpleThemeDark } from "./theme/DarkTheme";
import { router } from "./router";

const vuetify = createVuetify({
  icons: {
    defaultSet: "mdi",
    aliases,
    sets: {
      mdi,
    },
  },
  theme: {
    defaultTheme: "system",
    themes: {
      light: PurpleTheme,
      dark: PurpleThemeDark,
    },
  },
});

async function bootstrap(): Promise<void> {
  // Browser-only development uses the mock backend; `pnpm tauri dev` keeps
  // using the real Rust backend unless VITE_USE_MOCK=true is set explicitly.
  const runningInTauri = isTauri() || "__TAURI_INTERNALS__" in window;
  const mockEnabled =
    import.meta.env.DEV &&
    (import.meta.env.VITE_USE_MOCK === "true" || !runningInTauri);

  if (mockEnabled) {
    const { installMockBackend } = await import("./mocks");
    installMockBackend();
  }

  createApp(App).use(vuetify).use(router).mount("#app");
}

void bootstrap().catch((error: unknown) => {
  console.error("前端启动失败", error);
});
