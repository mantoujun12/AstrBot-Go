import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import BackupsView from "../views/BackupsView.vue";
import InstancesView from "../views/InstancesView.vue";
import OverviewView from "../views/OverviewView.vue";
import SettingsView from "../views/SettingsView.vue";
import VersionsView from "../views/VersionsView.vue";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "overview",
    component: OverviewView,
  },
  {
    path: "/instances",
    name: "instances",
    component: InstancesView,
  },
  {
    path: "/versions",
    name: "versions",
    component: VersionsView,
  },
  {
    path: "/backups",
    name: "backups",
    component: BackupsView,
  },
  {
    path: "/settings",
    name: "settings",
    component: SettingsView,
  },
  {
    path: "/:pathMatch(.*)*",
    redirect: "/",
  },
];

const router = createRouter({
  history: createWebHashHistory(import.meta.env.BASE_URL),
  routes,
  scrollBehavior: () => ({ top: 0 }),
});

export { router };
