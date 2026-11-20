// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Pinia-free island port of stores/headerContext.ts.

import { reactive } from "vue";

/**
 * Lets the chat page project contextual content into the host toolbar: the
 * page registers a component while it is visible, and the host renders it in
 * the toolbar slot.
 */
const store = reactive({
  component: null as unknown,
  SET_COMPONENT(component: unknown) {
    store.component = component;
  },
});

/** The reactive state itself, for host-side reads (toolbar rendering). */
export const headerContext = store;

export function useHeaderContextStore() {
  return store;
}
