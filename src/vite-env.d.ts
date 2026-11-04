/// <reference types="vite/client" />

interface Window {
  __ASTRBOT_THEME_PREFERENCE__?: string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
