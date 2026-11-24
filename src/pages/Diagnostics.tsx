import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import {
  Boxes,
  Copy,
  Download,
  ExternalLink,
  FileDown,
  FolderOpen,
  Gamepad2,
  Globe,
  HardDrive,
  LoaderCircle,
  Monitor,
  RefreshCw,
  ScrollText,
  Terminal,
  Trash,
  TriangleExclamationPoint,
  Wrench,
} from "lucide-react";
import type {
  DependencyKind,
  DependencyReport,
  DependencyStatus,
  DiagnosticItem,
  DiagnosticReport,
  LogFileInfo,
} from "@/types";
import type { MessageKey } from "@/i18n/dictionary";
import { useApp } from "@/state/AppContext";
import { dateLocale, useI18n } from "@/i18n";
import { api } from "@/services/ipc";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardBody, CardHeader, SectionLabel } from "@/components/ui/Card";
import { Badge, Dot, Pill } from "@/components/ui/Badge";
import { Button, IconButton } from "@/components/ui/Button";
import { EmptyState, InlineNote, KeyValue } from "@/components/ui/EmptyState";
import { SkeletonRows } from "@/components/ui/Progress";
import { cn, bytes, gib, localTime } from "@/lib/format";
import { dependencyTone, groupBy, humanizeGroup, isJobRunning, percentLabel } from "@/lib/ui";
import { openExternal, revealPath, writeClipboard } from "@/lib/tauri";

const itemTone: Record<DiagnosticItem["status"], MessageKey> = {
  ok: "diag.item.ok",
  warning: "diag.item.warning",
  error: "diag.item.error",
  info: "diag.item.info",
};

const itemToneColor: Record<DiagnosticItem["status"], "ok" | "warn" | "danger" | "neutral"> = {
  ok: "ok",
  warning: "warn",
  error: "danger",
  info: "neutral",
};

const dependencyKindLabel: Record<DependencyKind, MessageKey> = {
  required: "diag.dep.kind.required",
  recommended: "diag.dep.kind.recommended",
  optional: "diag.dep.kind.optional",
};

const dependencyStatusLabel: Record<DependencyStatus, MessageKey> = {
  installed: "diag.dep.status.installed",
  missing: "diag.dep.status.missing",
  outdated: "diag.dep.status.outdated",
  invalid: "diag.dep.status.invalid",
};

const groupLabel: Record<string, MessageKey> = {
  system: "diag.groups.system",
  toolchain: "diag.groups.toolchain",
  leonida: "diag.groups.leonida",
  game: "diag.groups.gameFiles",
  gamefiles: "diag.groups.gameFiles",
  storage: "diag.groups.storage",
  network: "diag.groups.network",
  dependencies: "diag.groups.dependencies",
};

function groupTitle(group: string): MessageKey | null {
  const normalized = group.toLowerCase().replace(/[^a-z]/g, "");
  return groupLabel[normalized] ?? null;
}

const itemLabel: Record<string, MessageKey> = {
  os: "diag.items.os",
  cpu: "diag.items.cpu",
  memory: "diag.items.memory",
  dependencies: "diag.items.dependencies",
  repository: "diag.items.repository",
  submodules: "diag.items.submodules",
  compatibility: "diag.items.compatibility",
  game: "diag.items.game",
  "disk-work": "diag.items.diskWork",
  network: "diag.items.network",
};

function groupIcon(group: string): ReactNode {
  switch (group.toLowerCase().replace(/[^a-z]/g, "")) {
    case "system":
      return <Monitor size={13} />;
    case "toolchain":
      return <Wrench size={13} />;
    case "leonida":
      return <Boxes size={13} />;
    case "game":
    case "gamefiles":
      return <Gamepad2 size={13} />;
    case "storage":
      return <HardDrive size={13} />;
    case "network":
      return <Globe size={13} />;
    default:
      return <ScrollText size={13} />;
  }
}

