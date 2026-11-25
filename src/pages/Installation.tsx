import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  CircleCheck as CheckCircle2,
  FolderOpen,
  FolderSearch,
  Gamepad2,
  HardDriveDownload,
  Monitor,
  RefreshCw,
  TriangleExclamationPoint as AlertTriangle,
  Wrench,
} from "lucide-react";
import type { GameCandidate, GameValidation, InstallEvent } from "@/types";
import type { StepDescriptor, StepStatus } from "@/components/StepList";
import { api } from "@/services/ipc";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";
import type { MessageKey } from "@/i18n/dictionary";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardBody, CardFooter, CardHeader } from "@/components/ui/Card";
import { Badge, Pill } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Checkbox, Field, PathField } from "@/components/ui/Field";
import { InlineNote, KeyValue } from "@/components/ui/EmptyState";
import { Progress } from "@/components/ui/Progress";
import { StepList } from "@/components/StepList";
import { LogConsole } from "@/components/LogConsole";
import { gib } from "@/lib/format";
import { revealPath } from "@/lib/tauri";
import { compatibilityTone, isJobRunning } from "@/lib/ui";

const STEP_IDS = ["preflight", "clone", "submodules", "compatibility"] as const;

export function InstallationPage() {
  const {
    settings,
    paths,
    summary,
    installJob,
    setInstallJob,
    refreshSummary,
    refreshCompatibility,
    compatibility,
    reportError,
  } = useApp();
  const t = useT();

  const [repoPath, setRepoPath] = useState("");
  const [outPath, setOutPath] = useState("");
  const [workPath, setWorkPath] = useState("");
  const [gamePath, setGamePath] = useState("");
  const [skipCompat, setSkipCompat] = useState(false);
  const [starting, setStarting] = useState(false);

  const [detecting, setDetecting] = useState(false);
  const [candidates, setCandidates] = useState<GameCandidate[]>([]);
  const [validation, setValidation] = useState<GameValidation | null>(null);

  const [steps, setSteps] = useState<Record<string, { status: StepStatus; detail?: string }>>({});
  const [lines, setLines] = useState<string[]>([]);
  const [freeOut, setFreeOut] = useState<{ free: number; volume: string } | null>(null);

  const seeded = useRef(false);
  const seenJob = useRef<string | null>(null);

  const installation = summary?.installation;
  const installed = installation?.installed ?? false;
  const running = isJobRunning(installJob.status);

  // Seed the form from shipped defaults once settings arrive.
  useEffect(() => {
    if (seeded.current || !settings) return;
    seeded.current = true;
    setRepoPath(settings.repositoryPath ?? paths?.defaultRepository ?? "");
    setOutPath(settings.outPath || paths?.defaultOut || "");
    setWorkPath(settings.workPath || paths?.defaultWork || "");
    setGamePath(settings.gamePath ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings, paths]);

  // Validate the game folder as the user types, never on every keystroke.
  useEffect(() => {
    if (!gamePath.trim()) {
      setValidation(null);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void api
        .validateGame(gamePath)
        .then((result) => {
          if (!cancelled) setValidation(result);
        })
        .catch(() => {
          if (!cancelled) setValidation(null);
        });
    }, 420);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [gamePath]);

  useEffect(() => {
    if (!outPath.trim()) return;
    let cancelled = false;
    void api
      .diskSpace(outPath)
      .then((disk) => {
        if (!cancelled) setFreeOut({ free: disk.freeBytes, volume: disk.volume });
      })
      .catch(() => {
        if (!cancelled) setFreeOut(null);
      });
    return () => {
      cancelled = true;
    };
  }, [outPath]);

  // Adopt a job started elsewhere (page switch, restart) exactly once.
  useEffect(() => {
    if (seenJob.current === installJob.id) return;
    seenJob.current = installJob.id;
    if (installJob.id) {
      setLines(installJob.lines.slice(-400));
    }
  }, [installJob.id, installJob.lines]);

  // Live install stream.
  useEffect(() => {
    let disposed = false;
    let off: (() => void) | undefined;
    void listen<InstallEvent>("install://event", (event) => {
      const payload = event.payload;
      if (payload.kind === "step" && payload.step) {
        setSteps((current) => ({
          ...current,
          [payload.step as string]: {
            status: (payload.status ?? "running") as StepStatus,
            detail: payload.detail,
          },
        }));
      }
      if (payload.kind === "log" && payload.line) {
        const line = payload.line;
        setLines((current) => [...current.slice(-399), line]);
      }
      if (payload.kind === "finished") {
        void refreshSummary();
        void refreshCompatibility();
      }
    }).then((unlisten) => {
      if (disposed) unlisten();
      else off = unlisten;
    });
    return () => {
      disposed = true;
      off?.();
    };
  }, [refreshCompatibility, refreshSummary]);

  const stepList: StepDescriptor[] = STEP_IDS.map((id) => {
    const recorded = steps[id];
    let status: StepStatus = recorded?.status ?? "pending";
    if (!recorded && installJob.status === "succeeded") status = "done";
    return {
      id,
      label: t(`install.step.${id}` as MessageKey),
      status,
      detail: recorded?.detail,
    };
  });

  const startInstall = async () => {
    if (!repoPath.trim() || !outPath.trim() || !workPath.trim()) return;
    setSteps({});
    setLines([]);
    setStarting(true);
    try {
      const job = await api.installLeonida({
        repositoryPath: repoPath,
        gamePath: gamePath.trim() || null,
        outPath,
        workPath,
        skipCompatibility: skipCompat,
      });
      seenJob.current = job.id;
      setInstallJob(job);
    } catch (error) {
      reportError(error, t("install.heading"));
    } finally {
      setStarting(false);
    }
  };

  const cancelInstall = async () => {
    try {
      await api.cancelInstall();
    } catch (error) {
      reportError(error, t("install.action.cancel"));
    }
  };

  const detectGame = async () => {
    setDetecting(true);
    try {
      const found = await api.detectGameFolders();
      setCandidates(found);
    } catch (error) {
      reportError(error, t("install.game.detect"));
    } finally {
      setDetecting(false);
    }
  };

  const repairSubmodules = async () => {
    try {
      const job = await api.updateSubmodules();
      seenJob.current = job.id;
      setInstallJob(job);
    } catch (error) {
      reportError(error, t("install.action.submodules"));
    }
  };

  const compatibilityStatus = compatibility?.status ?? "unknown";

  return (
    <div className="animate-fade-in">
      <PageHeader
        title={t("install.heading")}
        subtitle={t("install.subheading")}
        actions={
          <>
            {installed ? (
              <Button
                icon={<RefreshCw size={14} />}
                onClick={() => {
                  void refreshSummary();
                  void refreshCompatibility();
                }}
              >
                {t("install.action.recheck")}
              </Button>
            ) : null}
            {installed ? (
              <Button icon={<Wrench size={14} />} onClick={() => void repairSubmodules()} disabled={running}>
                {t("install.action.submodules")}
              </Button>
            ) : null}
          </>
        }
      />

      {installation && !installation.gitAvailable ? (
        <InlineNote tone="danger" icon={<AlertTriangle size={14} />} className="mb-4">
          {t("install.gitMissing")}
        </InlineNote>
      ) : null}

      {installed ? (
        <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)]">
          <Card>
            <CardHeader
              title={t("install.installed.title")}
              description={t("install.installed.body")}
              icon={<CheckCircle2 size={13} />}
              action={<Badge tone={compatibilityTone(compatibilityStatus)}>{t(`compat.${compatibilityStatus}` as MessageKey)}</Badge>}
            />
            <CardBody className="pt-2">
              <div className="divide-y divide-line-soft">
                <KeyValue
                  label={t("install.form.repository")}
                  value={installation?.repositoryPath ?? "—"}
                  mono
                  title={installation?.repositoryPath ?? undefined}
                  action={
                    installation?.repositoryPath ? (
                      <button
                        type="button"
                        aria-label={t("common.open")}
                        onClick={() => void revealPath(installation.repositoryPath)}
                        className="focus-ring rounded-[4px] p-[2px] text-faint hover:text-ink"
                      >
                        <FolderOpen size={12} />
                      </button>
                    ) : null
                  }
                />
                <KeyValue label={t("home.label.branch")} value={installation?.branch ?? "detached"} mono />
                <KeyValue
                  label={t("home.label.commit")}
                  value={`${installation?.shortCommit ?? "—"}${installation?.commitSubject ? ` · ${installation.commitSubject}` : ""}`}
                  title={installation?.commit ?? undefined}
                />
                <KeyValue label={t("common.time")} value={installation?.commitDate ?? "—"} />
                <KeyValue
                  label={t("home.label.worktree")}
                  value={
                    (installation?.dirtyFiles ?? 0) > 0
                      ? t("home.dirty", { count: installation?.dirtyFiles ?? 0 })
                      : t("home.clean")
                  }
                />
                <KeyValue
                  label={t("home.label.game")}
                  value={installation?.gamePath ?? t("home.noGame")}
                  mono
                  title={installation?.gamePath ?? undefined}
                />
              </div>

              {installation?.submodules.length ? (
                <div className="mt-3">
                  <div className="label-xs mb-1">{t("home.label.submodules")}</div>
                  <ul className="divide-y divide-line-soft">
                    {installation.submodules.map((submodule) => (
                      <li
                        key={submodule.path}
                        className="flex items-center justify-between gap-3 py-1.5"
                      >
                        <span className="truncate font-mono text-[11px] text-muted">{submodule.path}</span>
                        <Badge tone={submodule.initialized ? "ok" : "warn"}>
                          {submodule.initialized ? t("common.ok") : t("home.submodulesMissing")}
                        </Badge>
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
            </CardBody>
          </Card>

          <div className="space-y-4">
            <Card>
              <CardHeader title={t("install.review.title")} icon={<HardDriveDownload size={13} />} />
              <CardBody className="space-y-2 pt-3">
                <Progress
                  tone={
                    installJob.status === "failed"
                      ? "danger"
                      : installJob.status === "cancelled"
                        ? "warn"
                        : installJob.status === "succeeded"
                          ? "ok"
                          : "accent"
                  }
                  value={installJob.percent ?? (running ? null : installJob.status === "succeeded" ? 100 : 0)}
                />
                <div className="flex items-center justify-between text-[11.5px] text-muted">
                  <span>{installJob.currentStage ?? t("build.status.idle")}</span>
                  <span className="font-mono text-[10.5px] text-faint">
                    {t(`job.status.${installJob.status}` as MessageKey)}
                  </span>
                </div>
                <StepList steps={stepList} className="pt-2" />
                {installJob.error ? (
                  <InlineNote tone="danger">{installJob.error}</InlineNote>
                ) : null}
              </CardBody>
            </Card>

            <Card>
              <CardHeader title={t("build.paths.title")} icon={<FolderOpen size={13} />} />
              <CardBody className="pt-2">
                <div className="divide-y divide-line-soft">
                  <KeyValue label={t("build.paths.output")} value={settings?.outPath ?? "—"} mono title={settings?.outPath} />
                  <KeyValue label={t("build.paths.work")} value={settings?.workPath ?? "—"} mono title={settings?.workPath} />
                  <KeyValue
                    label={t("home.label.freeSpace")}
                    value={freeOut ? `${gib(freeOut.free)} ${freeOut.volume}` : "—"}
                    mono
                  />
                  <KeyValue
                    label={t("build.resume.title")}
                    value={installation?.workState.resumeAvailable ? t("common.ok") : t("common.none")}
                  />
                </div>
                <InlineNote className="mt-3" tone="neutral">
                  {t("install.review.requirement")}
                </InlineNote>
              </CardBody>
            </Card>
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
          <div className="space-y-4">
            <Card>
              <CardHeader title={t("install.form.title")} icon={<HardDriveDownload size={13} />} />
              <CardBody className="space-y-4">
                <Field label={t("install.form.repository")} hint={t("install.form.repository.hint")}>
                  <PathField value={repoPath} onCommit={setRepoPath} disabled={running} />
                </Field>

                <Field
                  label={t("install.form.game")}
                  hint={t("install.form.game.hint")}
                  action={
                    <Button
                      size="sm"
                      variant="ghost"
                      icon={<FolderSearch size={13} />}
                      loading={detecting}
                      onClick={() => void detectGame()}
                    >
                      {t("install.game.detect")}
                    </Button>
                  }
                >
                  <PathField
                    value={gamePath}
                    onCommit={setGamePath}
                    disabled={running}
                    invalid={Boolean(validation && !validation.valid)}
                  />
                </Field>

                {candidates.length > 0 ? (
                  <div className="space-y-1.5">
                    <div className="label-xs">{t("install.game.candidates")}</div>
                    {candidates.slice(0, 4).map((candidate) => (
                      <button
                        key={candidate.path}
                        type="button"
                        onClick={() => setGamePath(candidate.path)}
                        className="flex w-full items-center justify-between gap-3 rounded-[6px] border border-line-soft bg-base-soft px-3 py-2 text-left transition-colors hover:border-line hover:bg-raised"
                      >
                        <span className="min-w-0">
                          <span className="block truncate font-mono text-[11px] text-ink">{candidate.path}</span>
                          <span className="text-[10.5px] text-faint">
                            {t("install.game.archives", { count: candidate.archiveCount })}
                            {candidate.configPresent ? ` · ${t("install.game.config")}` : ""}
                          </span>
                        </span>
                        <span className="shrink-0 text-[11px] text-accent">{t("install.game.use")}</span>
                      </button>
                    ))}
                  </div>
                ) : null}

                {validation ? (
                  <InlineNote tone={validation.valid ? "ok" : "warn"} icon={<Gamepad2 size={13} />}>
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium text-ink">
                        {validation.valid ? t("install.game.valid") : t("install.game.invalid")}
                      </span>
                      <Badge tone={validation.valid ? "ok" : "warn"}>
                        {t("install.game.archives", { count: validation.archiveCount })}
                      </Badge>
                      {validation.expectedVersion ? (
                        <Pill>{t("install.game.expected", { version: validation.expectedVersion })}</Pill>
                      ) : null}
                    </div>
                    {validation.probesFound.length > 0 ? (
                      <p className="mt-1">
                        {t("install.game.probes", { list: validation.probesFound.join(", ") })}
                      </p>
                    ) : null}
                    {validation.issues.length > 0 ? (
                      <p className="mt-1">{validation.issues.join(" ")}</p>
                    ) : null}
                  </InlineNote>
                ) : null}

                <div className="grid grid-cols-1 gap-4">
                  <Field label={t("install.form.out")} hint={t("install.form.out.hint")}>
                    <PathField value={outPath} onCommit={setOutPath} disabled={running} />
                  </Field>
                  <Field label={t("install.form.work")} hint={t("install.form.work.hint")}>
                    <PathField value={workPath} onCommit={setWorkPath} disabled={running} />
                  </Field>
                </div>
              </CardBody>
              <CardFooter>
                <div className="min-w-0 truncate text-[11px] text-faint">
                  {freeOut
                    ? t("install.form.space", { free: gib(freeOut.free), volume: freeOut.volume })
                    : t("install.review.oneJob")}
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {running ? (
                    <Button variant="danger" onClick={() => void cancelInstall()}>
                      {t("install.action.cancel")}
                    </Button>
                  ) : (
                    <Button
                      variant="primary"
                      icon={<HardDriveDownload size={14} />}
                      loading={starting}
                      disabled={!repoPath.trim() || !outPath.trim() || !workPath.trim()}
                      onClick={() => void startInstall()}
                    >
                      {t("install.action.install")}
                    </Button>
                  )}
                </div>
              </CardFooter>
            </Card>

            <Card>
              <CardHeader title={t("install.steps.title")} icon={<RefreshCw size={13} />} />
              <CardBody className="pt-3">
                <StepList steps={stepList} />
                <InlineNote className="mt-3" tone="neutral">
                  {t("install.noPercent")}
                </InlineNote>
                <div className="mt-3 flex items-center justify-between gap-3">
                  <Checkbox
                    checked={skipCompat}
                    onChange={setSkipCompat}
                    disabled={running}
                    label={t("install.skipCompat.label")}
                    description={t("install.skipCompat.hint")}
                  />
                </div>
              </CardBody>
            </Card>

            <LogConsole
              lines={lines.length > 0 ? lines : installJob.lines}
              title={t("install.log.title")}
              emptyText={t("install.log.empty")}
            />
          </div>

          <div className="space-y-4">
            <Card>
              <CardHeader title={t("install.review.title")} icon={<CheckCircle2 size={13} />} />
              <CardBody className="space-y-3">
                <div className="divide-y divide-line-soft">
                  <KeyValue label={t("install.form.out")} value={outPath || "—"} mono title={outPath} />
                  <KeyValue label={t("install.form.work")} value={workPath || "—"} mono title={workPath} />
                  <KeyValue
                    label={t("home.label.freeSpace")}
                    value={freeOut ? `${gib(freeOut.free)} ${freeOut.volume}` : "—"}
                    mono
                  />
                  <KeyValue
                    label={t("home.label.compatibility")}
                    value={t(`compat.${compatibilityStatus}` as MessageKey)}
                  />
                </div>
                <InlineNote tone="neutral">{t("install.review.requirement")}</InlineNote>
                <InlineNote tone="accent">{t("install.review.oneJob")}</InlineNote>
              </CardBody>
            </Card>

            <Card>
              <CardHeader title={t("home.card.environment")} icon={<Monitor size={13} />} />
              <CardBody className="pt-2">
                <div className="divide-y divide-line-soft">
                  <KeyValue label={t("common.version")} value={summary?.metadata.version ?? "—"} mono />
                  <KeyValue
                    label="Windows"
                    value={
                      summary
                        ? `${summary.environment.osName} · ${summary.environment.osBuild}`
                        : "—"
                    }
                  />
                  <KeyValue
                    label="CPU"
                    value={summary ? `${summary.environment.cpuCores} × ${summary.environment.arch}` : "—"}
                    mono
                  />
                  <KeyValue
                    label="RAM"
                    value={
                      summary
                        ? `${Math.round(summary.environment.memoryTotalMb / 1024)} GB`
                        : "—"
                    }
                    mono
                  />
                </div>
              </CardBody>
            </Card>
          </div>
        </div>
      )}
    </div>
  );
}
