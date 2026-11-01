<script setup lang="ts">
const navigation = [
  { to: "/", label: "概览", icon: "mdi-view-dashboard-outline" },
  { to: "/instances", label: "实例", icon: "mdi-server-outline" },
  { to: "/versions", label: "版本", icon: "mdi-tag-multiple-outline" },
  { to: "/backups", label: "备份", icon: "mdi-backup-restore" },
];
</script>

<template>
  <v-app id="inspire">
    <v-navigation-drawer
      class="pt-4"
      color="grey-lighten-3"
      model-value
      rail
    >
      <router-link
        v-for="item in navigation"
        :key="item.to"
        v-slot="{ href, isActive, navigate }"
        :to="item.to"
        custom
      >
        <a
          :href="href"
          class="launcher-nav-link"
          :aria-label="item.label"
          @click="navigate"
        >
          <v-avatar
            :color="isActive ? 'grey-darken-1' : 'grey-lighten-1'"
            :size="isActive ? 36 : 20"
            class="d-block mx-auto mb-9"
          >
            <v-icon
              :icon="item.icon"
              :size="isActive ? 20 : 13"
            />
          </v-avatar>
        </a>
      </router-link>
    </v-navigation-drawer>

    <v-main>
      <router-view />
    </v-main>
  </v-app>
</template>

<style scoped>
.launcher-nav-link {
  display: block;
  color: inherit;
  text-decoration: none;
  outline: none;
}

.launcher-nav-link:focus-visible .v-avatar {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: 3px;
}

.launcher-nav-link .v-avatar {
  transition:
    width 160ms ease,
    height 160ms ease,
    background-color 160ms ease;
}

@media (prefers-reduced-motion: reduce) {
  .launcher-nav-link .v-avatar {
    transition: none;
  }
}
</style>
