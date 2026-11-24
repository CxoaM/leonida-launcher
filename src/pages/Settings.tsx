import { useEffect, useRef, useState } from "react";
import {
  Download,
  FolderOpen,
  Hammer,
  Languages,
  RefreshCw,
  RotateCcw,
  Save,
  Settings2,
  SlidersHorizontal,
  Sparkles,
  TriangleExclamationPoint,
} from "lucide-react";
import type { Settings } from "@/types";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardBody, CardFooter, CardHeader } from "@/components/ui/Card";
import { Badge, Pill } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Field, PathField, Segmented, Toggle } from "@/components/ui/Field";
import { InlineNote, KeyValue } from "@/components/ui/EmptyState";
import { Modal } from "@/components/ui/Modal";
import { Progress } from "@/components/ui/Progress";
import { bytes } from "@/lib/format";
import { revealPath } from "@/lib/tauri";

function NumberInput({
  value,
  min,
  max,
  onCommit,
  disabled = false,
  suffix,
  label,
}: {
  value: number;
  min: number;
  max: number;
  onCommit: (value: number) => void;
  disabled?: boolean;
  suffix?: string;
  label: string;
}) {
  const [draft, setDraft] = useState(String(value));
  const focused = useRef(false);

  useEffect(() => {
    if (!focused.current) setDraft(String(value));
  }, [value]);

  const commit = () => {
    const parsed = Number.parseInt(draft, 10);
    const next = Number.isFinite(parsed) ? Math.min(max, Math.max(min, parsed)) : value;
    setDraft(String(next));
    if (next !== value) onCommit(next);
  };

  return (
    <div className="flex items-center gap-2">
      <input
        aria-label={label}
        className="input input-mono w-[104px] text-right"
        inputMode="numeric"
        value={draft}
        disabled={disabled}
        onChange={(event) => setDraft(event.target.value)}
        onFocus={() => {
          focused.current = true;
        }}
        onBlur={() => {
          focused.current = false;
          commit();
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
            (event.target as HTMLInputElement).blur();
          }
          if (event.key === "Escape") {
            setDraft(String(value));
            (event.target as HTMLInputElement).blur();
          }
        }}
      />
      {suffix ? <span className="text-[11.5px] text-faint">{suffix}</span> : null}
    </div>
  );
}

