//! Bridges browser-style tab shortcuts out of embedded iframes.
//!
//! Keydown events that happen inside a cross-origin iframe never reach the
//! launcher window, so this script is injected into every frame and forwards
//! matching shortcuts to the parent window, which owns the tab logic.

const TAB_SHORTCUT_BRIDGE_SCRIPT: &str = r#"
(function () {
  if (window.top === window) {
    return;
  }

  var SOURCE = "astrbot-launcher:tab-shortcut";

  function matchesTabShortcut(event) {
    if (!event.ctrlKey || event.altKey || event.metaKey) {
      return false;
    }
    if (
      event.key === "Tab" ||
      event.key === "F4" ||
      event.key === "PageUp" ||
      event.key === "PageDown" ||
      event.key === "w" ||
      event.key === "W"
    ) {
      return true;
    }
    return /^[1-9]$/.test(event.key);
  }

  window.addEventListener(
    "keydown",
    function (event) {
      if (event.defaultPrevented || !matchesTabShortcut(event)) {
        return;
      }
      event.preventDefault();
      event.stopImmediatePropagation();
      window.top.postMessage(
        { source: SOURCE, key: event.key, shiftKey: event.shiftKey },
        "*"
      );
    },
    true
  );
})();
"#;

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("tab-shortcut-bridge")
        .js_init_script_on_all_frames(TAB_SHORTCUT_BRIDGE_SCRIPT)
        .build()
}
