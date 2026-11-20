// Island glue: contract the host app exposes so island components can read
// and switch the chat target instance. ChatUIHost provides the value; the
// instance & model panel consumes it.

import type { ComputedRef, InjectionKey, Ref } from "vue";

export interface ChatReadiness {
  /** Target instance process state is `running`. */
  running: boolean;
  /** Target instance has dashboard credentials, so chat can authenticate. */
  credentialsConfigured: boolean;
}

export interface ChatInstanceSelection {
  /** Instance the island currently serves; null before one is picked. */
  current: Ref<string | null>;
  /** Switches the chat target; the host owns what a switch implies. */
  select: (instanceId: string) => void;
  /** Whether the current instance can serve chat; no instance is not ready. */
  readiness: ComputedRef<ChatReadiness>;
}

export const chatInstanceSelectionKey: InjectionKey<ChatInstanceSelection> =
  Symbol("chatInstanceSelection");
