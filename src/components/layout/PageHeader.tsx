import type { ReactNode } from "react";
import { cn } from "@/lib/format";

export function PageHeader({
  title,
  subtitle,
  actions,
  className,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <header className={cn("mb-5 flex items-start justify-between gap-6", className)}>
      <div className="min-w-0">
        <h1 className="text-[20px] font-semibold leading-[1.2] tracking-[-0.02em] text-ink">
          {title}
        </h1>
        {subtitle ? (
          <p className="mt-1.5 max-w-[76ch] text-[12.5px] leading-[1.5] text-muted">{subtitle}</p>
        ) : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-2 pt-0.5">{actions}</div> : null}
    </header>
  );
}