export function DiagnosticsPage() {
  const {
    settings,
    paths,
    reportError,
    pushToast,
    dependencyJob,
    setDependencyJob,
  } = useApp();
  const { t, resolved } = useI18n();
  const locale = dateLocale(resolved);

  // The backend renders the report in English for support exports; the labels
  // are mapped here so the running UI still follows the selected language.
  const labelFor = useCallback(
    (item: DiagnosticItem) => {
      const key = itemLabel[item.id];
      return key ? t(key) : item.label;
    },
    [t],
  );

  const [report, setReport] = useState<DiagnosticReport | null>(null);
  const [running, setRunning] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [dependencies, setDependencies] = useState<DependencyReport[]>([]);
  const [installing, setInstalling] = useState<string | null>(null);
  const [logs, setLogs] = useState<LogFileInfo[]>([]);
  const [openLog, setOpenLog] = useState<string | null>(null);
  const [logDetail, setLogDetail] = useState<string>("");
  const [logLoading, setLogLoading] = useState(false);

  const loadLogs = useCallback(async () => {
    try {
      const files = await api.logs();
      setLogs(files);
    } catch (error) {
      reportError(error, t("diag.logs.title"));
    }
  }, [reportError, t]);

  const run = useCallback(
    async (announce: boolean) => {
      setRunning(true);
      try {
        const next = await api.diagnostics();
        setReport(next);
        setDependencies(next.dependencies);
        if (announce) pushToast({ tone: "ok", title: t("diag.refreshed") });
      } catch (error) {
        reportError(error, t("diag.heading"));
      } finally {
        setRunning(false);
      }
    },
    [pushToast, reportError, t],
  );

  useEffect(() => {
    void run(false);
    void loadLogs();
    // Diagnostics is read-only and cheap to re-run when the page mounts.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // winget does not stream events to the webview, so the job is polled while it runs.
  useEffect(() => {
    if (!isJobRunning(dependencyJob.status)) return;
    let cancelled = false;
    const timer = window.setInterval(() => {
      void api
        .dependencyInstallState()
        .then((snapshot) => {
          if (cancelled) return;
          setDependencyJob(snapshot);
          if (!isJobRunning(snapshot.status)) {
            window.clearInterval(timer);
            if (snapshot.status === "succeeded") {
              setInstalling(null);
              pushToast({ tone: "ok", title: t("diag.depInstallDone") });
              void api.dependencies().then(setDependencies).catch(() => undefined);
              void loadLogs();
            } else {
              setInstalling(null);
              pushToast({
                tone: "danger",
                title: t("common.error"),
                message: snapshot.error ?? t("toast.error"),
              });
            }
          }
        })
        .catch(() => undefined);
    }, 1200);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [dependencyJob.status, loadLogs, pushToast, setDependencyJob, t]);

  const install = async (dependency: DependencyReport, wingetId: string) => {
    setInstalling(dependency.id);
    try {
      const snapshot = await api.installDependency(wingetId);
      setDependencyJob(snapshot);
      pushToast({
        tone: "accent",
        title: t("diag.depInstallStarted", { name: dependency.name }),
      });
    } catch (error) {
      setInstalling(null);
      reportError(error, dependency.name);
    }
  };

  const exportReport = async () => {
    setExporting(true);
    try {
      const path = await api.exportDiagnostics();
      pushToast({ tone: "ok", title: t("diag.exported", { path }) });
    } catch (error) {
      reportError(error, t("diag.action.export"));
    } finally {
      setExporting(false);
    }
  };

  const copySummary = async () => {
    if (!report) return;
    const lines: string[] = [
      `${report.environment.launcherVersion} - ${localTime(report.runAt, locale)} - ${report.overall}`,
      "",
    ];
    for (const [group, items] of groupBy(report.items, (item) => item.group)) {
      lines.push(`[${group}]`);
      for (const item of items) {
        lines.push(`  ${labelFor(item)}: ${item.status} - ${item.detail}`);
      }
      lines.push("");
    }
    await writeClipboard(lines.join("\n"));
    pushToast({ tone: "ok", title: t("diag.copied") });
  };

  const viewLog = async (name: string) => {
    if (openLog === name) {
      setOpenLog(null);
      setLogDetail("");
      return;
    }
    setOpenLog(name);
    setLogLoading(true);
    try {
      setLogDetail(await api.readLog(name));
    } catch (error) {
      setLogDetail("");
      reportError(error, name);
    } finally {
      setLogLoading(false);
    }
  };

  const clearLogs = async () => {
    const days = settings?.logRetentionDays ?? 14;
    try {
      await api.clearLogs(days);
      await loadLogs();
      pushToast({ tone: "ok", title: t("diag.logs.title"), message: t("common.saved") });
    } catch (error) {
      reportError(error, t("diag.logs.title"));
    }
  };

  const groups = useMemo(() => (report ? groupBy(report.items, (item) => item.group) : []), [report]);

  const overallTone =
    report?.overall === "error" ? "danger" : report?.overall === "warning" ? "warn" : "ok";
  const overallKey: MessageKey =
    report?.overall === "error"
      ? "diag.overall.error"
      : report?.overall === "warning"
        ? "diag.overall.warning"
        : "diag.overall.ok";

  const problems = report?.items.filter((item) => item.status === "error").length ?? 0;
  const warnings = report?.items.filter((item) => item.status === "warning").length ?? 0;

  return (
    <div className="animate-fade-in">
      <PageHeader
        title={t("diag.heading")}
        subtitle={t("diag.subheading")}
        actions={
          <>
            <Button
              icon={<Copy size={14} />}
              onClick={() => void copySummary()}
              disabled={!report || running}
            >
              {t("diag.action.copy")}
            </Button>
            <Button
              icon={<FileDown size={14} />}
              loading={exporting}
              onClick={() => void exportReport()}
              disabled={running}
            >
              {t("diag.action.export")}
            </Button>
            <Button
              variant="primary"
              icon={<RefreshCw size={14} className={running ? "animate-spin" : ""} />}
              loading={running}
              onClick={() => void run(true)}
            >
              {running ? t("diag.action.running") : t("diag.action.run")}
            </Button>
          </>
        }
      />

      <div className="space-y-4">
        <Card className="relative">
          <div
            aria-hidden
            className="pointer-events-none absolute inset-0 bg-[radial-gradient(130%_150%_at_0%_0%,rgba(142,169,255,0.08),transparent_58%)]"
          />
          <CardBody className="relative px-5 py-4">
            {running && !report ? (
              <SkeletonRows rows={2} />
            ) : report ? (
              <div className="flex flex-wrap items-start justify-between gap-4">
                <div className="min-w-0">
                  <span className="flex items-center gap-2">
                    <Dot tone={overallTone} pulse={overallTone === "ok"} />
                    <span className="label-xs">{t("diag.heading")}</span>
                  </span>
                  <h2 className="mt-2 text-[16px] font-semibold tracking-[-0.015em] text-ink">
                    {t(overallKey)}
                  </h2>
                  <p className="mt-1 max-w-[70ch] text-[12px] leading-[1.5] text-muted">
                    {report.environment.osName} {report.environment.osVersion} (build{" "}
                    {report.environment.osBuild}) · {report.environment.cpuCores} cores ·{" "}
                    {Math.round(report.environment.memoryTotalMb / 1024)} GB RAM
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-1.5">
                  <Pill>{localTime(report.runAt, locale)}</Pill>
                  {problems > 0 ? (
                    <Badge tone="danger">{`${problems} × ${t("diag.item.error")}`}</Badge>
                  ) : null}
                  {warnings > 0 ? (
                    <Badge tone="warn">{`${warnings} × ${t("diag.item.warning")}`}</Badge>
                  ) : null}
                  <Pill>{`v${report.environment.launcherVersion}`}</Pill>
                </div>
              </div>
            ) : (
              <EmptyState
                compact
                icon={<TriangleExclamationPoint size={15} />}
                title={t("diag.notRun")}
                action={
                  <Button size="sm" variant="primary" onClick={() => void run(true)}>
                    {t("diag.action.run")}
                  </Button>
                }
              />
            )}
          </CardBody>
        </Card>

        {report ? (
          <>
            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
              {groups.map(([group, items]) => {
                const key = groupTitle(group);
                return (
                  <Card key={group}>
                    <CardHeader
                      title={key ? t(key) : humanizeGroup(group)}
                      icon={groupIcon(group)}
                      action={<Pill>{`${items.length}`}</Pill>}
                    />
                    <CardBody className="pt-1.5">
                      <ul className="divide-y divide-[#1d242d]">
                        {items.map((item) => (
                          <li key={item.id} className="py-2">
                            <div className="flex items-start justify-between gap-3">
                              <span className="min-w-0 text-[12.5px] font-medium text-ink">
                                {labelFor(item)}
                              </span>
                              <span className="flex shrink-0 items-center gap-1.5">
                                <Dot tone={itemToneColor[item.status]} />
                                <span
                                  className={cn(
                                    "text-[11px]",
                                    item.status === "error" && "text-danger",
                                    item.status === "warning" && "text-warn",
                                    item.status === "ok" && "text-ok",
                                    item.status === "info" && "text-faint",
                                  )}
                                >
                                  {t(itemTone[item.status])}
                                </span>
                              </span>
                            </div>
                            <p className="mt-0.5 text-[11.5px] leading-[1.5] text-muted">
                              {item.detail}
                            </p>
                          </li>
                        ))}
                      </ul>
                    </CardBody>
                  </Card>
                );
              })}
            </div>

            <Card>
              <CardHeader
                title={t("diag.groups.dependencies")}
                icon={<Wrench size={13} />}
                action={
                  <Badge tone={dependencies.some((dep) => dep.kind === "required" && dep.status !== "installed") ? "danger" : "ok"}>
                    {dependencies.filter((dep) => dep.kind === "required" && dep.status !== "installed").length > 0
                      ? t("diag.item.error")
                      : t("diag.item.ok")}
                  </Badge>
                }
              />
              <CardBody className="pt-1.5">
                {dependencies.length === 0 ? (
                  <EmptyState compact title={t("diag.notRun")} />
                ) : (
                  <ul className="divide-y divide-[#1d242d]">
                    {dependencies.map((dependency) => {
                      const runningThis = isJobRunning(dependencyJob.status) && installing === dependency.id;
                      return (
                        <li key={dependency.id} className="flex flex-wrap items-start justify-between gap-3 py-2.5">
                          <div className="min-w-0 flex-1">
                            <div className="flex flex-wrap items-center gap-2">
                              <span className="text-[12.5px] font-medium text-ink">{dependency.name}</span>
                              <Badge tone={dependencyTone(dependency.status)}>
                                {t(dependencyStatusLabel[dependency.status])}
                              </Badge>
                              <span className="text-[10.5px] uppercase tracking-[0.04em] text-faint">
                                {t(dependencyKindLabel[dependency.kind])}
                              </span>
                              {dependency.version ? (
                                <Pill>{dependency.version}</Pill>
                              ) : dependency.requiredVersion ? (
                                <Pill>{dependency.requiredVersion}</Pill>
                              ) : null}
                            </div>
                            <p className="mt-1 max-w-[86ch] text-[11.5px] leading-[1.5] text-muted">
                              {dependency.detail}
                            </p>
                            {dependency.affects.length > 0 ? (
                              <p className="mt-0.5 text-[11px] text-faint">
                                {t("diag.dep.affects", { list: dependency.affects.join(", ") })}
                              </p>
                            ) : null}
                            {dependency.fix?.notes ? (
                              <p className="mt-0.5 text-[11px] text-faint">{dependency.fix.notes}</p>
                            ) : null}
                          </div>
                          <div className="flex shrink-0 items-center gap-1.5">
                            {dependency.fix?.wingetId &&
                            dependency.status !== "installed" ? (
                              <Button
                                size="sm"
                                icon={
                                  runningThis ? (
                                    <LoaderCircle size={13} className="animate-spin" />
                                  ) : (
                                    <Download size={13} />
                                  )
                                }
                                disabled={isJobRunning(dependencyJob.status)}
                                onClick={() =>
                                  void install(dependency, dependency.fix?.wingetId ?? "")
                                }
                              >
                                {runningThis ? t("diag.dep.installing") : t("diag.dep.install")}
                              </Button>
                            ) : null}
                            {dependency.fix?.url ? (
                              <IconButton
                                label={t("diag.dep.openLink")}
                                onClick={() => void openExternal(dependency.fix?.url ?? "")}
                                className="h-8 w-8"
                              >
                                <ExternalLink size={13} />
                              </IconButton>
                            ) : null}
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                )}
                {isJobRunning(dependencyJob.status) ? (
                  <div className="mt-3 space-y-1.5 rounded-[10px] border border-line-soft bg-[#0d1116] p-3">
                    <div className="flex items-center justify-between text-[11.5px] text-muted">
                      <span className="flex items-center gap-1.5">
                        <LoaderCircle size={12} className="animate-spin text-accent" />
                        {dependencyJob.command ?? dependencyJob.kind}
                      </span>
                      {percentLabel(dependencyJob.percent) ? (
                        <span className="font-mono text-[10.5px]">{percentLabel(dependencyJob.percent)}</span>
                      ) : null}
                    </div>
                    <div className="log max-h-32 overflow-auto">
                      {dependencyJob.lines.slice(-8).map((line, index) => (
                        <div key={index}>{line}</div>
                      ))}
                    </div>
                  </div>
                ) : null}
              </CardBody>
            </Card>

            <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
              <Card>
                <CardHeader title={t("diag.groups.storage")} icon={<HardDrive size={13} />} />
                <CardBody className="pt-2">
                  {report.disks.length === 0 ? (
                    <EmptyState compact title={t("common.notAvailable")} />
                  ) : (
                    <div className="divide-y divide-[#1d242d]">
                      {report.disks.map((disk) => (
                        <KeyValue
                          key={`${disk.volume}-${disk.path}`}
                          label={disk.volume}
                          value={`${gib(disk.freeBytes)} / ${gib(disk.totalBytes)}`}
                          mono
                          title={disk.path}
                        />
                      ))}
                    </div>
                  )}
                  <div className="divider my-3" />
                  <div className="space-y-2">
                    <SectionLabel>{t("settings.group.paths")}</SectionLabel>
                    {[
                      { label: t("settings.paths.repository"), value: report.repositoryPath },
                      { label: t("settings.paths.out"), value: report.outPath },
                      { label: t("settings.paths.work"), value: report.workPath },
                      ...(report.gamePath
                        ? [{ label: t("settings.paths.game"), value: report.gamePath }]
                        : []),
                    ].map((entry) => (
                      <div key={entry.label} className="flex items-center justify-between gap-2">
                        <span className="shrink-0 text-[11.5px] text-faint">{entry.label}</span>
                        <span className="flex min-w-0 items-center gap-1">
                          <span className="truncate font-mono text-[11px] text-muted" title={entry.value}>
                            {entry.value}
                          </span>
                          <IconButton
                            label={t("common.open")}
                            onClick={() => void revealPath(entry.value)}
                            className="h-6 w-6 shrink-0"
                          >
                            <FolderOpen size={11} />
                          </IconButton>
                        </span>
                      </div>
                    ))}
                  </div>
                </CardBody>
              </Card>

              <Card>
                <CardHeader title={t("diag.groups.network")} icon={<Globe size={13} />} />
                <CardBody className="pt-2">
                  <div className="flex items-center justify-between gap-3">
                    <span className="flex items-center gap-2 text-[12.5px] text-ink">
                      <Dot tone={report.network.reachable ? "ok" : "warn"} />
                      {report.network.reachable ? t("diag.network.reachable") : t("diag.network.unreachable")}
                    </span>
                    {report.network.latencyMs !== null ? (
                      <Pill>{t("diag.network.latency", { ms: report.network.latencyMs })}</Pill>
                    ) : null}
                  </div>
                  <p className="mt-1.5 text-[11.5px] leading-[1.5] text-muted">{report.network.detail}</p>
                  <div className="divider my-3" />
                  <div className="space-y-2">
                    <SectionLabel>{t("home.card.environment")}</SectionLabel>
                    <KeyValue label="Windows" value={`${report.environment.osEdition} · ${report.environment.arch}`} />
                    <KeyValue label="CPU" value={report.environment.cpuName} title={report.environment.cpuName} />
                    <KeyValue
                      label="RAM"
                      value={`${Math.round(report.environment.memoryAvailableMb / 1024)} / ${Math.round(report.environment.memoryTotalMb / 1024)} GB`}
                      mono
                    />
                    <KeyValue
                      label="Long paths"
                      value={
                        report.environment.longPaths === null
                          ? t("common.unknown")
                          : report.environment.longPaths
                            ? t("common.ok")
                            : t("common.warning")
                      }
                    />
                  </div>
                </CardBody>
              </Card>
            </div>
          </>
        ) : null}

        <Card>
          <CardHeader
            title={t("diag.logs.title")}
            icon={<Terminal size={13} />}
            action={
              <>
                <Button
                  size="sm"
                  variant="ghost"
                  icon={<Trash size={12} />}
                  onClick={() => void clearLogs()}
                >
                  {t("diag.logs.clear", { days: settings?.logRetentionDays ?? 14 })}
                </Button>
                <Button
                  size="sm"
                  icon={<FolderOpen size={12} />}
                  disabled={!paths?.logsDir}
                  onClick={() => paths?.logsDir && void revealPath(paths.logsDir)}
                >
                  {t("diag.logs.open")}
                </Button>
              </>
            }
          />
          <CardBody className="pt-1.5">
            {logs.length === 0 ? (
              <EmptyState compact title={t("diag.logs.empty")} />
            ) : (
              <ul className="divide-y divide-[#1d242d]">
                {logs.map((file) => (
                  <li key={file.name} className="py-1.5">
                    <div className="flex items-center justify-between gap-3">
                      <span className="flex min-w-0 items-center gap-2">
                        <ScrollText size={12} className="shrink-0 text-faint" />
                        <span className="truncate font-mono text-[11.5px] text-ink" title={file.name}>
                          {file.name}
                        </span>
                      </span>
                      <span className="flex shrink-0 items-center gap-2">
                        <span className="text-[10.5px] text-faint">
                          {t("diag.logs.size", {
                            size: bytes(file.sizeBytes),
                            date: localTime(file.modified, locale),
                          })}
                        </span>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => void viewLog(file.name)}
                          loading={logLoading && openLog === file.name}
                        >
                          {openLog === file.name ? t("diag.logs.hide") : t("diag.logs.view")}
                        </Button>
                      </span>
                    </div>
                    {openLog === file.name && !logLoading ? (
                      <pre className="log scroll-thin mt-2 max-h-72 overflow-auto rounded-[8px] border border-line-soft bg-[#090c10] p-2.5">
                        {logDetail || t("diag.logs.empty")}
                      </pre>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
            <InlineNote className="mt-3" tone="neutral" icon={<TriangleExclamationPoint size={13} />}>
              {t("settings.logRetention.hint")}
            </InlineNote>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}
