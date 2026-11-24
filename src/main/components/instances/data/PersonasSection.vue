<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { DataTableHeader } from "vuetify";
import type { InstancePersona, InstancePersonaFolder } from "../../../types/instanceData";
import { parseLinesField, parseListField } from "../../../utils/instanceData";

const props = defineProps<{
  personas: InstancePersona[];
  folders: InstancePersonaFolder[];
  readonly: boolean;
}>();

const emit = defineEmits<{
  (event: "upsert", persona: InstancePersona): void;
  (event: "remove", personaIds: string[]): void;
  (event: "copy", personaId: string): void;
}>();

const { t } = useI18n();

const search = ref("");
const selected = ref<string[]>([]);

const headers = computed<DataTableHeader[]>(() => [
  {
    title: t("instanceData.personas.columns.personaId"),
    key: "persona_id",
    sortable: false,
  },
  {
    title: t("instanceData.personas.columns.folder"),
    key: "folder",
    sortable: false,
  },
  {
    title: t("instanceData.personas.columns.systemPrompt"),
    key: "system_prompt",
    sortable: false,
  },
  {
    title: t("instanceData.personas.columns.tools"),
    key: "tools",
    sortable: false,
  },
  {
    title: t("instanceData.personas.columns.skills"),
    key: "skills",
    sortable: false,
  },
  {
    title: t("instanceData.actionsColumn"),
    key: "actions",
    sortable: false,
    align: "end",
    width: 140,
  },
]);

function folderNameOf(folderId: string | null): string {
  if (folderId === null) {
    return t("instanceData.personas.fields.rootFolder");
  }
  return (
    props.folders.find((folder) => folder.folder_id === folderId)?.name ??
    t("instanceData.personas.fields.rootFolder")
  );
}

const filteredPersonas = computed(() => {
  const keyword = search.value.trim().toLowerCase();
  if (keyword === "") {
    return props.personas;
  }
  return props.personas.filter(
    (persona) =>
      persona.persona_id.toLowerCase().includes(keyword) ||
      persona.system_prompt.toLowerCase().includes(keyword) ||
      folderNameOf(persona.folder_id).toLowerCase().includes(keyword),
  );
});

const folderOptions = computed(() => [
  { title: t("instanceData.personas.fields.rootFolder"), value: "" },
  ...props.folders.map((folder) => ({
    title: folder.name,
    value: folder.folder_id,
  })),
]);

function listHint(value: string[] | null): string {
  if (value === null) {
    return t("instanceData.all");
  }
  if (value.length === 0) {
    return t("instanceData.none");
  }
  return value.join(", ");
}

const formOpen = ref(false);
const editing = ref<InstancePersona | null>(null);
const personaId = ref("");
const systemPrompt = ref("");
const beginDialogs = ref("");
const toolsText = ref("");
const skillsText = ref("");
const customErrorMessage = ref("");
const folderId = ref("");
const sortOrderText = ref("0");

function openAdd(): void {
  editing.value = null;
  personaId.value = "";
  systemPrompt.value = "";
  beginDialogs.value = "";
  toolsText.value = "";
  skillsText.value = "";
  customErrorMessage.value = "";
  folderId.value = "";
  sortOrderText.value = "0";
  formOpen.value = true;
}

function openEdit(persona: InstancePersona): void {
  editing.value = persona;
  personaId.value = persona.persona_id;
  systemPrompt.value = persona.system_prompt;
  beginDialogs.value = persona.begin_dialogs.join("\n");
  toolsText.value = persona.tools === null ? "" : persona.tools.join(", ");
  skillsText.value = persona.skills === null ? "" : persona.skills.join(", ");
  customErrorMessage.value = persona.custom_error_message ?? "";
  folderId.value = persona.folder_id ?? "";
  sortOrderText.value = String(persona.sort_order);
  formOpen.value = true;
}

const personaIdError = computed(() => {
  const id = personaId.value.trim();
  if (id === "") {
    return t("instanceData.personas.idRequired");
  }
  if (
    editing.value?.persona_id !== id &&
    props.personas.some((persona) => persona.persona_id === id)
  ) {
    return t("instanceData.personas.idExists");
  }
  return "";
});

const formValid = computed(() => personaIdError.value === "");

function submitForm(): void {
  if (!formValid.value) {
    return;
  }
  const sortOrder = Number(sortOrderText.value);
  emit("upsert", {
    persona_id: personaId.value.trim(),
    system_prompt: systemPrompt.value,
    begin_dialogs: parseLinesField(beginDialogs.value),
    tools: parseListField(toolsText.value),
    skills: parseListField(skillsText.value),
    custom_error_message: customErrorMessage.value.trim() || null,
    folder_id: folderId.value || null,
    sort_order: Number.isFinite(sortOrder) ? sortOrder : 0,
  });
  formOpen.value = false;
}

function removeSelected(): void {
  emit("remove", selected.value);
  selected.value = [];
}

function removeOne(personaIdToRemove: string): void {
  emit("remove", [personaIdToRemove]);
  selected.value = selected.value.filter((id) => id !== personaIdToRemove);
}
</script>

