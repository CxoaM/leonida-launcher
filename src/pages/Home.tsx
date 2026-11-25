import {
  Boxes,
  Cpu,
  FolderOpen,
  HardDriveDownload,
  Hammer,
  MemoryStick,
  Monitor,
  RefreshCw,
} from "lucide-react";
import type { PageId } from "@/types";
import type { MessageKey } from "@/i18n/dictionary";
import { useApp } from "@/state/AppContext";
import { dateLocale, useI18n } from "@/i18n";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Badge, Dot } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { EmptyState, InlineNote, KeyValue } from "@/components/ui/EmptyState";
import { SkeletonRows } from "@/components/ui/Progress";
import { StageRail, type RailStep } from "@/components/StepList";
import { basename, gib } from "@/lib/format";
import { revealPath } from "@/lib/tauri";
import { compatibilityTone } from "@/lib/ui";

const levelTone = (level: string) => {
  switch (level) {
    case "ERROR":
      return "text-danger";
    case "WARN":
      return "text-warn";
    case "DEBUG":
      return "text-faint";
    default:
      return "text-muted";
  }
};

export function HomePage() {
  const {
    summary,
    summaryLoading,
    refreshSummary,
    compatibility,
    upstreamCheck,
    activity,
    setPage,
    settings,
    launcherRelease,
  } = useApp();
  const { t, resolved } = useI18n();
  const locale = dateLocale(resolved);

  const installation = summary?.installation;
  const installed = installation?.installed ?? false;
  const dependencies = summary?.dependencies ?? [];

  const requiredMissing = dependencies.filter(
    (dep) => dep.kind === "required" && dep.status !== "installed",
  ).length;
  const recommendedMissing = dependencies.filter(
    (dep) => dep.kind !== "required" && dep.status !== "installed",
  ).length;
  const compatibilityStatus = compatibility?.status ?? "unknown";
  const buildStatus = summary?.build.status ?? "idle";

  const needsAttention =
    installed && (compatibilityStatus === "incompatible" || requiredMissing > 0);

  const statusKey: MessageKey = !installed
    ? "home.status.needsSetup"
    : needsAttention
      ? "home.status.attention"
      : "home.status.ready";
  const statusBodyKey: MessageKey = !installed
    ? "home.status.needsSetupBody"
    : needsAttention
      ? "home.status.attentionBody"
      : "home.status.readyBody";
  const statusTone = !installed ? "neutral" : needsAttention ? "warn" : "ok";

  const rail: RailStep[] = [
    {
      id: "installation",
      label: t("home.rail.checkout"),
      status: installed ? "done" : "pending",
      detail: installed ? (installation?.branch ?? t("common.ok")) : t("home.rail.missing"),
    },
    {
      id: "installation",
      label: t("home.rail.game"),
      status: installation?.gameValid ? "done" : "pending",
      detail: installation?.gameValid ? t("common.ok") : t("home.rail.missing"),
    },
    {
      id: "diagnostics",
      label: t("home.rail.deps"),
      status: requiredMissing === 0 ? "done" : "pending",
      detail:
        requiredMissing > 0
          ? t("home.depsMissing", { count: requiredMissing })
          : t("home.depsAllGood"),
    },
    {
      id: "updates",
      label: t("home.rail.compat"),
      status: compatibilityStatus === "compatible" ? "done" : "pending",
      detail: t(`compat.${compatibilityStatus}` as MessageKey),
    },
    {
      id: "build",
      label: t("home.rail.build"),
      status: buildStatus === "succeeded" ? "done" : "pending",
      detail:
        buildStatus === "succeeded"
          ? t("common.ok")
          : t(`build.status.${buildStatus}` as MessageKey),
    },
  ];

  const quickAction = !installed ? (
    <Button variant="primary" icon={<HardDriveDownload size={14} />} onClick={() => setPage("installation")}>
      {t("home.quick.install")}
    </Button>
  ) : (
    <Button variant="primary" icon={<Hammer size={14} />} onClick={() => setPage("build")}>
      {t("home.quick.build")}
    </Button>
  );

  return (
    <div className="animate-fade-in">
      <PageHeader
        title={t("home.heading")}
        subtitle={t("home.subheading")}
        actions={
          <>
            <Button
              size="md"
              icon={<RefreshCw size={14} className={summaryLoading ? "animate-spin" : ""} />}
              onClick={() => void refreshSummary()}
              disabled={summaryLoading}
            >
              {t("common.refresh")}
            </Button>
            {quickAction}
          </>
        }
      />

      <section className="pb-1">
        <div className="flex min-w-0 items-start gap-2.5">
          <span className="mt-[9px] shrink-0">
            <Dot tone={statusTone} />
          </span>
          <div className="min-w-0">
            <h2 className="display text-[22px] leading-[1.2] text-ink">{t(statusKey)}</h2>
            <p className="mt-1 max-w-[64ch] text-[12.5px] leading-[1.6] text-muted">
              {t(statusBodyKey)}
            </p>
          </div>
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-x-5 gap-y-2 pl-0 sm:pl-[22px]">
          <span className="font-mono text-[10.5px] text-faint">
            {t("home.label.launcher")}{" "}
            {summary?.metadata.version ?? launcherRelease?.currentVersion ?? "—"}
          </span>
          {installed ? (
            <>
              <span className="font-mono text-[10.5px] text-faint">
                {installation?.branch ?? "detached"} · {installation?.shortCommit ?? "—"}
              </span>
              <Badge tone={compatibilityTone(compatibilityStatus)}>
                {t(`compat.${compatibilityStatus}` as MessageKey)}
              </Badge>
            </>
          ) : null}
          {upstreamCheck?.updateAvailable ? (
            <Badge tone="accent">{t("home.updatesAvailable", { count: upstreamCheck.behind })}</Badge>
          ) : null}
          <button
            type="button"
            onClick={() => setPage("updates")}
            className="focus-ring rounded-[2px] font-mono text-[10.5px] text-faint transition-colors hover:text-ink"
          >
            {t("home.quick.update")}
          </button>
          <button
            type="button"
            onClick={() => setPage("diagnostics")}
            className="focus-ring rounded-[2px] font-mono text-[10.5px] text-faint transition-colors hover:text-ink"
          >
            {t("home.quick.diagnostics")}
          </button>
        </div>

        <StageRail
          className="mt-5"
          steps={rail}
          onSelect={(id) => setPage(id as PageId)}
        />
      </section>

      <div className="mt-5 grid grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1fr)_336px]">
        <div className="min-w-0 space-y-4">
          <Card>
            <CardHeader
              title={t("home.card.installation")}
              icon={<Boxes size={13} />}
              action={
                <Button size="sm" variant="ghost" onClick={() => setPage("installation")}>
                  {t("common.details")}
                </Button>
              }
            />
            <CardBody className="pt-3">
              {!summary ? (
                <SkeletonRows rows={6} />
              ) : !installed ? (
                <EmptyState
                  compact
                  icon={<HardDriveDownload size={15} />}
                  title={t("install.heading")}
                  body={t("home.status.needsSetupBody")}
                  action={
                    <Button size="sm" variant="primary" onClick={() => setPage("installation")}>
                      {t("home.quick.install")}
                    </Button>
                  }
                />
              ) : (
                <div className="divide-y divide-line-soft">
                  <KeyValue
                    label={t("home.label.branch")}
                    value={installation?.branch ?? "detached"}
                    mono
                  />
                  <KeyValue
                    label={t("home.label.commit")}
                    value={installation?.shortCommit ?? "—"}
                    mono
                    title={installation?.commit ?? undefined}
                  />
                  <KeyValue
                    label={t("home.label.worktree")}
                    value={
                      (installation?.dirtyFiles ?? 0) > 0
                        ? t("home.dirty", { count: installation?.dirtyFiles ?? 0 })
                        : t("home.clean")
                    }
                  />
                  <KeyValue
                    label={t("home.label.submodules")}
                    value={
                      installation?.submodulesReady
                        ? t("home.submodulesReady", { count: installation?.submodules.length ?? 0 })
                        : t("home.submodulesMissing")
                    }
                  />
                  <KeyValue
                    label={t("home.label.game")}
                    value={
                      installation?.gamePath
                        ? basename(installation.gamePath)
                        : t("home.noGame")
                    }
                    title={installation?.gamePath ?? undefined}
                  />
                  <KeyValue
                    label={t("home.label.output")}
                    value={basename(settings?.outPath)}
                    title={settings?.outPath}
                    mono
                    action={
                      settings?.outPath ? (
                        <button
                          type="button"
                          aria-label={t("common.open")}
                          onClick={() => void revealPath(settings.outPath)}
                          className="focus-ring rounded-[3px] p-[2px] text-faint transition-colors hover:text-ink"
                        >
                          <FolderOpen size={12} />
                        </button>
                      ) : null
                    }
                  />
                  <KeyValue
                    label={t("home.label.dependencies")}
                    value={
                      requiredMissing > 0
                        ? t("home.depsMissing", { count: requiredMissing })
                        : recommendedMissing > 0
                          ? t("home.depsRecommended", { count: recommendedMissing })
                          : t("home.depsAllGood")
                    }
                  />
                </div>
              )}
              {installed && installation?.restorePoint ? (
                <InlineNote tone="neutral" className="mt-3">
                  {t("updates.restore.hint")}
                </InlineNote>
              ) : null}
            </CardBody>
          </Card>

          <Card>
            <CardHeader
              title={t("home.card.activity")}
              action={
                <span className="font-mono text-[10.5px] text-faint">
                  {activity.length === 0 ? "—" : `${activity.length}`}
                </span>
              }
            />
            <CardBody className="pt-2">
              {activity.length === 0 ? (
                <EmptyState compact title={t("home.activity.empty")} />
              ) : (
                <ul className="divide-y divide-line-soft">
                  {activity
                    .slice(-7)
                    .reverse()
                    .map((line, index) => (
                      <li key={`${line.timestamp}-${index}`} className="flex items-start gap-3 py-[7px]">
                        <span className="mt-[2px] shrink-0 font-mono text-[10px] text-faint">
                          {new Date(line.timestamp).toLocaleTimeString(locale, {
                            hour: "2-digit",
                            minute: "2-digit",
                            second: "2-digit",
                          })}
                        </span>
                        <span
                          className={`mt-[2px] w-9 shrink-0 font-mono text-[10px] ${levelTone(line.level)}`}
                        >
                          {line.level}
                        </span>
                        <span
                          className="min-w-0 flex-1 truncate text-[12px] text-muted"
                          title={line.message}
                        >
                          {line.message}
                        </span>
                      </li>
                    ))}
                </ul>
              )}
            </CardBody>
          </Card>
        </div>

        <div className="min-w-0 space-y-4">
          <Card>
            <CardHeader title={t("home.card.environment")} icon={<Monitor size={13} />} />
            <CardBody className="pt-3">
              {!summary ? (
                <SkeletonRows rows={5} />
              ) : (
                <div className="divide-y divide-line-soft">
                  <KeyValue
                    label={summary.environment.osName || "Windows"}
                    value={`${summary.environment.osVersion} · ${summary.environment.osBuild}`}
                    mono
                  />
                  <KeyValue
                    label={
                      <span className="flex items-center gap-1.5">
                        <Cpu size={11} />
                        {t("home.label.cores", { count: summary.environment.cpuCores })}
                      </span>
                    }
                    value={summary.environment.cpuName}
                    title={summary.environment.cpuName}
                  />
                  <KeyValue
                    label={
                      <span className="flex items-center gap-1.5">
                        <MemoryStick size={11} />
                        RAM
                      </span>
                    }
                    value={`${Math.round(summary.environment.memoryAvailableMb / 1024)} / ${Math.round(summary.environment.memoryTotalMb / 1024)} GB`}
                    mono
                  />
                  <KeyValue
                    label={t("home.label.longPaths")}
                    value={
                      summary.environment.longPaths === null
                        ? t("common.unknown")
                        : summary.environment.longPaths
                          ? t("common.ok")
                          : t("common.warning")
                    }
                  />
                  <KeyValue
                    label={t("home.label.freeSpace")}
                    value={
                      summary.disks.length === 0
                        ? "—"
                        : summary.disks
                            .map((disk) => `${gib(disk.freeBytes)} ${disk.volume}`)
                            .join(" · ")
                    }
                    mono
                  />
                </div>
              )}
            </CardBody>
          </Card>

          <div className="border-l-2 border-line pl-3.5">
            <h3 className="text-[12.5px] font-medium text-ink">{t("home.playUnavailable.title")}</h3>
            <p className="mt-1 text-[11.5px] leading-[1.6] text-muted">
              {t("home.playUnavailable.body")}
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1.5">
              <button
                type="button"
                onClick={() => setPage("updates")}
                className="focus-ring rounded-[2px] text-[11.5px] text-muted transition-colors hover:text-ink"
              >
                {t("nav.updates")}
              </button>
              <button
                type="button"
                onClick={() => setPage("diagnostics")}
                className="focus-ring rounded-[2px] text-[11.5px] text-muted transition-colors hover:text-ink"
              >
                {t("nav.diagnostics")}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
