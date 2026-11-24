import { invoke } from "@tauri-apps/api/core";
import type {
  AppError,
  AppMetadata,
  BuildRequest,
  CompatibilityReport,
  DependencyReport,
  DiagnosticReport,
  DiskSpace,
  EnvironmentInfo,
  GameCandidate,
  GameValidation,
  InstallationState,
  JobSnapshot,
  LauncherRelease,
  LogFileInfo,
  LogLine,
  PathProbe,
  RestorePoint,
  Settings,
  StageInfo,
  UpstreamCheck,
  UpdateHistoryEntry,
  UpdateOutcome,
  WorkState,
  WorkspacePaths,
  WorkspaceSummary,
} from "@/types";

export class IpcError extends Error {
  readonly code: string;
  readonly detail?: string | null;
  constructor(error: AppError) {
    super(error.message);
    this.name = "IpcError";
    this.code = error.code;
    this.detail = error.detail;
  }
}

export function toIpcError(value: unknown): IpcError {
  if (value instanceof IpcError) return value;
  if (typeof value === "object" && value !== null && "message" in value) {
    const maybe = value as Partial<AppError>;
    return new IpcError({
      code: maybe.code ?? "unknown",
      message: String(maybe.message),
      detail: maybe.detail ?? null,
    });
  }
  return new IpcError({ code: "unknown", message: String(value), detail: null });
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toIpcError(error);
  }
}

export const api = {
  // Settings and environment ------------------------------------------------
  loadSettings: () => call<Settings>("settings_load"),
  saveSettings: (settings: Settings) => call<Settings>("settings_save", { settings }),
  resetSettings: () => call<Settings>("settings_reset"),
  paths: () => call<WorkspacePaths>("paths_info"),
  environment: () => call<EnvironmentInfo>("environment_info"),
  metadata: () => call<AppMetadata>("app_metadata"),
  probePath: (path: string, create = false) => call<PathProbe>("probe_path", { path, create }),
  diskSpace: (path: string) => call<DiskSpace>("disk_space", { path }),
  revealPath: (path: string) => call<null>("reveal_path", { path }),
  summary: () => call<WorkspaceSummary>("workspace_summary"),

  // Installation ------------------------------------------------------------
  installationState: () => call<InstallationState>("installation_state"),
  detectGameFolders: () => call<GameCandidate[]>("detect_game_folders"),
  validateGame: (path: string) => call<GameValidation>("validate_game_dir", { path }),
  installLeonida: (options: {
    repositoryPath: string;
    gamePath: string | null;
    outPath: string;
    workPath: string;
    skipCompatibility: boolean;
  }) => call<JobSnapshot>("install_leonida", { options }),
  cancelInstall: () => call<boolean>("cancel_install"),
  updateSubmodules: () => call<JobSnapshot>("update_submodules"),
  installState: () => call<JobSnapshot>("install_state"),

  // Compatibility and updates ----------------------------------------------
  compatibility: () => call<CompatibilityReport>("compatibility_report"),
  upstreamCheck: () => call<UpstreamCheck>("upstream_check"),
  applyUpdate: (allowUnverified: boolean) =>
    call<UpdateOutcome>("apply_upstream_update", { allowUnverified }),
  rollbackUpdate: (backupRef: string | null) =>
    call<UpdateOutcome>("rollback_upstream_update", { backupRef }),
  restorePoints: () => call<RestorePoint[]>("restore_points"),
  updateHistory: () => call<UpdateHistoryEntry[]>("update_history"),
  clearUpdateHistory: () => call<boolean>("clear_update_history"),

  // Dependencies ------------------------------------------------------------
  dependencies: () => call<DependencyReport[]>("check_dependencies"),
  installDependency: (packageId: string) => call<JobSnapshot>("install_dependency", { packageId }),
  cancelDependencyInstall: () => call<boolean>("cancel_dependency_install"),
  dependencyInstallState: () => call<JobSnapshot>("dependency_install_state"),

  // Build -------------------------------------------------------------------
  stages: () => call<StageInfo[]>("build_stages"),
  workState: () => call<WorkState>("build_work_state"),
  startBuild: (request: BuildRequest) => call<JobSnapshot>("start_build", { request }),
  cancelBuild: () => call<boolean>("cancel_build"),
  buildState: () => call<JobSnapshot>("build_state"),

  // Diagnostics and logs ----------------------------------------------------
  diagnostics: () => call<DiagnosticReport>("run_diagnostics"),
  exportDiagnostics: () => call<string>("export_diagnostics"),
  logs: () => call<LogFileInfo[]>("logs_list"),
  readLog: (name: string) => call<string>("logs_read", { name }),
  clearLogs: (keepDays: number) => call<boolean>("logs_clear", { keepDays }),
  recentLogs: (limit: number) => call<LogLine[]>("logs_recent", { limit }),

  // Launcher updates --------------------------------------------------------
  launcherUpdateInfo: () => call<LauncherRelease>("launcher_update_info"),
  checkLauncherUpdate: () => call<LauncherRelease>("launcher_update_check"),
  installLauncherUpdate: () => call<boolean>("launcher_update_install"),
};
