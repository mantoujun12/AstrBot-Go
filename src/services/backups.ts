import { invoke } from "@tauri-apps/api/core";

export function createBackup(
  instanceId: string,
  note?: string,
): Promise<string> {
  return invoke<string>("create_backup", {
    instanceId,
    note: note ?? null,
  });
}

export function restoreBackup(backupPath: string): Promise<void> {
  return invoke("restore_backup", { backupPath });
}

export function deleteBackup(backupPath: string): Promise<void> {
  return invoke("delete_backup", { backupPath });
}
