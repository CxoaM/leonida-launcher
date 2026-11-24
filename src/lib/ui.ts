import type { CompatibilityStatus, DependencyStatus, JobStatus, Severity, Tone } from "@/types";

export function compatibilityTone(status: CompatibilityStatus | null | undefined): Tone {
  switch (status) {
    case "compatible":
      return "ok";
    case "degraded":
      return "warn";
    case "incompatible":
      return "danger";
    default:
      return "neutral";
  }
}

export function severityTone(severity: Severity): Tone {
  switch (severity) {
    case "ok":
      return "ok";
    case "warning":
      return "warn";
    case "error":
      return "danger";
    default:
      return "neutral";
  }
}

export function dependencyTone(status: DependencyStatus): Tone {
  switch (status) {
    case "installed":
      return "ok";
    case "outdated":
      return "warn";
    case "missing":
    case "invalid":
      return "danger";
    default:
      return "neutral";
  }
}

export function jobTone(status: JobStatus | string | undefined | null): Tone {
  switch (status) {
    case "running":
    case "cancelling":
      return "accent";
    case "succeeded":
      return "ok";
    case "failed":
      return "danger";
    case "cancelled":
      return "warn";
    default:
      return "neutral";
  }
}

export function isJobRunning(status: JobStatus | string | undefined | null): boolean {
  return status === "running" || status === "cancelling";
}

export function isJobFinished(status: JobStatus | string | undefined | null): boolean {
  return status === "succeeded" || status === "failed" || status === "cancelled";
}

export function percentLabel(value: number | null | undefined): string | null {
  if (value === null || value === undefined || !Number.isFinite(value)) return null;
  return `${Math.round(value)}%`;
}

/** Groups diagnostics items while preserving the order the backend produced. */
export function groupBy<T>(items: T[], key: (item: T) => string): Array<[string, T[]]> {
  const order: string[] = [];
  const buckets = new Map<string, T[]>();
  for (const item of items) {
    const name = key(item);
    let bucket = buckets.get(name);
    if (!bucket) {
      bucket = [];
      buckets.set(name, bucket);
      order.push(name);
    }
    bucket.push(item);
  }
  return order.map((name) => [name, buckets.get(name) ?? []]);
}

export function humanizeGroup(group: string): string {
  return group
    .trim()
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ")
    .replace(/^\w/, (c) => c.toUpperCase());
}
