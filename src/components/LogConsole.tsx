import { useEffect, useRef } from "react";
import { Copy } from "lucide-react";
import { useT } from "@/i18n";
import { writeClipboard } from "@/lib/tauri";
import { cn } from "@/lib/format";
import { IconButton } from "./ui/Button";

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
    <div className={cn("overflow-hidden rounded-[8px] border border-line-soft bg-console", className)}>
      <div className="flex items-center justify-between gap-2 border-b border-line-soft px-3 py-1.5">
        <span className="truncate text-[11px] font-medium text-muted">
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
        className="scroll-thin overflow-auto p-3"
        style={{ maxHeight }}
      >
        {lines.length === 0 ? (
          <p className="py-4 text-[11.5px] leading-[1.6] text-faint">
            {emptyText ?? t("build.log.empty")}
          </p>
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