<template>
  <div class="personas">
    <div class="personas__toolbar page-row--between">
      <v-text-field
        v-model="search"
        variant="outlined"
        density="compact"
        hide-details
        clearable
        prepend-inner-icon="mdi-magnify"
        :placeholder="t('instanceData.searchPlaceholder')"
        class="personas__search"
      />
      <div class="page-row personas__actions">
        <v-btn
          size="small"
          variant="text"
          color="error"
          prepend-icon="mdi-delete-outline"
          :disabled="selected.length === 0 || readonly"
          @click="removeSelected"
        >
          {{ t("common.actions.delete") }}
        </v-btn>
        <v-btn
          size="small"
          color="primary"
          variant="tonal"
          prepend-icon="mdi-plus"
          :disabled="readonly"
          @click="openAdd"
        >
          {{ t("instanceData.personas.addTitle") }}
        </v-btn>
      </div>
    </div>

    <div class="page-table-card">
      <v-data-table
        v-model="selected"
        :headers="headers"
        :items="filteredPersonas"
        item-value="persona_id"
        show-select
        hide-default-footer
        :items-per-page="-1"
        disable-sort
        hover
        :no-data-text="t('instanceData.personas.empty')"
      >
        <template #item.system_prompt="{ item }">
          <span class="personas__prompt page-anywhere">
            {{ item.system_prompt || "—" }}
          </span>
        </template>

        <template #item.folder="{ item }">
          <span class="page-anywhere">{{ folderNameOf(item.folder_id) }}</span>
        </template>

        <template #item.tools="{ item }">
          <span class="page-anywhere">{{ listHint(item.tools) }}</span>
        </template>

        <template #item.skills="{ item }">
          <span class="page-anywhere">{{ listHint(item.skills) }}</span>
        </template>

        <template #item.actions="{ item }">
          <div class="page-row personas__row-actions">
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-pencil-outline"
                  variant="text"
                  size="small"
                  :aria-label="t('common.actions.edit')"
                  :disabled="readonly"
                  @click="openEdit(item)"
                />
              </template>
              {{ t("common.actions.edit") }}
            </v-tooltip>
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-content-duplicate"
                  variant="text"
                  size="small"
                  :aria-label="t('instanceData.copy.title')"
                  @click="emit('copy', item.persona_id)"
                />
              </template>
              {{ t("instanceData.copy.title") }}
            </v-tooltip>
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-delete-outline"
                  variant="text"
                  size="small"
                  :aria-label="t('common.actions.delete')"
                  :disabled="readonly"
                  @click="removeOne(item.persona_id)"
                />
              </template>
              {{ t("common.actions.delete") }}
            </v-tooltip>
          </div>
        </template>
      </v-data-table>
    </div>

    <v-dialog v-model="formOpen" max-width="560">
      <v-card class="persona-form">
        <v-card-title class="persona-form__title">
          {{
            editing
              ? t("instanceData.personas.editTitle")
              : t("instanceData.personas.addTitle")
          }}
        </v-card-title>
        <v-card-text class="persona-form__body">
          <v-text-field
            v-model="personaId"
            variant="outlined"
            density="compact"
            :label="t('instanceData.personas.fields.personaId')"
            :disabled="editing !== null"
            :error-messages="personaIdError === '' ? [] : [personaIdError]"
          />
          <v-textarea
            v-model="systemPrompt"
            variant="outlined"
            density="compact"
            rows="4"
            :label="t('instanceData.personas.fields.systemPrompt')"
          />
          <v-textarea
            v-model="beginDialogs"
            variant="outlined"
            density="compact"
            rows="3"
            :label="t('instanceData.personas.fields.beginDialogs')"
            :placeholder="t('instanceData.personas.fields.beginDialogsPlaceholder')"
          />
          <v-textarea
            v-model="toolsText"
            variant="outlined"
            density="compact"
            rows="2"
            :label="t('instanceData.personas.fields.tools')"
            :placeholder="t('instanceData.personas.fields.listPlaceholder')"
          />
          <v-textarea
            v-model="skillsText"
            variant="outlined"
            density="compact"
            rows="2"
            :label="t('instanceData.personas.fields.skills')"
            :placeholder="t('instanceData.personas.fields.listPlaceholder')"
          />
          <v-text-field
            v-model="customErrorMessage"
            variant="outlined"
            density="compact"
            hide-details
            :label="t('instanceData.personas.fields.customErrorMessage')"
          />
          <v-select
            v-model="folderId"
            :items="folderOptions"
            variant="outlined"
            density="compact"
            hide-details
            clearable
            :label="t('instanceData.personas.fields.folder')"
          />
          <v-text-field
            v-model="sortOrderText"
            type="number"
            variant="outlined"
            density="compact"
            hide-details
            :label="t('instanceData.personas.fields.sortOrder')"
          />
        </v-card-text>
        <v-card-actions class="persona-form__actions">
          <v-spacer />
          <v-btn variant="text" @click="formOpen = false">
            {{ t("common.actions.cancel") }}
          </v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :disabled="!formValid"
            @click="submitForm"
          >
            {{ t("common.actions.save") }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.personas__toolbar {
  gap: 12px;
  margin-bottom: 12px;
}

.personas__search {
  max-width: 280px;
}

.personas__actions {
  gap: 8px;
}

.personas__row-actions {
  justify-content: flex-end;
  gap: 2px;
}

.personas__prompt {
  display: inline-block;
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  vertical-align: bottom;
}

.persona-form__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.persona-form__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 20px 0;
}

.persona-form__actions {
  padding: 16px;
}
</style>
