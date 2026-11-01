/**
 * Development-only mock backend based on Tauri's official mockIPC helper.
 *
 * `main.ts` imports this module dynamically only when `import.meta.env.DEV`
 * is true, so production builds never load or execute the fixtures.
 */

import type { InvokeArgs } from "@tauri-apps/api/core";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { mockCommands, mockPluginCommands } from "./handlers";
import type { MockCommandHandler } from "./handlers";

const MOCK_LATENCY_MS = 120;

let installed = false;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function toRecord(payload: InvokeArgs | undefined): Record<string, unknown> {
  if (
    payload === undefined ||
    Array.isArray(payload) ||
    payload instanceof ArrayBuffer ||
    payload instanceof Uint8Array
  ) {
    return {};
  }
  return payload;
}

async function runWithLatency(
  handler: MockCommandHandler,
  args: Record<string, unknown>,
): Promise<unknown> {
  await delay(MOCK_LATENCY_MS);
  return handler(args);
}

/**
 * Installs the mock IPC layer and a fake `main` window.
 *
 * Must run before importing modules that call `getCurrentWindow()`, which is
 * why the real app entry lives in `bootstrap.ts` and is loaded dynamically.
 */
export function installMockBackend(): void {
  if (!import.meta.env.DEV) {
    throw new Error("Mock 后端仅允许在开发环境启用");
  }
  if (installed) {
    return;
  }
  installed = true;

  mockWindows("main");

  mockIPC(
    (cmd, payload) => {
      const args = toRecord(payload);

      const pluginHandler = mockPluginCommands[cmd];
      if (pluginHandler) {
        return pluginHandler(args);
      }

      const handler = mockCommands[cmd];
      if (!handler) {
        console.warn(`[mocks] 未实现的命令：${cmd}`);
        throw new Error(`Mock 未实现命令：${cmd}`);
      }

      return runWithLatency(handler, args);
    },
    { shouldMockEvents: true },
  );

  console.info("[mocks] 前端 Mock 后端已启用");
}
