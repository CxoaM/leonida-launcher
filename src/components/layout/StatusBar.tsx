import { GitBranch, LoaderCircle } from "lucide-react";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";
import type { MessageKey } from "@/i18n/dictionary";
import { cn } from "@/lib/format";
import { compatibilityTone, isJobRunning, percentLabel } from "@/lib/ui";
import { Dot } from "@/components/ui/Badge";

export function StatusBar() {
  const {
    summary,
    installJob,
    buildJob,
    updateProgress,
    compatibility,
    metadata,
    setPage,
  } = useApp();
  const t = useT();

  const installation = summary?.installation;
  const activeJob = isJobRunning(installJob.status)
    ? installJob
    : isJobRunning(buildJob.status)
      ? buildJob
      : null;

  const compatibilityStatus = compatibility?.status ?? "unknown";
  const tone = compatibilityTone(compatibilityStatus);

  return (
    <footer className="flex h-8 shrink-0 items-center justify-between gap-4 border-t border-line-soft bg-base-soft px-3 text-[11px] text-faint">
      <button
        type="button"
        onClick={() => setPage("installation")}
        className="focus-ring flex min-w-0 items-center gap-2 rounded-[4px] px-1 py-0.5 transition-colors hover:text-muted"
      >
        <GitBranch size={12} className="shrink-0" />
        <span className="truncate font-mono text-[10.5px]">
          {installation?.installed
            ? `${installation.branch ?? "detached"} @ ${installation.shortCommit ?? ""}`
            : t("status.noRepository")}
        </span>
        {installation?.installed && installation.dirtyFiles > 0 ? (
          <span className="shrink-0 text-warn">· {t("home.dirty", { count: installation.dirtyFiles })}</span>
        ) : null}
      </button>

      <div className="flex min-w-0 flex-1 justify-center">
        {activeJob ? (
          <span className="flex min-w-0 items-center gap-1.5 text-muted">
            <LoaderCircle size={11} className="shrink-0 animate-spin text-accent" />
            <span className="truncate">
              {activeJob.currentStage
                ? `${activeJob.kind}: ${activeJob.currentStage}`
                : `${activeJob.kind}: ${t("status.busy")}`}
            </span>
            {percentLabel(activeJob.percent) ? (
              <span className="font-mono text-[10.5px] text-accent">
                {percentLabel(activeJob.percent)}
              </span>
            ) : null}
          </span>
        ) : updateProgress ? (
          <span className="truncate text-muted">
            {updateProgress.label} · {updateProgress.detail}
          </span>
        ) : (
          <span className="truncate">{t("status.idle")}</span>
        )}
      </div>

      <div className="flex shrink-0 items-center gap-3">
        <span
          className={cn(
            "flex items-center gap-1.5",
            tone === "danger" ? "text-danger" : tone === "warn" ? "text-warn" : "text-muted",
          )}
        >
          <Dot tone={tone} />
          {t(`compat.${compatibilityStatus}` as MessageKey)}
        </span>
        <span className="font-mono text-[10.5px]">
          {metadata?.version ?? ""} · {metadata?.channel ?? ""}
        </span>
      </div>
    </footer>
  );
}
