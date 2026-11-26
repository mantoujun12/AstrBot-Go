//! Bridges browser-style tab shortcuts out of embedded iframes.
//!
//! Keydown events that happen inside a cross-origin iframe never reach the
//! launcher window, so this script is injected into every frame and forwards
//! matching shortcuts to the parent window, which owns the tab logic.
//! Delivery is restricted to known launcher origins (see
//! `setup::LAUNCHER_ORIGINS`): a sub-frame cannot read its parent's origin,
//! so one post per known origin is attempted and the browser delivers only to
//! the one that matches.

const TAB_SHORTCUT_BRIDGE_SCRIPT: &str = r#"
(function () {
  if (window.top === window) {
    return;
  }

  var SOURCE = "astrbot-launcher:tab-shortcut";
  var ALLOWED_ORIGINS = __LAUNCHER_ORIGINS__;

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
      // The launcher origin is unreadable from a cross-origin frame, so the
      // shortcut is posted once per known origin; the browser silently drops
      // every non-matching post and a foreign parent never sees the message.
      ALLOWED_ORIGINS.forEach(function (targetOrigin) {
        window.top.postMessage(
          { source: SOURCE, key: event.key, shiftKey: event.shiftKey },
          targetOrigin
        );
      });
    },
    true
  );
})();
"#;
fn tab_shortcut_bridge_script() -> String {
    TAB_SHORTCUT_BRIDGE_SCRIPT.replace(
        "__LAUNCHER_ORIGINS__",
        &crate::setup::launcher_origins_js_array(),
    )
}

pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("tab-shortcut-bridge")
        .js_init_script_on_all_frames(tab_shortcut_bridge_script())
        .build()
}
