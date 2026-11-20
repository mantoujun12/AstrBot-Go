<!-- Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
     Drawer layout reworked from the dashboard's ProviderSelectMenu overlay
     drawer: one section listing the launcher's instances, one reusing the
     shared model selector. -->
<template>
  <v-overlay
    v-model="open"
    class="instance-drawer-overlay"
    location="right"
    transition="slide-x-reverse-transition"
    :scrim="true"
  >
    <v-card class="instance-drawer-card" elevation="12">
      <div class="instance-drawer-header">
        <div class="instance-drawer-title">{{ tm("instanceModel.title") }}</div>
        <v-btn
          icon
          variant="text"
          :aria-label="t('core.common.close')"
          @click="open = false"
        >
          <v-icon>mdi-close</v-icon>
        </v-btn>
      </div>
      <div class="instance-drawer-content">
        <section class="instance-drawer-section">
          <div class="instance-drawer-section-title">
            {{ tm("instanceModel.instances") }}
          </div>
          <v-list density="compact" nav class="instance-list">
            <v-list-item
              v-for="instance in instances"
              :key="instance.id"
              :active="instance.id === currentInstanceId"
              rounded="lg"
              class="instance-item"
              :class="{ 'instance-item--unavailable': !isAvailable(instance) }"
              @click="selectInstance(instance.id)"
            >
              <v-list-item-title class="instance-item-title">
                {{ instance.name }}
              </v-list-item-title>
              <v-list-item-subtitle class="instance-item-subtitle">
                {{ statusLabel(instance) }}
              </v-list-item-subtitle>
              <template #append>
                <v-icon v-if="instance.id === currentInstanceId" size="18">
                  mdi-check
                </v-icon>
              </template>
            </v-list-item>
            <div v-if="instances.length === 0" class="instance-empty">
              {{ tm("instanceModel.noInstances") }}
            </div>
          </v-list>
        </section>
        <v-divider class="instance-drawer-divider" />
        <section class="instance-drawer-section">
          <div class="instance-drawer-section-title">
            {{ tm("instanceModel.model") }}
          </div>
          <ProviderModelMenu variant="input" />
        </section>
      </div>
    </v-card>
  </v-overlay>
</template>

<script setup lang="ts">
import { computed, inject } from "vue";
import { useLauncherStore } from "../../../stores/launcher";
import type { InstanceStatus } from "../../../types/launcher";
import { useI18n, useModuleI18n } from "@/i18n/composables";
import { chatInstanceSelectionKey } from "../../instanceSelection";
import ProviderModelMenu from "./ProviderModelMenu.vue";

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ "update:modelValue": [value: boolean] }>();

const open = computed({
  get: () => props.modelValue,
  set: (value: boolean) => emit("update:modelValue", value),
});

const { t } = useI18n();
const { tm } = useModuleI18n("features/chat");
const { instances } = useLauncherStore();

// Provided by ChatUIHost; switches route through the host back to ChatView,
// which owns the target and remounts the island per instance.
const selection = inject(chatInstanceSelectionKey, null);
const currentInstanceId = computed(() => selection?.current.value ?? null);

function isAvailable(instance: InstanceStatus): boolean {
  return instance.state === "running" && instance.dashboard_credentials_configured;
}

function statusLabel(instance: InstanceStatus): string {
  if (instance.state !== "running") {
    return tm("instanceModel.statusNotRunning");
  }
  if (!instance.dashboard_credentials_configured) {
    return tm("instanceModel.statusKeyMissing");
  }
  return tm("instanceModel.statusRunning");
}

function selectInstance(instanceId: string): void {
  if (!instanceId) return;
  selection?.select(instanceId);
}
</script>

<style scoped>
.instance-drawer-overlay {
  align-items: stretch;
  justify-content: flex-end;
}

.instance-drawer-card {
  display: flex;
  width: clamp(320px, 36vw, 440px);
  /* Start below the title-bar band so the window controls and the drawer
     header never overlap. */
  height: calc(100vh - var(--astrbot-toolbar-height, 50px) - 28px);
  flex-direction: column;
  margin: calc(var(--astrbot-toolbar-height, 50px) + 12px) 16px 16px;
  overflow: hidden;
  border-radius: var(--astrbot-radius-card);
}

.instance-drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 12px 8px 20px;
}

.instance-drawer-title {
  font-size: 1rem;
  font-weight: 600;
}

.instance-drawer-content {
  flex: 1;
  overflow-y: auto;
  padding: 4px 12px 16px;
}

.instance-drawer-section-title {
  padding: 8px 8px 6px;
  color: rgba(var(--v-theme-on-surface), 0.48);
  font-size: 12px;
  font-weight: 500;
}

.instance-list {
  padding: 0;
}

.instance-item {
  /* Keep the selected fill from accumulating a second state overlay. */
  --v-hover-opacity: 0;
  --v-activated-opacity: 0;
  margin: 0 0 2px !important;
  border-radius: 10px !important;
}

.instance-item:hover {
  background: rgba(var(--v-theme-on-surface), 0.05);
}

.instance-item.v-list-item--active {
  background: rgba(var(--v-theme-on-surface), 0.06);
  color: rgb(var(--v-theme-on-surface));
}

/* A stopped or keyless instance stays selectable (the send error surfaces
   naturally) but reads as muted so the capable targets stand out. */
.instance-item--unavailable :deep(.v-list-item__content) {
  opacity: 0.62;
}

.instance-item-title {
  font-size: 14px;
  font-weight: 600;
  line-height: 18px;
}

.instance-item-subtitle {
  font-size: 12px;
  line-height: 16px;
}

.instance-empty {
  padding: 16px;
  color: rgba(var(--v-theme-on-surface), 0.5);
  font-size: 12px;
  text-align: center;
}

.instance-drawer-divider {
  margin: 10px 8px;
}

@media (max-width: 600px) {
  .instance-drawer-overlay {
    align-items: stretch;
    justify-content: stretch;
  }

  .instance-drawer-card {
    width: 100dvw;
    height: calc(100dvh - var(--astrbot-toolbar-height, 50px));
    /* Keep the title-bar band free for the window controls on narrow windows too. */
    margin: var(--astrbot-toolbar-height, 50px) 0 0;
    border-radius: 0;
  }
}
</style>
