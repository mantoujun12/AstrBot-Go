import { i18n } from "../i18n";
import type { ScheduledJob } from "../types/launcher";
import { formatDateTime } from "./format";

const WEEKDAY_KEYS = [
  "sun",
  "mon",
  "tue",
  "wed",
  "thu",
  "fri",
  "sat",
] as const;

export function scheduleDescription(job: ScheduledJob): string {
  switch (job.mode) {
    case "once": {
      const raw = job.run_at ?? "";
      const time =
        raw !== "" && !Number.isNaN(Date.parse(raw))
          ? formatDateTime(raw)
          : raw;
      return i18n.global.t("tasks.scheduled.desc.once", { time });
    }
    case "interval":
      return i18n.global.t("tasks.scheduled.desc.interval", {
        count: job.interval_minutes ?? 0,
        unit: i18n.global.t("tasks.scheduled.units.minutes"),
      });
    case "daily":
      return i18n.global.t("tasks.scheduled.desc.daily", {
        time: job.time ?? "",
      });
    case "weekly": {
      const weekday = WEEKDAY_KEYS[job.weekday ?? 1] ?? "mon";
      return i18n.global.t("tasks.scheduled.desc.weekly", {
        weekday: i18n.global.t(`tasks.scheduled.weekdays.${weekday}`),
        time: job.time ?? "",
      });
    }
  }
}