// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Pinia-free island port of stores/mobileDrawer.ts.

import { reactive } from "vue";

/** Shared open state for the mobile navigation drawer. */
const store = reactive({
  open: false,
  SET(open: boolean) {
    store.open = open;
  },
  TOGGLE() {
    store.open = !store.open;
  },
});

export function useMobileDrawerStore() {
  return store;
}
