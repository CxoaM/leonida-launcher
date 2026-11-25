import { cn } from "@/lib/format";
import { useT } from "@/i18n";
import type { MessageKey } from "@/i18n/dictionary";

export type StepStatus = "pending" | "running" | "done" | "error";

export interface StepDescriptor {
  id: string;
  label: string;
  status: StepStatus;
  detail?: string;
}

const tickTone: Record<StepStatus, string> = {
  pending: "bg-line",
  running: "bg-accent",
  done: "bg-ok",
  error: "bg-danger",
};

const labelTone: Record<StepStatus, string> = {
  pending: "text-muted",
  running: "text-ink",
  done: "text-muted",
  error: "text-danger",
};

const statusKey: Record<StepStatus, MessageKey> = {
  pending: "install.step.pending",
  running: "install.step.running",
  done: "install.step.done",
  error: "install.step.error",
};

const statusTone: Record<StepStatus, string> = {
  pending: "text-faint",
  running: "text-accent",
  done: "text-ok",
  error: "text-danger",
};

export function StepList({ steps, className }: { steps: StepDescriptor[]; className?: string }) {
  const t = useT();
  return (
    <ol className={cn("space-y-0", className)}>
      {steps.map((step, index) => {
        const last = index === steps.length - 1;
        return (
          <li key={step.id} className="relative flex gap-4 pb-3.5 last:pb-0">
            <span className="relative flex w-3 shrink-0 justify-center">
              {!last ? (
                <span
                  aria-hidden
                  className={cn(
                    "absolute top-3 bottom-0 w-px",
                    step.status === "done" ? "bg-ok/35" : "bg-line-soft",
                  )}
                />
              ) : null}
              <span
                aria-hidden
                className={cn("mt-[7px] h-[2px] w-3 shrink-0", tickTone[step.status])}
              />
            </span>
            <div className="min-w-0 flex-1">
              <div className="flex items-baseline justify-between gap-3">
                <span className={cn("text-[12.5px]", labelTone[step.status])}>{step.label}</span>
                <span className={cn("shrink-0 font-mono text-[10px]", statusTone[step.status])}>
                  {t(statusKey[step.status])}
                </span>
              </div>
              {step.detail ? (
                <p className="mt-0.5 text-[11.5px] leading-[1.5] text-faint">{step.detail}</p>
              ) : null}
            </div>
          </li>
        );
      })}
    </ol>
  );
}

export interface RailStep {
  id: string;
  label: string;
  status: StepStatus;
  detail?: string;
}

/** Horizontal measured rail used as the workspace signature on the overview. */
export function StageRail({
  steps,
  className,
  onSelect,
}: {
  steps: RailStep[];
  className?: string;
  onSelect?: (id: string) => void;
}) {
  const t = useT();
  return (
    <ol className={cn("grid grid-cols-5 border-t border-line-soft", className)}>
      {steps.map((step, index) => (
        <li key={step.id} className="relative min-w-0">
          <button
            type="button"
            onClick={onSelect ? () => onSelect(step.id) : undefined}
            disabled={!onSelect}
            className={cn(
              "relative block w-full pr-3 pt-3 text-left",
              onSelect && "focus-ring rounded-[2px] hover:bg-raised/40",
            )}
          >
            <span
              aria-hidden
              className={cn("absolute -top-px left-0 h-2 w-[2px]", tickTone[step.status])}
            />
            <span className="font-mono text-[10px] text-faint">
              {String(index + 1).padStart(2, "0")}
            </span>
            <span className={cn("mt-1 block truncate text-[12px]", labelTone[step.status])}>
              {step.label}
            </span>
            <span className={cn("mt-0.5 block truncate text-[10.5px]", step.status === "error" ? "text-danger" : step.status === "running" ? "text-accent" : "text-faint")}>
              {step.detail ?? t(statusKey[step.status])}
            </span>
          </button>
        </li>
      ))}
    </ol>
  );
}
