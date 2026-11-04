import { ref } from "vue";
import { toUserMessage } from "../services/errors";

export type ToastType = "success" | "error" | "info" | "warning";

const message = ref("");
const type = ref<ToastType>("success");
const visible = ref(false);
const sequence = ref(0);

export function showToast(
  text: string,
  toastType: ToastType = "success",
): void {
  message.value = text;
  type.value = toastType;
  sequence.value += 1;
  visible.value = true;
}

export function showErrorToast(error: unknown): void {
  showToast(toUserMessage(error), "error");
}

export function useToastStore() {
  return {
    message,
    type,
    visible,
    sequence,
    showToast,
    showErrorToast,
  };
}
