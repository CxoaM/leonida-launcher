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
    <div className="flex h-full flex-col items-center justify-center gap-3 bg-base">
      <span className="flex h-11 w-11 items-center justify-center rounded-[13px] border border-accent-line bg-accent-soft text-accent">
        <LoaderCircle size={20} className="animate-spin" />
      </span>
      <div className="text-center">
        <p className="text-[13px] font-medium text-ink">{t("app.name")}</p>
        <p className="mt-0.5 text-[11.5px] text-faint">{t("boot.starting")}</p>
      </div>
    </div>
  );
}

function FatalScreen({ message }: { message: string }) {
  const t = useT();
  return (
    <div className="flex h-full items-center justify-center bg-base p-8">
      <div className="card w-full max-w-xl overflow-hidden">
        <div className="flex items-start gap-3 px-5 py-4">
          <span className="mt-[1px] flex h-8 w-8 shrink-0 items-center justify-center rounded-[9px] border border-[#482a2d] bg-[#241618] text-danger">
            <TriangleExclamationPoint size={15} />
          </span>
          <div className="min-w-0">
            <h1 className="text-[14px] font-semibold text-ink">{t("boot.failed.title")}</h1>
            <p className="mt-1 text-[12px] leading-[1.55] text-muted">{t("boot.failed.body")}</p>
          </div>
        </div>
        <pre className="log scroll-thin mx-5 mb-4 max-h-48 overflow-auto rounded-[8px] border border-line-soft bg-[#090c10] p-2.5">
          {message}
        </pre>
        <div className="flex justify-end border-t border-line-soft bg-[#12171e] px-5 py-3">
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
          <div className="mx-auto w-full max-w-[1180px] px-6 py-5">
            <Page />
          </div>
        </main>
      </div>
      <StatusBar />
      <Toasts />
    </div>
  );
}
