import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppMetadata,
  BuildEvent,
  CompatibilityReport,
  EnvironmentInfo,
  InstallEvent,
  JobSnapshot,
  LauncherRelease,
  LauncherUpdateProgress,
  LogLine,
  PageId,
  Settings,
  Tone,
  UpdateProgressEvent,
  UpstreamCheck,
  WorkspacePaths,
  WorkspaceSummary,
} from "@/types";
import { api, toIpcError } from "@/services/ipc";
import { isTauri } from "@/lib/tauri";
import { createTranslator, resolveLanguage, I18nProvider, type Translate } from "@/i18n";

export interface Toast {
  id: number;
  tone: Tone;
  title: string;
  message?: string;
  detail?: string | null;
}

interface AppValue {
  /** False until the first bootstrap round trip finished. */
  ready: boolean;
  /** Non-null when the UI cannot talk to the Rust backend at all. */
  fatal: string | null;
  page: PageId;
  setPage: (page: PageId) => void;

  settings: Settings | null;
  saving: boolean;
  updateSettings: (patch: Partial<Settings>, options?: { persist?: boolean }) => void;
  saveSettings: (next?: Settings) => Promise<void>;
  resetSettings: () => Promise<void>;

  paths: WorkspacePaths | null;
  environment: EnvironmentInfo | null;
  metadata: AppMetadata | null;

  summary: WorkspaceSummary | null;
  summaryLoading: boolean;
  refreshSummary: () => Promise<void>;
  compatibility: CompatibilityReport | null;
  refreshCompatibility: () => Promise<void>;

  installJob: JobSnapshot;
  buildJob: JobSnapshot;
  dependencyJob: JobSnapshot;
  setInstallJob: (job: JobSnapshot) => void;
  setBuildJob: (job: JobSnapshot) => void;
  setDependencyJob: (job: JobSnapshot) => void;

  upstreamCheck: UpstreamCheck | null;
  setUpstreamCheck: (check: UpstreamCheck | null) => void;
  runUpstreamCheck: () => Promise<UpstreamCheck>;

  launcherRelease: LauncherRelease | null;
  launcherProgress: LauncherUpdateProgress | null;
  checkLauncherUpdate: () => Promise<LauncherRelease>;
  installLauncherUpdate: () => Promise<void>;

  updateProgress: UpdateProgressEvent | null;

  activity: LogLine[];
  pushActivity: (line: LogLine) => void;

  toasts: Toast[];
  pushToast: (toast: Omit<Toast, "id">) => void;
  dismissToast: (id: number) => void;
  reportError: (error: unknown, context?: string) => void;
}

const idleJob = (kind: string): JobSnapshot => ({
  kind,
  status: "idle",
  id: null,
  command: null,
  stages: [],
  stageIndex: 0,
  stageCount: 0,
  currentStage: null,
  percent: null,
  eta: null,
  startedAt: null,
  finishedAt: null,
  error: null,
  lines: [],
  pid: null,
});

