<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import ChatUIHost from "../chatui";
import {
  getChatWindowTarget,
  listenChatTargetChanged,
} from "../services/chat";
import { useLauncherStore } from "../stores/launcher";

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

// The chat UI mounts unconditionally; send errors surface naturally when the
// target is stopped or keyless. This only picks the INITIAL instance: prefer
// one that can chat right away, then any running instance, then the first.
watch(
  instances,
  (list) => {
    if (instanceId.value !== null) {
      return;
    }
    const first =
      list.find(
        (entry) =>
          entry.state === "running" &&
          entry.dashboard_credentials_configured,
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
    <ChatUIHost
      class="chat-view__ui"
      :instance-id="instanceId"
      @select-instance="instanceId = $event"
    />
  </div>
</template>

<style scoped>
.chat-view {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.chat-view__ui {
  height: 100%;
}
</style>
