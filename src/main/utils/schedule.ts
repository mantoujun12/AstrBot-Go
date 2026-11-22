import { i18n } from "../i18n";
import type { ScheduledJob } from "../types/launcher";
import type { SupportedLocale } from "../types/launcher";
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

// In-source tests, stripped from the production bundle by the
// `import.meta.vitest` define in vite.config.ts.
if (import.meta.vitest) {
  const { afterAll, beforeAll, describe, expect, it } = import.meta.vitest;

  function job(patch: Partial<ScheduledJob>): ScheduledJob {
    return {
      id: "job-1",
      name: "",
      action: "backup",
      instance_id: "instance-1",
      mode: "daily",
      run_at: null,
      interval_minutes: null,
      time: null,
      weekday: null,
      enabled: true,
      next_run_at: null,
      last_run_at: null,
      last_success: null,
      last_error: null,
      ...patch,
    };
  }

  const initialLocale = i18n.global.locale.value;

  function useLocale(locale: SupportedLocale): void {
    i18n.global.locale.value = locale;
  }

  beforeAll(() => {
    useLocale("zh-CN");
  });

  afterAll(() => {
    useLocale(initialLocale);
  });

  describe("scheduleDescription", () => {
    it("describes a daily job with its time", () => {
      const text = scheduleDescription(job({ mode: "daily", time: "03:00" }));
      expect(text).toContain("03:00");
      expect(text).toContain("每天");

      useLocale("en-US");
      expect(scheduleDescription(job({ mode: "daily", time: "03:00" }))).toBe(
        "Every day at 03:00",
      );
      useLocale("zh-CN");
    });

    it("describes a weekly job with the localized weekday", () => {
      expect(
        scheduleDescription(job({ mode: "weekly", time: "03:00", weekday: 0 })),
      ).toContain("周日");
      expect(
        scheduleDescription(job({ mode: "weekly", time: "03:00", weekday: 3 })),
      ).toContain("周三");
      expect(
        scheduleDescription(job({ mode: "weekly", time: "03:00", weekday: 6 })),
      ).toContain("周六");
    });

    it("describes an interval job in minutes", () => {
      const text = scheduleDescription(
        job({ mode: "interval", interval_minutes: 90 }),
      );
      expect(text).toContain("90");
      expect(text).toContain("分钟");
    });

    it("describes a one-time job with its date and time", () => {
      const text = scheduleDescription(
        job({ mode: "once", run_at: "2026-11-20T18:30" }),
      );
      expect(text).toContain("18:30");
    });
  });
}