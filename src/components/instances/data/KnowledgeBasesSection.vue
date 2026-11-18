<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { DataTableHeader } from "vuetify";
import EmptyState from "../../common/EmptyState.vue";
import type { InstanceKnowledgeBase } from "../../../types/instanceData";
import { formatFileSize } from "../../../utils/format";

const props = defineProps<{
  kbs: InstanceKnowledgeBase[];
  loaded: boolean;
  loading: boolean;
  readonly: boolean;
}>();

const emit = defineEmits<{
  (event: "delete", kb: InstanceKnowledgeBase): void;
}>();

const { t } = useI18n();

const headers = computed<DataTableHeader[]>(() => [
  {
    title: t("instanceData.knowledgeBases.columns.name"),
    key: "name",
    sortable: false,
  },
  {
    title: t("instanceData.knowledgeBases.columns.description"),
    key: "description",
    sortable: false,
  },
  {
    title: t("instanceData.knowledgeBases.columns.docs"),
    key: "doc_count",
    sortable: false,
    width: 100,
  },
  {
    title: t("instanceData.knowledgeBases.columns.chunks"),
    key: "chunk_count",
    sortable: false,
    width: 100,
  },
  {
    title: t("instanceData.knowledgeBases.columns.size"),
    key: "size_bytes",
    sortable: false,
    width: 110,
  },
  {
    title: t("instanceData.actionsColumn"),
    key: "actions",
    sortable: false,
    align: "end",
    width: 80,
  },
]);
</script>

<template>
  <div class="knowledge-bases">
    <v-progress-linear
      v-if="loading"
      indeterminate
      color="primary"
      height="3"
      rounded
      class="knowledge-bases__progress"
    />
    <EmptyState
      v-else-if="loaded && kbs.length === 0"
      :text="t('instanceData.knowledgeBases.empty')"
      icon="mdi-book-outline"
    >
      <p>{{ t("instanceData.knowledgeBases.emptyHint") }}</p>
    </EmptyState>
    <div v-else class="page-table-card">
      <v-data-table
        :headers="headers"
        :items="kbs"
        item-value="kb_id"
        hide-default-footer
        :items-per-page="-1"
        disable-sort
        hover
        :no-data-text="t('instanceData.knowledgeBases.empty')"
      >
        <template #item.name="{ item }">
          <span class="page-anywhere">
            <span v-if="item.emoji" aria-hidden="true">{{ item.emoji }} </span
            >{{ item.name }}
          </span>
        </template>

        <template #item.description="{ item }">
          <span class="page-anywhere">{{ item.description || "—" }}</span>
        </template>

        <template #item.size_bytes="{ item }">
          <span class="page-nums">{{ formatFileSize(item.size_bytes) }}</span>
        </template>

        <template #item.actions="{ item }">
          <div class="page-row knowledge-bases__row-actions">
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-delete-outline"
                  variant="text"
                  size="small"
                  color="error"
                  :aria-label="t('common.actions.delete')"
                  :disabled="readonly"
                  @click="emit('delete', item)"
                />
              </template>
              {{ t("common.actions.delete") }}
            </v-tooltip>
          </div>
        </template>
      </v-data-table>
    </div>
  </div>
</template>

<style scoped>
.knowledge-bases__progress {
  margin-bottom: 12px;
}

.knowledge-bases__row-actions {
  justify-content: flex-end;
  gap: 2px;
}
</style>