export function SettingsPage() {
  const {
    settings,
    saving,
    updateSettings,
    resetSettings,
    metadata,
    paths,
    summary,
    reportError,
    pushToast,
    launcherRelease,
    launcherProgress,
    checkLauncherUpdate,
    installLauncherUpdate,
  } = useApp();
  const t = useT();

  const [resetOpen, setResetOpen] = useState(false);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);

  if (!settings) {
    return (
      <div className="animate-fade-in">
        <PageHeader title={t("settings.heading")} subtitle={t("settings.subheading")} />
        <Card>
          <CardBody>
            <p className="text-[12.5px] text-muted">{t("common.loading")}</p>
          </CardBody>
        </Card>
      </div>
    );
  }

  const patch = (changes: Partial<Settings>) => updateSettings(changes);

  const checkUpdate = async () => {
    setChecking(true);
    try {
      const release = await checkLauncherUpdate();
      pushToast({
        tone: release.available ? "accent" : "ok",
        title: release.available
          ? t("settings.updater.available", { version: release.version ?? "" })
          : t("settings.updater.current"),
        message: release.message || undefined,
      });
    } catch (error) {
      reportError(error, t("settings.updater.check"));
    } finally {
      setChecking(false);
    }
  };

  const installUpdate = async () => {
    setInstalling(true);
    try {
      await installLauncherUpdate();
      pushToast({ tone: "accent", title: t("settings.updater.restart") });
    } finally {
      setInstalling(false);
    }
  };

  const progressPercent =
    launcherProgress && launcherProgress.total && launcherProgress.total > 0
      ? (launcherProgress.downloaded / launcherProgress.total) * 100
      : null;

  return (
    <div className="animate-fade-in">
      <PageHeader
        title={t("settings.heading")}
        subtitle={t("settings.subheading")}
        actions={
          <Badge tone={saving ? "accent" : "neutral"}>
            <Save size={11} />
            {saving ? t("common.saving") : t("common.saved")}
          </Badge>
        }
      />

      <div className="space-y-4">
        <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <Card>
            <CardHeader
              title={t("settings.group.general")}
              icon={<Languages size={13} />}
              description={`${t("settings.language")} · ${t("settings.launchOnStartup")}`}
            />
            <CardBody className="space-y-3 pt-3">
              <Field label={t("settings.language")}>
                <Segmented
                  value={settings.language}
                  onChange={(language) => patch({ language })}
                  options={[
                    { value: "system", label: t("settings.language.system") },
                    { value: "en", label: "English" },
                    { value: "ru", label: "Русский" },
                  ]}
                />
              </Field>

              <div className="divider" />

              <Toggle
                checked={settings.launchOnStartup}
                onChange={(launchOnStartup) => patch({ launchOnStartup })}
                label={t("settings.launchOnStartup")}
                description={t("settings.launchOnStartup.hint")}
              />
              <Toggle
                checked={settings.checkLauncherUpdatesOnStart}
                onChange={(checkLauncherUpdatesOnStart) => patch({ checkLauncherUpdatesOnStart })}
                label={t("settings.checkLauncherUpdatesOnStart")}
              />
            </CardBody>
          </Card>

          <Card>
            <CardHeader
              title={t("nav.updates")}
              icon={<RefreshCw size={13} />}
              description={t("updates.subheading")}
            />
            <CardBody className="space-y-3 pt-3">
              <Toggle
                checked={settings.leonidaAutoCheck}
                onChange={(leonidaAutoCheck) => patch({ leonidaAutoCheck })}
                label={t("settings.leonidaAutoCheck")}
              />
              <Field
                label={t("settings.leonidaCheckInterval")}
                hint={t("settings.leonidaAutoCheck.hint")}
              >
                <NumberInput
                  label={t("settings.leonidaCheckInterval")}
                  value={settings.leonidaCheckIntervalMinutes}
                  min={15}
                  max={10080}
                  disabled={!settings.leonidaAutoCheck}
                  suffix={t("common.minutes")}
                  onCommit={(leonidaCheckIntervalMinutes) => patch({ leonidaCheckIntervalMinutes })}
                />
              </Field>
              <Toggle
                checked={settings.leonidaNotifications}
                onChange={(leonidaNotifications) => patch({ leonidaNotifications })}
                label={t("settings.leonidaNotifications")}
                disabled={!settings.leonidaAutoCheck}
              />
              <InlineNote tone="neutral" icon={<TriangleExclamationPoint size={13} />}>
                {t("updates.channelNote")}: {summary?.installation.branch ?? "main"} ·{" "}
                {summary?.installation.shortCommit ?? t("status.noRepository")}
              </InlineNote>
            </CardBody>
          </Card>
        </div>

        <Card>
          <CardHeader title={t("settings.group.paths")} icon={<FolderOpen size={13} />} />
          <CardBody className="grid grid-cols-1 gap-4 pt-3 lg:grid-cols-2">
            <Field label={t("settings.paths.repository")} hint={t("install.form.repository.hint")}>
              <PathField
                value={settings.repositoryPath ?? ""}
                placeholder={t("common.notSet")}
                onCommit={(value) => patch({ repositoryPath: value.trim() ? value : null })}
              />
            </Field>
            <Field label={t("settings.paths.game")} hint={t("install.form.game.hint")}>
              <PathField
                value={settings.gamePath ?? ""}
                placeholder={t("common.notSet")}
                onCommit={(value) => patch({ gamePath: value.trim() ? value : null })}
              />
            </Field>
            <Field label={t("settings.paths.out")} hint={t("install.form.out.hint")}>
              <PathField value={settings.outPath} onCommit={(outPath) => patch({ outPath })} />
            </Field>
            <Field label={t("settings.paths.work")} hint={t("install.form.work.hint")}>
              <PathField value={settings.workPath} onCommit={(workPath) => patch({ workPath })} />
            </Field>
            <Field label={t("settings.paths.cache")} hint={t("settings.paths.readonly")}>
              <PathField
                value={settings.cachePath}
                disabled
                allowReveal
                onCommit={() => undefined}
              />
            </Field>
          </CardBody>
          <CardFooter>
            <span className="truncate font-mono text-[11px] text-faint" title={paths?.dataDir}>
              {paths?.dataDir}
            </span>
            <Button
              size="sm"
              variant="ghost"
              icon={<FolderOpen size={12} />}
              disabled={!paths?.logsDir}
              onClick={() => paths?.logsDir && void revealPath(paths.logsDir)}
            >
              {t("diag.logs.open")}
            </Button>
          </CardFooter>
        </Card>

        <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <Card>
            <CardHeader title={t("settings.group.build")} icon={<Hammer size={13} />} />
            <CardBody className="space-y-3 pt-3">
              <Field label={t("settings.jobs")} hint={t("build.options.jobs.hint")}>
                <NumberInput
                  label={t("settings.jobs")}
                  value={settings.jobs}
                  min={0}
                  max={256}
                  onCommit={(jobs) => patch({ jobs })}
                />
              </Field>
              <Field label={t("settings.wave64")}>
                <Segmented
                  value={settings.wave64Emulate}
                  onChange={(wave64Emulate) => patch({ wave64Emulate })}
                  options={[
                    { value: "auto", label: t("build.options.wave64.auto") },
                    { value: "on", label: t("build.options.wave64.on") },
                    { value: "off", label: t("build.options.wave64.off") },
                  ]}
                />
              </Field>
              <div className="divider" />
              <Field label={t("settings.logRetention")} hint={t("settings.logRetention.hint")}>
                <NumberInput
                  label={t("settings.logRetention")}
                  value={settings.logRetentionDays}
                  min={1}
                  max={365}
                  suffix={t("common.days")}
                  onCommit={(logRetentionDays) => patch({ logRetentionDays })}
                />
              </Field>
              <Toggle
                checked={settings.verboseLogging}
                onChange={(verboseLogging) => patch({ verboseLogging })}
                label={t("settings.verbose")}
                description={t("settings.verbose.hint")}
              />
            </CardBody>
          </Card>

          <div className="space-y-4">
            <Card>
              <CardHeader title={t("settings.group.advanced")} icon={<SlidersHorizontal size={13} />} />
              <CardBody className="space-y-3 pt-3">
                <Toggle
                  checked={settings.advanced.allowUnverifiedUpstream}
                  onChange={(allowUnverifiedUpstream) =>
                    patch({ advanced: { ...settings.advanced, allowUnverifiedUpstream } })
                  }
                  label={t("settings.advanced.allowUnverified")}
                  description={t("settings.advanced.allowUnverified.hint")}
                />
                <Field
                  label={t("settings.advanced.keepRestorePoints")}
                  hint={t("settings.advanced.keepRestorePoints.hint")}
                >
                  <NumberInput
                    label={t("settings.advanced.keepRestorePoints")}
                    value={settings.advanced.keepRestorePoints}
                    min={1}
                    max={25}
                    onCommit={(keepRestorePoints) =>
                      patch({ advanced: { ...settings.advanced, keepRestorePoints } })
                    }
                  />
                </Field>
              </CardBody>
            </Card>

            <Card>
              <CardHeader title={t("settings.group.danger")} icon={<RotateCcw size={13} />} />
              <CardBody className="pt-3">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <p className="max-w-[46ch] text-[11.5px] leading-[1.5] text-muted">
                    {t("settings.danger.reset.hint")}
                  </p>
                  <Button
                    variant="danger"
                    icon={<RotateCcw size={14} />}
                    onClick={() => setResetOpen(true)}
                  >
                    {t("settings.danger.reset")}
                  </Button>
                </div>
              </CardBody>
            </Card>
          </div>
        </div>

        <Card>
          <CardHeader
            title={t("settings.updater.heading")}
            icon={<Sparkles size={13} />}
            description={
              metadata?.updaterConfigured
                ? t("settings.updater.configured")
                : t("settings.updater.title")
            }
            action={
              <Badge tone={metadata?.updaterConfigured ? "ok" : "warn"}>
                {metadata?.updaterConfigured ? t("common.ok") : t("common.warning")}
              </Badge>
            }
          />
          <CardBody className="pt-3">
            {metadata?.updaterConfigured ? (
              <div className="space-y-3">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <Pill>
                      {t("settings.updater.current")} · {launcherRelease?.currentVersion ?? metadata.version}
                    </Pill>
                    {launcherRelease?.available && launcherRelease.version ? (
                      <Badge tone="accent">
                        {t("settings.updater.available", { version: launcherRelease.version })}
                      </Badge>
                    ) : null}
                    {launcherRelease?.downloadBytes ? (
                      <Pill>{bytes(launcherRelease.downloadBytes)}</Pill>
                    ) : null}
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      size="sm"
                      icon={<RefreshCw size={13} className={checking ? "animate-spin" : ""} />}
                      loading={checking}
                      onClick={() => void checkUpdate()}
                    >
                      {t("settings.updater.check")}
                    </Button>
                    {launcherRelease?.available ? (
                      <Button
                        size="sm"
                        variant="primary"
                        icon={<Download size={13} />}
                        loading={installing}
                        onClick={() => void installUpdate()}
                      >
                        {installing ? t("settings.updater.installing") : t("settings.updater.install")}
                      </Button>
                    ) : null}
                  </div>
                </div>

                {launcherRelease?.notes ? (
                  <pre className="log scroll-thin max-h-40 overflow-auto rounded-[8px] border border-line-soft bg-[#090c10] p-2.5">
                    {launcherRelease.notes}
                  </pre>
                ) : null}

                {launcherProgress ? (
                  <div className="space-y-1.5">
                    <Progress value={progressPercent} />
                    <p className="text-[11.5px] text-muted">
                      {launcherProgress.message ||
                        (launcherProgress.total
                          ? `${bytes(launcherProgress.downloaded)} / ${bytes(launcherProgress.total)}`
                          : bytes(launcherProgress.downloaded))}
                    </p>
                  </div>
                ) : null}

                <InlineNote tone="neutral" icon={<Settings2 size={13} />}>
                  {t("about.updates.body")}
                </InlineNote>
              </div>
            ) : (
              <div className="space-y-3">
                <InlineNote tone="warn" icon={<TriangleExclamationPoint size={13} />}>
                  {t("settings.updater.body")}
                </InlineNote>
                <KeyValue label={t("about.version")} value={metadata?.version ?? "—"} mono />
                <KeyValue label={t("about.channel")} value={metadata?.channel ?? "—"} />
              </div>
            )}
          </CardBody>
        </Card>
      </div>

      <Modal
        open={resetOpen}
        tone="danger"
        title={t("settings.danger.confirm.title")}
        description={t("settings.danger.confirm.body")}
        onClose={() => setResetOpen(false)}
        footer={
          <>
            <Button onClick={() => setResetOpen(false)}>{t("common.cancel")}</Button>
            <Button
              variant="danger"
              icon={<RotateCcw size={14} />}
              onClick={() => {
                setResetOpen(false);
                void resetSettings();
              }}
            >
              {t("common.reset")}
            </Button>
          </>
        }
      />
    </div>
  );
}
