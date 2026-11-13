<script setup lang="ts">
import { useI18n } from "vue-i18n";
import MarkdownContent from "../common/MarkdownContent.vue";
import type { GitHubRelease } from "../../types/launcher";

defineProps<{
  modelValue: boolean;
  release: GitHubRelease | null;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const { t } = useI18n();
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="720"
    scrollable
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title class="release-notes__title">
        {{ release?.tag_name ?? t("versions.releaseNotes.title") }}
      </v-card-title>
      <v-card-text class="release-notes__body">
        <MarkdownContent
          v-if="release?.body"
          :content="release.body"
        />
        <p v-else class="release-notes__empty">
          {{ t("versions.releaseNotes.empty") }}
        </p>
      </v-card-text>
      <v-card-actions class="release-notes__actions">
        <v-spacer />
        <v-btn variant="text" @click="emit('update:modelValue', false)">
          {{ t("common.actions.close") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.release-notes__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
  white-space: normal;
}

.release-notes__body {
  max-height: 60vh;
  padding: 12px 20px;
  overflow-y: auto;
}

.release-notes__empty {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.875rem;
}

.release-notes__actions {
  padding: 12px 16px 16px;
}
</style>
