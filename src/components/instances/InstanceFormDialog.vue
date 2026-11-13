<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { VForm } from "vuetify/components";
import type {
  InstanceStatus,
  InstalledVersion,
  UpdateInstanceRequest,
} from "../../types/launcher";

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    instance: InstanceStatus | null;
    versions: InstalledVersion[];
    usedPorts: number[];
    loading?: boolean;
  }>(),
  { loading: false },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "create", payload: { name: string; version: string; port: number }): void;
  (event: "update", instanceId: string, request: UpdateInstanceRequest): void;
  (event: "go-versions"): void;
}>();

const { t } = useI18n();
const formRef = ref<VForm | null>(null);
const name = ref("");
const version = ref("");
const host = ref("");
const portText = ref("");
const checkUpdate = ref(true);
const autoRestart = ref(false);
const maxRetriesText = ref("3");
const autoRestartNotify = ref(true);

const isEditing = computed(() => props.instance !== null);

const otherPorts = computed(() => {
  const selfPort = props.instance?.configured_port;
  return props.usedPorts.filter((port) => port !== selfPort);
});

const versionOptions = computed(() => {
  const values = props.versions.map((item) => item.version);
  const current = props.instance?.version;
  if (current !== undefined && current.trim() !== "" && !values.includes(current)) {
    values.push(current);
  }
  return Array.from(new Set(values)).map((value) => ({
    title: value,
    value,
  }));
});

const selectedVersion = computed(() =>
  typeof version.value === "string" ? version.value.trim() : "",
);

const canSubmit = computed(
  () =>
    !props.loading &&
    versionOptions.value.length > 0 &&
    selectedVersion.value !== "",
);

function firstFreePort(usedPorts: number[]): number {
  const used = new Set(
    usedPorts.filter((port) => Number.isInteger(port) && port > 0),
  );
  let port = 6185;
  while (used.has(port) && port < 65535) {
    port += 1;
  }
  return port;
}

function initialize(): void {
  const target = props.instance;
  name.value = target?.name ?? "";
  version.value = target?.version ?? props.versions[0]?.version ?? "";
  host.value = target?.configured_host ?? "";
  portText.value = String(
    target ? target.configured_port : firstFreePort(props.usedPorts),
  );
  checkUpdate.value = target?.check_update_enabled ?? true;
  autoRestart.value = target?.auto_restart_enabled ?? false;
  maxRetriesText.value = String(target?.auto_restart_max_retries ?? 3);
  autoRestartNotify.value = target?.auto_restart_notify ?? true;
}

function validateName(value: unknown): true | string {
  return typeof value === "string" && value.trim() !== ""
    ? true
    : t("instances.form.validation.nameRequired");
}

function validateVersion(value: unknown): true | string {
  return typeof value === "string" && value.trim() !== ""
    ? true
    : t("instances.form.validation.versionRequired");
}

function validatePort(value: unknown): true | string {
  const text = typeof value === "string" ? value.trim() : "";
  const parsed = Number(text);
  if (
    text === "" ||
    !Number.isFinite(parsed) ||
    !Number.isInteger(parsed) ||
    parsed < 1 ||
    parsed > 65535
  ) {
    return t("instances.form.validation.portRange", { min: 1, max: 65535 });
  }
  if (otherPorts.value.includes(parsed)) {
    return t("instances.form.validation.portInUse");
  }
  return true;
}

const nameRules = [validateName];
const versionRules = [validateVersion];
const portRules = [validatePort];

async function submit(): Promise<void> {
  if (props.loading) {
    return;
  }

  const result = await formRef.value?.validate();
  if (!result || !result.valid || selectedVersion.value === "") {
    return;
  }

  const port = Number(
    typeof portText.value === "string" ? portText.value.trim() : "",
  );
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    return;
  }

  const nameValue = typeof name.value === "string" ? name.value.trim() : "";
  const hostValue = typeof host.value === "string" ? host.value.trim() : "";
  const target = props.instance;
  if (target) {
    const maxRetries = Number(maxRetriesText.value.trim());
    const request: UpdateInstanceRequest = {
      name: nameValue,
      version: selectedVersion.value,
      host: hostValue,
      port,
      checkUpdateEnabled: checkUpdate.value,
      autoRestartEnabled: autoRestart.value,
      autoRestartMaxRetries: Number.isInteger(maxRetries)
        ? Math.min(20, Math.max(1, maxRetries))
        : 3,
      autoRestartNotify: autoRestartNotify.value,
    };
    emit("update", target.id, request);
    return;
  }

  emit("create", {
    name: nameValue,
    version: selectedVersion.value,
    port,
  });
}

