<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { NetworkSourceDefinition } from "../../constants/networkSources";
import { toUserMessage } from "../../services/errors";
import {
  testNetworkSource,
  type NetworkProbeResult,
  type NetworkProbeStage,
} from "../../services/network";
import { formatLatency, formatSpeed } from "../../utils/network";

const props = defineProps<{
  definition: NetworkSourceDefinition;
  modelValue: string;
  dirty: boolean;
  saving: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  save: [];
}>();

const { t } = useI18n();

const DEFAULT_KEY = "__default__";
const CUSTOM_KEY = "__custom__";

interface ProbeState {
  pending: boolean;
  latency?: number;
  speed?: number | null;
  error?: string;
}

interface TestTarget {
  key: string;
  value: string;
  label: string;
}

const selection = ref(DEFAULT_KEY);
const customValue = ref("");
const probeStates = reactive<Record<string, ProbeState>>({});
const isTesting = ref(false);
const testDone = ref(0);
const testTotal = ref(0);
const testError = ref("");

const presetValues = computed(
  () => new Set(props.definition.presets.map((preset) => preset.value)),
);

const presetOptions = computed(() =>
  props.definition.presets.map((preset) => ({
    key: preset.value,
    value: preset.value,
    label: preset.labelKey ? t(preset.labelKey) : (preset.label ?? preset.value),
    url: preset.value,
  })),
);

const options = computed(() => [
  {
    key: DEFAULT_KEY,
    value: "",
    label: t(props.definition.defaultLabelKey),
    url: props.definition.defaultUrl,
  },
  ...presetOptions.value,
  {
    key: CUSTOM_KEY,
    value: customValue.value,
    label: t("settings.network.sources.custom"),
    url: customValue.value || t("settings.network.sources.customPlaceholder"),
  },
]);

function selectionForValue(value: string): string {
  if (value === "") {
    return DEFAULT_KEY;
  }
  return presetValues.value.has(value) ? value : CUSTOM_KEY;
}

watch(
  () => props.modelValue,
  (value) => {
    const next = selectionForValue(value);
    selection.value = next;
    if (next === CUSTOM_KEY) {
      customValue.value = value;
    }
  },
  { immediate: true },
);

function onSelectionUpdate(value: unknown): void {
  if (typeof value !== "string") {
    return;
  }
  selectSource(value);
}

function selectSource(key: string): void {
  selection.value = key;
  if (key === CUSTOM_KEY) {
    const value = customValue.value.trim();
    if (value !== "") {
      emit("update:modelValue", value);
    }
    return;
  }
  const option = options.value.find((item) => item.key === key);
  emit("update:modelValue", option?.value ?? "");
}

function onCustomInput(value: string): void {
  customValue.value = value;
  delete probeStates[CUSTOM_KEY];
  if (selection.value !== CUSTOM_KEY) {
    return;
  }
  const trimmed = value.trim();
  if (trimmed !== "") {
    emit("update:modelValue", trimmed);
  }
}

function isValidHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

function defaultTarget(): TestTarget {
  return {
    key: DEFAULT_KEY,
    value: "",
    label: t(props.definition.defaultLabelKey),
  };
}

function presetTargets(): TestTarget[] {
  return presetOptions.value.map((option) => ({
    key: option.key,
    value: option.value,
    label: option.label,
  }));
}

function customTarget(): TestTarget | null {
  const value = customValue.value.trim();
  if (!isValidHttpUrl(value)) {
    return null;
  }
  return {
    key: CUSTOM_KEY,
    value,
    label: t("settings.network.sources.custom"),
  };
}

function allTargets(): TestTarget[] {
  const targets = [defaultTarget(), ...presetTargets()];
  const custom = customTarget();
  return custom ? [...targets, custom] : targets;
}

function selectedTarget(): TestTarget | null {
  if (selection.value === DEFAULT_KEY) {
    return defaultTarget();
  }
  if (selection.value === CUSTOM_KEY) {
    return customTarget();
  }
  return (
    presetTargets().find((target) => target.key === selection.value) ?? null
  );
}

