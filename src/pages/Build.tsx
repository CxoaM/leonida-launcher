import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  Boxes,
  CircleCheck as CheckCircle2,
  FolderOpen,
  Hammer,
  ListChecks,
  RefreshCw,
  TriangleExclamationPoint as AlertTriangle,
  X,
} from "lucide-react";
import type { BuildEvent, StageInfo } from "@/types";
import { api } from "@/services/ipc";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardBody, CardFooter, CardHeader } from "@/components/ui/Card";
import { Pill } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Checkbox, Field, Segmented, Toggle } from "@/components/ui/Field";
import { EmptyState, InlineNote, KeyValue } from "@/components/ui/EmptyState";
import { Progress } from "@/components/ui/Progress";
import { LogConsole } from "@/components/LogConsole";
import { duration, etaLabel } from "@/lib/format";
import { isJobRunning, percentLabel } from "@/lib/ui";

function elapsedLabel(startedAt: string | null, finishedAt: string | null, nowMs: number): string {
  if (!startedAt) return "—";
  const start = Date.parse(startedAt);
  if (Number.isNaN(start)) return "—";
  const end = finishedAt ? Date.parse(finishedAt) : nowMs;
  if (Number.isNaN(end)) return "—";
  return duration(Math.max(0, (end - start) / 1000));
}

