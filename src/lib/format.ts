import type { Translate } from "@/i18n";

export function cn(...values: Array<string | false | null | undefined>): string {
  return values.filter(Boolean).join(" ");
}

export function bytes(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(value)) return "—";
  if (value < 1024) return `${value} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let amount = value / 1024;
  let unit = 0;
  while (amount >= 1024 && unit < units.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  const digits = amount >= 100 ? 0 : amount >= 10 ? 1 : 2;
  return `${amount.toFixed(digits)} ${units[unit]}`;
}

export function gib(value: number | null | undefined): string {
  if (value === null || value === undefined) return "—";
  const gb = value / 1024 / 1024 / 1024;
  return gb >= 100 ? `${Math.round(gb)} GB` : `${gb.toFixed(1)} GB`;
}

export function shortCommit(sha: string | null | undefined, length = 8): string {
  if (!sha) return "—";
  return sha.slice(0, length);
}

export function duration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "—";
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) return `${h}h ${m.toString().padStart(2, "0")}m`;
  if (m > 0) return `${m}m ${s.toString().padStart(2, "0")}s`;
  return `${s}s`;
}

export function relativeTime(
  iso: string | null | undefined,
  t: Translate,
  locale?: string,
): string {
  if (!iso) return "—";
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return iso;
  const delta = (Date.now() - then) / 1000;
  if (delta < 45) return t("time.justNow");
  if (delta < 90) return t("time.minuteAgo");
  if (delta < 3600) return t("time.minutesAgo", { count: Math.round(delta / 60) });
  if (delta < 86400) return t("time.hoursAgo", { count: Math.round(delta / 3600) });
  if (delta < 604800) return t("time.daysAgo", { count: Math.round(delta / 86400) });
  return new Date(then).toLocaleDateString(locale);
}

export function localTime(iso: string | null | undefined, locale?: string): string {
  if (!iso) return "—";
  const parsed = Date.parse(iso);
  if (Number.isNaN(parsed)) return iso;
  return new Date(parsed).toLocaleString(locale, {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function etaLabel(value: string | null): string {
  if (!value) return "—";
  return value
    .replace(/(\d+)h/g, "$1h ")
    .replace(/(\d+)m/g, "$1m ")
    .replace(/(\d+)s/g, "$1s")
    .trim();
}

export function capitalize(value: string): string {
  return value.length === 0 ? value : value[0].toUpperCase() + value.slice(1);
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function basename(path: string | null | undefined): string {
  if (!path) return "—";
  const parts = path.replace(/[\\/]+$/, "").split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

export function pluralize(count: number, one: string, many: string): string {
  return count === 1 ? one : many;
}