function rotateFromSelected(targets: TestTarget[]): TestTarget[] {
  const selected = selectedTarget();
  const index = selected
    ? targets.findIndex((target) => target.key === selected.key)
    : -1;
  if (index <= 0) {
    return targets;
  }
  return [...targets.slice(index), ...targets.slice(0, index)];
}

function probeFor(key: string): ProbeState | undefined {
  return probeStates[key];
}

function setPending(target: TestTarget, pending: boolean): void {
  probeStates[target.key] = { ...probeStates[target.key], pending };
}

function applyResult(
  target: TestTarget,
  stage: NetworkProbeStage,
  result: NetworkProbeResult,
): void {
  const state = probeStates[target.key] ?? { pending: false };
  probeStates[target.key] = {
    ...state,
    pending: false,
    ...(stage === "both" || stage === "latency"
      ? { latency: result.latency_ms }
      : {}),
    ...(stage === "both" || stage === "speed"
      ? { speed: result.speed_bps }
      : {}),
    error: undefined,
  };
}

function applyError(target: TestTarget, cause: unknown): void {
  probeStates[target.key] = {
    pending: false,
    error: toUserMessage(cause),
  };
}

async function runSerial(targets: TestTarget[]): Promise<void> {
  testDone.value = 0;
  testTotal.value = targets.length;
  for (const target of targets) {
    setPending(target, true);
    try {
      const result = await testNetworkSource(
        props.definition.key,
        target.value,
        "both",
      );
      applyResult(target, "both", result);
    } catch (cause) {
      applyError(target, cause);
    } finally {
      testDone.value += 1;
    }
  }
}

async function runParallel(targets: TestTarget[]): Promise<void> {
  testDone.value = 0;
  testTotal.value = targets.length * 2;

  for (const target of targets) {
    setPending(target, true);
  }
  await Promise.all(
    targets.map(async (target) => {
      try {
        const result = await testNetworkSource(
          props.definition.key,
          target.value,
          "latency",
        );
        applyResult(target, "latency", result);
      } catch (cause) {
        applyError(target, cause);
      } finally {
        testDone.value += 1;
      }
    }),
  );

  for (const target of targets) {
    setPending(target, true);
  }
  await Promise.all(
    targets.map(async (target) => {
      try {
        const result = await testNetworkSource(
          props.definition.key,
          target.value,
          "speed",
        );
        applyResult(target, "speed", result);
      } catch (cause) {
        applyError(target, cause);
      } finally {
        setPending(target, false);
      }
    }),
  );
}

async function runTest(event: MouseEvent): Promise<void> {
  if (isTesting.value) {
    return;
  }

  testError.value = "";
  const parallel = event.shiftKey;
  const single = event.ctrlKey || event.metaKey;
  const targets = allTargets();

  if (single && !parallel) {
    const target = selectedTarget();
    if (!target) {
      testError.value = t("settings.network.sources.invalidUrl");
      return;
    }
    isTesting.value = true;
    try {
      await runSerial([target]);
    } finally {
      isTesting.value = false;
    }
    return;
  }

  if (!parallel && targets.length === 0) {
    return;
  }

  isTesting.value = true;
  try {
    if (parallel) {
      await runParallel(targets);
    } else {
      await runSerial(rotateFromSelected(targets));
    }
  } finally {
    isTesting.value = false;
  }
}
</script>

