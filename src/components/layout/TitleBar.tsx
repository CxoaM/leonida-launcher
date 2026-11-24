import { useEffect, useState } from "react";
import { Maximize2, Minimize2, Minus, X } from "lucide-react";
import { appWindow } from "@/lib/tauri";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";
import { cn } from "@/lib/format";

function Mark() {
  return (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" aria-hidden className="shrink-0">
      <defs>
        <linearGradient id="leonida-mark" x1="4" y1="3" x2="20" y2="21" gradientUnits="userSpaceOnUse">
          <stop stopColor="#B8C8FF" />
          <stop offset="1" stopColor="#7E9CF7" />
        </linearGradient>
      </defs>
      <rect x="2.25" y="2.25" width="19.5" height="19.5" rx="5.5" stroke="url(#leonida-mark)" strokeOpacity="0.55" strokeWidth="1.2" />
      <path d="M8 17V8.2L12 12l4-3.8V17" stroke="url(#leonida-mark)" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function TitleBar() {
  const { metadata, summary } = useApp();
  const t = useT();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let off: (() => void) | undefined;
    void appWindow
      .onResized(() => {
        void appWindow.isMaximized().then(setMaximized);
      })
      .then((unlisten) => {
        off = unlisten;
      });
    void appWindow.isMaximized().then(setMaximized);
    return () => off?.();
  }, []);

  return (
    <header className="app-drag relative z-40 flex h-10 shrink-0 items-center justify-between border-b border-line-soft bg-[#0c0f13]">
      <div className="flex min-w-0 items-center gap-2.5 pl-3.5">
        <Mark />
        <span className="text-[12px] font-medium tracking-[-0.005em] text-[#dbe1ea]">
          {t("app.name")}
        </span>
        <span className="font-mono text-[10.5px] text-faint">
          {metadata?.version ?? summary?.metadata.version ?? ""}
        </span>
      </div>

      <div className="no-drag flex h-full items-stretch">
        <button
          type="button"
          aria-label={t("titlebar.minimize")}
          title={t("titlebar.minimize")}
          className="window-control"
          onClick={() => appWindow.minimize()}
        >
          <Minus size={14} />
        </button>
        <button
          type="button"
          aria-label={maximized ? t("titlebar.restore") : t("titlebar.maximize")}
          title={maximized ? t("titlebar.restore") : t("titlebar.maximize")}
          className="window-control"
          onClick={() => {
            void appWindow.toggleMaximize().then(setMaximized);
          }}
        >
          {maximized ? <Minimize2 size={12.5} /> : <Maximize2 size={12.5} />}
        </button>
        <button
          type="button"
          data-kind="close"
          aria-label={t("titlebar.close")}
          title={t("titlebar.close")}
          className={cn("window-control")}
          onClick={() => appWindow.close()}
        >
          <X size={14} />
        </button>
      </div>
    </header>
  );
}
