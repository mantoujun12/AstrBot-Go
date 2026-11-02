import { i18n } from "./index";

/**
 * Translates a key supplied by the Rust backend, falling back to the backend
 * message when the current locale does not define the key.
 */
export function translateDynamicKey(
  key: string | null | undefined,
  fallback: string,
  params?: Record<string, string | number>,
): string {
  if (!key) {
    return fallback;
  }
  if (!i18n.global.te(key)) {
    return fallback;
  }
  return i18n.global.t(key, params ?? {});
}