export function BuildPage() {
  const {
    summary,
    compatibility,
    buildJob,
    setBuildJob,
    settings,
    updateSettings,
    reportError,
    refreshSummary,
    setPage,
  } = useApp();
  const t = useT();

  const [stages, setStages] = useState<StageInfo[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [wav, setWav] = useState(false);
  const [fullCheck, setFullCheck] = useState(false);
  const [starting, setStarting] = useState(false);
  const [lines, setLines] = useState<string[]>([]);
  const [now, setNow] = useState(() => Date.now());

  const seenJob = useRef<string | null>(null);

  const installation = summary?.installation;
  const installed = installation?.installed ?? false;
  const incompatible = compatibility?.status === "incompatible";
  const buildSupported = compatibility?.capabilities.build ?? false;
  const running = isJobRunning(buildJob.status);

  useEffect(() => {
    let cancelled = false;
    void api
      .stages()
      .then((found) => {
        if (cancelled) return;
        setStages(found);
        setSelected((current) => {
          const valid = current.filter((id) => found.some((stage) => stage.id === id));
          if (valid.length > 0) return valid;
          return found.filter((stage) => stage.selectedByDefault).map((stage) => stage.id);
        });
      })
      .catch(() => {
        if (!cancelled) setStages([]);
      });
    return () => {
      cancelled = true;
    };
  }, [compatibility?.installedCommit]);

  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [running]);

  useEffect(() => {
    if (seenJob.current === buildJob.id) return;
    seenJob.current = buildJob.id;
    setLines(buildJob.lines.slice(-500));
  }, [buildJob.id, buildJob.lines]);

  useEffect(() => {
    let disposed = false;
    let off: (() => void) | undefined;
    void listen<BuildEvent>("build://event", (event) => {
      const payload = event.payload;
      if (payload.kind === "log" && payload.line) {
        const line = payload.line;
        setLines((current) => [...current.slice(-499), line]);
      }
      if (payload.kind === "finished") {
        void refreshSummary();
      }
    }).then((unlisten) => {
      if (disposed) unlisten();
      else off = unlisten;
    });
    return () => {
      disposed = true;
      off?.();
    };
  }, [refreshSummary]);

  const toggleStage = (id: string) => {
    setSelected((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    );
  };

  const start = async () => {
    if (!settings || selected.length === 0) return;
    setLines([]);
    setStarting(true);
    try {
      const job = await api.startBuild({
        stages: selected,
        gamePath: settings.gamePath,
        outPath: settings.outPath,
        workPath: settings.workPath,
        jobs: settings.jobs,
        wave64Emulate: settings.wave64Emulate,
        wav,
        fullCheck,
        fallbackFlags: [],
      });
      seenJob.current = job.id;
      setBuildJob(job);
    } catch (error) {
      reportError(error, t("build.heading"));
    } finally {
      setStarting(false);
    }
  };

  const cancel = async () => {
    try {
      await api.cancelBuild();
    } catch (error) {
      reportError(error, t("build.action.cancel"));
    }
  };

  const stageInfo = useMemo(
    () => stages.filter((stage) => selected.includes(stage.id)),
    [stages, selected],
  );

  const statusLabel = useMemo(() => {
    switch (buildJob.status) {
      case "running":
        return t("build.status.running");
      case "cancelling":
        return t("build.status.cancelling");
      case "succeeded":
        return t("build.status.succeeded");
      case "failed":
        return t("build.status.failed");
      case "cancelled":
        return t("build.status.cancelled");
      default:
        return t("build.status.idle");
    }
  }, [buildJob.status, t]);

  return (
    <div className="animate-fade-in">
      <PageHeader
        title={t("build.heading")}
        subtitle={t("build.subheading")}
        actions={
          running ? (
            <Button variant="danger" icon={<X size={14} />} onClick={() => void cancel()}>
              {t("build.action.cancel")}
            </Button>
          ) : (
            <Button
              variant="primary"
              icon={<Hammer size={14} />}
              loading={starting}
              disabled={!installed || incompatible || !buildSupported || selected.length === 0}
              onClick={() => void start()}
            >
              {t("build.action.start")}
            </Button>
          )
        }
      />

      {!installed ? (
        <Card>
          <CardBody>
            <EmptyState
              icon={<Boxes size={16} />}
              title={t("build.notInstalled.title")}
              body={t("build.notInstalled.body")}
              action={
                <Button variant="primary" onClick={() => setPage("installation")}>
                  {t("install.heading")}
                </Button>
              }
            />
          </CardBody>
        </Card>
      ) : incompatible ? (
        <Card>
          <CardBody>
            <EmptyState
              tone="danger"
              icon={<AlertTriangle size={16} />}
              title={t("build.incompatible.title")}
              body={t("build.incompatible.body")}
              action={
                <Button variant="primary" onClick={() => setPage("updates")}>
                  {t("nav.updates")}
                </Button>
              }
            />
          </CardBody>
        </Card>
      ) : !buildSupported ? (
        <Card>
          <CardBody>
            <EmptyState
              tone="warn"
              icon={<AlertTriangle size={16} />}
              title={t("build.noBuildSupport.title")}
              body={t("build.noBuildSupport.body")}
              action={
                <Button onClick={() => setPage("diagnostics")}>
                  {t("nav.diagnostics")}
                </Button>
              }
            />
          </CardBody>
        </Card>
      ) : (
        <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-[minmax(0,1.05fr)_minmax(0,1fr)]">
          <div className="space-y-4">
            <Card>
              <CardHeader
                title={t("build.stages.title")}
                description={t("build.stages.hint")}
                icon={<ListChecks size={13} />}
                action={<Pill>{selected.length}</Pill>}
              />
              <CardBody className="space-y-1.5 pt-3">
                {stages.length === 0 ? (
                  <InlineNote tone="warn">{t("build.stages.none")}</InlineNote>
                ) : (
                  stages.map((stage) => (
                    <Checkbox
                      key={stage.id}
                      checked={selected.includes(stage.id)}
                      onChange={() => toggleStage(stage.id)}
                      disabled={running}
                      label={stage.label}
                      description={stage.description}
                      meta={
                        stage.requiresDependencies.length > 0 ? (
                          <span className="shrink-0 font-mono text-[10px] text-faint">
                            {t("build.stages.requires", { list: stage.requiresDependencies.join(", ") })}
                          </span>
                        ) : undefined
                      }
                    />
                  ))
                )}
              </CardBody>
            </Card>

            <Card>
              <CardHeader title={t("build.options.title")} icon={<RefreshCw size={13} />} />
              <CardBody className="space-y-3 pt-3">
                <div className="grid grid-cols-2 gap-3">
                  <Field label={t("build.options.jobs")} hint={t("build.options.jobs.hint")}>
                    <input
                      type="number"
                      min={0}
                      max={64}
                      className="input w-24 mono"
                      value={settings?.jobs ?? 0}
                      disabled={running}
                      onChange={(event) => {
                        const parsed = Number.parseInt(event.target.value, 10);
                        updateSettings({ jobs: Number.isFinite(parsed) ? parsed : 0 });
                      }}
                    />
                  </Field>
                  <Field label={t("build.options.wave64")}>
                    <Segmented
                      value={settings?.wave64Emulate ?? "auto"}
                      disabled={running || !(compatibility?.capabilities.wave64Emulate ?? false)}
                      onChange={(value) => updateSettings({ wave64Emulate: value })}
                      options={[
                        { value: "auto", label: t("build.options.wave64.auto") },
                        { value: "on", label: t("build.options.wave64.on") },
                        { value: "off", label: t("build.options.wave64.off") },
                      ]}
                    />
                  </Field>
                </div>

                <div className="divider" />

                <Toggle
                  checked={wav}
                  onChange={setWav}
                  disabled={running || !(compatibility?.capabilities.wav ?? false)}
                  label={t("build.options.wav")}
                  description={t("build.options.wav.hint")}
                />
                <div className="divider" />
                <Toggle
                  checked={fullCheck}
                  onChange={setFullCheck}
                  disabled={running || !(compatibility?.capabilities.fullCheck ?? false)}
                  label={t("build.options.fullCheck")}
                  description={t("build.options.fullCheck.hint")}
                />

                {installation?.workState.resumeAvailable ? (
                  <InlineNote tone="ok" icon={<CheckCircle2 size={13} />}>
                    <span className="font-medium">{t("build.resume.title")}. </span>
                    {t("build.resume.body")}
                  </InlineNote>
                ) : null}
              </CardBody>
              <CardFooter>
                <span className="truncate font-mono text-[10.5px] text-faint">
                  {stageInfo.map((stage) => stage.id).join(" → ") || t("build.stages.none")}
                </span>
              </CardFooter>
            </Card>

            <Card>
              <CardHeader title={t("build.paths.title")} icon={<FolderOpen size={13} />} />
              <CardBody className="pt-2">
                <div className="divide-y divide-line-soft">
                  <KeyValue label={t("build.paths.output")} value={settings?.outPath ?? "—"} mono title={settings?.outPath} />
                  <KeyValue label={t("build.paths.work")} value={settings?.workPath ?? "—"} mono title={settings?.workPath} />
                  <KeyValue label={t("home.label.game")} value={settings?.gamePath ?? t("home.noGame")} mono title={settings?.gamePath ?? undefined} />
                </div>
              </CardBody>
            </Card>
          </div>

          <div className="space-y-4">
            <Card>
              <CardHeader title={statusLabel} icon={<Hammer size={13} />} />
              <CardBody className="space-y-3 pt-3">
                <Progress
                  value={buildJob.percent}
                  tone={buildJob.status === "failed" ? "danger" : buildJob.status === "succeeded" ? "ok" : "accent"}
                  height="h-1.5"
                />
                <div className="flex items-center justify-between text-[11.5px]">
                  <span className="truncate text-muted">
                    {buildJob.currentStage
                      ? buildJob.currentStage
                      : buildJob.stageCount > 0
                        ? t("build.progress.stage", {
                            index: Math.min(buildJob.stageIndex + 1, buildJob.stageCount),
                            count: buildJob.stageCount,
                          })
                        : t("build.status.idle")}
                  </span>
                  <span className="shrink-0 font-mono text-[11px] text-faint">
                    {percentLabel(buildJob.percent) ?? "—"}
                  </span>
                </div>

                <div className="grid grid-cols-3 gap-4 border-t border-line-soft pt-2.5">
                  <div>
                    <div className="label-xs">{t("common.time")}</div>
                    <div className="num mt-0.5 text-[13px] text-ink">
                      {elapsedLabel(buildJob.startedAt, buildJob.finishedAt, now)}
                    </div>
                  </div>
                  <div>
                    <div className="label-xs">ETA</div>
                    <div className="num mt-0.5 truncate text-[13px] text-ink">
                      {buildJob.eta ? etaLabel(buildJob.eta) : "—"}
                    </div>
                  </div>
                  <div>
                    <div className="label-xs">{t("build.stages.title")}</div>
                    <div className="num mt-0.5 text-[13px] text-ink">
                      {buildJob.stageCount > 0
                        ? `${Math.min(buildJob.stageIndex + 1, buildJob.stageCount)}/${buildJob.stageCount}`
                        : "—"}
                    </div>
                  </div>
                </div>

                {buildJob.command ? (
                  <p className="truncate font-mono text-[10.5px] text-faint" title={buildJob.command}>
                    {buildJob.command}
                  </p>
                ) : null}

                {buildJob.error ? (
                  <InlineNote tone="danger" icon={<AlertTriangle size={13} />}>
                    {buildJob.error}
                  </InlineNote>
                ) : null}
              </CardBody>
            </Card>

            <LogConsole
              lines={lines.length > 0 ? lines : buildJob.lines}
              title={t("build.log.title")}
              emptyText={t("build.log.empty")}
              maxHeight={430}
            />
          </div>
        </div>
      )}
    </div>
  );
}
