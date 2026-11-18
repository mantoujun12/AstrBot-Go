<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../../common/ConfirmDialog.vue";
import ConfigFileEditorDialog from "./ConfigFileEditorDialog.vue";
import ConfigFilesSection from "./ConfigFilesSection.vue";
import CopyToInstancesDialog from "./CopyToInstancesDialog.vue";
import CronJobsSection from "./CronJobsSection.vue";
import KnowledgeBasesSection from "./KnowledgeBasesSection.vue";
import McpSection from "./McpSection.vue";
import PersonasSection from "./PersonasSection.vue";
import {
  deleteInstanceKnowledgeBase,
  isConfigFileMissing,
  isDataV4Missing,
  listInstanceConfigFiles,
  listInstanceCronJobs,
  listInstanceKnowledgeBases,
  listInstancePersonaFolders,
  listInstancePersonas,
  readInstanceConfigFile,
  saveInstanceCronJobs,
  saveInstancePersonas,
  writeInstanceConfigFile,
} from "../../../services/instanceData";
import { toUserMessage } from "../../../services/errors";
import { showErrorToast, showToast } from "../../../stores/toast";
import { useLauncherStore } from "../../../stores/launcher";
import type { InstanceStatus } from "../../../types/launcher";
import type {
  CopyPreselect,
  InstanceConfigFile,
  InstanceCronJob,
  InstanceCronJobInput,
  InstanceKnowledgeBase,
  InstancePersona,
  InstancePersonaFolder,
} from "../../../types/instanceData";
import { diffRecords, isSameConfigContent } from "../../../utils/instanceData";

type SectionKey =
  | "personas"
  | "cronJobs"
  | "mcpServers"
  | "configFiles"
  | "knowledgeBases";

/** AstrBot creates this file on first run; missing file = empty MCP config. */
const MCP_FILE = "mcp_server.json";

