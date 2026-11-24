import { Check, LoaderCircle, X } from "lucide-react";
import { cn } from "@/lib/format";

export type StepStatus = "pending" | "running" | "done" | "error";

export interface StepDescriptor {
  id: string;
  label: string;
  status: StepStatus;
  detail?: string;
}

export function StepList({ steps, className }: { steps: StepDescriptor[]; className?: string }) {
  return (
    <ol className={cn("space-y-0", className)}>
      {steps.map((step, index) => {
        const last = index === steps.length - 1;
        return (
          <li key={step.id} className="relative flex gap-3 pb-3 last:pb-0">
            {!last ? (
              <span
                aria-hidden
                className={cn(
                  "absolute left-[10px] top-[22px] h-[calc(100%-14px)] w-px",
                  step.status === "done" ? "bg-[#2e5a45]" : "bg-line",
                )}
              />
            ) : null}
            <span
              className={cn(
                "z-[1] mt-[1px] flex h-[21px] w-[21px] shrink-0 items-center justify-center rounded-full border text-[10px]",
                step.status === "done" && "border-[#2f6149] bg-[#16251d] text-ok",
                step.status === "running" && "border-accent/45 bg-accent-soft text-accent",
                step.status === "error" && "border-[#543033] bg-[#241618] text-danger",
                step.status === "pending" && "border-line bg-[#12171e] text-faint",
              )}
            >
              {step.status === "done" ? (
                <Check size={11} strokeWidth={2.6} />
              ) : step.status === "running" ? (
                <LoaderCircle size={11} className="animate-spin" />
              ) : step.status === "error" ? (
                <X size={11} strokeWidth={2.6} />
              ) : (
                <span className="h-[3px] w-[3px] rounded-full bg-current" />
              )}
            </span>
            <div className="min-w-0 flex-1 pt-[1px]">
              <div className="flex items-baseline justify-between gap-3">
                <span
                  className={cn(
                    "text-[12.5px] font-medium",
                    step.status === "pending" ? "text-muted" : "text-ink",
                  )}
                >
                  {step.label}
                </span>
              </div>
              {step.detail ? (
                <p className="mt-0.5 text-[11.5px] leading-[1.45] text-faint">{step.detail}</p>
              ) : null}
            </div>
          </li>
        );
      })}
    </ol>
  );
}
