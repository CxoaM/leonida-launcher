import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

/** True when the page runs inside the Tauri webview rather than a plain browser. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export const appWindow = {
  minimize() {
    if (!isTauri()) return;
    void getCurrentWindow().minimize();
  },
  async toggleMaximize() {
    if (!isTauri()) return false;
    const win = getCurrentWindow();
    await win.toggleMaximize();
    return win.isMaximized();
  },
  async isMaximized() {
    if (!isTauri()) return false;
    return getCurrentWindow().isMaximized();
  },
  close() {
    if (!isTauri()) return;
    void getCurrentWindow().close();
  },
  async onFocusChanged(handler: (focused: boolean) => void) {
    if (!isTauri()) return () => {};
    return getCurrentWindow().onFocusChanged(({ payload }) => handler(payload));
  },
  async onResized(handler: () => void) {
    if (!isTauri()) return () => {};
    return getCurrentWindow().onResized(() => handler());
  },
};

/** Opens a folder in Explorer through the launcher's own validated command. */
export async function revealPath(path: string): Promise<void> {
  if (!isTauri()) return;
  await invoke("reveal_path", { path });
}

export async function openExternal(url: string): Promise<void> {
  if (!isTauri()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}

export async function pickFolder(defaultPath?: string | null): Promise<string | null> {
  if (!isTauri()) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select folder",
    defaultPath: defaultPath ?? undefined,
  });
  if (Array.isArray(selected)) return selected[0] ?? null;
  return selected ?? null;
}

export async function writeClipboard(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    return;
  } catch {
    // WebView2 can refuse the async clipboard API; a hidden textarea still works
    // because the call always originates from a real click.
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    document.execCommand("copy");
    document.body.removeChild(area);
  }
}
