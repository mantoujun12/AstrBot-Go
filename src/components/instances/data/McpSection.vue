<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../../common/ConfirmDialog.vue";
import EmptyState from "../../common/EmptyState.vue";
import { parseLinesField } from "../../../utils/instanceData";

const props = defineProps<{
  /** Raw mcp_server.json content; "" means nothing was loaded yet. */
  content: string;
  /** The file does not exist yet (AstrBot creates it on first run). */
  missing: boolean;
  loading: boolean;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (event: "stage", content: string): void;
}>();

const { t } = useI18n();

interface McpServerDraft {
  /** Stable per-draft key; the server name itself may be edited. */
  key: string;
  name: string;
  /** New servers may be renamed; existing keys stay fixed. */
  isNew: boolean;
  /** Original config object; structured fields overlay it on save. */
  original: Record<string, unknown>;
  command: string;
  args: string;
  url: string;
  active: boolean;
  env: string;
}

const parseError = ref(false);
const drafts = ref<McpServerDraft[]>([]);
const newServerName = ref("");
const removeOpen = ref(false);
const removeTarget = ref<McpServerDraft | null>(null);

let originalRoot: Record<string, unknown> = {};
let lastEmitted: string | null = null;
let draftSeq = 0;

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function toDraft(
  name: string,
  value: Record<string, unknown>,
  isNew: boolean,
): McpServerDraft {
  draftSeq += 1;
  return {
    key: `s${draftSeq}`,
    name,
    isNew,
    original: value,
    command: typeof value.command === "string" ? value.command : "",
    args: Array.isArray(value.args)
      ? value.args.filter((arg): arg is string => typeof arg === "string").join("\n")
      : "",
    url: typeof value.url === "string" ? value.url : "",
    active: value.active !== false,
    env: isPlainObject(value.env) ? JSON.stringify(value.env, null, 2) : "",
  };
}

function rebuild(content: string): void {
  try {
    const parsed: unknown = JSON.parse(content);
    if (!isPlainObject(parsed) || !isPlainObject(parsed.mcpServers)) {
      throw new Error("not an MCP config");
    }
    originalRoot = parsed;
    parseError.value = false;
    drafts.value = Object.entries(parsed.mcpServers).map(([name, value]) =>
      toDraft(name, isPlainObject(value) ? value : {}, false),
    );
  } catch {
    parseError.value = true;
    drafts.value = [];
  }
}

watch(
  () => props.content,
  (content) => {
    // Skip the echo of our own staged content to keep edit state stable.
    if (content === lastEmitted) {
      return;
    }
    lastEmitted = null;
    rebuild(content);
  },
  { immediate: true },
);

