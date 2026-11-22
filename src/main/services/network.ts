import { invoke } from "@tauri-apps/api/core";
import type { ProxySettings } from "./config";

export type NetworkSourceKind =
  | "github"
  | "pypi"
  | "nodejs"
  | "npm"
  | "python"
  | "uv";

export type NetworkProbeStage = "both" | "latency" | "speed";

export interface NetworkProbeResult {
  latency_ms: number;
  speed_bps: number | null;
  bytes: number;
}

export function testNetworkSource(
  kind: NetworkSourceKind,
  value: string,
  stage: NetworkProbeStage = "both",
): Promise<NetworkProbeResult> {
  return invoke<NetworkProbeResult>("test_network_source", {
    kind,
    value,
    stage,
  });
}

export function testHttpProxy(
  input: ProxySettings,
): Promise<NetworkProbeResult> {
  return invoke<NetworkProbeResult>("test_http_proxy", {
    proxyUrl: input.proxyUrl,
    proxyPort: input.proxyPort,
    proxyUsername: input.proxyUsername,
    proxyPassword: input.proxyPassword,
  });
}
