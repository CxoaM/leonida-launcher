import type { ReactNode } from "react";
import { cn } from "@/lib/format";

const iconTone = {
  neutral: "text-faint",
  warn: "text-warn",
  danger: "text-danger",
};

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
        "flex flex-col items-start",
        compact ? "gap-1.5 py-4" : "gap-2 py-8",
      )}
    >
      {icon ? (
        <span className={cn("mb-0.5 shrink-0", compact ? "" : "mb-1", iconTone[tone])}>
          {icon}
        </span>
      ) : null}
      <h3 className={cn("font-medium text-ink", compact ? "text-[12.5px]" : "text-[14px]")}>
        {title}
      </h3>
      {body ? (
        <p className="max-w-[62ch] text-[12px] leading-[1.55] text-muted">{body}</p>
      ) : null}
      {action ? <div className="mt-1.5 flex items-center gap-2">{action}</div> : null}
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
    <div
      className={cn(
        "flex min-h-8 items-start justify-between gap-6 py-1.5",
        className,
      )}
    >
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
    neutral: "border-line bg-base-soft text-muted",
    accent: "border-accent-line bg-accent-soft text-muted",
    warn: "border-[#453821] bg-[#1c1811] text-[#d8c194]",
    danger: "border-[#4a2e2b] bg-[#1e1513] text-[#e0a49c]",
    ok: "border-[#313d2b] bg-[#151a13] text-[#b6ccae]",
  }[tone];

  return (
    <div
      className={cn(
        "flex items-start gap-2.5 rounded-r-[6px] border-y-0 border-r-0 border-l-2 px-3 py-2 text-[11.5px] leading-[1.55]",
        styles,
        className,
      )}
    >
      {icon ? <span className="mt-[1px] shrink-0">{icon}</span> : null}
      <div className="min-w-0">{children}</div>
    </div>
  );
}
