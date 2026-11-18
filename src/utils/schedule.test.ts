import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { i18n } from "../i18n";
import type { SupportedLocale } from "../types/launcher";
import type { ScheduledJob } from "../types/launcher";
import { scheduleDescription } from "./schedule";

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