<template>
  <div class="network-source">
    <div class="network-source__header">
      <div class="network-source__title">{{ t(definition.titleKey) }}</div>
      <div class="network-source__actions">
        <v-tooltip
          location="top"
          :text="t('settings.network.sources.testHint')"
        >
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              variant="tonal"
              color="primary"
              size="small"
              :loading="isTesting"
              :disabled="isTesting"
              :aria-label="t('settings.network.sources.test')"
              @click="runTest"
            >
              {{
                isTesting
                  ? t("settings.network.sources.testing", {
                      done: testDone,
                      total: testTotal,
                    })
                  : t("settings.network.sources.test")
              }}
            </v-btn>
          </template>
        </v-tooltip>
        <v-btn
          color="primary"
          size="small"
          :disabled="!dirty || saving"
          :loading="saving"
          @click="emit('save')"
        >
          {{ t("common.actions.save") }}
        </v-btn>
      </div>
    </div>

    <div v-if="testError" class="network-source__error" role="alert">
      {{ testError }}
    </div>

    <v-radio-group
      :model-value="selection"
      hide-details
      class="network-source__list"
      @update:model-value="onSelectionUpdate"
    >
      <v-radio
        v-for="source in options"
        :key="source.key"
        :value="source.key"
        class="network-source__row"
      >
        <template #label>
          <div class="network-source__row-main">
            <div class="network-source__label">{{ source.label }}</div>
            <div class="network-source__url">{{ source.url }}</div>
          </div>
          <div class="network-source__result">
            <v-progress-circular
              v-if="probeFor(source.key)?.pending"
              size="14"
              width="2"
              indeterminate
            />
            <template v-else-if="probeFor(source.key)">
              <v-chip
                v-if="probeFor(source.key)?.latency !== undefined"
                size="x-small"
                color="success"
                variant="tonal"
                :aria-label="`${t('settings.network.sources.latency')} ${formatLatency(probeFor(source.key)?.latency ?? 0)}`"
              >
                {{ formatLatency(probeFor(source.key)?.latency ?? 0) }}
              </v-chip>
              <v-chip
                v-if="probeFor(source.key)?.speed !== undefined"
                size="x-small"
                color="primary"
                variant="tonal"
                :aria-label="`${t('settings.network.sources.speed')} ${formatSpeed(probeFor(source.key)?.speed ?? null)}`"
              >
                {{ formatSpeed(probeFor(source.key)?.speed ?? null) }}
              </v-chip>
              <v-chip
                v-if="probeFor(source.key)?.error"
                size="x-small"
                color="error"
                variant="tonal"
                :title="probeFor(source.key)?.error"
              >
                {{ t("settings.network.sources.failed") }}
              </v-chip>
            </template>
          </div>
        </template>
      </v-radio>
    </v-radio-group>

    <div class="network-source__custom">
      <v-text-field
        v-model="customValue"
        :label="t('settings.network.sources.custom')"
        :placeholder="t('settings.network.sources.customPlaceholder')"
        variant="outlined"
        density="compact"
        hide-details
        clearable
        :disabled="saving"
        @focus="selection = CUSTOM_KEY"
        @update:model-value="onCustomInput"
      />
    </div>
  </div>
</template>

<style scoped>
.network-source {
  padding: 16px;
}

.network-source__header {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: flex-start;
  justify-content: space-between;
}

.network-source__title {
  font-size: 1rem;
  font-weight: 600;
  line-height: 1.4;
}

.network-source__actions {
  display: flex;
  flex: 0 0 auto;
  flex-wrap: wrap;
  gap: 8px;
}

.network-source__error {
  margin-top: 8px;
  color: rgb(var(--v-theme-error));
  font-size: 0.8rem;
}

.network-source__list {
  margin-top: 8px;
}

.network-source__row {
  padding: 8px 0;
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.network-source__row :deep(.v-label) {
  display: flex;
  flex: 1 1 auto;
  gap: 12px;
  align-items: center;
  min-width: 0;
  opacity: 1;
}

.network-source__row-main {
  flex: 1 1 auto;
  min-width: 0;
}

.network-source__label {
  color: rgba(var(--v-theme-on-surface), 0.88);
  font-size: 0.86rem;
  line-height: 1.4;
}

.network-source__url {
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-family: "Consolas", "Cascadia Mono", monospace;
  font-size: 0.72rem;
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.network-source__result {
  display: flex;
  flex: 0 0 auto;
  flex-wrap: wrap;
  gap: 4px;
  justify-content: flex-end;
}

.network-source__custom {
  padding-left: 32px;
}

@media (max-width: 720px) {
  .network-source {
    padding: 14px;
  }

  .network-source__actions {
    width: 100%;
  }

  .network-source__row :deep(.v-label) {
    flex-wrap: wrap;
  }

  .network-source__result {
    width: 100%;
    justify-content: flex-start;
    margin-left: 32px;
  }

  .network-source__custom {
    padding-left: 0;
  }
}
</style>
