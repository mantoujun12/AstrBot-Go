<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import ScheduledTaskPanel from "../components/tasks/ScheduledTaskPanel.vue";
import TaskHistoryPanel from "../components/tasks/TaskHistoryPanel.vue";
import { useTaskStore } from "../stores/tasks";

type TaskTab = "history" | "scheduled";

const { t } = useI18n();
const { loaded, tasks, activeCount, clearFinishedTasks } = useTaskStore();

const tabs = [
  { value: "history", icon: "mdi-history", label: "tasks.tabs.history" },
  { value: "scheduled", icon: "mdi-calendar-clock", label: "tasks.tabs.scheduled" },
] as const satisfies readonly { value: TaskTab; icon: string; label: string }[];

const activeTab = ref<TaskTab>("history");
const scheduledPanel = ref<InstanceType<typeof ScheduledTaskPanel> | null>(null);
</script>

<template>
  <div class="page-shell tasks-view">
    <div class="tasks-view__toolbar">
      <div class="tasks-view__tabs-scroll">
        <nav
          class="tasks-view__tabs"
          role="tablist"
          :aria-label="t('shell.nav.tasks')"
        >
          <button
            v-for="tab in tabs"
            :key="tab.value"
            type="button"
            class="tasks-view__tab"
            :class="{ 'tasks-view__tab--active': activeTab === tab.value }"
            role="tab"
            :aria-selected="activeTab === tab.value"
            @click="activeTab = tab.value"
          >
            <v-icon :icon="tab.icon" size="16" aria-hidden="true" />
            <span>{{ t(tab.label) }}</span>
          </button>
        </nav>
      </div>

      <div class="page-actions">
        <v-btn
          v-if="activeTab === 'history'"
          variant="tonal"
          prepend-icon="mdi-broom"
          :disabled="!loaded || activeCount === tasks.length"
          @click="clearFinishedTasks"
        >
          {{ t("tasks.actions.clearFinished") }}
        </v-btn>
        <v-btn
          v-else
          color="primary"
          variant="flat"
          prepend-icon="mdi-plus"
          @click="scheduledPanel?.openCreate()"
        >
          {{ t("tasks.scheduled.actions.create") }}
        </v-btn>
      </div>
    </div>

    <TaskHistoryPanel v-if="activeTab === 'history'" />
    <ScheduledTaskPanel v-else ref="scheduledPanel" />
  </div>
</template>

<style scoped>
.tasks-view__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}

.tasks-view__tabs-scroll {
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: none;
}

.tasks-view__tabs-scroll::-webkit-scrollbar {
  display: none;
}

.tasks-view__tabs {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  min-width: max-content;
}

.tasks-view__tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 34px;
  padding: 0 12px;
  border: 0;
  border-radius: var(--astrbot-radius-control);
  background: none;
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-family: inherit;
  font-size: 0.8125rem;
  font-weight: 500;
  cursor: pointer;
}

.tasks-view__tab:hover {
  background: rgba(var(--v-theme-on-surface), 0.045);
  color: rgba(var(--v-theme-on-surface), 0.78);
}

.tasks-view__tab:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: -2px;
}

.tasks-view__tab--active {
  background: rgba(var(--v-theme-on-surface), 0.065);
  color: rgba(var(--v-theme-on-surface), 0.86);
}

.tasks-view__tab--active:hover {
  background: rgba(var(--v-theme-on-surface), 0.09);
  color: rgba(var(--v-theme-on-surface), 0.9);
}
</style>