// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// v1 stub of message_list_comps/PluginViewMessage.vue: plugin view parts have
// no bridge in the standalone window, so only their plain-text metadata shows.

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ part: Record<string, any> }>();

const fallbackText = computed(() => {
  const metadata = props.part?.view_metadata;
  const title = metadata?.title || metadata?.name || "";
  const description = metadata?.description || "";
  return [title, description].filter(Boolean).join("\n");
});
</script>

<template>
  <div v-if="fallbackText" class="plugin-view-fallback">{{ fallbackText }}</div>
</template>

<style scoped>
.plugin-view-fallback {
  margin: 8px 0;
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(var(--v-theme-on-surface), 0.06);
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
}
</style>
