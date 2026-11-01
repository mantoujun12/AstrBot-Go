import { reactive } from "vue";
import { runMutation } from "../stores/launcher";
import { parseBackendError } from "../services/errors";
import { showToast } from "../stores/toast";

const busyKeys = reactive<Record<string, boolean>>({});

export interface RunOperationOptions {
  success?: string;
  /**
   * Return `true` when the error was already handled (for example by opening a
   * lock dialog), which suppresses the default toast.
   */
  onError?: (error: unknown) => boolean | void;
}

export function useOperation() {
  function isBusy(key: string): boolean {
    return busyKeys[key] === true;
  }

  async function run<T>(
    key: string,
    task: () => Promise<T>,
    options: RunOperationOptions = {},
  ): Promise<boolean> {
    if (busyKeys[key]) {
      return false;
    }

    busyKeys[key] = true;
    try {
      await runMutation(task);
      if (options.success) {
        showToast(options.success, "success");
      }
      return true;
    } catch (cause) {
      const handled = options.onError?.(cause);
      if (!handled) {
        showToast(parseBackendError(cause).message, "error");
      }
      return false;
    } finally {
      busyKeys[key] = false;
    }
  }

  return { isBusy, run };
}