function envOf(draft: McpServerDraft): Record<string, unknown> | null {
  const text = draft.env.trim();
  if (text === "") {
    return {};
  }
  try {
    const parsed: unknown = JSON.parse(text);
    return isPlainObject(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function envInvalid(draft: McpServerDraft): boolean {
  return envOf(draft) === null;
}

/** Serializes the drafts over the original objects and stages the file. */
function stage(): void {
  if (
    props.readonly ||
    props.loading ||
    props.missing ||
    parseError.value ||
    drafts.value.some(envInvalid) ||
    drafts.value.some((draft) => draft.name.trim() === "")
  ) {
    return;
  }
  const names = new Set(drafts.value.map((draft) => draft.name));
  if (names.size !== drafts.value.length) {
    // Duplicate names would silently merge configs in the object map.
    return;
  }
  const mcpServers: Record<string, unknown> = {};
  for (const draft of drafts.value) {
    const merged: Record<string, unknown> = { ...draft.original };
    merged.command = draft.command;
    merged.args = parseLinesField(draft.args);
    if (draft.url.trim() !== "") {
      merged.url = draft.url;
    } else {
      delete merged.url;
    }
    merged.active = draft.active;
    const env = envOf(draft);
    if (env !== null) {
      if (draft.env.trim() === "") {
        delete merged.env;
      } else {
        merged.env = env;
      }
    }
    mcpServers[draft.name] = merged;
  }
  // Spread keeps the original root key order for string keys.
  const output = JSON.stringify({ ...originalRoot, mcpServers }, null, 2);
  lastEmitted = output;
  emit("stage", output);
}

watch(drafts, stage, { deep: true });

const newNameError = computed(() => {
  const name = newServerName.value.trim();
  if (name === "") {
    return "";
  }
  if (drafts.value.some((draft) => draft.name === name)) {
    return t("instanceData.mcpServers.duplicateName");
  }
  return "";
});

function addServer(): void {
  const name = newServerName.value.trim();
  if (props.readonly || name === "" || newNameError.value !== "") {
    return;
  }
  drafts.value.push(toDraft(name, {}, true));
  newServerName.value = "";
}

function requestRemove(draft: McpServerDraft): void {
  removeTarget.value = draft;
  removeOpen.value = true;
}

function confirmRemove(): void {
  const target = removeTarget.value;
  removeTarget.value = null;
  removeOpen.value = false;
  drafts.value = drafts.value.filter((draft) => draft !== target);
}
</script>

<template>
  <div class="mcp">
    <v-progress-linear
      v-if="loading"
      indeterminate
      color="primary"
      height="3"
      rounded
    />
    <EmptyState
      v-else-if="missing"
      :text="t('instanceData.mcpServers.fileMissing')"
      icon="mdi-server-network-outline"
    />
    <template v-else-if="!parseError">
      <div class="mcp__toolbar page-row">
        <v-text-field
          v-model="newServerName"
          variant="outlined"
          density="compact"
          hide-details
          class="mcp__new-name"
          :label="t('instanceData.mcpServers.newServerName')"
          :error-messages="newNameError === '' ? [] : [newNameError]"
          :disabled="readonly"
          @keydown.enter.prevent="addServer"
        />
        <v-btn
          size="small"
          color="primary"
          variant="tonal"
          prepend-icon="mdi-plus"
          :disabled="readonly || newServerName.trim() === ''"
          @click="addServer"
        >
          {{ t("instanceData.mcpServers.addServer") }}
        </v-btn>
      </div>

      <div
        v-if="drafts.length === 0"
        class="mcp__empty"
      >
        <EmptyState :text="t('instanceData.mcpServers.empty')" />
      </div>

      <v-card
        v-for="draft in drafts"
        :key="draft.key"
        variant="outlined"
        class="mcp__server"
      >
        <div class="mcp__server-head">
          <v-text-field
            v-model="draft.name"
            variant="outlined"
            density="compact"
            hide-details
            :label="t('instanceData.mcpServers.fields.name')"
            :disabled="readonly || !draft.isNew"
          />
          <v-tooltip location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-delete-outline"
                variant="text"
                size="small"
                color="error"
                :aria-label="t('instanceData.mcpServers.removeServer')"
                :disabled="readonly"
                @click="requestRemove(draft)"
              />
            </template>
            {{ t("instanceData.mcpServers.removeServer") }}
          </v-tooltip>
        </div>
        <v-text-field
          v-model="draft.command"
          variant="outlined"
          density="compact"
          hide-details
          :label="t('instanceData.mcpServers.fields.command')"
          :disabled="readonly"
        />
        <v-text-field
          v-model="draft.url"
          variant="outlined"
          density="compact"
          hide-details
          :label="t('instanceData.mcpServers.fields.url')"
          :disabled="readonly"
        />
        <v-textarea
          v-model="draft.args"
          variant="outlined"
          density="compact"
          rows="2"
          class="page-mono"
          :label="t('instanceData.mcpServers.fields.args')"
          :placeholder="t('instanceData.personas.fields.beginDialogsPlaceholder')"
          :disabled="readonly"
        />
        <v-textarea
          v-model="draft.env"
          variant="outlined"
          density="compact"
          rows="3"
          class="page-mono"
          :label="t('instanceData.mcpServers.fields.env')"
          :error-messages="envInvalid(draft) ? [t('instanceData.configFiles.jsonInvalid')] : []"
          :disabled="readonly"
        />
        <v-switch
          v-model="draft.active"
          inset
          color="primary"
          density="compact"
          hide-details
          :label="t('instanceData.mcpServers.fields.active')"
          :disabled="readonly"
        />
      </v-card>
    </template>
    <v-alert
      v-else
      type="error"
      variant="tonal"
      density="comfortable"
    >
      {{ t("instanceData.mcpServers.parseError") }}
    </v-alert>

    <ConfirmDialog
      v-model="removeOpen"
      :title="t('instanceData.mcpServers.removeServer')"
      :message="removeTarget === null ? '' : t('instanceData.mcpServers.removeMessage', { name: removeTarget.name })"
      :confirm-label="t('common.actions.delete')"
      color="error"
      @confirm="confirmRemove"
    />
  </div>
</template>

<style scoped>
.mcp__toolbar {
  gap: 8px;
  margin-bottom: 12px;
}

.mcp__new-name {
  max-width: 280px;
}

.mcp__empty {
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  border-radius: var(--astrbot-radius-card);
}

.mcp__server {
  padding: 14px 16px 6px;
  margin-bottom: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.mcp__server-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mcp__server-head :deep(.v-field) {
  background: transparent;
}
</style>
