/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set to "true" to force the development mock backend inside Tauri. */
  readonly VITE_USE_MOCK?: string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