const AppContext = createContext<AppValue | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const [ready, setReady] = useState(false);
  const [fatal, setFatal] = useState<string | null>(null);
  const [page, setPage] = useState<PageId>("home");

  const [settings, setSettings] = useState<Settings | null>(null);
  const [saving, setSaving] = useState(false);
  const [paths, setPaths] = useState<WorkspacePaths | null>(null);
  const [environment, setEnvironment] = useState<EnvironmentInfo | null>(null);
  const [metadata, setMetadata] = useState<AppMetadata | null>(null);

  const [summary, setSummary] = useState<WorkspaceSummary | null>(null);
  const [summaryLoading, setSummaryLoading] = useState(false);
  const [compatibility, setCompatibility] = useState<CompatibilityReport | null>(null);

  const [installJob, setInstallJob] = useState<JobSnapshot>(() => idleJob("install"));
  const [buildJob, setBuildJob] = useState<JobSnapshot>(() => idleJob("build"));
  const [dependencyJob, setDependencyJob] = useState<JobSnapshot>(() => idleJob("dependency"));

  const [upstreamCheck, setUpstreamCheck] = useState<UpstreamCheck | null>(null);
  const [launcherRelease, setLauncherRelease] = useState<LauncherRelease | null>(null);
  const [launcherProgress, setLauncherProgress] = useState<LauncherUpdateProgress | null>(null);
  const [updateProgress, setUpdateProgress] = useState<UpdateProgressEvent | null>(null);
  const [activity, setActivity] = useState<LogLine[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);

  const toastId = useRef(0);
  const saveTimer = useRef<number | null>(null);
  const pendingSettings = useRef<Settings | null>(null);
  const tRef = useRef<Translate>(createTranslator("en"));
  const t = useMemo(
    () => createTranslator(resolveLanguage(settings?.language)),
    [settings?.language],
  );
  tRef.current = t;

  const dismissToast = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const pushToast = useCallback(
    (toast: Omit<Toast, "id">) => {
      toastId.current += 1;
      const id = toastId.current;
      setToasts((current) => [...current.slice(-3), { ...toast, id }]);
      window.setTimeout(() => {
        setToasts((current) => current.filter((item) => item.id !== id));
      }, toast.tone === "danger" ? 9000 : 5200);
    },
    [],
  );

  const reportError = useCallback(
    (error: unknown, context?: string) => {
      const ipc = toIpcError(error);
      const title = context ?? tRef.current("toast.error");
      pushToast({ tone: "danger", title, message: ipc.message, detail: ipc.detail });
    },
    [pushToast],
  );

  const pushActivity = useCallback((line: LogLine) => {
    setActivity((current) => [...current.slice(-199), line]);
  }, []);

  const refreshCompatibility = useCallback(async () => {
    try {
      setCompatibility(await api.compatibility());
    } catch (error) {
      reportError(error, tRef.current("home.label.compatibility"));
    }
  }, [reportError]);

  const refreshSummary = useCallback(async () => {
    setSummaryLoading(true);
    try {
      const next = await api.summary();
      setSummary(next);
      setCompatibility(next.compatibility);
      setInstallJob(next.install);
      setBuildJob(next.build);
      setMetadata(next.metadata);
    } catch (error) {
      reportError(error, tRef.current("home.updated"));
    } finally {
      setSummaryLoading(false);
    }
  }, [reportError]);

  const persistSettings = useCallback(
    async (next: Settings) => {
      setSaving(true);
      try {
        const saved = await api.saveSettings(next);
        setSettings(saved);
        pendingSettings.current = null;
      } catch (error) {
        reportError(error, tRef.current("settings.heading"));
      } finally {
        setSaving(false);
      }
    },
    [reportError],
  );

  const flushSettings = useCallback(() => {
    if (saveTimer.current !== null) {
      window.clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
    const pending = pendingSettings.current;
    if (pending) {
      pendingSettings.current = null;
      void persistSettings(pending);
    }
  }, [persistSettings]);

  const updateSettings = useCallback(
    (patch: Partial<Settings>, options?: { persist?: boolean }) => {
      setSettings((current) => {
        if (!current) return current;
        const next: Settings = { ...current, ...patch };
        if (options?.persist !== false) {
          pendingSettings.current = next;
          if (saveTimer.current !== null) window.clearTimeout(saveTimer.current);
          saveTimer.current = window.setTimeout(() => {
            saveTimer.current = null;
            const queued = pendingSettings.current;
            pendingSettings.current = null;
            if (queued) void persistSettings(queued);
          }, 320);
        }
        return next;
      });
    },
    [persistSettings],
  );

  const saveSettings = useCallback(
    async (next?: Settings) => {
      const target = next ?? pendingSettings.current ?? settings;
      if (!target) return;
      if (saveTimer.current !== null) {
        window.clearTimeout(saveTimer.current);
        saveTimer.current = null;
      }
      await persistSettings(target);
    },
    [persistSettings, settings],
  );

  const resetSettings = useCallback(async () => {
    try {
      const saved = await api.resetSettings();
      setSettings(saved);
      pushToast({ tone: "ok", title: tRef.current("settings.saved") });
      void refreshSummary();
    } catch (error) {
      reportError(error, tRef.current("settings.group.danger"));
    }
  }, [pushToast, refreshSummary, reportError]);

  const runUpstreamCheck = useCallback(async () => {
    const check = await api.upstreamCheck();
    setUpstreamCheck(check);
    return check;
  }, []);

  const checkLauncherUpdate = useCallback(async () => {
    const release = await api.checkLauncherUpdate();
    setLauncherRelease(release);
    return release;
  }, []);

  const installLauncherUpdate = useCallback(async () => {
    try {
      setLauncherProgress({ phase: "downloading", downloaded: 0, total: null, message: "" });
      await api.installLauncherUpdate();
    } catch (error) {
      setLauncherProgress(null);
      reportError(error, tRef.current("settings.updater.installing"));
    }
  }, [reportError]);

  // Bootstrap -------------------------------------------------------------
  useEffect(() => {
    let cancelled = false;

    async function boot() {
      if (!isTauri()) {
        setFatal(
          "The desktop backend is not connected. Start the launcher with `npm run dev` (Tauri) instead of opening the page in a browser.",
        );
        setReady(true);
        return;
      }
      try {
        const [loadedSettings, loadedPaths, loadedMeta] = await Promise.all([
          api.loadSettings(),
          api.paths(),
          api.metadata(),
        ]);
        if (cancelled) return;
        setSettings(loadedSettings);
        setPaths(loadedPaths);
        setMetadata(loadedMeta);

        const [env, recent] = await Promise.all([
          api.environment().catch(() => null),
          api.recentLogs(40).catch(() => [] as LogLine[]),
        ]);
        if (cancelled) return;
        setEnvironment(env);
        setActivity(recent);

        const jobs = await Promise.all([
          api.installState().catch(() => idleJob("install")),
          api.buildState().catch(() => idleJob("build")),
          api.dependencyInstallState().catch(() => idleJob("dependency")),
        ]);
        if (cancelled) return;
        setInstallJob(jobs[0]);
        setBuildJob(jobs[1]);
        setDependencyJob(jobs[2]);

        setReady(true);
        await refreshSummary();

        if (loadedSettings.checkLauncherUpdatesOnStart) {
          void checkLauncherUpdate().catch(() => undefined);
        }
      } catch (error) {
        if (cancelled) return;
        const ipc = toIpcError(error);
        setFatal(`${ipc.message}${ipc.detail ? `\n\n${ipc.detail}` : ""}`);
        setReady(true);
      }
    }

    void boot();
    return () => {
      cancelled = true;
      flushSettings();
    };
    // Bootstrap must run exactly once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Event bridge ----------------------------------------------------------
  useEffect(() => {
    if (!isTauri()) return;
    const unlisteners: UnlistenFn[] = [];
    let disposed = false;

    const register = async <T,>(name: string, handler: (payload: T) => void) => {
      const off = await listen<T>(name, (event) => handler(event.payload));
      if (disposed) {
        off();
      } else {
        unlisteners.push(off);
      }
    };

    void register<Settings>("settings://changed", (payload) => {
      setSettings((current) => ({ ...(current ?? payload), ...payload }));
    });

    void register<LogLine>("log://line", (payload) => {
      pushActivity(payload);
    });

    void register<InstallEvent>("install://event", (payload) => {
      if (payload.snapshot) {
        setInstallJob(payload.snapshot);
      }
      if (payload.kind === "finished") {
        const status = payload.snapshot?.status;
        if (status === "succeeded") {
          setPage("installation");
        }
        void refreshSummary();
        void refreshCompatibility();
      }
    });

    void register<BuildEvent>("build://event", (payload) => {
      if (payload.snapshot) {
        setBuildJob(payload.snapshot);
      } else {
        setBuildJob((current) => ({
          ...current,
          status: payload.kind === "error" ? "failed" : current.status,
          currentStage: payload.stage ?? current.currentStage,
          percent: payload.kind === "progress" ? payload.percent ?? current.percent : current.percent,
          eta: payload.kind === "progress" ? payload.eta ?? current.eta : current.eta,
        }));
      }
      if (payload.kind === "finished") {
        void refreshSummary();
      }
    });

    void register<UpdateProgressEvent>("update://progress", (payload) => {
      setUpdateProgress(payload);
    });

    void register<LauncherUpdateProgress>("launcher://update-progress", (payload) => {
      setLauncherProgress(payload);
    });

    return () => {
      disposed = true;
      for (const off of unlisteners) off();
    };
  }, [pushActivity, refreshCompatibility, refreshSummary]);

  const setDependencyJobStable = useCallback((job: JobSnapshot) => setDependencyJob(job), []);

  const value = useMemo<AppValue>(
    () => ({
      ready,
      fatal,
      page,
      setPage,
      settings,
      saving,
      updateSettings,
      saveSettings,
      resetSettings,
      paths,
      environment,
      metadata,
      summary,
      summaryLoading,
      refreshSummary,
      compatibility,
      refreshCompatibility,
      installJob,
      buildJob,
      dependencyJob,
      setInstallJob,
      setBuildJob,
      setDependencyJob: setDependencyJobStable,
      upstreamCheck,
      setUpstreamCheck,
      runUpstreamCheck,
      launcherRelease,
      launcherProgress,
      checkLauncherUpdate,
      installLauncherUpdate,
      updateProgress,
      activity,
      pushActivity,
      toasts,
      pushToast,
      dismissToast,
      reportError,
    }),
    [
      ready,
      fatal,
      page,
      settings,
      saving,
      updateSettings,
      saveSettings,
      resetSettings,
      paths,
      environment,
      metadata,
      summary,
      summaryLoading,
      refreshSummary,
      compatibility,
      refreshCompatibility,
      installJob,
      buildJob,
      dependencyJob,
      setDependencyJobStable,
      upstreamCheck,
      runUpstreamCheck,
      launcherRelease,
      launcherProgress,
      checkLauncherUpdate,
      installLauncherUpdate,
      updateProgress,
      activity,
      pushActivity,
      toasts,
      pushToast,
      dismissToast,
      reportError,
    ],
  );

  return (
    <AppContext.Provider value={value}>
      <I18nProvider language={settings?.language}>{children}</I18nProvider>
    </AppContext.Provider>
  );
}

export function useApp(): AppValue {
  const value = useContext(AppContext);
  if (!value) {
    throw new Error("useApp must be used inside AppProvider");
  }
  return value;
}
