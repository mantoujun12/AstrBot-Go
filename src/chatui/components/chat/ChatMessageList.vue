<!-- Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE. -->
<!-- shortcut: replaced by the upstream renderer in the next layer -->
<template>
  <div
    ref="listRoot"
    class="chat-message-list"
    :class="[`variant-${variant}`, { 'is-dark': isDark }]"
  >
    <div class="messages-list">
      <div
        v-for="(msg, msgIndex) in messages"
        :key="msg.id || `${msgIndex}-${msg.created_at || ''}`"
        class="message-row"
        :class="isUserMessage(msg) ? 'from-user' : 'from-bot'"
        :data-message-id="msg.id == null ? undefined : String(msg.id)"
      >
        <v-avatar
          v-if="!isUserMessage(msg)"
          class="bot-avatar"
          :size="avatarSize"
        >
          <span class="bot-avatar-symbol" aria-hidden="true">✦</span>
        </v-avatar>

        <div class="message-stack">
          <div
            class="message-bubble"
            :class="{ user: isUserMessage(msg), bot: !isUserMessage(msg) }"
          >
            <span class="message-plain-text">{{ messageText(msg) }}</span>
            <span
              v-if="isStreaming(msg, msgIndex)"
              class="streaming-indicator"
              aria-hidden="true"
            >
              <span></span><span></span><span></span>
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { RegenerateModelSelection } from "@/components/chat/RegenerateMenu.vue";
import type {
  ChatRecord,
  ChatThread,
  MessagePart,
} from "@/composables/useMessages";

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

// Kept identical to the upstream surface so Chat.vue needs no changes.
const emit = defineEmits<{
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
// The plain list has nothing to emit yet; the declaration stays for parity.
void emit;

const listRoot = ref<HTMLElement | null>(null);
const avatarSize = computed(() => (props.variant === "thread" ? 36 : 56));

function messageContent(message: ChatRecord) {
  return message.content || { type: "bot", message: [] as MessagePart[] };
}

function isUserMessage(message: ChatRecord) {
  return messageContent(message).type === "user";
}

function messageParts(message: ChatRecord): MessagePart[] {
  const parts = messageContent(message).message;
  if (Array.isArray(parts)) return parts;
  if (typeof parts === "string") return [{ type: "plain", text: parts }];
  return [];
}

function messageText(message: ChatRecord): string {
  return messageParts(message)
    .map((part) => (typeof part.text === "string" ? part.text : ""))
    .filter(Boolean)
    .join("\n");
}

function isStreaming(message: ChatRecord, index: number): boolean {
  if (props.isMessageStreaming?.(message, index)) return true;
  return Boolean(messageContent(message).isLoading);
}
</script>

<style scoped>
.chat-message-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 20px;
}

.messages-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 900px;
  margin: 0 auto;
}

.message-row {
  display: flex;
  gap: 12px;
  align-items: flex-start;
}

.message-row.from-user {
  flex-direction: row-reverse;
}

.message-stack {
  min-width: 0;
  max-width: 78%;
}

.message-bubble {
  padding: 10px 14px;
  border-radius: var(--astrbot-radius-card, 12px);
  line-height: 1.55;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  background: rgba(var(--v-theme-surface-variant), 0.5);
}

.message-bubble.user {
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
}

.message-plain-text {
  display: inline;
}

.streaming-indicator {
  display: inline-flex;
  gap: 3px;
  margin-left: 6px;
  vertical-align: middle;
}

.streaming-indicator span {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.4;
  animation: chat-plain-pulse 1s infinite ease-in-out;
}

.streaming-indicator span:nth-child(2) {
  animation-delay: 0.15s;
}

.streaming-indicator span:nth-child(3) {
  animation-delay: 0.3s;
}

@keyframes chat-plain-pulse {
  0%,
  100% {
    opacity: 0.25;
  }
  50% {
    opacity: 0.9;
  }
}
</style>
