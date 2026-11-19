// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island port of utils/toast.js: bridges onto the host app's snackbar store.

import { showToast } from "../../stores/toast";

export function useToast() {
  const toast = (message, color = "info", _opts = {}) =>
    showToast(String(message || ""), color === "primary" ? "info" : color);

  return {
    toast,
    success: (msg) => toast(msg, "success"),
    error: (msg) => toast(msg, "error"),
    info: (msg) => toast(msg, "info"),
    warning: (msg) => toast(msg, "warning"),
  };
}
