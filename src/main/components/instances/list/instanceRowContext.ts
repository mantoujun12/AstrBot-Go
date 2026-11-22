import type {
  DiscoveredInstance,
  InstanceStatus,
  InstanceUpdateInfo,
} from "../../../types/launcher";
import type { InstanceRow } from "../../../composables/useInstanceList";

export type ClearKind = "clear-data" | "clear-venv" | "clear-pycache";

export interface ExternalMetaPart {
  icon: string;
  text: string;
}

/**
 * Row-scoped helpers and flags precomputed by `InstancesView` and shared by
 * the table and cards views, so both render identical rules.
 */
export interface InstanceRowContext {
  batchUpgradeRunning: boolean;
  healthLoading: boolean;
  startKey(instanceId: string): string;
  stopKey(instanceId: string): string;
  restartKey(instanceId: string): string;
  upgradeKey(instanceId: string): string;
  isBusy(key: string): boolean;
  isInstanceInteractionDisabled(instanceId: string): boolean;
  portConflictOf(instance: InstanceStatus): number | null;
  updateInfoFor(instanceId: string): InstanceUpdateInfo | null;
  sourceTitleOf(sourceId: string): string;
  sourceIconOf(sourceId: string): string;
  rowSourceOf(row: InstanceRow): string;
  externalMetaParts(row: DiscoveredInstance): ExternalMetaPart[];
}
