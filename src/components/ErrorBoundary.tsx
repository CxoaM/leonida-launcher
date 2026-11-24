import { Component, type ReactNode } from "react";
import { useT, type Translate } from "@/i18n";

interface BoundaryState {
  error: Error | null;
}

class Boundary extends Component<{ t: Translate; children: ReactNode }, BoundaryState> {
  state: BoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): BoundaryState {
    return { error };
  }

  componentDidCatch(error: Error) {
    // Keep the failure visible in the devtools console without shipping a crash
    // reporter: the launcher never transmits diagnostics on its own.
    console.error("Interface error", error);
  }

  render() {
    const { error } = this.state;
    const { t } = this.props;
    if (!error) return this.props.children;

    return (
      <div className="flex h-full items-center justify-center bg-base p-8">
        <div className="card w-full max-w-xl overflow-hidden">
          <div className="px-5 py-4">
            <h1 className="text-[14px] font-semibold text-ink">{t("error.boundary.title")}</h1>
            <p className="mt-1 text-[12px] leading-[1.55] text-muted">{t("error.boundary.body")}</p>
            <details className="mt-3">
              <summary className="cursor-pointer text-[11px] text-faint hover:text-muted">
                {t("error.detail")}
              </summary>
              <pre className="log scroll-thin mt-1.5 max-h-52 overflow-auto rounded-[8px] border border-line-soft bg-[#090c10] p-2.5">
                {error.stack || error.message}
              </pre>
            </details>
          </div>
          <div className="flex justify-end border-t border-line-soft bg-[#12171e] px-5 py-3">
            <button
              type="button"
              onClick={() => window.location.reload()}
              className="focus-ring inline-flex h-9 items-center justify-center rounded-[var(--radius-control)] bg-accent px-3.5 text-[13px] font-medium text-[#0B0D10] transition-colors hover:bg-[#a3b9ff]"
            >
              {t("error.boundary.reload")}
            </button>
          </div>
        </div>
      </div>
    );
  }
}

export function ErrorBoundary({ children }: { children: ReactNode }) {
  const t = useT();
  return <Boundary t={t}>{children}</Boundary>;
}
