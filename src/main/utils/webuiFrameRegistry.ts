/**
 * Live embedded instance-WebUI frames, used to authenticate the tab-shortcut
 * bridge. The bridge script injected by `src-tauri/src/webview_shortcuts.rs`
 * forwards keydowns from inside cross-origin frames, and a `postMessage`
 * sender cannot be verified beyond its frame identity, so the launcher only
 * honors messages whose `source` is the window of an iframe it embedded
 * itself. Every other sender (model-generated HTML panels, popups, foreign
 * frames) is ignored. Scripts running inside an embedded WebUI share that
 * frame's JS context with the bridge script, so they can still post bridge
 * messages; that residual is inherent to the feature and limited to the
 * instance WebUIs the user embedded.
 */
const embeddedWebuiFrames = new Set<HTMLIFrameElement>();

/** Registers a mounted instance-WebUI iframe as a trusted bridge sender. */
export function registerEmbeddedWebuiFrame(frame: HTMLIFrameElement | null): void {
  if (frame) {
    embeddedWebuiFrames.add(frame);
  }
}

/** Drops an unmounted iframe from the trusted bridge senders. */
export function unregisterEmbeddedWebuiFrame(frame: HTMLIFrameElement | null): void {
  if (frame) {
    embeddedWebuiFrames.delete(frame);
  }
}

/**
 * Returns whether `source` is the window of a live embedded instance-WebUI
 * frame. `contentWindow` is read at call time so a reloaded frame (a fresh
 * element created by the `:key` remount) matches as soon as it mounts.
 */
export function isEmbeddedWebuiSource(source: MessageEventSource | null): boolean {
  if (!source) {
    return false;
  }
  for (const frame of embeddedWebuiFrames) {
    if (frame.contentWindow === source) {
      return true;
    }
  }
  return false;
}
