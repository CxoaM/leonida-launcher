import {
  Activity,
  Hammer,
  HardDriveDownload,
  Info,
  LayoutDashboard,
  RefreshCw,
  Settings,
} from "lucide-react";
import type { ComponentType } from "react";
import type { PageId } from "@/types";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";
import { cn } from "@/lib/format";
import { compatibilityTone } from "@/lib/ui";
import type { MessageKey } from "@/i18n/dictionary";
import { Badge, Dot } from "@/components/ui/Badge";

interface NavEntry {
  page: PageId;
  labelKey: MessageKey;
  Icon: ComponentType<{ size?: number | string; className?: string; strokeWidth?: number }>;
  badge?: string;
}

export function Sidebar() {
  const { page, setPage, summary, compatibility, upstreamCheck, installJob, buildJob, metadata } = useApp();
  const t = useT();

  const running = installJob.status === "running" || buildJob.status === "running";

  const workspace: NavEntry[] = [
    { page: "home", labelKey: "nav.home", Icon: LayoutDashboard },
    { page: "installation", labelKey: "nav.installation", Icon: HardDriveDownload },
    {
      page: "updates",
      labelKey: "nav.updates",
      Icon: RefreshCw,
      badge: upstreamCheck?.updateAvailable ? String(upstreamCheck.behind) : undefined,
    },
    { page: "build", labelKey: "nav.build", Icon: Hammer },
  ];

  const system: NavEntry[] = [
    { page: "diagnostics", labelKey: "nav.diagnostics", Icon: Activity },
    { page: "settings", labelKey: "nav.settings", Icon: Settings },
    { page: "about", labelKey: "nav.about", Icon: Info },
  ];

  const renderGroup = (titleKey: MessageKey, entries: NavEntry[]) => (
    <div className="mb-1">
      <div className="label-xs px-2 pb-1.5 pt-3">{t(titleKey)}</div>
      <ul className="space-y-[2px]">
        {entries.map((entry) => (
          <li key={entry.page}>
            <button
              type="button"
              onClick={() => setPage(entry.page)}
              data-active={page === entry.page}
              aria-current={page === entry.page ? "page" : undefined}
              className="nav-item focus-ring w-full"
            >
              <entry.Icon
                size={15}
                strokeWidth={1.85}
                className={cn("shrink-0", page === entry.page ? "text-accent" : "text-faint")}
              />
              <span className="flex-1 truncate text-left">{t(entry.labelKey)}</span>
              {entry.badge ? (
                <span className="rounded-full border border-accent-line bg-accent-soft px-1.5 font-mono text-[10px] leading-[15px] text-accent">
                  {entry.badge}
                </span>
              ) : null}
              {entry.page === "build" && running ? <Dot tone="accent" pulse /> : null}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );

  const compatibilityStatus = compatibility?.status ?? "unknown";

  return (
    <aside className="flex w-[226px] shrink-0 flex-col border-r border-line-soft bg-[#0c0f13]">
      <nav className="scroll-thin min-h-0 flex-1 overflow-y-auto px-2.5 pb-3">
        {renderGroup("nav.group.workspace", workspace)}
        {renderGroup("nav.group.system", system)}
      </nav>

      <div className="border-t border-line-soft p-2.5">
        <div className="rounded-[9px] border border-line-soft bg-[#11161c] px-2.5 py-2">
          <div className="flex items-center justify-between gap-2">
            <span className="truncate text-[10.5px] uppercase tracking-[0.05em] text-faint">
              {t("status.compatibility")}
            </span>
            <span className="shrink-0 font-mono text-[10.5px] text-faint">{metadata?.version ?? ""}</span>
          </div>
          <div className="mt-1.5 flex items-center justify-between gap-2">
            <Badge tone={compatibilityTone(compatibilityStatus)}>
              {t(`compat.${compatibilityStatus}` as MessageKey)}
            </Badge>
            <span className="min-w-0 truncate text-right text-[10.5px] text-faint">
              {summary?.installation.installed
                ? `${summary.installation.branch ?? "detached"} · ${summary.installation.shortCommit ?? ""}`
                : t("status.noRepository")}
            </span>
          </div>
        </div>
      </div>
    </aside>
  );
}
