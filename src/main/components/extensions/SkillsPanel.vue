<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useDisplay } from "vuetify";
import { useI18n } from "vue-i18n";
import type { DataTableHeader } from "vuetify";
import ConfirmDialog from "../common/ConfirmDialog.vue";
import EmptyState from "../common/EmptyState.vue";
import TargetInstanceList from "./TargetInstanceList.vue";
import { pickSkillZip } from "../../services/app";
import { toUserMessage } from "../../services/errors";
import {
  deleteInstanceSkill,
  installInstanceSkillZip,
  listInstancePluginSkills,
  listInstanceSkillFiles,
  listInstanceSkills,
  readInstanceSkillFile,
  setInstanceSkillEnabled,
  writeInstanceSkillFile,
} from "../../services/skills";
import { useLauncherStore } from "../../stores/launcher";
import { showErrorToast, showToast } from "../../stores/toast";
import type { InstanceStatus } from "../../types/launcher";
import type {
  InstancePluginSkill,
  InstanceSkill,
  SkillDirEntry,
} from "../../types/skills";

const { t } = useI18n();
const { mdAndDown } = useDisplay();
const { instances } = useLauncherStore();

const instanceId = ref<string | null>(null);
const search = ref("");
const loading = ref(false);
const loadError = ref<string | null>(null);
const skills = ref<InstanceSkill[]>([]);
const pluginSkills = ref<InstancePluginSkill[]>([]);
const pluginSkillsLoaded = ref(false);

const togglingName = ref<string | null>(null);

const batchMode = ref(false);
const selectedNames = ref<string[]>([]);
const batchConfirm = ref(false);
const batchDeleting = ref(false);

const pendingDelete = ref<InstanceSkill | null>(null);
const deleting = ref(false);

const uploadOpen = ref(false);
const uploadTarget = ref<string | null>(null);
const uploadFile = ref<{ path: string; nameHint: string } | null>(null);
const uploading = ref(false);

const editorOpen = ref(false);
const editorTitle = ref("");
const editorSkillName = ref("");
const editorPluginName = ref<string | null>(null);
const editorBuiltin = ref(false);
/** Directory currently listed in the file pane, "" is the skill root. */
const editorSubPath = ref("");
const editorEntries = ref<SkillDirEntry[]>([]);
const editorFilesLoading = ref(false);
const editorFilesError = ref<string | null>(null);
const editorFilePath = ref("");
const editorFileLoading = ref(false);
const editorFileError = ref<string | null>(null);
const editorContent = ref("");
const editorOriginal = ref("");
const saving = ref(false);

const selectedInstance = computed<InstanceStatus | null>(() => {
  if (!instanceId.value) {
    return null;
  }
  return instances.value.find((item) => item.id === instanceId.value) ?? null;
});

/** Skill edits write instance files, so a running instance rejects them. */
const canManage = computed(() => selectedInstance.value?.state === "stopped");

/** Plugin and builtin skills are read-only, matching upstream. */
const editorEditable = computed(
  () =>
    editorPluginName.value === null &&
    !editorBuiltin.value &&
    canManage.value &&
    !editorFileLoading.value,
);

const editorDirty = computed(() => editorContent.value !== editorOriginal.value);

function matchesQuery(haystacks: string[]): boolean {
  const query = search.value.trim().toLowerCase();
  return haystacks.some((value) => value.toLowerCase().includes(query));
}

const filteredSkills = computed(() => {
  const query = search.value.trim();
  if (!query) {
    return skills.value;
  }
  return skills.value.filter((skill) =>
    matchesQuery([skill.name, skill.description]),
  );
});

const filteredPluginSkills = computed(() => {
  const query = search.value.trim();
  if (!query) {
    return pluginSkills.value;
  }
  return pluginSkills.value.filter((skill) =>
    matchesQuery([skill.name, skill.description, skill.plugin_name]),
  );
});

const localHeaders = computed<DataTableHeader[]>(() => {
  const headers: DataTableHeader[] = [
    { title: t("skills.table.name"), key: "name", sortable: false },
    { title: t("skills.table.description"), key: "description", sortable: false },
  ];
  if (!batchMode.value) {
    headers.push(
      { title: t("skills.table.enabled"), key: "enabled", sortable: false, width: 90 },
      {
        title: t("instanceData.actionsColumn"),
        key: "actions",
        sortable: false,
        align: "end",
        width: 90,
      },
    );
  }
  return headers;
});

