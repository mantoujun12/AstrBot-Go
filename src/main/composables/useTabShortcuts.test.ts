import { describe, expect, it } from "vitest";

import {
  registerEmbeddedWebuiFrame,
  unregisterEmbeddedWebuiFrame,
} from "../utils/webuiFrameRegistry";
import { isTrustedBridgeEvent } from "./useTabShortcuts";

const BRIDGE_SOURCE = "astrbot-launcher:tab-shortcut";

function bridgeMessage(): unknown {
  return { source: BRIDGE_SOURCE, key: "Tab", shiftKey: false };
}

function mountedFrame(): HTMLIFrameElement {
  const frame = document.createElement("iframe");
  document.body.appendChild(frame);
  return frame;
}

describe("isTrustedBridgeEvent", () => {
  it("accepts a bridge message from a registered embedded WebUI frame", () => {
    const frame = mountedFrame();
    registerEmbeddedWebuiFrame(frame);
    try {
      expect(
        isTrustedBridgeEvent({ source: frame.contentWindow, data: bridgeMessage() }),
      ).toBe(true);
    } finally {
      unregisterEmbeddedWebuiFrame(frame);
    }
  });

  it("rejects the same message once the frame is unregistered", () => {
    const frame = mountedFrame();
    const source = frame.contentWindow;
    registerEmbeddedWebuiFrame(frame);
    unregisterEmbeddedWebuiFrame(frame);
    expect(isTrustedBridgeEvent({ source, data: bridgeMessage() })).toBe(false);
  });

  it("rejects a bridge message from an unregistered frame", () => {
    const panel = mountedFrame();
    expect(isTrustedBridgeEvent({ source: panel.contentWindow, data: bridgeMessage() })).toBe(
      false,
    );
  });

  it("rejects a registered frame with a malformed payload", () => {
    const frame = mountedFrame();
    registerEmbeddedWebuiFrame(frame);
    try {
      expect(isTrustedBridgeEvent({ source: frame.contentWindow, data: { key: "Tab" } })).toBe(
        false,
      );
    } finally {
      unregisterEmbeddedWebuiFrame(frame);
    }
  });

  it("rejects a null source", () => {
    expect(isTrustedBridgeEvent({ source: null, data: bridgeMessage() })).toBe(false);
  });
});