function close(): void {
  if (props.loading) {
    return;
  }
  emit("update:modelValue", false);
}

function goVersions(): void {
  emit("go-versions");
}

watch(
  () => props.modelValue,
  (open) => {
    if (!open) {
      return;
    }
    initialize();
    void nextTick(() => {
      formRef.value?.resetValidation();
    });
  },
  { immediate: true },
);
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="480"
    scrollable
    :persistent="loading"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title class="instance-form__title">
        {{
          isEditing
            ? t("instances.form.titles.edit")
            : t("instances.form.titles.create")
        }}
      </v-card-title>
      <v-card-text class="instance-form__body">
        <v-form ref="formRef" @submit.prevent="submit">
          <v-text-field
            v-model="name"
            :label="t('instances.form.fields.name')"
            variant="outlined"
            density="comfortable"
            autofocus
            :rules="nameRules"
            :disabled="loading"
          />

          <v-select
            v-if="versionOptions.length > 0"
            v-model="version"
            :label="t('instances.form.fields.version')"
            variant="outlined"
            density="comfortable"
            :items="versionOptions"
            :rules="versionRules"
            :disabled="loading"
          />
          <div v-else class="instance-form__empty">
            <span>{{ t("instances.form.emptyVersions") }}</span>
            <v-btn
              size="small"
              variant="text"
              color="primary"
              :disabled="loading"
              @click="goVersions"
            >
              {{ t("instances.form.goVersions") }}
            </v-btn>
          </div>

          <v-text-field
            v-if="isEditing"
            v-model="host"
            :label="t('instances.form.fields.host')"
            placeholder="127.0.0.1"
            variant="outlined"
            density="comfortable"
            :disabled="loading"
          />

          <v-text-field
            v-model="portText"
            :label="t('instances.form.fields.port')"
            type="number"
            min="1"
            max="65535"
            variant="outlined"
            density="comfortable"
            :rules="portRules"
            :disabled="loading"
          />

          <v-switch
            v-if="isEditing"
            v-model="checkUpdate"
            :label="t('instances.form.fields.checkUpdate')"
            color="primary"
            inset
            hide-details
            :disabled="loading"
          />

          <v-switch
            v-if="isEditing"
            v-model="autoRestart"
            :label="t('instances.form.fields.autoRestart')"
            color="primary"
            inset
            hide-details
            :disabled="loading"
          />

          <v-text-field
            v-if="isEditing && autoRestart"
            v-model="maxRetriesText"
            :label="t('instances.form.fields.autoRestartMaxRetries')"
            type="number"
            min="1"
            max="20"
            variant="outlined"
            density="comfortable"
            :disabled="loading"
          />

          <v-switch
            v-if="isEditing && autoRestart"
            v-model="autoRestartNotify"
            :label="t('instances.form.fields.autoRestartNotify')"
            color="primary"
            inset
            hide-details
            :disabled="loading"
          />
        </v-form>
      </v-card-text>
      <v-card-actions class="instance-form__actions">
        <v-spacer />
        <v-btn variant="text" :disabled="loading" @click="close">
          {{ t("common.actions.cancel") }}
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :loading="loading"
          :disabled="!canSubmit"
          @click="submit"
        >
          {{
            isEditing ? t("common.actions.save") : t("common.actions.create")
          }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.instance-form__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.instance-form__body {
  padding: 16px 20px 4px;
}

.instance-form__body :deep(.v-input) {
  margin-bottom: 4px;
}

.instance-form__empty {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgba(var(--v-theme-on-surface), 0.64);
  font-size: 0.85rem;
}

.instance-form__actions {
  padding: 14px 16px 16px;
}
</style>
