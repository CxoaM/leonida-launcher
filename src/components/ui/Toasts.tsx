import {
  CircleCheck as CheckCircle2,
  CircleX as XCircle,
  Info,
  TriangleExclamationPoint as AlertTriangle,
  X,
} from "lucide-react";
import { useApp } from "@/state/AppContext";
import { cn } from "@/lib/format";
import type { Tone } from "@/types";

const toneStyles: Record<Tone, { border: string; icon: string; Icon: typeof Info }> = {
  neutral: { border: "border-line", icon: "text-muted", Icon: Info },
  accent: { border: "border-accent/35", icon: "text-accent", Icon: Info },
  ok: { border: "border-[#2b5140]", icon: "text-ok", Icon: CheckCircle2 },
  warn: { border: "border-[#54421f]", icon: "text-warn", Icon: AlertTriangle },
  danger: { border: "border-[#543033]", icon: "text-danger", Icon: XCircle },
};

export function Toasts() {
  const { toasts, dismissToast } = useApp();

  return (
    <div
      aria-live="polite"
      className="pointer-events-none fixed bottom-[42px] right-4 z-[60] flex w-[min(400px,calc(100vw-2rem))] flex-col gap-2"
    >
      {toasts.map((toast) => {
        const style = toneStyles[toast.tone];
        return (
          <div
            key={toast.id}
            className={cn(
              "card animate-fade-up pointer-events-auto flex items-start gap-3 px-3.5 py-3",
              style.border,
            )}
            role="status"
          >
            <style.Icon size={15} className={cn("mt-[1px] shrink-0", style.icon)} />
            <div className="min-w-0 flex-1">
              <p className="text-[12.5px] font-medium leading-[1.4] text-ink">{toast.title}</p>
              {toast.message ? (
                <p className="mt-0.5 text-[11.5px] leading-[1.45] text-muted">{toast.message}</p>
              ) : null}
              {toast.detail ? (
                <details className="mt-1.5">
                  <summary className="cursor-pointer text-[11px] text-faint hover:text-muted">
                    Details
                  </summary>
                  <pre className="log mt-1 max-h-40 overflow-auto whitespace-pre-wrap rounded-[6px] bg-[#0a0d11] p-2 text-[10.5px] text-muted">
                    {toast.detail}
                  </pre>
                </details>
              ) : null}
            </div>
            <button
              type="button"
              onClick={() => dismissToast(toast.id)}
              aria-label="Dismiss"
              className="focus-ring -mr-1 rounded-[5px] p-1 text-faint transition-colors hover:bg-[#1b222b] hover:text-ink"
            >
              <X size={13} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
