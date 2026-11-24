import { describe, expect, it, vi } from "vitest";
import { createTranslator } from "@/i18n";
import {
  basename,
  bytes,
  capitalize,
  clamp,
  cn,
  duration,
  etaLabel,
  gib,
  localTime,
  pluralize,
  relativeTime,
  shortCommit,
} from "./format";

describe("cn", () => {
  it("drops every falsy value and keeps order", () => {
    expect(cn("a", false, null, undefined, "b", "")).toBe("a b");
    expect(cn()).toBe("");
  });
});

describe("bytes", () => {
  it("formats each unit with a sensible precision", () => {
    expect(bytes(0)).toBe("0 B");
    expect(bytes(512)).toBe("512 B");
    expect(bytes(1024)).toBe("1.00 KB");
    expect(bytes(1536)).toBe("1.50 KB");
    expect(bytes(1024 * 1024)).toBe("1.00 MB");
    expect(bytes(150 * 1024 * 1024)).toBe("150 MB");
    expect(bytes(3 * 1024 ** 4)).toBe("3.00 TB");
  });

  it("shows an em dash for missing values", () => {
    expect(bytes(null)).toBe("—");
    expect(bytes(undefined)).toBe("—");
    expect(bytes(Number.NaN)).toBe("—");
  });
});

describe("gib", () => {
  it("switches to whole numbers for large volumes", () => {
    expect(gib(1024 ** 3)).toBe("1.0 GB");
    expect(gib(2.5 * 1024 ** 3)).toBe("2.5 GB");
    expect(gib(200 * 1024 ** 3)).toBe("200 GB");
    expect(gib(null)).toBe("—");
  });
});

describe("duration", () => {
  it("formats hours, minutes and seconds", () => {
    expect(duration(0)).toBe("0s");
    expect(duration(9)).toBe("9s");
    expect(duration(65)).toBe("1m 05s");
    expect(duration(3725)).toBe("1h 02m");
  });

  it("rejects impossible input", () => {
    expect(duration(-1)).toBe("—");
    expect(duration(Number.POSITIVE_INFINITY)).toBe("—");
  });
});

describe("shortCommit", () => {
  it("truncates to the requested length", () => {
    expect(shortCommit("a28927849adb4e40dfdec8c32fb6758dcd205a37")).toBe("a2892784");
    expect(shortCommit("a28927849adb4e40dfdec8c32fb6758dcd205a37", 4)).toBe("a289");
    expect(shortCommit(null)).toBe("—");
    expect(shortCommit("")).toBe("—");
  });
});

describe("basename", () => {
  it("handles Windows separators and trailing slashes", () => {
    expect(basename("D:\\Leonida\\workspace\\repository")).toBe("repository");
    expect(basename("D:\\Leonida\\workspace\\repository\\")).toBe("repository");
    expect(basename("D:/a/b/c")).toBe("c");
    expect(basename(null)).toBe("—");
  });
});

describe("etaLabel", () => {
  it("adds spaces between the units make_game.py prints", () => {
    expect(etaLabel("1h2m3s")).toBe("1h 2m 3s");
    expect(etaLabel("12m30s")).toBe("12m 30s");
    expect(etaLabel(null)).toBe("—");
  });
});

describe("misc helpers", () => {
  it("clamps into range", () => {
    expect(clamp(5, 0, 10)).toBe(5);
    expect(clamp(-5, 0, 10)).toBe(0);
    expect(clamp(50, 0, 10)).toBe(10);
  });

  it("capitalizes only the first character", () => {
    expect(capitalize("leonida")).toBe("Leonida");
    expect(capitalize("")).toBe("");
  });

  it("pluralizes by count", () => {
    expect(pluralize(1, "commit", "commits")).toBe("commit");
    expect(pluralize(0, "commit", "commits")).toBe("commits");
    expect(pluralize(2, "commit", "commits")).toBe("commits");
  });
});

describe("time helpers", () => {
  const en = createTranslator("en");
  const ru = createTranslator("ru");

  it("returns the input when the timestamp cannot be parsed", () => {
    expect(relativeTime("not a date", en)).toBe("not a date");
    expect(localTime("not a date")).toBe("not a date");
    expect(relativeTime(null, en)).toBe("—");
    expect(localTime(null)).toBe("—");
  });

  it("describes recent timestamps humanely", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-11-24T12:00:00Z"));
    expect(relativeTime("2026-11-24T11:59:40Z", en)).toBe("just now");
    expect(relativeTime("2026-11-24T11:58:00Z", en)).toBe("2 min ago");
    expect(relativeTime("2026-11-24T09:00:00Z", en)).toBe("3 h ago");
    expect(relativeTime("2026-11-22T12:00:00Z", en)).toBe("2 d ago");
    vi.useRealTimers();
  });

  it("follows the interface language", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-11-24T12:00:00Z"));
    expect(relativeTime("2026-11-24T11:59:50Z", ru)).toBe("только что");
    expect(relativeTime("2026-11-24T11:58:00Z", ru)).toBe("2 мин назад");
    expect(relativeTime("2026-11-24T09:00:00Z", ru)).toBe("3 ч назад");
    expect(relativeTime("2026-11-22T12:00:00Z", ru)).toBe("2 дн. назад");
    vi.useRealTimers();
  });
});
