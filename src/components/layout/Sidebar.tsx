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
  const { page, setPage, summary, compatibility, upstreamCheck, installJob, buildJob } = useApp();
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
    <div>
      <div className="px-2 pb-1.5 pt-4 text-[11px] text-faint">{t(titleKey)}</div>
      <ul className="space-y-0.5">
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
                size={16}
                strokeWidth={1.7}
                className={page === entry.page ? "shrink-0 text-ink" : "shrink-0 text-faint"}
              />
              <span className="flex-1 truncate text-left">{t(entry.labelKey)}</span>
              {entry.badge ? (
                <span className="shrink-0 font-mono text-[10.5px] text-accent">{entry.badge}</span>
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
    <aside className="flex w-[240px] shrink-0 flex-col border-r border-line-soft bg-base-soft">
      <nav className="scroll-thin min-h-0 flex-1 overflow-y-auto px-3 pb-3 pt-1">
        {renderGroup("nav.group.workspace", workspace)}
        {renderGroup("nav.group.system", system)}
      </nav>

      <div className="border-t border-line-soft px-4 py-3">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-faint">{t("status.compatibility")}</span>
          <Badge tone={compatibilityTone(compatibilityStatus)}>
            {t(`compat.${compatibilityStatus}` as MessageKey)}
          </Badge>
        </div>
        <div className="mt-1 truncate font-mono text-[10.5px] text-faint">
          {summary?.installation.installed
            ? `${summary.installation.branch ?? "detached"} @ ${summary.installation.shortCommit ?? ""}`
            : t("status.noRepository")}
        </div>
      </div>
    </aside>
  );
}
