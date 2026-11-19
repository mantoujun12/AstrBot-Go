<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { UnlistenFn } from "@tauri-apps/api/event";
import EmptyState from "../components/common/EmptyState.vue";
import ChatUIHost from "../chatui";
import {
  getChatWindowTarget,
  listenChatTargetChanged,
} from "../services/chat";
import { useLauncherStore } from "../stores/launcher";

const { t } = useI18n();
const { instances } = useLauncherStore();

const instanceId = ref<string | null>(null);
let unlistenTarget: UnlistenFn | null = null;

/** Target instance injected by the window before any page script runs. */
function readInjectedChatTarget(): string {
  const value = (
    window as unknown as { __ASTRBOT_CHAT_TARGET__?: unknown }
  ).__ASTRBOT_CHAT_TARGET__;
  return typeof value === "string" ? value : "";
}

const instance = computed(
  () => instances.value.find((entry) => entry.id === instanceId.value) ?? null,
);

const instanceAlert = computed(() => {
  const target = instance.value;
  if (target === null) {
    return "";
  }
  if (target.state !== "running") {
    return t("chat.states.notRunning");
  }
  if (!target.openapi_key_configured) {
    return t("chat.states.keyMissing");
  }
  return "";
});

// Fallback when the window has no stored target yet: prefer an instance that
// can chat right away, then any running instance, then the first instance —
// the guidance states lead the user to start and configure from there.
watch(
  instances,
  (list) => {
    if (instanceId.value !== null) {
      return;
    }
    const first =
      list.find(
        (entry) => entry.state === "running" && entry.openapi_key_configured,
      ) ??
      list.find((entry) => entry.state === "running") ??
      list[0];
    if (first) {
      instanceId.value = first.id;
    }
  },
  { immediate: true },
);

onMounted(() => {
  // The window injects the target before any page script runs; the command
  // and the switch event only refine or update it afterwards.
  const injected = readInjectedChatTarget();
  if (injected !== "") {
    instanceId.value = injected;
  }
  void getChatWindowTarget()
    .then((target) => {
      if (target !== "") {
        instanceId.value = target;
      }
    })
    .catch(() => {
      // No target stored (window opened outside open_chat_window); the
      // instances watcher above picks a capable instance instead.
    });
  void listenChatTargetChanged((target) => {
    if (target !== instanceId.value) {
      instanceId.value = target;
    }
  })
    .then((unlisten) => {
      unlistenTarget = unlisten;
    })
    .catch(() => {
      // A missed switch notification only delays the next manual switch.
    });
});

onBeforeUnmount(() => {
  unlistenTarget?.();
  unlistenTarget = null;
});
</script>

<template>
  <div class="chat-view">
    <template v-if="instanceId === null">
      <EmptyState
        class="chat-view__messages"
        :text="t('chat.emptyNoInstance')"
        icon="mdi-server-off"
      />
    </template>
    <template v-else>
      <v-alert
        v-if="instanceAlert !== ''"
        class="chat-view__alert"
        type="info"
        variant="tonal"
        density="comfortable"
      >
        {{ instanceAlert }}
      </v-alert>
      <ChatUIHost
        v-else
        class="chat-view__ui"
        :instance-id="instanceId"
      />
    </template>
  </div>
</template>

<style scoped>
.chat-view {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.chat-view__alert {
  margin: 16px;
}

.chat-view__ui {
  height: 100%;
}

.chat-view__messages {
  height: 100%;
}
</style>