const pluginHeaders = computed<DataTableHeader[]>(() => [
  { title: t("skills.table.name"), key: "name", sortable: false },
  { title: t("skills.pluginSkills.plugin"), key: "plugin_name", sortable: false },
  { title: t("skills.table.description"), key: "description", sortable: false },
  {
    title: t("instanceData.actionsColumn"),
    key: "actions",
    sortable: false,
    align: "end",
    width: 70,
  },
]);

let loadGeneration = 0;

async function loadSkills(): Promise<void> {
  if (!instanceId.value) {
    skills.value = [];
    pluginSkills.value = [];
    pluginSkillsLoaded.value = false;
    return;
  }
  // A fast instance switch can overlap loads; only the newest one may win.
  const generation = ++loadGeneration;
  loading.value = true;
  loadError.value = null;
  try {
    const [local, plugin] = await Promise.all([
      listInstanceSkills(instanceId.value),
      listInstancePluginSkills(instanceId.value),
    ]);
    if (generation === loadGeneration) {
      skills.value = local;
      pluginSkills.value = plugin;
      pluginSkillsLoaded.value = true;
    }
  } catch (requestError) {
    if (generation === loadGeneration) {
      skills.value = [];
      pluginSkills.value = [];
      loadError.value = toUserMessage(requestError);
    }
  } finally {
    if (generation === loadGeneration) {
      loading.value = false;
    }
  }
}

async function toggleSkill(skill: InstanceSkill, enabled: boolean): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  togglingName.value = skill.name;
  try {
    await setInstanceSkillEnabled(instanceId.value, skill.name, enabled);
    skill.active = enabled;
  } catch (requestError) {
    showErrorToast(requestError);
  } finally {
    togglingName.value = null;
  }
}

/** Open the skill viewer, auto-selecting SKILL.md when the root has one. */
async function openViewer(
  skillName: string,
  pluginName: string | null,
  builtin: boolean,
): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  editorSkillName.value = skillName;
  editorPluginName.value = pluginName;
  editorBuiltin.value = builtin;
  editorTitle.value = pluginName ? `${pluginName} / ${skillName}` : skillName;
  editorSubPath.value = "";
  editorFilePath.value = "";
  editorContent.value = "";
  editorOriginal.value = "";
  editorFileError.value = null;
  editorOpen.value = true;
  await loadEditorEntries();
  const skillMd = editorEntries.value.find(
    (entry) => !entry.is_dir && (entry.name === "SKILL.md" || entry.name === "skill.md"),
  );
  if (skillMd) {
    await openEditorFile(skillMd);
  }
}

function fileRequest(path: string) {
  return {
    skill_name: editorSkillName.value,
    plugin_name: editorPluginName.value,
    builtin: editorBuiltin.value,
    path,
  };
}

async function loadEditorEntries(): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  editorFilesLoading.value = true;
  editorFilesError.value = null;
  try {
    editorEntries.value = await listInstanceSkillFiles(instanceId.value, fileRequest(editorSubPath.value));
  } catch (error) {
    editorEntries.value = [];
    editorFilesError.value = toUserMessage(error);
  } finally {
    editorFilesLoading.value = false;
  }
}

function openEditorEntry(entry: SkillDirEntry): void {
  if (entry.is_dir) {
    editorSubPath.value = entry.path;
    void loadEditorEntries();
  } else {
    void openEditorFile(entry);
  }
}

async function openEditorFile(entry: SkillDirEntry): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  editorFileLoading.value = true;
  editorFileError.value = null;
  try {
    const content = await readInstanceSkillFile(instanceId.value, fileRequest(entry.path));
    editorFilePath.value = entry.path;
    editorOriginal.value = content;
    editorContent.value = content;
  } catch (error) {
    editorFileError.value = toUserMessage(error);
  } finally {
    editorFileLoading.value = false;
  }
}

/** Navigate one directory up from the current file pane location. */
function goUp(): void {
  const current = editorSubPath.value;
  editorSubPath.value = current.includes("/")
    ? current.slice(0, current.lastIndexOf("/"))
    : "";
  void loadEditorEntries();
}

