import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  CircleCheck as CheckCircle2,
  Clock,
  GitBranch,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
  Trash,
  TriangleExclamationPoint as AlertTriangle,
  Zap,
} from "lucide-react";
import type {
  AppError,
  CommitInfo,
  RestorePoint,
  UpdateHistoryEntry,
  UpstreamCheck,
} from "@/types";
import { api, toIpcError } from "@/services/ipc";
import { useApp } from "@/state/AppContext";
import { dateLocale, useI18n } from "@/i18n";
import type { MessageKey } from "@/i18n/dictionary";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardBody, CardFooter, CardHeader } from "@/components/ui/Card";
import { Badge, Dot, Pill } from "@/components/ui/Badge";
import { Button, IconButton } from "@/components/ui/Button";
import { EmptyState, InlineNote, KeyValue } from "@/components/ui/EmptyState";
import { Modal } from "@/components/ui/Modal";
import { Progress, SkeletonRows } from "@/components/ui/Progress";
import { StepList, type StepDescriptor } from "@/components/StepList";
import { relativeTime, shortCommit } from "@/lib/format";

const STAGES = ["preflight", "fetch", "backup", "merge", "submodules", "verify", "done"] as const;

function CommitRow({ commit, tone }: { commit: CommitInfo; tone?: "accent" }) {
  return (
    <li className="flex items-start gap-2.5 py-[7px]">
      <span className="mt-[3px]">
        <Dot tone={tone ?? "neutral"} />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12.5px] text-ink" title={commit.subject}>
          {commit.subject}
        </span>
        <span className="text-[10.5px] text-faint">
          {commit.shortSha} · {commit.author} · {commit.dateHuman}
        </span>
      </span>
    </li>
  );
}

