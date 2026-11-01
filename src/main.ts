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
    defaultTheme: "PurpleTheme",
    themes: {
      PurpleTheme,
      PurpleThemeDark,
    },
  },
});

createApp(App).use(vuetify).use(router).mount("#app");
