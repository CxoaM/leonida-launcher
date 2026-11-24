import {
  ArrowRight,
  Boxes,
  CircleCheck as CheckCircle2,
  Cpu,
  FolderOpen,
  Gauge,
  GitBranch,
  HardDriveDownload,
  Hammer,
  Info,
  MemoryStick,
  Monitor,
  PackageCheck,
  RefreshCw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import type { PageId } from "@/types";
import type { MessageKey } from "@/i18n/dictionary";
import { useApp } from "@/state/AppContext";
import { dateLocale, useI18n } from "@/i18n";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Badge, Dot, Pill } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { EmptyState, InlineNote, KeyValue } from "@/components/ui/EmptyState";
import { SkeletonRows } from "@/components/ui/Progress";
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

  const steps: Array<{ key: MessageKey; done: boolean; page: PageId }> = [
    { key: "home.nextStep.install", done: installed, page: "installation" },
    { key: "home.nextStep.game", done: installation?.gameValid ?? false, page: "installation" },
    { key: "home.nextStep.deps", done: requiredMissing === 0, page: "diagnostics" },
    { key: "home.nextStep.compat", done: compatibilityStatus === "compatible", page: "updates" },
    { key: "home.nextStep.build", done: summary?.build.status === "succeeded", page: "build" },
  ];
  const nextStep = steps.find((step) => !step.done);

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

      <div className="space-y-4">
        <Card className="relative">
          <div
            aria-hidden
            className="pointer-events-none absolute inset-0 bg-[radial-gradient(130%_150%_at_0%_0%,rgba(142,169,255,0.10),transparent_58%)]"
          />
          <CardBody className="relative px-5 py-4.5">
            <div className="flex flex-wrap items-start justify-between gap-4">
              <div className="min-w-0">
                <span className="flex items-center gap-2">
                  <Dot tone={statusTone} pulse={statusTone === "ok"} />
                  <span className="label-xs">{t("app.tagline")}</span>
                </span>
                <h2 className="mt-2 text-[17px] font-semibold tracking-[-0.015em] text-ink">
                  {t(statusKey)}
                </h2>
                <p className="mt-1 max-w-[68ch] text-[12.5px] leading-[1.55] text-muted">
                  {t(statusBodyKey)}
                </p>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button size="sm" icon={<RefreshCw size={13} />} onClick={() => setPage("updates")}>
                  {t("home.quick.update")}
                </Button>
                <Button size="sm" variant="ghost" icon={<Gauge size={13} />} onClick={() => setPage("diagnostics")}>
                  {t("home.quick.diagnostics")}
                </Button>
              </div>
            </div>

            <div className="mt-4 flex flex-wrap items-center gap-1.5">
              <Pill>
                {t("home.label.launcher")} {summary?.metadata.version ?? launcherRelease?.currentVersion ?? "—"}
              </Pill>
              {installed ? (
                <>
                  <Pill>
                    <GitBranch size={10} className="mr-1" />
                    {installation?.branch ?? "detached"} · {installation?.shortCommit ?? "—"}
                  </Pill>
                  <Badge tone={compatibilityTone(compatibilityStatus)}>
                    {t(`compat.${compatibilityStatus}` as MessageKey)}
                  </Badge>
                </>
              ) : null}
              {upstreamCheck?.updateAvailable ? (
                <Badge tone="accent">{t("home.updatesAvailable", { count: upstreamCheck.behind })}</Badge>
              ) : null}
            </div>
          </CardBody>
        </Card>

        <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
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
            <CardBody className="pt-2">
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
                <div className="divide-y divide-[#1d242d]">
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
                          className="focus-ring rounded-[4px] p-[2px] text-faint transition-colors hover:text-ink"
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
            </CardBody>
          </Card>

          <div className="space-y-4">
            <Card>
              <CardHeader title={t("home.card.environment")} icon={<Monitor size={13} />} />
              <CardBody className="pt-2">
                {!summary ? (
                  <SkeletonRows rows={5} />
                ) : (
                  <div className="divide-y divide-[#1d242d]">
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
                      label={<span className="flex items-center gap-1.5"><MemoryStick size={11} />RAM</span>}
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

            <Card>
              <CardHeader title={t("home.card.nextSteps")} icon={<Sparkles size={13} />} />
              <CardBody className="space-y-1 pt-2">
                {steps.map((step) => (
                  <button
                    key={step.key}
                    type="button"
                    onClick={() => setPage(step.page)}
                    className="group flex w-full items-center gap-2.5 rounded-[7px] px-1.5 py-[7px] text-left transition-colors hover:bg-[#171d25]"
                  >
                    {step.done ? (
                      <CheckCircle2 size={14} className="shrink-0 text-ok" />
                    ) : (
                      <span className="h-[13px] w-[13px] shrink-0 rounded-full border border-[#39424f]" />
                    )}
                    <span
                      className={
                        step.done
                          ? "flex-1 text-[12.5px] text-faint line-through decoration-[#39424f]"
                          : "flex-1 text-[12.5px] text-ink"
                      }
                    >
                      {t(step.key)}
                    </span>
                    {!step.done ? (
                      <ArrowRight
                        size={12}
                        className="shrink-0 text-faint opacity-0 transition-opacity group-hover:opacity-100"
                      />
                    ) : null}
                  </button>
                ))}
                {!nextStep ? (
                  <p className="px-1.5 pt-1 text-[11.5px] text-faint">
                    {t("home.nextStep.done")}
                  </p>
                ) : null}
              </CardBody>
            </Card>
          </div>
        </div>

        <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1.05fr_1fr]">
          <Card>
            <CardHeader
              title={t("home.card.activity")}
              icon={<Sparkles size={13} />}
              action={
                <Pill>{`${activity.length}`}</Pill>
              }
            />
            <CardBody className="pt-1.5">
              {activity.length === 0 ? (
                <EmptyState compact title={t("home.activity.empty")} />
              ) : (
                <ul className="divide-y divide-[#1d242d]">
                  {activity
                    .slice(-7)
                    .reverse()
                    .map((line, index) => (
                      <li key={`${line.timestamp}-${index}`} className="flex items-start gap-2.5 py-[6px]">
                        <span className="mt-[3px] font-mono text-[10px] text-faint">
                          {new Date(line.timestamp).toLocaleTimeString(locale, {
                            hour: "2-digit",
                            minute: "2-digit",
                            second: "2-digit",
                          })}
                        </span>
                        <span className={`mt-[2px] text-[10px] font-medium uppercase ${levelTone(line.level)}`}>
                          {line.level}
                        </span>
                        <span className="min-w-0 flex-1 truncate text-[11.5px] text-muted" title={line.message}>
                          {line.message}
                        </span>
                      </li>
                    ))}
                </ul>
              )}
            </CardBody>
          </Card>

          <Card>
            <CardBody className="space-y-3">
              <div className="flex items-start gap-2.5">
                <span className="mt-[1px] flex h-6 w-6 shrink-0 items-center justify-center rounded-[7px] border border-line-soft bg-[#171d25] text-muted">
                  <Info size={13} />
                </span>
                <div>
                  <h3 className="text-[12.5px] font-semibold text-ink">{t("home.playUnavailable.title")}</h3>
                  <p className="mt-1 text-[11.5px] leading-[1.55] text-muted">
                    {t("home.playUnavailable.body")}
                  </p>
                </div>
              </div>
              <div className="divider" />
              <div className="grid grid-cols-2 gap-2">
                <Button size="sm" icon={<ShieldCheck size={13} />} onClick={() => setPage("updates")}>
                  {t("nav.updates")}
                </Button>
                <Button size="sm" icon={<PackageCheck size={13} />} onClick={() => setPage("diagnostics")}>
                  {t("nav.diagnostics")}
                </Button>
              </div>
              {installation?.restorePoint ? (
                <InlineNote tone="warn">
                  {t("updates.restore.hint")}
                </InlineNote>
              ) : null}
            </CardBody>
          </Card>
        </div>
      </div>
    </div>
  );
}
