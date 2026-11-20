// Typed facade for the (type-check-excluded) chatui island; see index.js.
import type { DefineComponent } from "vue";

/** Host wrapper around the ported AstrBot ChatUI (chatbox mode). */
export declare const ChatUIHost: DefineComponent<{
  /** Target AstrBot instance; a change remounts the conversation. */
  instanceId: string | null;
}>;
