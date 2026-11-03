export function formatLatency(milliseconds: number): string {
  if (!Number.isFinite(milliseconds)) {
    return "—";
  }
  return `${Math.max(0, Math.round(milliseconds))} ms`;
}

export function formatSpeed(bytesPerSecond: number | null): string {
  if (
    bytesPerSecond === null ||
    !Number.isFinite(bytesPerSecond) ||
    bytesPerSecond <= 0
  ) {
    return "—";
  }
  if (bytesPerSecond >= 1024 * 1024) {
    return `${(bytesPerSecond / (1024 * 1024)).toFixed(1)} MB/s`;
  }
  if (bytesPerSecond >= 1024) {
    return `${Math.round(bytesPerSecond / 1024)} KB/s`;
  }
  return `${Math.round(bytesPerSecond)} B/s`;
}
