<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../../common/ConfirmDialog.vue";

const props = defineProps<{
  modelValue: boolean;
  name: string;
  content: string;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "save", content: string): void;
}>();

const { t } = useI18n();

const text = ref("");
const parseError = ref("");
const discardOpen = ref(false);

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      text.value = props.content;
      parseError.value = "";
    }
  },
);

const dirty = computed(() => text.value !== props.content);

function requestClose(): void {
  if (dirty.value) {
    discardOpen.value = true;
    return;
  }
  emit("update:modelValue", false);
}

function handleDialogUpdate(value: boolean): void {
  if (!value) {
    requestClose();
  }
}

// ConfirmDialog does not close itself on confirm; the parent owns the flag.
function discardAndClose(): void {
  discardOpen.value = false;
  emit("update:modelValue", false);
}

function save(): void {
  if (!dirty.value || props.readonly) {
    return;
  }
  if (props.name.endsWith(".json")) {
    try {
      JSON.parse(text.value);
    } catch {
      parseError.value = t("instanceData.configFiles.jsonInvalid");
      return;
    }
  }
  parseError.value = "";
  emit("save", text.value);
  emit("update:modelValue", false);
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    fullscreen
    transition="dialog-bottom-transition"
    @update:model-value="handleDialogUpdate"
  >
    <v-card class="config-editor">
      <v-toolbar color="surface" flat>
        <v-btn
          icon="mdi-close"
          :aria-label="t('common.actions.close')"
          @click="requestClose"
        />
        <v-toolbar-title class="config-editor__name page-mono page-anywhere">
          {{ name }}
        </v-toolbar-title>
        <template #append>
          <v-btn
            :text="t('common.actions.save')"
            variant="text"
            :disabled="!dirty || readonly"
            @click="save"
          />
        </template>
      </v-toolbar>

      <div class="config-editor__body">
        <v-alert
          v-if="parseError"
          type="error"
          variant="tonal"
          density="comfortable"
          class="config-editor__error"
        >
          {{ parseError }}
        </v-alert>
        <v-textarea
          v-model="text"
          class="config-editor__textarea"
          variant="outlined"
          no-resize
          :readonly="readonly"
        />
      </div>
    </v-card>
  </v-dialog>

  <ConfirmDialog
    v-model="discardOpen"
    :title="t('instanceData.unsavedTitle')"
    :message="t('instanceData.unsavedMessage')"
    :confirm-label="t('common.actions.close')"
    @confirm="discardAndClose"
  />
</template>

<style scoped>
.config-editor {
  display: flex;
  flex-direction: column;
}

.config-editor__name {
  font-size: 0.95rem;
  font-weight: 600;
}

.config-editor__body {
  position: relative;
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 12px 16px 16px;
}

.config-editor__error {
  margin-bottom: 12px;
}

.config-editor__textarea {
  flex: 1 1 auto;
}

.config-editor__textarea :deep(.v-field) {
  height: 100%;
}

.config-editor__textarea :deep(.v-field__input) {
  height: 100%;
  font-family: "Consolas", "Cascadia Mono", monospace;
  font-size: 0.85rem;
  line-height: 1.5;
  overflow-wrap: normal;
  white-space: pre;
  overflow: auto;
}
</style>