async function saveEditor(): Promise<void> {
  if (!instanceId.value || !editorEditable.value || !editorDirty.value) {
    editorOpen.value = false;
    return;
  }
  saving.value = true;
  try {
    await writeInstanceSkillFile(
      instanceId.value,
      fileRequest(editorFilePath.value),
      editorContent.value,
    );
    showToast(t("skills.toasts.saved", { name: editorFilePath.value || editorSkillName.value }));
    editorOpen.value = false;
    await loadSkills();
  } catch (error) {
    showErrorToast(error);
  } finally {
    saving.value = false;
  }
}

function confirmDelete(skill: InstanceSkill): void {
  if (!canManage.value) {
    return;
  }
  pendingDelete.value = skill;
}

async function performDelete(): Promise<void> {
  const skill = pendingDelete.value;
  if (!skill || !instanceId.value) {
    return;
  }
  deleting.value = true;
  try {
    await deleteInstanceSkill(instanceId.value, skill.name);
    showToast(t("skills.toasts.deleted", { name: skill.name }));
    pendingDelete.value = null;
    await loadSkills();
  } catch (requestError) {
    showErrorToast(requestError);
  } finally {
    deleting.value = false;
  }
}

function toggleBatchMode(): void {
  batchMode.value = !batchMode.value;
  selectedNames.value = [];
}

function confirmBatchDelete(): void {
  if (selectedNames.value.length > 0) {
    batchConfirm.value = true;
  }
}

async function performBatchDelete(): Promise<void> {
  if (!instanceId.value || batchDeleting.value) {
    return;
  }
  batchDeleting.value = true;
  const names = [...selectedNames.value];
  let failed = 0;
  for (const name of names) {
    try {
      await deleteInstanceSkill(instanceId.value, name);
    } catch {
      failed += 1;
    }
  }
  batchDeleting.value = false;
  batchConfirm.value = false;
  batchMode.value = false;
  selectedNames.value = [];
  if (failed > 0) {
    showToast(t("skills.toasts.batchPartial", { count: failed }), "warning");
  } else {
    showToast(t("skills.toasts.batchDeleted", { count: names.length }));
  }
  await loadSkills();
}

function openUpload(): void {
  uploadTarget.value = null;
  uploadFile.value = null;
  uploadOpen.value = true;
}

async function pickFile(): Promise<void> {
  try {
    const picked = await pickSkillZip();
    if (picked) {
      uploadFile.value = picked;
    }
  } catch (cause) {
    showErrorToast(cause);
  }
}

async function performUpload(): Promise<void> {
  const file = uploadFile.value;
  const target = uploadTarget.value;
  if (!file || !target) {
    return;
  }
  uploading.value = true;
  try {
    const installed = await installInstanceSkillZip(target, {
      zip_path: file.path,
      name_hint: file.nameHint,
    });
    showToast(t("skills.toasts.installed", { names: installed.join(", ") }));
    uploadOpen.value = false;
    if (instanceId.value !== target) {
      // The instance watcher reloads the skills for the new selection.
      instanceId.value = target;
    } else {
      await loadSkills();
    }
  } catch (requestError) {
    showErrorToast(requestError);
  } finally {
    uploading.value = false;
  }
}

watch(instanceId, () => {
  void loadSkills();
});

defineExpose({ openUpload });
</script>

