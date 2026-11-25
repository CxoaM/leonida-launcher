import type { ComponentType } from "react";
import { LoaderCircle, TriangleExclamationPoint } from "lucide-react";
import type { PageId } from "@/types";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";
import { TitleBar } from "@/components/layout/TitleBar";
import { Sidebar } from "@/components/layout/Sidebar";
import { StatusBar } from "@/components/layout/StatusBar";
import { Toasts } from "@/components/ui/Toasts";
import { Button } from "@/components/ui/Button";
import { HomePage } from "@/pages/Home";
import { InstallationPage } from "@/pages/Installation";
import { UpdatesPage } from "@/pages/Updates";
import { BuildPage } from "@/pages/Build";
import { DiagnosticsPage } from "@/pages/Diagnostics";
import { SettingsPage } from "@/pages/Settings";
import { AboutPage } from "@/pages/About";

const pages: Record<PageId, ComponentType> = {
  home: HomePage,
  installation: InstallationPage,
  updates: UpdatesPage,
  build: BuildPage,
  diagnostics: DiagnosticsPage,
  settings: SettingsPage,
  about: AboutPage,
};

function BootScreen() {
  const t = useT();
  return (
    <div className="flex h-full flex-col items-start justify-center gap-4 bg-base px-8">
      <span className="text-accent">
        <LoaderCircle size={18} className="animate-spin" />
      </span>
      <div>
        <p className="display text-[18px] text-ink">{t("app.name")}</p>
        <p className="mt-1 text-[12px] text-faint">{t("boot.starting")}</p>
      </div>
    </div>
  );
}

function FatalScreen({ message }: { message: string }) {
  const t = useT();
  return (
    <div className="flex h-full items-start justify-center bg-base px-8 pt-24">
      <div className="card w-full max-w-xl">
        <div className="flex items-start gap-3 border-b border-line-soft px-5 py-4">
          <span className="mt-[1px] shrink-0 text-danger">
            <TriangleExclamationPoint size={16} />
          </span>
          <div className="min-w-0">
            <h1 className="display text-[16px] text-ink">{t("boot.failed.title")}</h1>
            <p className="mt-1 text-[12px] leading-[1.55] text-muted">{t("boot.failed.body")}</p>
          </div>
        </div>
        <pre className="log scroll-thin mx-5 my-4 max-h-48 overflow-auto rounded-[8px] bg-console p-3">
          {message}
        </pre>
        <div className="flex justify-end border-t border-line-soft bg-base-soft px-5 py-3">
          <Button variant="primary" onClick={() => window.location.reload()}>
            {t("boot.retry")}
          </Button>
        </div>
      </div>
    </div>
  );
}

export function App() {
  const { ready, fatal, page } = useApp();

  if (!ready) {
    return (
      <div className="flex h-full flex-col bg-base">
        <TitleBar />
        <BootScreen />
      </div>
    );
  }

  if (fatal) {
    return (
      <div className="flex h-full flex-col bg-base">
        <TitleBar />
        <FatalScreen message={fatal} />
      </div>
    );
  }

  const Page = pages[page];

  return (
    <div className="flex h-full flex-col overflow-hidden bg-base">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main className="scroll-thin min-w-0 flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-[1360px] px-8 py-6">
            <Page />
          </div>
        </main>
      </div>
      <StatusBar />
      <Toasts />
    </div>
  );
}
