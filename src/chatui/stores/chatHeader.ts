// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Pinia-free island port of stores/chatHeader.ts.

import { reactive } from "vue";

interface ChatHeaderContext {
  title: string;
  subtitle: string;
  projectId?: string;
}

const store = reactive({
  title: "",
  subtitle: "",
  workbenchWidth: null as number | null,
  workspaceFilesOpen: false,
  SET_CONTEXT({ title, subtitle }: ChatHeaderContext) {
    store.title = title || "";
    store.subtitle = subtitle || "";
  },
  SET_WORKSPACE_FILES_OPEN(open: boolean) {
    store.workspaceFilesOpen = open;
  },
  CLEAR_CONTEXT() {
    store.title = "";
    store.subtitle = "";
  },
});

export function useChatHeaderStore() {
  return store;
}
