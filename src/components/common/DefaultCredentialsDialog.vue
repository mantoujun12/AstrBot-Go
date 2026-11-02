<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import type { DefaultCredentials } from "../../types/launcher";
import { showToast } from "../../stores/toast";

const props = defineProps<{
  modelValue: boolean;
  credentials: DefaultCredentials | null;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const { t } = useI18n();
const revealPassword = ref(false);

function close(): void {
  emit("update:modelValue", false);
}

async function copy(value: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(value);
    showToast(t("instances.credentials.copied"), "success");
  } catch {
    showToast(t("instances.credentials.copyFailed"), "error");
  }
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="420"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title class="credentials-dialog__title">
        {{ t("instances.credentials.title") }}
      </v-card-title>
      <v-card-text class="credentials-dialog__body">
        <div class="credentials-dialog__row">
          <span class="credentials-dialog__label">
            {{ t("instances.credentials.instance") }}
          </span>
          <span class="credentials-dialog__value">
            {{ credentials?.display_name ?? "—" }}
          </span>
        </div>
        <div class="credentials-dialog__row">
          <span class="credentials-dialog__label">
            {{ t("instances.credentials.username") }}
          </span>
          <code class="credentials-dialog__value">
            {{ credentials?.username ?? "—" }}
          </code>
          <v-btn
            icon="mdi-content-copy"
            size="x-small"
            variant="text"
            :aria-label="t('instances.credentials.aria.copyUsername')"
            :disabled="!credentials"
            @click="credentials && copy(credentials.username)"
          />
        </div>
        <div class="credentials-dialog__row">
          <span class="credentials-dialog__label">
            {{ t("instances.credentials.password") }}
          </span>
          <code class="credentials-dialog__value">
            {{ revealPassword ? (credentials?.password ?? "—") : "••••••" }}
          </code>
          <v-btn
            :icon="revealPassword ? 'mdi-eye-off' : 'mdi-eye'"
            size="x-small"
            variant="text"
            :aria-label="
              revealPassword
                ? t('instances.credentials.aria.hidePassword')
                : t('instances.credentials.aria.showPassword')
            "
            @click="revealPassword = !revealPassword"
          />
          <v-btn
            icon="mdi-content-copy"
            size="x-small"
            variant="text"
            :aria-label="t('instances.credentials.aria.copyPassword')"
            :disabled="!credentials"
            @click="credentials && copy(credentials.password)"
          />
        </div>
      </v-card-text>
      <v-card-actions class="credentials-dialog__actions">
        <v-spacer />
        <v-btn variant="text" @click="close">
          {{ t("common.actions.close") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.credentials-dialog__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 650;
}

.credentials-dialog__body {
  padding: 12px 20px 4px;
}

.credentials-dialog__row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 38px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.credentials-dialog__row:last-child {
  border-bottom: 0;
}

.credentials-dialog__label {
  flex: 0 0 56px;
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-size: 0.78rem;
}

.credentials-dialog__value {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  color: rgb(var(--v-theme-on-surface));
  font-family: "Consolas", "Cascadia Mono", monospace;
  font-size: 0.84rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.credentials-dialog__actions {
  padding: 12px 16px 16px;
}
</style>
