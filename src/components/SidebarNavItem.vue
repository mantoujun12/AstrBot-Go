<script setup lang="ts">
defineProps<{
  icon: string;
  label: string;
  to: string;
  tooltipDisabled: boolean;
}>();

const emit = defineEmits<{
  (event: "select"): void;
}>();
</script>

<template>
  <v-tooltip
    :disabled="tooltipDisabled"
    color="surface"
    location="right"
    :open-delay="0"
    :text="label"
  >
    <template #activator="{ props: tooltipProps }">
      <v-list-item
        v-bind="tooltipProps"
        exact
        :aria-label="label"
        rounded="lg"
        :to="to"
        @click="emit('select')"
      >
        <template #prepend>
          <v-icon :icon="icon" size="18" />
        </template>
        <v-list-item-title>{{ label }}</v-list-item-title>
      </v-list-item>
    </template>
  </v-tooltip>
</template>