const props = defineProps<{
  modelValue: boolean;
  instance: InstanceStatus | null;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const { t } = useI18n();
const { instances } = useLauncherStore();

const section = ref<SectionKey>("personas");
const loading = ref(false);
const loadError = ref<string | null>(null);
const saving = ref(false);

const personas = ref<InstancePersona[]>([]);
const personaFolders = ref<InstancePersonaFolder[]>([]);
const cronJobs = ref<InstanceCronJob[]>([]);
const configFiles = ref<InstanceConfigFile[]>([]);
/** Lazily read file contents, keyed by file name. */
const configContents = ref<Record<string, string>>({});
/** Raw file content as loaded; dirty = staged content differs semantically. */
const configOriginal = ref<Record<string, string>>({});
const selectedConfigName = ref<string | null>(null);

const knowledgeBases = ref<InstanceKnowledgeBase[]>([]);
const kbsLoaded = ref(false);
const kbsLoading = ref(false);
const mcpTried = ref(false);
const mcpMissing = ref(false);
const mcpLoading = ref(false);

let originalPersonas: InstancePersona[] = [];
let originalCronJobs: InstanceCronJob[] = [];

const currentInstance = computed(
  () =>
    instances.value.find((item) => item.id === props.instance?.id) ??
    props.instance,
);

const readonly = computed(() => currentInstance.value?.state !== "stopped");

const personaDiff = computed(() =>
  diffRecords(originalPersonas, personas.value, (item) => item.persona_id),
);
const personasDirty = computed(
  () => personaDiff.value.upserts.length > 0 || personaDiff.value.deletes.length > 0,
);

const cronDiff = computed(() =>
  diffRecords(originalCronJobs, cronJobs.value, (item) => item.job_id),
);
const cronJobsDirty = computed(
  () => cronDiff.value.upserts.length > 0 || cronDiff.value.deletes.length > 0,
);

/**
 * A staged file counts as changed only when its content differs semantically
 * from the loaded file: reverting an edit (like AstrBot's config editor) or a
 * pure re-serialization clears the dirty state instead of asking to save.
 */
const configDirty = computed<ReadonlySet<string>>(() => {
  const dirty = new Set<string>();
  for (const [name, content] of Object.entries(configContents.value)) {
    const original = configOriginal.value[name];
    if (
      original === undefined ||
      !isSameConfigContent(name, original, content)
    ) {
      dirty.add(name);
    }
  }
  return dirty;
});

const dirty = computed(
  () =>
    personasDirty.value ||
    cronJobsDirty.value ||
    configDirty.value.size > 0,
);

const sections = computed(() => [
  {
    key: "personas" as const,
    icon: "mdi-account-box-outline",
    title: t("instanceData.sections.personas"),
  },
  {
    key: "cronJobs" as const,
    icon: "mdi-clock-outline",
    title: t("instanceData.sections.cronJobs"),
  },
  {
    key: "mcpServers" as const,
    icon: "mdi-server-network-outline",
    title: t("instanceData.sections.mcpServers"),
  },
  {
    key: "configFiles" as const,
    icon: "mdi-file-cog-outline",
    title: t("instanceData.sections.configFiles"),
  },
  {
    key: "knowledgeBases" as const,
    icon: "mdi-book-outline",
    title: t("instanceData.sections.knowledgeBases"),
  },
]);

async function load(): Promise<void> {
  const instance = props.instance;
  if (!instance) {
    return;
  }
  loading.value = true;
  loadError.value = null;
  // Folder failures alone must not blank the whole dialog.
  const folderPromise = listInstancePersonaFolders(instance.id).catch(
    () => [] as InstancePersonaFolder[],
  );
  try {
    const [nextPersonas, nextJobs, nextFiles] = await Promise.all([
      listInstancePersonas(instance.id),
      listInstanceCronJobs(instance.id),
      listInstanceConfigFiles(instance.id),
    ]);
    personas.value = nextPersonas;
    cronJobs.value = nextJobs;
    configFiles.value = nextFiles;
    personaFolders.value = await folderPromise;
    originalPersonas = nextPersonas;
    originalCronJobs = nextJobs;
    configContents.value = {};
    configOriginal.value = {};
    selectedConfigName.value = null;
    knowledgeBases.value = [];
    kbsLoaded.value = false;
    mcpTried.value = false;
    mcpMissing.value = false;
  } catch (cause) {
    personas.value = [];
    personaFolders.value = [];
    cronJobs.value = [];
    configFiles.value = [];
    originalPersonas = [];
    originalCronJobs = [];
    configContents.value = {};
    configOriginal.value = {};
    loadError.value = isDataV4Missing(cause)
      ? t("instanceData.v4Unsupported")
      : toUserMessage(cause);
  } finally {
    loading.value = false;
    // The active section may rely on lazily loaded content reset above.
    if (section.value === "mcpServers") {
      void ensureMcpContent();
    }
    if (section.value === "knowledgeBases") {
      void ensureKnowledgeBases();
    }
  }
}

/* MCP server config (staged like config files) */

async function ensureMcpContent(): Promise<void> {
  const instance = props.instance;
  if (!instance || mcpTried.value) {
    return;
  }
  mcpTried.value = true;
  if (configContents.value[MCP_FILE] !== undefined) {
    return;
  }
  mcpLoading.value = true;
  try {
    const content = await readInstanceConfigFile(instance.id, MCP_FILE);
    configOriginal.value[MCP_FILE] = content;
    configContents.value[MCP_FILE] = content;
  } catch (cause) {
    if (isConfigFileMissing(cause)) {
      // AstrBot creates mcp_server.json on first run; show the empty state
      // instead of creating it here.
      mcpMissing.value = true;
    } else {
      showErrorToast(cause);
    }
  } finally {
    mcpLoading.value = false;
  }
}

function handleMcpStage(content: string): void {
  configContents.value[MCP_FILE] = content;
}

/* Knowledge bases (immediate deletes, not staged) */

async function ensureKnowledgeBases(): Promise<void> {
  const instance = props.instance;
  if (!instance || kbsLoaded.value || kbsLoading.value) {
    return;
  }
  kbsLoading.value = true;
  try {
    knowledgeBases.value = await listInstanceKnowledgeBases(instance.id);
    kbsLoaded.value = true;
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    kbsLoading.value = false;
  }
}

const kbDeleteOpen = ref(false);
const kbDeleteTarget = ref<InstanceKnowledgeBase | null>(null);
const kbDeleting = ref(false);

function requestKbDelete(kb: InstanceKnowledgeBase): void {
  kbDeleteTarget.value = kb;
  kbDeleteOpen.value = true;
}

async function confirmKbDelete(): Promise<void> {
  const instance = props.instance;
  const kb = kbDeleteTarget.value;
  if (!instance || !kb || kbDeleting.value) {
    return;
  }
  kbDeleting.value = true;
  try {
    await deleteInstanceKnowledgeBase(instance.id, kb.kb_id);
    showToast(t("instanceData.knowledgeBases.deleted"));
    kbDeleteOpen.value = false;
    await ensureKnowledgeBases();
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    kbDeleting.value = false;
  }
}

watch(section, (value) => {
  if (value === "mcpServers") {
    void ensureMcpContent();
  }
  if (value === "knowledgeBases") {
    void ensureKnowledgeBases();
  }
});

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      section.value = "personas";
      void load();
    }
  },
);

