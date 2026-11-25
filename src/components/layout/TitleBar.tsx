import { useEffect, useState } from "react";
import { Maximize2, Minimize2, Minus, X } from "lucide-react";
import { appWindow } from "@/lib/tauri";
import { useApp } from "@/state/AppContext";
import { useT } from "@/i18n";

function Mark() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden className="shrink-0">
      <rect x="1.5" y="2.5" width="13" height="2.5" fill="currentColor" />
      <rect x="1.5" y="6.75" width="9" height="2.5" fill="currentColor" opacity="0.55" />
      <rect x="1.5" y="11" width="5.5" height="2.5" fill="var(--color-accent)" />
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
    <header className="app-drag relative z-40 flex h-10 shrink-0 items-center justify-between border-b border-line-soft bg-base-soft">
      <div className="flex min-w-0 items-center gap-2.5 pl-4">
        <span className="text-ink">
          <Mark />
        </span>
        <span className="display text-[12.5px] text-ink">{t("app.name")}</span>
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
          className="window-control"
          onClick={() => appWindow.close()}
        >
          <X size={14} />
        </button>
      </div>
    </header>
  );
}
