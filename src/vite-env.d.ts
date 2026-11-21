/// <reference types="vite/client" />
/// <reference types="vitest/importMeta" />

interface Window {
  __ASTRBOT_THEME_PREFERENCE__?: string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
