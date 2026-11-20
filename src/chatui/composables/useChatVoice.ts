// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// v1 stub of composables/useChatVoice.ts: voice input is disabled, so the
// composable stays permanently idle and all controls are inert.

import { ref } from "vue";

export function useChatVoice(_options: unknown) {
  const mode = ref<"transcribe" | "audio" | null>(null);
  const busy = ref(false);
  const levels = ref<number[]>([]);
  const isRecording = ref(false);
  const sttProviders = ref<{ title: string; value: string }[]>([]);
  const selectedSttProvider = ref("");

  return {
    mode,
    busy,
    levels,
    isRecording,
    sttProviders,
    selectedSttProvider,
    loadProviders: async () => {},
    start: () => {},
    finish: () => {},
    cancel: () => {},
  };
}
