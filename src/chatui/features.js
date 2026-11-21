// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// v1 stub list: entry points hidden while the Tauri backend has no matching
// command. Flip a flag here (and add the command + adapter route) to enable.

export const CHATUI_FEATURES = {
  // The host renders the toolbar context (model selector + session title).
  toolbar: true,
  // File/image upload needs chat upload commands; the "+" menu keeps only its
  // config entry while upload items stay hidden.
  uploads: false,
  // Config profile routes are served by the dashboard chat API, so the
  // composer "+" menu keeps its config entry reachable.
  configSelection: true,
  // Voice input needs the STT pipeline (useChatVoice is stubbed).
  voice: false,
  // Session rename/hide go through the dashboard chat API.
  sessionWrites: true,
  // The Rust transport always streams; upstream transport toggles stay hidden.
  transportSelect: false,
  // Streaming/reasoning request flags are not forwarded by the Rust transport.
  streamToggles: false,
  // shortcut: no provider workspace commands exist in v1, port the dashboard workspace view to enable it.
  providerWorkspace: false,
  // shortcut: thread load/send commands are missing from api/v1.ts, add them to enable the thread panel.
  threads: false,
};
