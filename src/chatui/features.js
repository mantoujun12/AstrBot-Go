// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// v1 stub list: entry points hidden while the Tauri backend has no matching
// command. Flip a flag here (and add the command + adapter route) to enable.

export const CHATUI_FEATURES = {
  // The host renders the toolbar context (model selector + session title).
  toolbar: true,
  // File/image upload needs chat upload commands; the "+" menu stays hidden.
  uploads: false,
  // Voice input needs the STT pipeline (useChatVoice is stubbed).
  voice: false,
  // Session rename/delete need chat session write commands.
  sessionWrites: false,
  // The Rust transport always streams; upstream transport toggles stay hidden.
  transportSelect: false,
  // Streaming/reasoning request flags are not forwarded by the Rust transport.
  streamToggles: false,
};
