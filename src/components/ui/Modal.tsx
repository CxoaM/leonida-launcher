import { useEffect, type ReactNode } from "react";
import { X } from "lucide-react";
import { cn } from "@/lib/format";
import { useT } from "@/i18n";
import { Button } from "./Button";

export function Modal({
  open,
  title,
  description,
  children,
  onClose,
  footer,
  tone = "neutral",
  width = "max-w-lg",
}: {
  open: boolean;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
  tone?: "neutral" | "danger" | "warn";
  width?: string;
}) {
  const t = useT();

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-6">
      <div
        className="absolute inset-0 bg-[#05070a]/72 backdrop-blur-[2px] animate-fade-in"
        onClick={onClose}
        aria-hidden
      />
      <div
        role="dialog"
        aria-modal="true"
        className={cn(
          "card animate-scale-in relative w-full overflow-hidden",
          width,
          tone === "danger" && "border-[#4a2b2d]",
          tone === "warn" && "border-[#4a3a1d]",
        )}
      >
        <div className="flex items-start justify-between gap-4 px-5 pt-4">
          <div className="min-w-0">
            <h2
              className={cn(
                "text-[14px] font-semibold tracking-[-0.01em]",
                tone === "danger" ? "text-danger" : tone === "warn" ? "text-warn" : "text-ink",
              )}
            >
              {title}
            </h2>
            {description ? (
              <p className="mt-1 text-[12.5px] leading-[1.5] text-muted">{description}</p>
            ) : null}
          </div>
          <button
            type="button"
            aria-label={t("common.close")}
            onClick={onClose}
            className="focus-ring -mr-1 -mt-1 rounded-[6px] p-1 text-faint transition-colors hover:bg-[#1b222b] hover:text-ink"
          >
            <X size={15} />
          </button>
        </div>
        {children ? <div className="px-5 py-4">{children}</div> : <div className="h-2" />}
        <div className="flex items-center justify-end gap-2 border-t border-line-soft bg-[#12171e] px-5 py-3">
          {footer ?? <Button onClick={onClose}>{t("common.close")}</Button>}
        </div>
      </div>
    </div>
  );
}
