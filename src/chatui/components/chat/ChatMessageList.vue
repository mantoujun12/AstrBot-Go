<!-- Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE. -->
<!-- shortcut: plain-text placeholder for the ported message renderer. Markdown,
     reasoning blocks and tool cards arrive with feat/chatui-render, which
     replaces this file with the real one. -->
<template>
  <div class="chat-message-list" :class="`chat-message-list--${variant}`">
    <div
      v-for="(message, index) in messages"
      :key="message.id ?? index"
      class="message-row"
      :class="{ 'message-row--user': isUserMessage(message) }"
      :data-message-id="message.id"
    >
      <div class="message-bubble">
        <p v-for="(text, partIndex) in textParts(message)" :key="partIndex" class="message-text">{{ text }}</p>
        <span
          v-if="isPending(message, index)"
          class="message-typing"
          role="status"
          aria-label="…"
        >
          <i /><i /><i />
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  displayParts,
  type ChatRecord,
  type ChatThread,
} from "@/composables/useMessages";
import type { RegenerateModelSelection } from "@/components/chat/RegenerateMenu.vue";

const props = withDefaults(
  defineProps<{
    messages: ChatRecord[];
    isDark?: boolean;
    isMessageStreaming?: (message: ChatRecord, index: number) => boolean;
    variant?: "main" | "thread";
    enableEdit?: boolean;
    enableRegenerate?: boolean;
    enableThreadSelection?: boolean;
    enableCopy?: boolean;
    manageRefsSidebar?: boolean;
    editingMessageId?: string | number | null;
    editDraft?: string;
    savingEdit?: boolean;
  }>(),
  {
    isDark: false,
    variant: "main",
    enableEdit: false,
    enableRegenerate: false,
    enableThreadSelection: false,
    enableCopy: true,
    manageRefsSidebar: true,
    editingMessageId: null,
    editDraft: "",
    savingEdit: false,
  },
);

// Declared so the host's listeners stay valid once the real list lands.
defineEmits<{
  configureModels: [];
  "update:editDraft": [value: string];
  openEdit: [message: ChatRecord];
  cancelEdit: [];
  saveEdit: [];
  regenerate: [message: ChatRecord];
  regenerateWithModel: [
    message: ChatRecord,
    selection: RegenerateModelSelection,
  ];
  selectBotText: [event: MouseEvent, message: ChatRecord];
  openThread: [thread: ChatThread];
  openRefs: [refs: unknown];
  buttonClick: [payload: { message: ChatRecord; callbackData: string }];
}>();

function isUserMessage(message: ChatRecord): boolean {
  return (message.content?.type ?? "bot") === "user";
}

function textParts(message: ChatRecord): string[] {
  const content = message.content ?? { type: "bot", message: [] };
  return displayParts(content)
    .map((part) => part.text ?? part.code ?? part.think ?? part.message ?? "")
    .filter((text): text is string => typeof text === "string" && text !== "");
}

function isPending(message: ChatRecord, index: number): boolean {
  return (
    props.isMessageStreaming?.(message, index) === true ||
    message.content?.isLoading === true
  );
}
</script>

<style scoped>
.chat-message-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
}

.message-row {
  display: flex;
  justify-content: flex-start;
}

.message-row--user {
  justify-content: flex-end;
}

.message-bubble {
  max-width: min(72ch, 80%);
  padding: 10px 14px;
  border-radius: 12px;
  background: rgba(var(--v-theme-on-surface), 0.06);
  color: rgb(var(--v-theme-on-surface));
  white-space: pre-wrap;
  word-break: break-word;
}

.message-row--user .message-bubble {
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
}

.message-text {
  margin: 0;
}

.message-text + .message-text {
  margin-top: 8px;
}

.message-typing {
  display: inline-flex;
  gap: 4px;
  padding: 4px 0;
}

.message-typing i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.4;
  animation: chat-typing 1.2s infinite ease-in-out;
}

.message-typing i:nth-child(2) {
  animation-delay: 0.15s;
}

.message-typing i:nth-child(3) {
  animation-delay: 0.3s;
}

@keyframes chat-typing {
  0%,
  80%,
  100% {
    opacity: 0.25;
  }
  40% {
    opacity: 0.9;
  }
}
</style>