function upsertPersona(persona: InstancePersona): void {
  const index = personas.value.findIndex(
    (item) => item.persona_id === persona.persona_id,
  );
  if (index === -1) {
    personas.value.push(persona);
  } else {
    personas.value.splice(index, 1, persona);
  }
}

function removePersonas(personaIds: string[]): void {
  personas.value = personas.value.filter(
    (item) => !personaIds.includes(item.persona_id),
  );
}

function upsertCronJob(job: InstanceCronJob): void {
  const index = cronJobs.value.findIndex((item) => item.job_id === job.job_id);
  if (index === -1) {
    cronJobs.value.push(job);
  } else {
    cronJobs.value.splice(index, 1, job);
  }
}

function removeCronJobs(jobIds: string[]): void {
  cronJobs.value = cronJobs.value.filter(
    (item) => !jobIds.includes(item.job_id),
  );
}

function toCronInput(job: InstanceCronJob): InstanceCronJobInput {
  return {
    job_id: job.job_id,
    name: job.name,
    description: job.description,
    job_type: job.job_type,
    cron_expression: job.cron_expression,
    timezone: job.timezone,
    payload: job.payload,
    enabled: job.enabled,
    persistent: job.persistent,
    run_once: job.run_once,
  };
}

async function save(): Promise<void> {
  const instance = props.instance;
  if (!instance || readonly.value || !dirty.value || saving.value) {
    return;
  }
  saving.value = true;
  try {
    const personaDiff = diffRecords(
      originalPersonas,
      personas.value,
      (item) => item.persona_id,
    );
    if (personaDiff.upserts.length > 0 || personaDiff.deletes.length > 0) {
      await saveInstancePersonas(
        instance.id,
        personaDiff.upserts,
        personaDiff.deletes,
      );
    }
    const cronDiff = diffRecords(
      originalCronJobs,
      cronJobs.value,
      (item) => item.job_id,
    );
    if (cronDiff.upserts.length > 0 || cronDiff.deletes.length > 0) {
      await saveInstanceCronJobs(
        instance.id,
        cronDiff.upserts.map(toCronInput),
        cronDiff.deletes,
      );
    }
    for (const name of configDirty.value) {
      await writeInstanceConfigFile(
        instance.id,
        name,
        configContents.value[name] ?? "",
      );
    }
    showToast(t("instanceData.saved"));
    await load();
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    saving.value = false;
  }
}

const confirmDiscardOpen = ref(false);

function requestClose(): void {
  if (dirty.value) {
    confirmDiscardOpen.value = true;
    return;
  }
  emit("update:modelValue", false);
}

// ConfirmDialog does not close itself on confirm; the parent owns the flag.
function discardAndClose(): void {
  confirmDiscardOpen.value = false;
  emit("update:modelValue", false);
}

function handleDialogUpdate(value: boolean): void {
  if (!value) {
    requestClose();
  }
}

/* Config file editor */

const editorOpen = ref(false);
const editorName = ref("");
const editorContent = ref("");

