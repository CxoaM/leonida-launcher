import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/lib/format";

export function Card({ className, children, ...rest }: HTMLAttributes<HTMLElement>) {
  return (
    <section className={cn("card overflow-hidden", className)} {...rest}>
      {children}
    </section>
  );
}

export function CardHeader({
  title,
  description,
  icon,
  action,
  className,
  compact = false,
}: {
  title: ReactNode;
  description?: ReactNode;
  icon?: ReactNode;
  action?: ReactNode;
  className?: string;
  compact?: boolean;
}) {
  return (
    <header
      className={cn(
        "flex items-start justify-between gap-4 border-b border-line-soft",
        compact ? "px-4 py-2.5" : "px-5 py-3",
        className,
      )}
    >
      <div className="flex min-w-0 items-start gap-2.5">
        {icon ? <span className="mt-[3px] shrink-0 text-faint">{icon}</span> : null}
        <div className="min-w-0">
          <h2 className="truncate text-[13.5px] font-medium text-ink">{title}</h2>
          {description ? (
            <p className="mt-0.5 text-[12px] leading-[1.5] text-muted">{description}</p>
          ) : null}
        </div>
      </div>
      {action ? <div className="flex shrink-0 items-center gap-2">{action}</div> : null}
    </header>
  );
}

export function CardBody({
  className,
  children,
  ...rest
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn("px-5 py-4", className)} {...rest}>
      {children}
    </div>
  );
}

export function CardFooter({ className, children }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "flex items-center justify-between gap-3 border-t border-line-soft bg-base-soft px-5 py-3",
        className,
      )}
    >
      {children}
    </div>
  );
}

export function SectionLabel({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("label-xs", className)}>{children}</div>;
}
