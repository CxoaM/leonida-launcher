import { useEffect, useRef } from "react";
import { Copy, Terminal } from "lucide-react";
import { useT } from "@/i18n";
import { writeClipboard } from "@/lib/tauri";
import { cn } from "@/lib/format";
import { IconButton } from "./ui/Button";
import { EmptyState } from "./ui/EmptyState";

function classify(line: string): string {
  const lower = line.toLowerCase();
  if (lower.includes("error") || lower.includes("fatal") || lower.includes("failed")) {
    return "log-line-error";
  }
  if (lower.includes("warn")) return "log-line-warn";
  if (lower.includes("done") || lower.includes("success") || lower.includes("complete")) {
    return "log-line-ok";
  }
  if (line.startsWith("$") || line.startsWith(">")) return "log-line-dim";
  return "";
}

export function LogConsole({
  lines,
  title,
  emptyText,
  maxHeight = 280,
  className,
  copyable = true,
}: {
  lines: string[];
  title?: string;
  emptyText?: string;
  maxHeight?: number;
  className?: string;
  copyable?: boolean;
}) {
  const t = useT();
  const scroller = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);

  useEffect(() => {
    const element = scroller.current;
    if (!element || !pinned.current) return;
    element.scrollTop = element.scrollHeight;
  }, [lines]);

  return (
    <div className={cn("overflow-hidden rounded-[10px] border border-line-soft bg-[#090c10]", className)}>
      <div className="flex items-center justify-between gap-2 border-b border-line-soft bg-[#0d1116] px-3 py-1.5">
        <span className="flex items-center gap-1.5 text-[10.5px] font-medium uppercase tracking-[0.05em] text-faint">
          <Terminal size={11} />
          {title ?? t("build.log.title")}
        </span>
        {copyable && lines.length > 0 ? (
          <IconButton
            label={t("common.copy")}
            onClick={() => void writeClipboard(lines.join("\n"))}
            className="h-6 w-6"
          >
            <Copy size={12} />
          </IconButton>
        ) : null}
      </div>
      <div
        ref={scroller}
        onScroll={(event) => {
          const element = event.currentTarget;
          pinned.current = element.scrollHeight - element.scrollTop - element.clientHeight < 40;
        }}
        className="scroll-thin overflow-auto p-2.5"
        style={{ maxHeight }}
      >
        {lines.length === 0 ? (
          <EmptyState compact title={emptyText ?? t("build.log.empty")} />
        ) : (
          <div className="log">
            {lines.map((line, index) => (
              <div key={index} className={classify(line)}>
                {line}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
