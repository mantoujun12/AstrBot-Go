import { computed, ref, watch } from "vue";
import type { ReduceMotionPreference } from "../types/launcher";

const preference = ref<ReduceMotionPreference>("system");
const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
const systemPrefersReduced = ref(mediaQuery.matches);

mediaQuery.addEventListener("change", (event) => {
  systemPrefersReduced.value = event.matches;
});

const reduced = computed(() => {
  if (preference.value === "system") {
    return systemPrefersReduced.value;
  }
  return preference.value === "on";
});

watch(
  reduced,
  (value) => {
    document.documentElement.toggleAttribute("data-reduce-motion", value);
  },
  { immediate: true },
);

export function setReduceMotionPreference(value: ReduceMotionPreference): void {
  preference.value = value;
}

export function useReduceMotion() {
  return { reduced };
}
