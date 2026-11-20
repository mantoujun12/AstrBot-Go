<!-- Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
     Promise-based confirmation dialog. ChatUIHost provides `$confirm` from it;
     WebView2 silently drops window.confirm, so the island needs this instead. -->
<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "@/i18n/composables";
import type { ConfirmDialogOptions } from "@/utils/confirmDialog";

const { t } = useI18n();

const isOpen = ref(false);
const title = ref("");
const message = ref("");
let resolvePromise: ((confirmed: boolean) => void) | null = null;

function settle(confirmed: boolean) {
  const resolve = resolvePromise;
  resolvePromise = null;
  resolve?.(confirmed);
}

function open(options: ConfirmDialogOptions): Promise<boolean> {
  // A second request supersedes the first instead of leaving it pending.
  settle(false);
  title.value = options.title || t("core.common.dialog.confirmTitle");
  message.value = options.message || t("core.common.dialog.confirmMessage");
  isOpen.value = true;
  return new Promise<boolean>((resolve) => {
    resolvePromise = resolve;
  });
}

function confirm() {
  isOpen.value = false;
  settle(true);
}

// Also reached by Escape and a backdrop click (v-dialog emits false).
function cancel() {
  isOpen.value = false;
  settle(false);
}

function onToggle(value: boolean) {
  if (value) {
    isOpen.value = true;
  } else {
    cancel();
  }
}

defineExpose({ open });
</script>

<template>
  <v-dialog :model-value="isOpen" max-width="400" @update:model-value="onToggle">
    <v-card class="confirm-dialog__card">
      <v-card-title>{{ title }}</v-card-title>
      <v-card-text class="confirm-dialog__message">{{ message }}</v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" size="small" @click="cancel">
          {{ t("core.common.dialog.cancelButton") }}
        </v-btn>
        <v-btn color="error" variant="flat" size="small" @click="confirm">
          {{ t("core.common.dialog.confirmButton") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
/* The shared `.v-dialog` rule pins dialog cards to the dialog token with
   !important; this confirm surface opts for the compact card token instead. */
.confirm-dialog__card {
  border-radius: var(--astrbot-radius-card) !important;
}
.confirm-dialog__message {
  padding: 10px 20px 0;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.875rem;
  line-height: 1.6;
  white-space: pre-wrap;
}
</style>
