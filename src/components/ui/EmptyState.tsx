import type { ReactNode } from "react";
import { cn } from "@/lib/format";

export function EmptyState({
  icon,
  title,
  body,
  action,
  tone = "neutral",
  compact = false,
}: {
  icon?: ReactNode;
  title: ReactNode;
  body?: ReactNode;
  action?: ReactNode;
  tone?: "neutral" | "warn" | "danger";
  compact?: boolean;
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center text-center",
        compact ? "gap-2 px-4 py-6" : "gap-2.5 px-6 py-12",
      )}
    >
      {icon ? (
        <span
          className={cn(
            "mb-1 flex items-center justify-center rounded-[10px] border",
            compact ? "h-8 w-8" : "h-10 w-10",
            tone === "warn" && "border-[#4a3a1d] bg-[#241d12] text-warn",
            tone === "danger" && "border-[#482a2d] bg-[#241618] text-danger",
            tone === "neutral" && "border-line-soft bg-[#141a21] text-muted",
          )}
        >
          {icon}
        </span>
      ) : null}
      <h3 className="text-[13px] font-medium text-ink">{title}</h3>
      {body ? (
        <p className="max-w-[56ch] text-[12px] leading-[1.55] text-muted">{body}</p>
      ) : null}
      {action ? <div className="mt-2 flex items-center gap-2">{action}</div> : null}
    </div>
  );
}

export function KeyValue({
  label,
  value,
  mono = false,
  title,
  action,
  className,
}: {
  label: ReactNode;
  value: ReactNode;
  mono?: boolean;
  title?: string;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex items-start justify-between gap-6 py-[7px]", className)}>
      <span className="shrink-0 text-[12px] text-muted">{label}</span>
      <span className="flex min-w-0 items-center gap-1.5">
        <span
          title={title}
          className={cn(
            "truncate text-right text-[12.5px] text-ink",
            mono && "font-mono text-[11.5px]",
          )}
        >
          {value}
        </span>
        {action}
      </span>
    </div>
  );
}

export function InlineNote({
  tone = "neutral",
  icon,
  children,
  className,
}: {
  tone?: "neutral" | "accent" | "warn" | "danger" | "ok";
  icon?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  const styles = {
    neutral: "border-line-soft bg-[#141a21] text-muted",
    accent: "border-accent-line bg-accent-soft text-[#b9c9ff]",
    warn: "border-[#4a3a1d] bg-[#221b10] text-[#e0c58f]",
    danger: "border-[#482a2d] bg-[#231618] text-[#eaa4a4]",
    ok: "border-[#234333] bg-[#131f19] text-[#a8dcc2]",
  }[tone];

  return (
    <div
      className={cn(
        "flex items-start gap-2.5 rounded-[10px] border px-3 py-2.5 text-[11.5px] leading-[1.5]",
        styles,
        className,
      )}
    >
      {icon ? <span className="mt-[1px] shrink-0">{icon}</span> : null}
      <div className="min-w-0">{children}</div>
    </div>
  );
}
