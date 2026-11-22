<script setup lang="ts">
import { computed, ref, watch } from "vue";
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
  listInstanceSkills,
  readInstancePluginSkill,
  readInstanceSkill,
  setInstanceSkillEnabled,
  writeInstanceSkill,
} from "../../services/skills";
import { useLauncherStore } from "../../stores/launcher";
import { showErrorToast, showToast } from "../../stores/toast";
import type { InstanceStatus } from "../../types/launcher";
import type { InstancePluginSkill, InstanceSkill } from "../../types/skills";

const { t } = useI18n();
const { instances } = useLauncherStore();

const instanceId = ref<string | null>(null);
const loading = ref(false);
const loadError = ref<string | null>(null);
const skills = ref<InstanceSkill[]>([]);
const pluginSkills = ref<InstancePluginSkill[]>([]);
const pluginSkillsLoaded = ref(false);

const togglingName = ref<string | null>(null);

const pendingDelete = ref<InstanceSkill | null>(null);
const deleting = ref(false);

const uploadOpen = ref(false);
const uploadTarget = ref<string | null>(null);
const uploadFile = ref<{ path: string; nameHint: string } | null>(null);
const uploading = ref(false);

const editorOpen = ref(false);
const editorReadonly = ref(false);
const editorTitle = ref("");
const editorPluginName = ref<string | null>(null);
const editorSkillName = ref("");
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

const editorDirty = computed(() => editorContent.value !== editorOriginal.value);

const localHeaders = computed<DataTableHeader[]>(() => [
  { title: t("skills.table.name"), key: "name", sortable: false },
  { title: t("skills.table.description"), key: "description", sortable: false },
  { title: t("skills.table.enabled"), key: "enabled", sortable: false, width: 90 },
  {
    title: t("instanceData.actionsColumn"),
    key: "actions",
    sortable: false,
    align: "end",
    width: 90,
  },
]);

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

async function loadSkills(): Promise<void> {
  if (!instanceId.value) {
    skills.value = [];
    pluginSkills.value = [];
    pluginSkillsLoaded.value = false;
    return;
  }
  loading.value = true;
  loadError.value = null;
  try {
    const [local, plugin] = await Promise.all([
      listInstanceSkills(instanceId.value),
      listInstancePluginSkills(instanceId.value),
    ]);
    skills.value = local;
    pluginSkills.value = plugin;
    pluginSkillsLoaded.value = true;
  } catch (requestError) {
    skills.value = [];
    pluginSkills.value = [];
    loadError.value = toUserMessage(requestError);
  } finally {
    loading.value = false;
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

async function openLocalEditor(skill: InstanceSkill): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  try {
    const content = await readInstanceSkill(instanceId.value, skill.name);
    editorReadonly.value = false;
    editorPluginName.value = null;
    editorTitle.value = skill.name;
    editorSkillName.value = skill.name;
    editorOriginal.value = content;
    editorContent.value = content;
    editorOpen.value = true;
  } catch (requestError) {
    showErrorToast(requestError);
  }
}

async function openPluginEditor(skill: InstancePluginSkill): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  try {
    const content = await readInstancePluginSkill(
      instanceId.value,
      skill.plugin_name,
      skill.name,
    );
    editorReadonly.value = true;
    editorPluginName.value = skill.plugin_name;
    editorTitle.value = `${skill.plugin_name} / ${skill.name}`;
    editorSkillName.value = skill.name;
    editorOriginal.value = content;
    editorContent.value = content;
    editorOpen.value = true;
  } catch (requestError) {
    showErrorToast(requestError);
  }
}

async function saveEditor(): Promise<void> {
  if (!instanceId.value || editorReadonly.value || !editorDirty.value) {
    editorOpen.value = false;
    return;
  }
  saving.value = true;
  try {
    await writeInstanceSkill(instanceId.value, editorSkillName.value, editorContent.value);
    showToast(t("skills.toasts.saved", { name: editorSkillName.value }));
    editorOpen.value = false;
    await loadSkills();
  } catch (requestError) {
    showErrorToast(requestError);
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

function openUpload(): void {
  uploadTarget.value = null;
  uploadFile.value = null;
  uploadOpen.value = true;
}

async function pickFile(): Promise<void> {
  const picked = await pickSkillZip();
  if (picked) {
    uploadFile.value = picked;
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
          </div>
          <EmptyState
            v-if="skills.length === 0"
            :text="t('skills.empty')"
            icon="mdi-school-outline"
          />
          <div v-else class="page-table-card">
            <v-data-table
              :headers="localHeaders"
              :items="skills"
              item-value="name"
              hide-default-footer
              :items-per-page="-1"
              disable-sort
              hover
              :no-data-text="t('skills.empty')"
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
                        @click="openLocalEditor(item)"
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
              :items="pluginSkills"
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
                <span class="page-anywhere">{{ item.plugin_name }}</span>
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
                        @click="openPluginEditor(item)"
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
            prepend-icon="mdi-file-archive-outline"
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
      max-width="720"
      :persistent="saving"
      scroll-strategy="block"
    >
      <v-card>
        <v-card-title class="skills-panel__editor-title">
          {{ editorTitle }}
        </v-card-title>
        <v-card-text>
          <v-textarea
            v-model="editorContent"
            :readonly="editorReadonly"
            :label="t('skills.editor.label')"
            variant="outlined"
            rows="18"
            auto-grow
            class="skills-panel__editor-textarea"
            spellcheck="false"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" :disabled="saving" @click="editorOpen = false">
            {{ t("common.actions.cancel") }}
          </v-btn>
          <v-btn
            v-if="!editorReadonly"
            color="primary"
            variant="flat"
            :loading="saving"
            :disabled="!editorDirty"
            @click="saveEditor()"
          >
            {{ t("common.actions.save") }}
          </v-btn>
        </v-card-actions>
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

.skills-panel__editor-title {
  font-size: 1rem;
  font-weight: 600;
}

.skills-panel__editor-textarea {
  font-family: "Consolas", "Cascadia Mono", monospace;
}
</style>