async function openConfigEditor(name: string): Promise<void> {
  const instance = props.instance;
  if (!instance || name === "") {
    return;
  }
  selectedConfigName.value = name;
  if (configContents.value[name] === undefined) {
    try {
      const content = await readInstanceConfigFile(instance.id, name);
      if (configContents.value[name] === undefined) {
        configOriginal.value[name] = content;
        configContents.value[name] = content;
      }
    } catch (cause) {
      showErrorToast(cause);
      return;
    }
  }
  editorName.value = name;
  editorContent.value = configContents.value[name] ?? "";
  editorOpen.value = true;
}

function handleEditorSave(content: string): void {
  const name = editorName.value;
  // Both the fullscreen editor and McpSection stage mcp_server.json into
  // this map; the last staged content wins. Dirtiness is derived from the
  // semantic diff against configOriginal.
  configContents.value[name] = content;
}

/* Copy to instances */

const copyOpen = ref(false);
const copyPreselect = ref<CopyPreselect>({
  personas: [],
  cronJobs: [],
  configFiles: [],
});

function openCopy(preselect: CopyPreselect): void {
  copyPreselect.value = preselect;
  copyOpen.value = true;
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="1200"
    @update:model-value="handleDialogUpdate"
  >
    <v-card class="instance-data">
      <div class="instance-data__header">
        <div class="page-cell">
          <v-card-title class="instance-data__title">
            {{ t("instanceData.title") }}
          </v-card-title>
          <v-card-subtitle class="instance-data__subtitle">
            {{ instance?.name }}
          </v-card-subtitle>
        </div>
        <v-btn
          icon="mdi-close"
          variant="text"
          size="small"
          :aria-label="t('common.actions.close')"
          @click="requestClose"
        />
      </div>
      <v-divider />

      <div class="instance-data__body">
        <v-list nav density="compact" class="instance-data__nav" mandatory>
          <v-list-item
            v-for="item in sections"
            :key="item.key"
            :prepend-icon="item.icon"
            :title="item.title"
            :active="section === item.key"
            @click="section = item.key"
          />
        </v-list>

        <div class="instance-data__content">
          <v-progress-linear
            v-if="loading"
            indeterminate
            color="primary"
            height="3"
          />
          <v-alert
            v-else-if="loadError"
            type="error"
            variant="tonal"
            density="comfortable"
          >
            {{ loadError }}
          </v-alert>
          <template v-else>
            <div v-show="section === 'personas'">
              <PersonasSection
                :personas="personas"
                :folders="personaFolders"
                :readonly="readonly"
                @upsert="upsertPersona"
                @remove="removePersonas"
                @copy="(personaId) => openCopy({ personas: [personaId], cronJobs: [], configFiles: [] })"
              />
            </div>
            <div v-show="section === 'cronJobs'">
              <CronJobsSection
                :jobs="cronJobs"
                :readonly="readonly"
                @upsert="upsertCronJob"
                @remove="removeCronJobs"
                @copy="(jobId) => openCopy({ personas: [], cronJobs: [jobId], configFiles: [] })"
              />
            </div>
            <div v-show="section === 'mcpServers'">
              <McpSection
                :content="configContents[MCP_FILE] ?? ''"
                :missing="mcpMissing"
                :loading="mcpLoading"
                :readonly="readonly"
                @stage="handleMcpStage"
              />
            </div>
            <div v-show="section === 'configFiles'">
              <ConfigFilesSection
                :files="configFiles"
                :selected-name="selectedConfigName"
                @select="selectedConfigName = $event"
                @edit="openConfigEditor"
                @copy="(name) => openCopy({ personas: [], cronJobs: [], configFiles: [name] })"
              />
            </div>
            <div v-show="section === 'knowledgeBases'">
              <KnowledgeBasesSection
                :kbs="knowledgeBases"
                :loaded="kbsLoaded"
                :loading="kbsLoading"
                :readonly="readonly"
                @delete="requestKbDelete"
              />
            </div>
          </template>

          <div class="instance-data__fab">
            <v-tooltip location="left">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-content-save"
                  size="small"
                  color="primary"
                  :variant="dirty ? 'flat' : 'tonal'"
                  :disabled="!dirty || readonly"
                  :loading="saving"
                  :aria-label="t('common.actions.save')"
                  @click="save"
                />
              </template>
              {{
                readonly
                  ? t("instanceData.readonlyTooltip")
                  : t("common.actions.save")
              }}
            </v-tooltip>
            <v-tooltip location="left">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-file-edit-outline"
                  size="small"
                  color="primary"
                  variant="tonal"
                  :disabled="configFiles.length === 0"
                  :aria-label="t('instanceData.editConfigFile')"
                  @click="
                    openConfigEditor(
                      selectedConfigName ?? configFiles[0]?.name ?? '',
                    )
                  "
                />
              </template>
              {{ t("instanceData.editConfigFile") }}
            </v-tooltip>
          </div>
        </div>
      </div>
    </v-card>
  </v-dialog>

  <ConfigFileEditorDialog
    v-model="editorOpen"
    :name="editorName"
    :content="editorContent"
    :readonly="readonly"
    @save="handleEditorSave"
  />

  <CopyToInstancesDialog
    v-model="copyOpen"
    :instance="instance"
    :personas="personas"
    :cron-jobs="cronJobs"
    :config-files="configFiles"
    :preselect="copyPreselect"
  />

  <ConfirmDialog
    v-model="confirmDiscardOpen"
    :title="t('instanceData.unsavedTitle')"
    :message="t('instanceData.unsavedMessage')"
    :confirm-label="t('common.actions.close')"
    @confirm="discardAndClose"
  />

  <ConfirmDialog
    v-model="kbDeleteOpen"
    :title="t('instanceData.knowledgeBases.deleteTitle')"
    :message="kbDeleteTarget === null ? '' : t('instanceData.knowledgeBases.deleteMessage', { name: kbDeleteTarget.name })"
    :confirm-label="t('common.actions.delete')"
    color="error"
    :loading="kbDeleting"
    @confirm="confirmKbDelete"
  />
</template>

<style scoped>
/* The nav paints the card's full height on the left, so it would otherwise
   paint a square corner over the card's dialog radius. */
.instance-data {
  overflow: hidden;
}

.instance-data__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px 8px 8px;
}