<template>
  <div class="skills-panel">
    <div class="skills-panel__controls">
      <v-select
        v-model="instanceId"
        :items="instances"
        item-title="name"
        item-value="id"
        :label="t('skills.instance')"
        variant="outlined"
        density="compact"
        hide-details
        class="skills-panel__instance"
      />
      <v-text-field
        v-model="search"
        :label="t('skills.search')"
        prepend-inner-icon="mdi-magnify"
        variant="outlined"
        density="compact"
        hide-details
        clearable
        class="skills-panel__search"
      />
    </div>

    <v-alert
      v-if="selectedInstance && !canManage"
      type="warning"
      variant="tonal"
      density="comfortable"
      class="skills-panel__banner"
    >
      {{ t("skills.instanceRunning") }}
    </v-alert>

    <EmptyState
      v-if="!instanceId"
      :text="t('skills.noInstance')"
      icon="mdi-cursor-default-click-outline"
    />
    <template v-else>
      <v-progress-linear
        v-if="loading"
        indeterminate
        color="primary"
        height="5"
        rounded
        class="skills-panel__progress"
      />
      <v-alert v-else-if="loadError" type="error" variant="tonal" density="comfortable">
        {{ loadError }}
      </v-alert>
      <template v-else>
        <div class="page-section">
          <div class="page-section-head">
            <span class="page-section-title">{{ t("skills.localTitle") }}</span>
            <div class="page-actions">
              <v-btn
                v-if="!batchMode"
                variant="text"
                size="small"
                color="primary"
                :disabled="!canManage || skills.length === 0"
                @click="toggleBatchMode()"
              >
                {{ t("skills.batch.select") }}
              </v-btn>
              <template v-else>
                <v-btn
                  variant="text"
                  size="small"
                  :disabled="batchDeleting"
                  @click="toggleBatchMode()"
                >
                  {{ t("skills.batch.cancel") }}
                </v-btn>
                <v-btn
                  variant="text"
                  size="small"
                  color="error"
                  :disabled="selectedNames.length === 0"
                  @click="confirmBatchDelete()"
                >
                  {{ t("skills.batch.deleteSelected", { count: selectedNames.length }) }}
                </v-btn>
              </template>
            </div>
          </div>
          <EmptyState
            v-if="skills.length === 0"
            :text="t('skills.empty')"
            icon="mdi-lightning-bolt-outline"
          />
          <div v-else class="page-table-card">
            <v-data-table
              v-model:selected="selectedNames"
              :headers="localHeaders"
              :items="filteredSkills"
              item-value="name"
              :show-select="batchMode"
              hide-default-footer
              :items-per-page="-1"
              disable-sort
              hover
              :no-data-text="t('skills.searchEmpty')"
            >
              <template #item.name="{ item }">
                <span class="page-anywhere">{{ item.name }}</span>
              </template>
              <template #item.description="{ item }">
                <span class="page-anywhere">{{ item.description || "—" }}</span>
              </template>
              <template #item.enabled="{ item }">
                <v-switch
                  :model-value="item.active"
                  inset
                  color="primary"
                  hide-details
                  density="compact"
                  :disabled="!canManage || togglingName === item.name"
                  :aria-label="t('skills.toggleAria', { name: item.name })"
                  @update:model-value="
                    (value: unknown) => toggleSkill(item, value === true)
                  "
                />
              </template>
              <template #item.actions="{ item }">
                <div class="page-row skills-panel__row-actions">
                  <v-tooltip location="top">
                    <template #activator="{ props: tooltipProps }">
                      <v-btn
                        v-bind="tooltipProps"
                        icon="mdi-pencil-outline"
                        variant="text"
                        size="small"
                        color="primary"
                        :aria-label="t('skills.edit')"
                        :disabled="!canManage"
                        @click="openViewer(item.name, null, false)"
                      />
                    </template>
                    {{ t("skills.edit") }}
                  </v-tooltip>
                  <v-tooltip location="top">
                    <template #activator="{ props: tooltipProps }">
                      <v-btn
                        v-bind="tooltipProps"
                        icon="mdi-delete-outline"
                        variant="text"
                        size="small"
                        color="error"
                        :aria-label="t('common.actions.delete')"
                        :disabled="!canManage"
                        @click="confirmDelete(item)"
                      />
                    </template>
                    {{ t("common.actions.delete") }}
                  </v-tooltip>
                </div>
              </template>
            </v-data-table>
          </div>
        </div>

        <div v-if="pluginSkillsLoaded" class="page-section">
          <div class="page-section-head">
            <span class="page-section-title">{{ t("skills.pluginSkills.title") }}</span>
          </div>
          <div class="page-table-card">
            <v-data-table
              :headers="pluginHeaders"
              :items="filteredPluginSkills"
              item-value="name"
              hide-default-footer
              :items-per-page="-1"
              disable-sort
              hover
              :no-data-text="t('skills.pluginSkills.empty')"
            >
              <template #item.name="{ item }">
                <span class="page-anywhere">{{ item.name }}</span>
              </template>
              <template #item.plugin_name="{ item }">
                <span class="skills-panel__plugin-cell">
                  <span class="page-anywhere">{{ item.plugin_name }}</span>
                  <v-chip v-if="item.builtin" size="x-small" variant="tonal">
                    {{ t("skills.pluginSkills.builtin") }}
                  </v-chip>
                </span>
              </template>
              <template #item.description="{ item }">
                <span class="page-anywhere">{{ item.description || "—" }}</span>
              </template>
              <template #item.actions="{ item }">
                <div class="page-row skills-panel__row-actions">
                  <v-tooltip location="top">
                    <template #activator="{ props: tooltipProps }">
                      <v-btn
                        v-bind="tooltipProps"
                        icon="mdi-eye-outline"
                        variant="text"
                        size="small"
                        :aria-label="t('skills.pluginSkills.view')"
                        @click="openViewer(item.name, item.plugin_name, item.builtin)"
                      />
                    </template>
                    {{ t("skills.pluginSkills.view") }}
                  </v-tooltip>
                </div>
              </template>
            </v-data-table>
          </div>
        </div>
      </template>
    </template>

    <ConfirmDialog
      :model-value="pendingDelete !== null"
      :title="t('skills.deleteConfirm.title')"
      :message="
        pendingDelete
          ? t('skills.deleteConfirm.message', { name: pendingDelete.name })
          : ''
      "
      :confirm-label="t('common.actions.delete')"
      color="error"
      :loading="deleting"
      @update:model-value="(value: boolean) => !value && (pendingDelete = null)"
      @confirm="performDelete"
    />

    <ConfirmDialog
      :model-value="batchConfirm"
      :title="t('skills.batchConfirm.title')"
      :message="t('skills.batchConfirm.message', { count: selectedNames.length })"
      :confirm-label="t('common.actions.delete')"
      color="error"
      :loading="batchDeleting"
      @update:model-value="(value: boolean) => !value && (batchConfirm = false)"
      @confirm="performBatchDelete"
    />

    <ConfirmDialog
      :model-value="uploadOpen"
      :title="t('skills.uploadDialog.title')"
      :confirm-label="t('skills.upload')"
      :confirm-disabled="uploadTarget === null || uploadFile === null"
      :loading="uploading"
      max-width="520"
      @update:model-value="(value: boolean) => !value && (uploadOpen = false)"
      @confirm="performUpload"
    >
      <div class="skills-panel__upload">
        <TargetInstanceList :instances="instances" v-model="uploadTarget" />
        <div class="skills-panel__upload-file">
          <v-btn
            variant="outlined"
            size="small"
            prepend-icon="mdi-folder-zip-outline"
            @click="pickFile()"
          >
            {{ t("skills.uploadDialog.pickFile") }}
          </v-btn>
          <span v-if="uploadFile" class="page-anywhere skills-panel__upload-name">
            {{ uploadFile.nameHint }}.zip
          </span>
          <span v-else class="skills-panel__upload-none">
            {{ t("skills.uploadDialog.noFile") }}
          </span>
        </div>
      </div>
    </ConfirmDialog>

    <v-dialog
      v-model="editorOpen"
      max-width="1180"
      :fullscreen="mdAndDown"
      :persistent="saving"
      scroll-strategy="block"
    >
      <v-card class="skill-viewer">
        <div class="skill-viewer__header">
          <p class="skill-viewer__title">{{ editorTitle }}</p>
          <v-btn
            icon="mdi-close"
            variant="text"
            :disabled="saving"
            :aria-label="t('common.actions.cancel')"
            @click="editorOpen = false"
          />
        </div>

        <div class="skill-viewer__body">
          <div class="skill-viewer__files">
            <div class="skill-viewer__files-header">
              <v-btn
                icon="mdi-arrow-up"
                size="small"
                variant="text"
                :disabled="editorSubPath === '' || editorFilesLoading"
                :aria-label="t('skills.viewer.up')"
                @click="goUp()"
              />
              <span class="skill-viewer__dir">{{ editorSubPath || "/" }}</span>
            </div>
            <v-progress-linear
              v-if="editorFilesLoading"
              indeterminate
              color="primary"
              height="3"
            />
            <v-alert
              v-else-if="editorFilesError"
              type="error"
              variant="tonal"
              density="compact"
            >
              {{ editorFilesError }}
            </v-alert>
            <div v-else class="skill-viewer__file-list">
              <button
                v-for="entry in editorEntries"
                :key="`${entry.is_dir}:${entry.path}`"
                type="button"
                class="skill-viewer__file-row"
                :class="{ 'skill-viewer__file-row--active': editorFilePath === entry.path }"
                @click="openEditorEntry(entry)"
              >
                <v-icon size="18" aria-hidden="true">
                  {{ entry.is_dir ? "mdi-folder-outline" : "mdi-file-document-outline" }}
                </v-icon>
                <span class="skill-viewer__file-name">{{ entry.name }}</span>
              </button>
            </div>
          </div>

          <div class="skill-viewer__content">
            <div class="skill-viewer__content-header">
              <span class="skill-viewer__path">
                {{ editorFilePath || t("skills.viewer.noFile") }}
              </span>
              <v-chip v-if="editorDirty" size="small" color="warning" variant="tonal">
                {{ t("skills.viewer.unsaved") }}
              </v-chip>
            </div>
            <v-alert
              v-if="editorFileError"
              type="error"
              variant="tonal"
              density="compact"
            >
              {{ editorFileError }}
            </v-alert>
            <v-textarea
              v-else
              v-model="editorContent"
              :readonly="!editorEditable"
              variant="outlined"
              rows="24"
              no-resize
              class="skill-viewer__textarea"
              spellcheck="false"
            />
          </div>
        </div>

        <div class="skill-viewer__actions">
          <v-spacer />
          <v-btn variant="text" :disabled="saving" @click="editorOpen = false">
            {{ t("common.actions.cancel") }}
          </v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :loading="saving"
            :disabled="!editorEditable || !editorDirty"
            @click="saveEditor()"
          >
            {{ t("common.actions.save") }}
          </v-btn>
        </div>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.skills-panel__controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.skills-panel__instance {
  max-width: 280px;
  flex: 1 1 200px;
}

