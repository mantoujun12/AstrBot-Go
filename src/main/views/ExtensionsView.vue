<script setup lang="ts">
import { useI18n } from "vue-i18n";
import ExtensionCard from "../components/extensions/ExtensionCard.vue";
import PageHeader from "../components/common/PageHeader.vue";
import { EXTENSIONS } from "../constants/extensions";

const { t } = useI18n();
</script>

<template>
  <div class="page-shell extensions-page">
    <PageHeader :title="t('extensions.title')" />
    <div class="extensions-page__grid">
      <ExtensionCard
        v-for="extension in EXTENSIONS"
        :key="extension.id"
        :icon="extension.icon"
        :title="t(`extensions.items.${extension.id}.title`)"
        :description="t(`extensions.items.${extension.id}.description`)"
        :to="extension.to"
      />
    </div>
  </div>
</template>

<style scoped>
.extensions-page__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
}
</style>