.instance-data__title {
  padding: 8px 8px 0;
  font-size: 1.15rem;
  font-weight: 600;
}

.instance-data__subtitle {
  padding: 2px 8px 0;
}

.instance-data__body {
  display: flex;
  min-height: 0;
  height: min(70vh, 720px);
}

/* Nav item metrics mirror LauncherSidebar so the rail reads as the same
   component family. */
.instance-data__nav {
  flex: 0 0 200px;
  border-right: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  padding: 2px 12px 12px;
  background: transparent;
}

.instance-data__nav :deep(.v-list-item) {
  min-height: 36px;
  margin-bottom: 2px;
  padding-inline: 10px;
  border-radius: var(--astrbot-radius-navigation);
  color: rgba(var(--v-theme-on-surface), 0.76);
  font-size: 0.9rem;
  font-weight: 500;
  transition:
    background-color 160ms ease,
    color 160ms ease;
}

.instance-data__nav :deep(.v-list-item:hover) {
  color: rgb(var(--v-theme-on-surface));
  background: rgba(var(--v-theme-on-surface), 0.05);
}

.instance-data__nav :deep(.v-list-item--active) {
  color: rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.08);
  font-weight: 600;
}

.instance-data__nav :deep(.v-list-item__overlay) {
  opacity: 0 !important;
}

.instance-data__nav :deep(.v-list-item__prepend) {
  min-width: 0;
  margin-inline-end: 10px;
}

.instance-data__nav :deep(.v-list-item__spacer) {
  width: 0;
}

.instance-data__nav :deep(.v-list-item__content) {
  min-width: 0;
}

.instance-data__nav :deep(.v-icon) {
  opacity: 0.9;
}

.instance-data__nav :deep(.v-list-item-title) {
  font-size: 14px;
  font-weight: 500;
  line-height: 20px;
}

.instance-data__content {
  position: relative;
  flex: 1 1 auto;
  min-width: 0;
  overflow-y: auto;
  padding: 16px 20px;
}

.instance-data__fab {
  position: absolute;
  right: 24px;
  bottom: 24px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
</style>