.skills-panel__search {
  max-width: 280px;
  flex: 1 1 200px;
}

.skills-panel__banner {
  margin-bottom: 12px;
}

.skills-panel__progress {
  margin-bottom: 12px;
}

.skills-panel__row-actions {
  justify-content: flex-end;
  gap: 2px;
}

.skills-panel__plugin-cell {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.skills-panel__upload {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.skills-panel__upload-file {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.skills-panel__upload-name {
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.8125rem;
}

.skills-panel__upload-none {
  color: rgba(var(--v-theme-on-surface), 0.45);
  font-size: 0.8125rem;
}

/* Mirrors the upstream skill editor dialog: a capped card with two panes
 * that each scroll internally instead of growing past the viewport. */
.skill-viewer {
  display: flex;
  flex-direction: column;
  max-height: min(88vh, 980px);
  overflow: hidden;
}

.skill-viewer__header {
  align-items: flex-start;
  display: flex;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 8px 8px 24px;
}

.skill-viewer__title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  line-height: 1.3;
  overflow-wrap: anywhere;
}

.skill-viewer__body {
  flex: 1 1 auto;
  display: grid;
  grid-template-columns: minmax(220px, 280px) minmax(0, 1fr);
  gap: 16px;
  min-height: 0;
  padding: 8px 24px 16px;
  overflow: hidden;
}

.skill-viewer__files {
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: var(--astrbot-radius-control);
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.skill-viewer__files-header {
  align-items: center;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  display: flex;
  gap: 8px;
  min-height: 44px;
  padding: 6px 10px;
}

.skill-viewer__dir {
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-size: 0.8125rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.skill-viewer__file-list {
  min-height: 0;
  overflow-y: auto;
  padding: 4px;
}

.skill-viewer__file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-height: 34px;
  padding: 4px 10px;
  border: 0;
  border-radius: var(--astrbot-radius-control);
  background: none;
  color: rgba(var(--v-theme-on-surface), 0.8);
  font-family: inherit;
  font-size: 0.8125rem;
  text-align: left;
  cursor: pointer;
}

.skill-viewer__file-row:hover {
  background: rgba(var(--v-theme-on-surface), 0.045);
}

.skill-viewer__file-row--active {
  background: rgba(var(--v-theme-on-surface), 0.065);
  color: rgb(var(--v-theme-on-surface));
}

.skill-viewer__file-name {
  min-width: 0;
  overflow-wrap: anywhere;
}

.skill-viewer__content {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  overflow: hidden;
}

.skill-viewer__content-header {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 32px;
}

.skill-viewer__path {
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-size: 0.8125rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.skill-viewer__textarea {
  flex: 1;
  min-height: 0;
  font-family: "Consolas", "Cascadia Mono", monospace;
}

.skill-viewer__textarea :deep(.v-field__input) {
  min-height: 340px;
  overflow-y: auto;
}

.skill-viewer__actions {
  display: flex;
  align-items: center;
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  padding: 12px 24px;
}
</style>
