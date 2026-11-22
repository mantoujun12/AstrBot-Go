<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { openExternalUrl } from "../../services/app";

type FrameState = "idle" | "loading" | "ready" | "error";

const props = defineProps<{
  name: string;
  url: string;
}>();

const { t } = useI18n();

const frameState = ref<FrameState>("idle");
const frameKey = ref(0);
let loadTimer: ReturnType<typeof setTimeout> | null = null;

function clearLoadTimer(): void {
  if (loadTimer !== null) {
    clearTimeout(loadTimer);
    loadTimer = null;
  }
}

function startFrameLoad(): void {
  clearLoadTimer();
  frameState.value = "loading";
  loadTimer = setTimeout(() => {
    loadTimer = null;
    if (frameState.value === "loading") {
      frameState.value = "error";
    }
  }, 12000);
}

function handleFrameLoad(): void {
  clearLoadTimer();
  frameState.value = "ready";
}

function reloadFrame(): void {
  frameKey.value += 1;
  startFrameLoad();
}

async function openInBrowser(): Promise<void> {
  try {
    await openExternalUrl(props.url);
  } catch {
    // The external browser handoff has no in-app error surface; failures are
    // silent by design.
  }
}

const showIframe = computed(() => props.url !== "");

watch(
  () => props.url,
  (url) => {
    if (url !== "") {
      startFrameLoad();
    } else {
      clearLoadTimer();
      frameState.value = "idle";
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  clearLoadTimer();
});
</script>

<template>
  <div class="instance-webui">
    <div class="instance-webui__toolbar">
      <span class="instance-webui__version">{{ name }}</span>
      <span
        class="instance-webui__address page-mono"
        :title="url"
      >
        {{ url }}
      </span>
      <v-spacer />
      <v-btn
        icon="mdi-refresh"
        variant="text"
        size="small"
        :aria-label="t('instances.webui.aria.refresh')"
        :disabled="!showIframe"
        @click="reloadFrame"
      />
      <v-btn
        icon="mdi-open-in-new"
        variant="text"
        size="small"
        :aria-label="t('instances.actions.openInBrowser')"
        :disabled="!showIframe"
        @click="openInBrowser"
      />
    </div>

    <div class="instance-webui__content">
      <div
        v-if="showIframe"
        class="instance-webui__frame-wrap"
      >
        <iframe
          :key="frameKey"
          :src="url"
          title="AstrBot WebUI"
          allow="clipboard-write; microphone; autoplay; fullscreen"
          class="instance-webui__frame"
          @load="handleFrameLoad"
        />
        <div
          v-if="frameState !== 'ready'"
          class="instance-webui__overlay"
        >
          <template v-if="frameState === 'loading'">
            <v-progress-circular
              indeterminate
              color="primary"
              size="36"
              :aria-label="t('instances.loading')"
            />
          </template>
          <template v-else-if="frameState === 'error'">
            <v-icon
              icon="mdi-alert-circle-outline"
              size="36"
              color="warning"
            />
            <p class="instance-webui__overlay-text">
              {{ t("instances.webui.loadTimeout") }}
            </p>
            <div class="instance-webui__overlay-actions">
              <v-btn
                variant="tonal"
                color="primary"
                prepend-icon="mdi-refresh"
                @click="reloadFrame"
              >
                {{ t("common.actions.retry") }}
              </v-btn>
              <v-btn
                variant="text"
                prepend-icon="mdi-open-in-new"
                @click="openInBrowser"
              >
                {{ t("instances.actions.openInBrowser") }}
              </v-btn>
            </div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.instance-webui {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.instance-webui__toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  box-sizing: border-box;
  flex: 0 0 48px;
  height: 48px;
  padding: 0 12px;
  background: rgb(var(--v-theme-surface));
}

.instance-webui__version {
  flex: 0 0 auto;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.8rem;
  font-weight: 600;
  white-space: nowrap;
}

.instance-webui__address {
  min-width: 0;
  overflow: hidden;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.78rem;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.instance-webui__content {
  position: relative;
  display: flex;
  flex: 1 1 auto;
  min-height: 0;
}

.instance-webui__frame-wrap {
  position: relative;
  flex: 1 1 auto;
  min-height: 0;
}

.instance-webui__frame {
  display: block;
  width: 100%;
  height: 100%;
  border: none;
  background: #ffffff;
}

.instance-webui__overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px;
  background: rgb(var(--v-theme-surface));
}

.instance-webui__overlay-text {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.9rem;
}

.instance-webui__overlay-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
