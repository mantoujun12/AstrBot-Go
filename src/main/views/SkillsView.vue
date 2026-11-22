<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../components/common/ConfirmDialog.vue";
import EmptyState from "../components/common/EmptyState.vue";
import PageHeader from "../components/common/PageHeader.vue";
import { pickSkillZip } from "../services/app";
import { toUserMessage } from "../services/errors";
import {
  deleteInstanceSkill,
  installInstanceSkillZip,
  listInstanceSkills,
  readInstanceSkill,
  setInstanceSkillEnabled,
  writeInstanceSkill,
} from "../services/skills";
import { useLauncherStore } from "../stores/launcher";
import { showErrorToast, showToast } from "../stores/toast";
import type { InstanceStatus } from "../types/launcher";
import type { InstanceSkill } from "../types/skills";

const { t } = useI18n();
const { instances } = useLauncherStore();

const instanceId = ref<string | null>(null);
const loading = ref(false);
const loadError = ref<string | null>(null);
const skills = ref<InstanceSkill[]>([]);

const togglingName = ref<string | null>(null);
const uploading = ref(false);

const pendingDelete = ref<InstanceSkill | null>(null);
const deleting = ref(false);

const editorOpen = ref(false);
const editorName = ref("");
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

async function loadSkills(): Promise<void> {
  if (!instanceId.value) {
    skills.value = [];
    return;
  }
  loading.value = true;
  loadError.value = null;
  try {
    skills.value = await listInstanceSkills(instanceId.value);
  } catch (error) {
    skills.value = [];
    loadError.value = toUserMessage(error);
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
  } catch (error) {
    showErrorToast(error);
  } finally {
    togglingName.value = null;
  }
}

async function openEditor(skill: InstanceSkill): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  try {
    const content = await readInstanceSkill(instanceId.value, skill.name);
    editorName.value = skill.name;
    editorOriginal.value = content;
    editorContent.value = content;
    editorOpen.value = true;
  } catch (error) {
    showErrorToast(error);
  }
}

async function saveEditor(): Promise<void> {
  if (!instanceId.value || !editorDirty.value) {
    editorOpen.value = false;
    return;
  }
  saving.value = true;
  try {
    await writeInstanceSkill(instanceId.value, editorName.value, editorContent.value);
    showToast(t("skills.toasts.saved", { name: editorName.value }));
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
  } catch (error) {
    showErrorToast(error);
  } finally {
    deleting.value = false;
  }
}

async function uploadZip(): Promise<void> {
  if (!instanceId.value || !canManage.value || uploading.value) {
    return;
  }
  const picked = await pickSkillZip();
  if (!picked) {
    return;
  }
  uploading.value = true;
  try {
    const installed = await installInstanceSkillZip(instanceId.value, {
      zip_path: picked.path,
      name_hint: picked.nameHint,
    });
    showToast(
      t("skills.toasts.installed", { names: installed.join(", ") }),
    );
    await loadSkills();
  } catch (error) {
    showErrorToast(error);
  } finally {
    uploading.value = false;
  }
}

watch(instanceId, () => {
  void loadSkills();
});
</script>

<template>
  <div class="page-shell skills-page">
    <PageHeader :title="t('skills.title')">
      <template #actions>
        <v-btn
          color="primary"
          variant="flat"
          :disabled="!canManage"
          :loading="uploading"
          @click="uploadZip()"
        >
          {{ t("skills.upload") }}
        </v-btn>
      </template>
    </PageHeader>

    <div class="skills-page__controls">
      <v-select
        v-model="instanceId"
        :items="instances"
        item-title="name"
        item-value="id"
        :label="t('skills.instance')"
        variant="outlined"
        density="compact"
        hide-details
        class="skills-page__instance"
      />
    </div>

    <v-alert
      v-if="selectedInstance && !canManage"
      type="warning"
      variant="tonal"
      density="comfortable"
      class="skills-page__banner"
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
        class="skills-page__progress"
      />
      <v-alert
        v-else-if="loadError"
        type="error"
        variant="tonal"
        density="comfortable"
      >
        {{ loadError }}
      </v-alert>
      <EmptyState
        v-else-if="skills.length === 0"
        :text="t('skills.empty')"
        icon="mdi-school-outline"
      />
      <div v-else class="skills-page__grid">
        <article
          v-for="skill in skills"
          :key="skill.name"
          class="skill-card page-card"
        >
          <div class="skill-card__head">
            <v-avatar size="32" class="skill-card__fallback">
              <span>{{ skill.name.charAt(0).toUpperCase() }}</span>
            </v-avatar>
            <div class="skill-card__heading">
              <p class="skill-card__title">{{ skill.name }}</p>
              <p class="skill-card__desc">{{ skill.description || "—" }}</p>
            </div>
            <v-switch
              :model-value="skill.active"
              inset
              color="primary"
              hide-details
              density="compact"
              class="skill-card__switch"
              :disabled="!canManage || togglingName === skill.name"
              :aria-label="t('skills.toggleAria', { name: skill.name })"
              @update:model-value="(value: unknown) => toggleSkill(skill, value === true)"
            />
          </div>
          <div class="skill-card__actions">
            <v-spacer />
            <v-btn
              variant="text"
              size="small"
              color="primary"
              :disabled="!canManage"
              @click="openEditor(skill)"
            >
              {{ t("skills.edit") }}
            </v-btn>
            <v-btn
              variant="text"
              size="small"
              color="error"
              :disabled="!canManage"
              @click="confirmDelete(skill)"
            >
              {{ t("common.actions.delete") }}
            </v-btn>
          </div>
        </article>
      </div>
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

    <v-dialog
      v-model="editorOpen"
      max-width="720"
      :persistent="saving"
      scroll-strategy="block"
    >
      <v-card>
        <v-card-title class="skill-editor__title">
          {{ editorName }}
        </v-card-title>
        <v-card-text>
          <v-textarea
            v-model="editorContent"
            :label="t('skills.editor.label')"
            variant="outlined"
            rows="18"
            auto-grow
            class="skill-editor__textarea"
            spellcheck="false"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn
            variant="text"
            :disabled="saving"
            @click="editorOpen = false"
          >
            {{ t("common.actions.cancel") }}
          </v-btn>
          <v-btn
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
.skills-page__controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.skills-page__instance {
  max-width: 280px;
  flex: 1 1 200px;
}

.skills-page__banner {
  margin-bottom: 12px;
}

.skills-page__progress {
  margin-bottom: 12px;
}

.skills-page__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}

.skill-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
}

.skill-card__head {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.skill-card__fallback {
  flex: none;
}

.skill-card__heading {
  min-width: 0;
  flex: 1;
}

.skill-card__title {
  margin: 0;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.875rem;
  font-weight: 600;
  line-height: 1.3;
  overflow-wrap: anywhere;
}

.skill-card__desc {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.55);
  font-size: 0.75rem;
  line-height: 1.35;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.skill-card__switch {
  flex: none;
}

.skill-card__actions {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: auto;
}

.skill-editor__title {
  font-size: 1rem;
  font-weight: 600;
}

.skill-editor__textarea {
  font-family: "Consolas", "Cascadia Mono", monospace;
}
</style>
