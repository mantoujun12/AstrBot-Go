<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { LegacyDataDirPrompt } from "../../types/launcher";
import {
  dismissLegacyDataDirMigration,
  migrateLegacyDataDir,
} from "../../services/app";
import { toUserMessage } from "../../services/errors";
import { refresh } from "../../stores/launcher";
import { showToast } from "../../stores/toast";

const props = defineProps<{
  modelValue: boolean;
  prompt: LegacyDataDirPrompt | null;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const { t } = useI18n();
const migrating = ref(false);
const errorMessage = ref<string | null>(null);

const open = computed({
  get: () => props.modelValue && props.prompt !== null,
  set: (value: boolean) => {
    if (!migrating.value) {
      emit("update:modelValue", value);
    }
  },
});

async function migrate(): Promise<void> {
  if (!props.prompt || migrating.value) {
    return;
  }
  migrating.value = true;
  errorMessage.value = null;
  try {
    const result = await migrateLegacyDataDir();
    await refresh();
    showToast(
      t("legacyMigration.success", { count: result.copied_entries }),
      "success",
    );
    emit("update:modelValue", false);
  } catch (cause) {
    errorMessage.value = toUserMessage(cause);
  } finally {
    migrating.value = false;
  }
}

async function keep(): Promise<void> {
  if (migrating.value) {
    return;
  }
  try {
    await dismissLegacyDataDirMigration();
  } catch {
    // The prompt reappearing next launch is harmless; do not block closing.
  }
  emit("update:modelValue", false);
}
</script>

<template>
  <v-dialog
    v-model="open"
    persistent
    max-width="480"
  >
    <v-card>
      <v-card-title class="legacy-migration__title">
        {{ t("legacyMigration.title") }}
      </v-card-title>
      <v-card-text class="legacy-migration__body">
        <p class="legacy-migration__description">
          {{ t("legacyMigration.description") }}
        </p>
        <div class="legacy-migration__row">
          <span class="legacy-migration__label">
            {{ t("legacyMigration.legacyDir") }}
          </span>
          <code class="legacy-migration__value">
            {{ prompt?.legacy_dir ?? "—" }}
          </code>
        </div>
        <div class="legacy-migration__row">
          <span class="legacy-migration__label">
            {{ t("legacyMigration.dataDir") }}
          </span>
          <code class="legacy-migration__value">
            {{ prompt?.data_dir ?? "—" }}
          </code>
        </div>
        <v-alert
          v-if="errorMessage"
          type="error"
          variant="tonal"
          density="compact"
          class="legacy-migration__error"
        >
          {{ errorMessage }}
        </v-alert>
      </v-card-text>
      <v-card-actions class="legacy-migration__actions">
        <v-spacer />
        <v-btn
          variant="text"
          :disabled="migrating"
          @click="keep"
        >
          {{ t("legacyMigration.keep") }}
        </v-btn>
        <v-btn
          color="primary"
          :loading="migrating"
          @click="migrate"
        >
          {{ t("legacyMigration.migrate") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.legacy-migration__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.legacy-migration__body {
  padding: 12px 20px 4px;
}

.legacy-migration__description {
  margin-bottom: 12px;
}

.legacy-migration__row {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-bottom: 10px;
}

.legacy-migration__label {
  font-size: 0.8rem;
  opacity: 0.7;
}

.legacy-migration__value {
  font-size: 0.85rem;
  word-break: break-all;
}

.legacy-migration__error {
  margin-top: 8px;
}

.legacy-migration__actions {
  padding: 8px 20px 16px;
}
</style>
