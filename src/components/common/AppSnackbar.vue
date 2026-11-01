<script setup lang="ts">
import { computed } from "vue";
import { useToastStore } from "../../stores/toast";

const { message, type, visible, sequence } = useToastStore();

const color = computed(() => {
  if (type.value === "error") {
    return "error";
  }
  if (type.value === "info") {
    return "info";
  }
  return "success";
});
</script>

<template>
  <v-snackbar
    :key="sequence"
    v-model="visible"
    :color="color"
    :timeout="3200"
    location="bottom"
    rounded="lg"
  >
    {{ message }}
  </v-snackbar>
</template>
