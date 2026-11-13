<script setup lang="ts">
import { computed } from "vue";
import { openExternalUrl } from "../../services/app";
import { renderMarkdown } from "../../utils/markdown";

const props = defineProps<{
  content: string;
}>();

const html = computed(() => renderMarkdown(props.content));

function openLink(event: MouseEvent): void {
  const target = event.target;
  if (!(target instanceof Element)) {
    return;
  }

  const anchor = target.closest("a");
  if (!(anchor instanceof HTMLAnchorElement)) {
    return;
  }

  const href = anchor.getAttribute("href")?.trim();
  if (!href) {
    return;
  }

  event.preventDefault();
  if (/^https?:\/\//i.test(href)) {
    void openExternalUrl(href).catch(() => undefined);
  }
}

function handleAuxClick(event: MouseEvent): void {
  if (event.button === 1) {
    openLink(event);
  }
}
</script>

<template>
  <div
    class="markdown-content"
    @click="openLink"
    @auxclick="handleAuxClick"
    v-html="html"
  />
</template>

<style scoped>
.markdown-content {
  color: rgba(var(--v-theme-on-surface), 0.88);
  font-size: 0.875rem;
  line-height: 1.7;
  overflow-wrap: anywhere;
}

.markdown-content :deep(:first-child) {
  margin-top: 0;
}

.markdown-content :deep(:last-child) {
  margin-bottom: 0;
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4),
.markdown-content :deep(h5),
.markdown-content :deep(h6) {
  margin: 1.1em 0 0.45em;
  color: rgb(var(--v-theme-on-surface));
  font-weight: 600;
  letter-spacing: -0.01em;
  line-height: 1.35;
}

.markdown-content :deep(h1) {
  font-size: 1.35rem;
}

.markdown-content :deep(h2) {
  font-size: 1.18rem;
}

.markdown-content :deep(h3) {
  font-size: 1.05rem;
}

.markdown-content :deep(h4),
.markdown-content :deep(h5),
.markdown-content :deep(h6) {
  font-size: 0.95rem;
}

.markdown-content :deep(p) {
  margin: 0.6em 0;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin: 0.55em 0;
  padding-inline-start: 1.5em;
}

.markdown-content :deep(li) {
  margin: 0.2em 0;
}

.markdown-content :deep(li > p) {
  margin: 0.25em 0;
}

.markdown-content :deep(a) {
  color: rgb(var(--v-theme-primary));
  text-decoration: none;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

.markdown-content :deep(strong) {
  color: rgb(var(--v-theme-on-surface));
  font-weight: 600;
}

.markdown-content :deep(code) {
  padding: 0.15em 0.35em;
  border-radius: 4px;
  background: rgba(var(--v-theme-on-surface), 0.07);
  font-family: "Consolas", "Cascadia Mono", "Courier New", monospace;
  font-size: 0.85em;
}

.markdown-content :deep(pre) {
  margin: 0.75em 0;
  padding: 12px 14px;
  overflow-x: auto;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  border-radius: var(--astrbot-radius-control);
  background: rgba(var(--v-theme-on-surface), 0.05);
}

.markdown-content :deep(pre code) {
  padding: 0;
  background: transparent;
  font-size: 0.8rem;
  line-height: 1.6;
}

.markdown-content :deep(blockquote) {
  margin: 0.75em 0;
  padding: 0.15em 0 0.15em 12px;
  border-inline-start: 3px solid rgba(var(--v-theme-primary), 0.35);
  color: rgba(var(--v-theme-on-surface), 0.68);
}

.markdown-content :deep(blockquote > :first-child) {
  margin-top: 0;
}

.markdown-content :deep(blockquote > :last-child) {
  margin-bottom: 0;
}

.markdown-content :deep(table) {
  display: block;
  width: 100%;
  margin: 0.8em 0;
  overflow-x: auto;
  border-collapse: collapse;
  font-size: 0.82rem;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  padding: 6px 10px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  text-align: start;
}

.markdown-content :deep(th) {
  background: rgba(var(--v-theme-on-surface), 0.04);
  font-weight: 600;
}

.markdown-content :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: var(--astrbot-radius-control);
}

.markdown-content :deep(hr) {
  margin: 1em 0;
  border: 0;
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.12);
}
</style>
