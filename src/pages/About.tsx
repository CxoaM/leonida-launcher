import { BookOpen, Copy, ExternalLink, FolderOpen, GitBranch, Scale, ShieldCheck } from "lucide-react";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardBody, CardHeader } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { Button, IconButton } from "@/components/ui/Button";
import { InlineNote, KeyValue } from "@/components/ui/EmptyState";
import { TriangleExclamationPoint } from "lucide-react";
import { openExternal, revealPath, writeClipboard } from "@/lib/tauri";

export function AboutPage() {
  const { metadata, environment, paths, summary, pushToast, reportError } = useApp();
  const t = useT();

  const repositoryUrl = metadata?.repositoryUrl ?? "";
  const upstreamUrl = metadata?.upstreamUrl ?? "";
  const published = Boolean(repositoryUrl) && (metadata?.updaterConfigured ?? false);

  const copyVersion = async () => {
    const text = [
      `${metadata?.productName ?? "Leonida Launcher"} ${metadata?.version ?? ""}`,
      `channel: ${metadata?.channel ?? ""}`,
      `identifier: ${metadata?.identifier ?? ""}`,
      environment ? `os: ${environment.osName} ${environment.osVersion} (${environment.osBuild}) ${environment.arch}` : null,
      summary?.installation.installed
        ? `leonida: ${summary.installation.branch ?? "detached"} @ ${summary.installation.shortCommit ?? ""}`
        : "leonida: not installed",
    ]
      .filter(Boolean)
      .join("\n");
    try {
      await writeClipboard(text);
      pushToast({ tone: "ok", title: t("common.copied") });
    } catch (error) {
      reportError(error, t("common.copy"));
    }
  };

  return (
    <div className="animate-fade-in">
      <PageHeader
        title={t("about.heading")}
        subtitle={t("about.subheading")}
        actions={
          <>
            <Button icon={<Copy size={14} />} onClick={() => void copyVersion()}>
              {t("common.copy")}
            </Button>
            {upstreamUrl ? (
              <Button
                variant="primary"
                icon={<ExternalLink size={14} />}
                onClick={() => void openExternal(upstreamUrl)}
              >
                {t("about.openUpstream")}
              </Button>
            ) : null}
          </>
        }
      />

      <div className="space-y-4">
        <Card>
          <CardBody className="flex flex-wrap items-start justify-between gap-6 px-5 py-4">
            <div className="min-w-0 max-w-[54ch]">
              <h2 className="display text-[21px] leading-[1.2] text-ink">
                {metadata?.productName ?? t("app.name")}
              </h2>
              <p className="mt-1 text-[12px] leading-[1.55] text-muted">{t("app.tagline")}</p>
              <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2 font-mono text-[10.5px] text-faint">
                <span>v{metadata?.version ?? environment?.launcherVersion ?? "—"}</span>
                <span>{metadata?.channel ?? "stable"}</span>
                <span>{metadata?.license ?? "MIT"}</span>
                <Badge tone={published ? "ok" : "warn"}>
                  {published ? t("common.ok") : t("common.warning")}
                </Badge>
              </div>
            </div>
            <div className="grid min-w-[260px] grid-cols-1 gap-x-6">
              <KeyValue label={t("about.identifier")} value={metadata?.identifier ?? "—"} mono />
              <KeyValue
                label={t("about.license")}
                value={metadata?.license ?? "MIT"}
                mono
              />
              <KeyValue
                label={t("about.channel")}
                value={metadata?.channel ?? "stable"}
                mono
              />
              <KeyValue
                label={t("about.owner")}
                value={metadata?.owner && metadata.owner !== "YOUR_GITHUB_OWNER" ? metadata.owner : t("common.notSet")}
                mono
              />
            </div>
          </CardBody>
        </Card>

        {!published ? (
          <InlineNote tone="warn" icon={<TriangleExclamationPoint size={14} />}>
            <strong className="font-medium text-warn">{t("about.notPublished.title")}</strong>
            <span className="mt-0.5 block">{t("about.notPublished.body")}</span>
          </InlineNote>
        ) : null}

        <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-2">
          <Card>
            <CardHeader title={t("about.repository")} icon={<GitBranch size={13} />} />
            <CardBody className="space-y-3 pt-3">
              <div className="flex items-center justify-between gap-3">
                <span className="min-w-0 truncate font-mono text-[11.5px] text-muted" title={repositoryUrl}>
                  {repositoryUrl || t("common.notAvailable")}
                </span>
                {repositoryUrl ? (
                  <Button
                    size="sm"
                    icon={<ExternalLink size={12} />}
                    onClick={() => void openExternal(repositoryUrl)}
                  >
                    {t("common.open")}
                  </Button>
                ) : (
                  <Badge tone="warn">{t("common.notSet")}</Badge>
                )}
              </div>
              <div className="divider" />
              <div className="flex items-center justify-between gap-3">
                <span className="min-w-0 truncate font-mono text-[11.5px] text-muted" title={upstreamUrl}>
                  {upstreamUrl || t("common.notAvailable")}
                </span>
                {upstreamUrl ? (
                  <Button
                    size="sm"
                    icon={<ExternalLink size={12} />}
                    onClick={() => void openExternal(upstreamUrl)}
                  >
                    {t("about.openUpstream")}
                  </Button>
                ) : null}
              </div>
            </CardBody>
          </Card>

          <Card>
            <CardHeader title={t("about.updates.title")} icon={<BookOpen size={13} />} />
            <CardBody className="space-y-3 pt-3">
              <p className="text-[12px] leading-[1.55] text-muted">{t("about.updates.body")}</p>
              <div className="divider" />
              <KeyValue
                label={t("updates.label.installed")}
                value={
                  summary?.installation.installed
                    ? `${summary.installation.branch ?? "detached"} @ ${summary.installation.shortCommit ?? ""}`
                    : t("status.noRepository")
                }
                mono
              />
              <KeyValue
                label={t("about.version")}
                value={metadata?.version ?? environment?.launcherVersion ?? "—"}
                mono
              />
              <KeyValue
                label={t("about.signedUpdates")}
                value={metadata?.updaterConfigured ? t("common.ok") : t("common.notSet")}
              />
            </CardBody>
          </Card>
        </div>

        <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-[1.15fr_1fr]">
          <Card>
            <CardHeader title={t("about.disclaimer.title")} icon={<ShieldCheck size={13} />} />
            <CardBody className="pt-3">
              <p className="text-[12px] leading-[1.6] text-muted">{t("about.disclaimer.body")}</p>
            </CardBody>
          </Card>

          <Card>
            <CardHeader title={t("settings.group.paths")} icon={<FolderOpen size={13} />} />
            <CardBody className="space-y-1 pt-2">
              {environment ? (
                <>
                  <PathRow label={t("about.configDir")} value={environment.configDir} />
                  <PathRow label={t("about.dataDir")} value={environment.dataDir} />
                  <PathRow label={t("about.logsDir")} value={environment.logsDir} />
                </>
              ) : null}
              {paths?.defaultRepository ? (
                <PathRow label={t("settings.paths.repository")} value={paths.defaultRepository} />
              ) : null}
            </CardBody>
          </Card>
        </div>

        <Card>
          <CardHeader title={t("about.credits.title")} icon={<Scale size={13} />} />
          <CardBody className="pt-3">
            <p className="text-[12px] leading-[1.6] text-muted">{t("about.credits.body")}</p>
          </CardBody>
        </Card>
      </div>
    </div>
  );
}

function PathRow({ label, value }: { label: string; value: string }) {
  const t = useT();
  return (
    <div className="flex items-center justify-between gap-3 py-[5px]">
      <span className="shrink-0 text-[12px] text-muted">{label}</span>
      <span className="flex min-w-0 items-center gap-1">
        <span className="truncate font-mono text-[11px] text-ink" title={value}>
          {value}
        </span>
        <IconButton
          label={t("common.open")}
          onClick={() => void revealPath(value)}
          className="h-6 w-6 shrink-0"
        >
          <FolderOpen size={11} />
        </IconButton>
      </span>
    </div>
  );
}