export function UpdatesPage() {
  const {
    summary,
    compatibility,
    upstreamCheck,
    setUpstreamCheck,
    runUpstreamCheck,
    refreshSummary,
    refreshCompatibility,
    updateProgress,
    settings,
    reportError,
    pushToast,
    setPage,
  } = useApp();
  const { t, resolved } = useI18n();
  const locale = dateLocale(resolved);

  const [checking, setChecking] = useState(false);
  const [applying, setApplying] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [restorePoints, setRestorePoints] = useState<RestorePoint[]>([]);
  const [history, setHistory] = useState<UpdateHistoryEntry[]>([]);
  const [loadingMeta, setLoadingMeta] = useState(true);

  const [showApply, setShowApply] = useState(false);
  const [allowUnverified, setAllowUnverified] = useState(false);
  const [verifyError, setVerifyError] = useState<AppError | null>(null);
  const [pendingRestore, setPendingRestore] = useState<RestorePoint | null>(null);
  const [showClear, setShowClear] = useState(false);

  const installed = summary?.installation.installed ?? false;
  const dirty = (upstreamCheck?.modifiedFiles ?? summary?.installation.dirtyFiles ?? 0) > 0;

  const loadMeta = useCallback(async () => {
    setLoadingMeta(true);
    try {
      const [points, entries] = await Promise.all([
        api.restorePoints().catch(() => [] as RestorePoint[]),
        api.updateHistory().catch(() => [] as UpdateHistoryEntry[]),
      ]);
      setRestorePoints(points);
      setHistory(entries);
    } finally {
      setLoadingMeta(false);
    }
  }, []);

  useEffect(() => {
    void loadMeta();
  }, [loadMeta]);

  useEffect(() => {
    setAllowUnverified(settings?.advanced.allowUnverifiedUpstream ?? false);
  }, [settings?.advanced.allowUnverifiedUpstream]);

  const check = async () => {
    setChecking(true);
    try {
      const result = await runUpstreamCheck();
      await loadMeta();
      if (!result.updateAvailable) {
        pushToast({ tone: "ok", title: t("updates.state.upToDate") });
      }
      await refreshCompatibility();
    } catch (error) {
      reportError(error, t("updates.heading"));
    } finally {
      setChecking(false);
    }
  };

  const apply = async (allow: boolean) => {
    setApplying(true);
    setShowApply(false);
    setVerifyError(null);
    try {
      const outcome = await api.applyUpdate(allow);
      pushToast({ tone: "ok", title: t("updates.updated"), message: outcome.message });
      await Promise.all([refreshSummary(), refreshCompatibility(), loadMeta()]);
      setUpstreamCheck(null);
    } catch (error) {
      const ipc = toIpcError(error);
      if (ipc.code === "verification_required") {
        setVerifyError({ code: ipc.code, message: ipc.message, detail: ipc.detail ?? null });
      } else if (ipc.code === "incompatible_upstream") {
        setVerifyError({ code: ipc.code, message: ipc.message, detail: ipc.detail ?? null });
      } else {
        reportError(error, t("updates.heading"));
      }
      await Promise.all([refreshSummary(), refreshCompatibility(), loadMeta()]);
      setUpstreamCheck(null);
    } finally {
      setApplying(false);
    }
  };

  const restore = async (point: RestorePoint) => {
    setRestoring(true);
    setPendingRestore(null);
    try {
      const outcome = await api.rollbackUpdate(point.backupRef);
      pushToast({ tone: "ok", title: t("updates.rolledBack"), message: outcome.message });
      await Promise.all([refreshSummary(), refreshCompatibility(), loadMeta()]);
      setUpstreamCheck(null);
    } catch (error) {
      reportError(error, t("updates.action.rollback"));
    } finally {
      setRestoring(false);
    }
  };

  const clearHistory = async () => {
    setShowClear(false);
    try {
      await api.clearUpdateHistory();
      setHistory([]);
    } catch (error) {
      reportError(error, t("updates.action.clearHistory"));
    }
  };

  const stageIndex = useMemo(() => {
    if (!updateProgress) return -1;
    const index = STAGES.indexOf(updateProgress.stage as (typeof STAGES)[number]);
    return index === -1 ? STAGES.length - 1 : index;
  }, [updateProgress]);

  const stageSteps: StepDescriptor[] = STAGES.map((id, index) => ({
    id,
    label: t(`updates.stage.${id}` as MessageKey),
    status: !applying
      ? "pending"
      : stageIndex === -1
        ? "pending"
        : index < stageIndex || updateProgress?.stage === "done"
          ? "done"
          : index === stageIndex
            ? "running"
            : "pending",
    detail: index === stageIndex ? updateProgress?.detail : undefined,
  }));

  const stateBadge = (input: UpstreamCheck | null) => {
    if (!input) return null;
    if (input.behind > 0) return <Badge tone="accent">{t("updates.state.available")}</Badge>;
    if (input.ahead > 0) return <Badge tone="warn">{t("updates.state.ahead")}</Badge>;
    return <Badge tone="ok">{t("updates.state.upToDate")}</Badge>;
  };

  const compatibilityStatus = compatibility?.status ?? "unknown";

  return (
    <div className="animate-fade-in">
      <PageHeader
        title={t("updates.heading")}
        subtitle={t("updates.subheading")}
        actions={
          <>
            <Button
              icon={<RefreshCw size={14} className={checking ? "animate-spin" : ""} />}
              loading={checking}
              disabled={!installed || applying}
              onClick={() => void check()}
            >
              {t("updates.action.check")}
            </Button>
            <Button
              variant="primary"
              icon={<Zap size={14} />}
              disabled={!installed || !upstreamCheck?.updateAvailable || applying || dirty}
              loading={applying}
              onClick={() => setShowApply(true)}
            >
              {t("updates.action.apply")}
            </Button>
          </>
        }
      />

      {!installed ? (
        <Card>
          <EmptyState
            icon={<GitBranch size={16} />}
            title={t("updates.notInstalled.title")}
            body={t("updates.notInstalled.body")}
            action={
              <Button variant="primary" onClick={() => setPage("installation")}>
                {t("install.heading")}
              </Button>
            }
          />
        </Card>
      ) : (
        <div className="space-y-4">
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1.25fr_1fr]">
            <Card>
              <CardHeader
                title={`${t("updates.heading")} · ${t("updates.label.installed")}`}
                icon={<GitBranch size={13} />}
                action={stateBadge(upstreamCheck)}
              />
              <CardBody className="pt-2">
                {!upstreamCheck ? (
                  checking ? (
                    <SkeletonRows rows={4} />
                  ) : (
                    <EmptyState
                      compact
                      icon={<Clock size={15} />}
                      title={t("home.updatesUnknown")}
                      body={t("updates.subheading")}
                      action={
                        <Button size="sm" onClick={() => void check()} loading={checking}>
                          {t("updates.action.check")}
                        </Button>
                      }
                    />
                  )
                ) : (
                  <>
                    <div className="flex flex-wrap items-center gap-2">
                      {upstreamCheck.behind > 0 ? (
                        <Badge tone="accent">
                          {t(upstreamCheck.behind === 1 ? "updates.behind.one" : "updates.behind.many", {
                            count: upstreamCheck.behind,
                          })}
                        </Badge>
                      ) : null}
                      {upstreamCheck.ahead > 0 ? (
                        <Badge tone="warn">
                          {t(upstreamCheck.ahead === 1 ? "updates.ahead.one" : "updates.ahead.many", {
                            count: upstreamCheck.ahead,
                          })}
                        </Badge>
                      ) : null}
                      <Badge tone={upstreamCheck.remoteIsOfficial ? "ok" : "warn"}>
                        {upstreamCheck.remoteIsOfficial ? t("updates.official") : t("updates.unofficial")}
                      </Badge>
                      <Badge tone={dirty ? "warn" : "neutral"}>
                        {dirty ? t("updates.local") : t("updates.clean")}
                      </Badge>
                      <Pill>{relativeTime(upstreamCheck.checkedAt, t, locale)}</Pill>
                    </div>

                    <div className="mt-3 divide-y divide-[#1d242d]">
                      <KeyValue
                        label={t("updates.label.installed")}
                        value={`${shortCommit(upstreamCheck.installedCommit)} · ${upstreamCheck.installedSubject ?? "—"}`}
                        title={upstreamCheck.installedCommit ?? undefined}
                      />
                      <KeyValue
                        label={t("updates.label.latest")}
                        value={`${shortCommit(upstreamCheck.latestCommit)} · ${upstreamCheck.latestSubject ?? "—"}`}
                        title={upstreamCheck.latestCommit ?? undefined}
                      />
                      <KeyValue label={t("updates.label.remote")} value={upstreamCheck.remoteUrl ?? "—"} mono />
                      <KeyValue
                        label={t("home.label.compatibility")}
                        value={t(`compat.${compatibilityStatus}` as MessageKey)}
                      />
                    </div>

                    {dirty ? (
                      <InlineNote tone="warn" icon={<AlertTriangle size={13} />} className="mt-3">
                        <span className="font-medium">{t("updates.state.blocked")}. </span>
                        {t("updates.dirty.body")}
                      </InlineNote>
                    ) : null}

                    {upstreamCheck.commits.length > 0 ? (
                      <div className="mt-4">
                        <div className="label-xs mb-1">{t("updates.label.commits")}</div>
                        <ul className="divide-y divide-[#1d242d]">
                          {upstreamCheck.commits.slice(0, 8).map((commit) => (
                            <CommitRow key={commit.sha} commit={commit} tone="accent" />
                          ))}
                        </ul>
                      </div>
                    ) : null}

                    <p className="mt-3 text-[11px] leading-[1.5] text-faint">
                      <span className="text-muted">{t("updates.channelNote")}: </span>
                      {upstreamCheck.channelNote}
                    </p>
                  </>
                )}
              </CardBody>
            </Card>

            <div className="space-y-4">
              <Card>
                <CardHeader title={t("updates.stage.backup")} icon={<ShieldCheck size={13} />} />
                <CardBody className="space-y-2.5 pt-3">
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-[12px] text-muted">
                      {applying
                        ? updateProgress?.label ?? t("status.busy")
                        : t("build.status.idle")}
                    </span>
                    {applying ? <Badge tone="accent">{t("jobs.running")}</Badge> : null}
                  </div>
                  <Progress value={null} className={applying ? "" : "opacity-40"} />
                  <StepList steps={stageSteps} className="pt-1" />
                  {updateProgress && applying ? (
                    <p className="truncate font-mono text-[10.5px] text-faint" title={updateProgress.detail}>
                      {updateProgress.detail}
                    </p>
                  ) : null}
                </CardBody>
              </Card>

              <Card>
                <CardHeader
                  title={t("updates.verify.title")}
                  icon={<ShieldCheck size={13} />}
                  action={
                    <Badge tone={compatibilityStatus === "compatible" ? "ok" : compatibilityStatus === "incompatible" ? "danger" : "warn"}>
                      {t(`compat.${compatibilityStatus}` as MessageKey)}
                    </Badge>
                  }
                />
                <CardBody className="pt-3">
                  <p className="text-[11.5px] leading-[1.5] text-muted">{t("updates.verify.body")}</p>
                  <div className="mt-3 flex items-center gap-2">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => void refreshCompatibility()}
                    >
                      {t("install.action.recheck")}
                    </Button>
                  </div>
                </CardBody>
              </Card>
            </div>
          </div>

          <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
            <Card>
              <CardHeader
                title={t("updates.restore.title")}
                icon={<RotateCcw size={13} />}
                action={<Pill>{restorePoints.length}</Pill>}
              />
              <CardBody className="pt-2">
                {loadingMeta ? (
                  <SkeletonRows rows={2} />
                ) : restorePoints.length === 0 ? (
                  <EmptyState compact title={t("updates.restore.empty")} body={t("updates.restore.hint")} />
                ) : (
                  <ul className="divide-y divide-[#1d242d]">
                    {restorePoints.slice(0, 6).map((point) => (
                      <li key={point.backupRef} className="flex items-center justify-between gap-3 py-2">
                        <span className="min-w-0">
                          <span className="block truncate font-mono text-[11.5px] text-ink">
                            {point.backupRef}
                          </span>
                          <span className="text-[10.5px] text-faint">
                            {shortCommit(point.commit)} · {relativeTime(point.createdAt, t, locale)}
                          </span>
                        </span>
                        <Button
                          size="sm"
                          variant="ghost"
                          icon={<RotateCcw size={12} />}
                          onClick={() => setPendingRestore(point)}
                          disabled={applying || restoring}
                        >
                          {t("updates.action.rollback")}
                        </Button>
                      </li>
                    ))}
                  </ul>
                )}
              </CardBody>
            </Card>

            <Card>
              <CardHeader
                title={t("updates.history.title")}
                icon={<Clock size={13} />}
                action={
                  history.length > 0 ? (
                    <IconButton label={t("common.clear")} onClick={() => setShowClear(true)}>
                      <Trash size={13} />
                    </IconButton>
                  ) : undefined
                }
              />
              <CardBody className="pt-2">
                {loadingMeta ? (
                  <SkeletonRows rows={2} />
                ) : history.length === 0 ? (
                  <EmptyState compact title={t("updates.history.empty")} />
                ) : (
                  <ul className="divide-y divide-[#1d242d]">
                    {history.slice(0, 8).map((entry) => (
                      <li key={entry.id} className="flex items-start justify-between gap-3 py-2">
                        <span className="min-w-0">
                          <span className="flex items-center gap-2">
                            <Badge tone={entry.action === "rollback" ? "warn" : "accent"}>
                              {entry.action === "rollback" ? t("updates.history.rollback") : t("updates.history.update")}
                            </Badge>
                            <Badge
                              tone={
                                entry.result === "success"
                                  ? "ok"
                                  : entry.result === "rolled_back"
                                    ? "warn"
                                    : "danger"
                              }
                            >
                              {t(`updates.history.${entry.result}` as MessageKey)}
                            </Badge>
                          </span>
                          <span className="mt-1 block truncate text-[11px] text-muted" title={entry.message}>
                            {entry.message}
                          </span>
                          <span className="text-[10.5px] text-faint">
                            {shortCommit(entry.fromCommit)} <ArrowRight size={9} className="inline" />{" "}
                            {shortCommit(entry.toCommit)} · {relativeTime(entry.at, t, locale)}
                          </span>
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </CardBody>
              <CardFooter className="text-[10.5px] text-faint">
                <span>{history.length > 0 ? relativeTime(history[0].at, t, locale) : ""}</span>
                <span className="font-mono">{history[0]?.adapterVersion ?? ""}</span>
              </CardFooter>
            </Card>
          </div>
        </div>
      )}

      <Modal
        open={showApply}
        onClose={() => setShowApply(false)}
        title={t("updates.verify.title")}
        description={t("updates.verify.body")}
        footer={
          <>
            <Button variant="ghost" onClick={() => setShowApply(false)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              icon={<ShieldCheck size={14} />}
              onClick={() => void apply(allowUnverified)}
            >
              {t("updates.verify.confirm")}
            </Button>
          </>
        }
      >
        <div className="space-y-2">
          <KeyValue
            label={t("updates.label.installed")}
            value={shortCommit(upstreamCheck?.installedCommit)}
            mono
          />
          <KeyValue label={t("updates.label.latest")} value={shortCommit(upstreamCheck?.latestCommit)} mono />
          <KeyValue
            label={t("updates.label.commits")}
            value={String(upstreamCheck?.behind ?? 0)}
            mono
          />
          <label className="mt-3 flex cursor-pointer items-start gap-2.5 rounded-[8px] border border-line-soft bg-[#11161c] p-2.5">
            <input
              type="checkbox"
              checked={allowUnverified}
              onChange={(event) => setAllowUnverified(event.target.checked)}
              className="mt-[2px] accent-[#8EA9FF]"
            />
            <span className="text-[11.5px] leading-[1.5] text-muted">
              {t("updates.verify.allowUnverified")}
            </span>
          </label>
        </div>
      </Modal>

      <Modal
        open={Boolean(verifyError)}
        onClose={() => setVerifyError(null)}
        tone="warn"
        title={t("updates.verify.title")}
        description={verifyError?.message}
        footer={
          <>
            <Button variant="ghost" onClick={() => setVerifyError(null)}>
              {t("common.dismiss")}
            </Button>
            {verifyError?.code === "verification_required" ? (
              <Button variant="primary" onClick={() => void apply(true)} loading={applying}>
                {t("updates.action.applyUnverified")}
              </Button>
            ) : null}
          </>
        }
      >
        <div className="space-y-2">
          <InlineNote tone="warn" icon={<CheckCircle2 size={13} />}>
            {t("updates.verify.body")}
          </InlineNote>
          {verifyError?.detail ? (
            <pre className="log max-h-52 overflow-auto whitespace-pre-wrap rounded-[8px] bg-[#0a0d11] p-2.5 text-[10.5px] text-muted">
              {verifyError.detail}
            </pre>
          ) : null}
        </div>
      </Modal>

      <Modal
        open={Boolean(pendingRestore)}
        onClose={() => setPendingRestore(null)}
        tone="warn"
        title={t("updates.action.rollback")}
        description={pendingRestore?.backupRef}
        footer={
          <>
            <Button variant="ghost" onClick={() => setPendingRestore(null)}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="primary"
              icon={<RotateCcw size={14} />}
              onClick={() => pendingRestore && void restore(pendingRestore)}
            >
              {t("common.confirm")}
            </Button>
          </>
        }
      >
        <KeyValue label={t("updates.label.installed")} value={shortCommit(pendingRestore?.commit)} mono />
      </Modal>

      <Modal
        open={showClear}
        onClose={() => setShowClear(false)}
        tone="danger"
        title={t("updates.action.clearHistory")}
        footer={
          <>
            <Button variant="ghost" onClick={() => setShowClear(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="danger" onClick={() => void clearHistory()}>
              {t("common.clear")}
            </Button>
          </>
        }
      />
    </div>
  );
}
