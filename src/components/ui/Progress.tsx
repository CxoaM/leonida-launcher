import { cn } from "@/lib/format";

export function Progress({
  value,
  tone = "accent",
  className,
  height = "h-1.5",
}: {
  value: number | null;
  tone?: "accent" | "ok" | "warn" | "danger";
  className?: string;
  height?: string;
}) {
  const filled = {
    accent: "bg-accent",
    ok: "bg-ok",
    warn: "bg-warn",
    danger: "bg-danger",
  }[tone];

  const clamped = value === null ? null : Math.max(0, Math.min(100, value));

  return (
    <div
      className={cn("relative w-full overflow-hidden rounded-full bg-[#20272f]", height, className)}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={clamped === null ? undefined : Math.round(clamped)}
    >
      {clamped === null ? (
        <div className={cn("h-full w-2/5 rounded-full animate-indeterminate", filled)} />
      ) : (
        <div
          className={cn("h-full rounded-full transition-[width] duration-300 ease-out", filled)}
          style={{ width: `${clamped}%` }}
        />
      )}
    </div>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <div className={cn("shimmer rounded-[6px] bg-[#1a212a]", className)} />;
}

export function SkeletonRows({ rows = 3 }: { rows?: number }) {
  return (
    <div className="space-y-2.5">
      {Array.from({ length: rows }).map((_, index) => (
        <div key={index} className="flex items-center justify-between gap-6">
          <Skeleton className="h-3 w-24" />
          <Skeleton className="h-3 w-40" />
        </div>
      ))}
    </div>
  );
}
