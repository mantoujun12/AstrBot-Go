<script setup lang="ts">
defineProps<{
  icon: string;
  label: string;
  to?: string;
  /** Right-edge arrow hint for entries that open something instead of navigating. */
  appendIcon?: string;
  beta?: boolean;
  tooltipDisabled: boolean;
  active?: boolean;
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
        :active="active"
        :aria-label="label"
        rounded="lg"
        :to="to"
        @click="emit('select')"
      >
        <template #prepend>
          <v-icon :icon="icon" size="18" />
        </template>
        <v-list-item-title>{{ label }}</v-list-item-title>
        <template v-if="beta || appendIcon" #append>
          <v-chip v-if="beta" size="x-small" color="purple" variant="tonal">
            Beta
          </v-chip>
          <v-icon v-if="appendIcon" :icon="appendIcon" size="16" class="sidebar-nav-append" />
        </template>
      </v-list-item>
    </template>
  </v-tooltip>
</template>

<style scoped>
/* A hint arrow stays quieter than the item text in both themes. */
.sidebar-nav-append {
  opacity: 0.45;
}
</style>
