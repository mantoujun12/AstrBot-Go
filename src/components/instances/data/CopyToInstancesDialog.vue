<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { copyInstanceData } from "../../../services/instanceData";
import { useLauncherStore } from "../../../stores/launcher";
import { showErrorToast, showToast } from "../../../stores/toast";
import type { InstanceStatus } from "../../../types/launcher";
import type {
  CopyPreselect,
  InstanceConfigFile,
  InstanceCronJob,
  InstancePersona,
} from "../../../types/instanceData";

const props = defineProps<{
  modelValue: boolean;
  instance: InstanceStatus | null;
  personas: InstancePersona[];
  cronJobs: InstanceCronJob[];
  configFiles: InstanceConfigFile[];
  preselect: CopyPreselect;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const { t } = useI18n();
const { instances } = useLauncherStore();

const targets = computed(() =>
  instances.value.filter(
    (item) =>
      item.state === "stopped" && item.id !== props.instance?.id,
  ),
);

const targetIds = ref<string[]>([]);
const personaIds = ref<string[]>([]);
const cronJobIds = ref<string[]>([]);
const configFileNames = ref<string[]>([]);
const overwrite = ref(false);
const saving = ref(false);

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      targetIds.value = [];
      personaIds.value = [...props.preselect.personas];
      cronJobIds.value = [...props.preselect.cronJobs];
      configFileNames.value = [...props.preselect.configFiles];
      overwrite.value = false;
    }
  },
);

const hasItems = computed(
  () =>
    personaIds.value.length > 0 ||
    cronJobIds.value.length > 0 ||
    configFileNames.value.length > 0,
);

async function submit(): Promise<void> {
  const source = props.instance;
  if (
    source === null ||
    saving.value ||
    targetIds.value.length === 0 ||
    !hasItems.value
  ) {
    return;
  }
  saving.value = true;
  try {
    await copyInstanceData({
      sourceInstanceId: source.id,
      targetInstanceIds: targetIds.value,
      personaIds: personaIds.value,
      cronJobIds: cronJobIds.value,
      configFiles: configFileNames.value,
      overwrite: overwrite.value,
    });
    showToast(t("instanceData.copy.copied"));
    emit("update:modelValue", false);
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="520"
    :persistent="saving"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card class="copy-dialog">
      <v-card-title class="copy-dialog__title">
        {{ t("instanceData.copy.title") }}
      </v-card-title>
      <v-card-text class="copy-dialog__body">
        <v-alert
          v-if="targets.length === 0"
          type="info"
          variant="tonal"
          density="comfortable"
        >
          {{ t("instanceData.copy.noTargets") }}
        </v-alert>
        <template v-else>
          <v-select
            v-model="targetIds"
            :items="targets"
            item-title="name"
            item-value="id"
            :label="t('instanceData.copy.targets')"
            variant="outlined"
            density="compact"
            multiple
            hide-details
            :disabled="saving"
          />
          <v-select
            v-if="personas.length > 0"
            v-model="personaIds"
            :items="personas"
            item-title="persona_id"
            item-value="persona_id"
            :label="t('instanceData.sections.personas')"
            variant="outlined"
            density="compact"
            multiple
            hide-details
            :disabled="saving"
          />
          <v-select
            v-if="cronJobs.length > 0"
            v-model="cronJobIds"
            :items="cronJobs"
            item-title="name"
            item-value="job_id"
            :label="t('instanceData.sections.cronJobs')"
            variant="outlined"
            density="compact"
            multiple
            hide-details
            :disabled="saving"
          />
          <v-select
            v-if="configFiles.length > 0"
            v-model="configFileNames"
            :items="configFiles"
            item-title="name"
            item-value="name"
            :label="t('instanceData.sections.configFiles')"
            variant="outlined"
            density="compact"
            multiple
            hide-details
            :disabled="saving"
          />
          <v-switch
            v-model="overwrite"
            inset
            color="primary"
            density="compact"
            hide-details
            :label="t('instanceData.copy.overwrite')"
            :disabled="saving"
          />
        </template>
      </v-card-text>
      <v-card-actions class="copy-dialog__actions">
        <v-spacer />
        <v-btn variant="text" :disabled="saving" @click="emit('update:modelValue', false)">
          {{ t("common.actions.cancel") }}
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :loading="saving"
          :disabled="targets.length === 0 || !hasItems"
          @click="submit"
        >
          {{ t("common.actions.copy") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.copy-dialog__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.copy-dialog__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 20px 0;
}

.copy-dialog__actions {
  padding: 16px;
}
</style>
