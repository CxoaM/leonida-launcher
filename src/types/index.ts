export type PageId =
  | "home"
  | "installation"
  | "updates"
  | "build"
  | "diagnostics"
  | "settings"
  | "about";

export type Tone = "neutral" | "accent" | "ok" | "warn" | "danger";

export interface AppError {
  code: string;
  message: string;
  detail?: string | null;
}

export interface AdvancedSettings {
  allowUnverifiedUpstream: boolean;
  keepRestorePoints: number;
}

export interface Settings {
  schemaVersion: number;
  theme: string;
  language: "en" | "ru" | "system";
  launchOnStartup: boolean;
  checkLauncherUpdatesOnStart: boolean;
  leonidaAutoCheck: boolean;
  leonidaNotifications: boolean;
  leonidaCheckIntervalMinutes: number;
  repositoryPath: string | null;
  gamePath: string | null;
  outPath: string;
  workPath: string;
  cachePath: string;
  jobs: number;
  wave64Emulate: "auto" | "on" | "off";
  logRetentionDays: number;
  verboseLogging: boolean;
  advanced: AdvancedSettings;
}

export interface WorkspacePaths {
  configDir: string;
  dataDir: string;
  cacheDir: string;
  logsDir: string;
  documentsDir: string;
  defaultRepository: string;
  defaultOut: string;
  defaultWork: string;
  defaultCache: string;
}

export interface EnvironmentInfo {
  launcherVersion: string;
  osName: string;
  osEdition: string;
  osVersion: string;
  osBuild: number;
  arch: string;
  cpuName: string;
  cpuCores: number;
  memoryTotalMb: number;
  memoryAvailableMb: number;
  longPaths: boolean | null;
  executable: string;
  configDir: string;
  dataDir: string;
  logsDir: string;
}

export interface DiskSpace {
  path: string;
  totalBytes: number;
  freeBytes: number;
  volume: string;
}

export interface PathProbe {
  path: string;
  exists: boolean;
  isDirectory: boolean;
  writable: boolean | null;
  freeBytes: number | null;
  totalBytes: number | null;
  error: string | null;
}

export interface SubmoduleState {
  path: string;
  sha: string | null;
  initialized: boolean;
  modified: boolean;
}

export interface CommitInfo {
  sha: string;
  shortSha: string;
  subject: string;
  author: string;
  date: string;
  dateHuman: string;
}

export interface WorkState {
  workPath: string;
  outPath: string;
  shaderOutputPresent: boolean;
  doneMarkers: string[];
  runtimeBuilt: boolean;
  resumeAvailable: boolean;
}

export interface RestorePoint {
  createdAt: string;
  branch: string;
  commit: string;
  backupRef: string;
  target: string;
}

export interface InstallationState {
  installed: boolean;
  repositoryPath: string;
  repositoryExists: boolean;
  branch: string | null;
  commit: string | null;
  shortCommit: string | null;
  commitSubject: string | null;
  commitDate: string | null;
  dirtyFiles: number;
  untrackedFiles: number;
  submodules: SubmoduleState[];
  submodulesReady: boolean;
  gamePath: string | null;
  gameValid: boolean;
  workState: WorkState;
  restorePoint: RestorePoint | null;
  gitAvailable: boolean;
}

export interface GameCandidate {
  path: string;
  archiveCount: number;
  configPresent: boolean;
  score: number;
}

export interface GameValidation {
  path: string;
  exists: boolean;
  configPresent: boolean;
  archiveCount: number;
  archiveSamples: string[];
  probesFound: string[];
  valid: boolean;
  issues: string[];
  notes: string[];
  disk: DiskSpace | null;
  expectedVersion: string;
  expectedFreeGb: number;
}

export type Severity = "ok" | "warning" | "error" | "skipped";
export type CompatibilityStatus = "compatible" | "degraded" | "incompatible" | "unknown";

export interface CompatibilityCheck {
  id: string;
  label: string;
  severity: Severity;
  detail: string;
}

export interface RevisionRelation {
  kind: "verified" | "newer" | "older" | "unknown";
  commitsAheadOfVerified: number;
  commitsBehindVerified: number;
  detail: string;
}

export interface Capabilities {
  install: boolean;
  update: boolean;
  build: boolean;
  assets: boolean;
  runtime: boolean;
  pipelines: boolean;
  resume: boolean;
  wave64Emulate: boolean;
  wav: boolean;
  fullCheck: boolean;
  gameLaunch: boolean;
  gameLaunchReason: string;
}

export interface CompatibilityReport {
  status: CompatibilityStatus;
  adapterVersion: string;
  launcherVersion: string;
  specSchema: number;
  verifiedCommit: string;
  verifiedAt: string;
  installedCommit: string | null;
  installedCommitSubject: string | null;
  branch: string | null;
  revision: RevisionRelation;
  checks: CompatibilityCheck[];
  capabilities: Capabilities;
  stages: string[];
  helpFlags: string[];
  missingRequiredFlags: string[];
  probedHelp: string | null;
  probedAt: string;
  warnings: string[];
}

export type DependencyKind = "required" | "recommended" | "optional";
export type DependencyStatus = "installed" | "missing" | "outdated" | "invalid";

export interface FixSuggestion {
  summary: string;
  url: string | null;
  wingetId: string | null;
  notes: string | null;
}

export interface DependencyReport {
  id: string;
  name: string;
  kind: DependencyKind;
  status: DependencyStatus;
  version: string | null;
  requiredVersion: string | null;
  detail: string;
  path: string | null;
  affects: string[];
  fix: FixSuggestion | null;
}

export interface StageInfo {
  id: string;
  label: string;
  description: string;
  requiresDependencies: string[];
  selectedByDefault: boolean;
}

export interface UpstreamCheck {
  repository: string;
  branch: string;
  checkedAt: string;
  installedCommit: string | null;
  installedSubject: string | null;
  latestCommit: string | null;
  latestSubject: string | null;
  behind: number;
  ahead: number;
  updateAvailable: boolean;
  commits: CommitInfo[];
  remoteUrl: string | null;
  remoteIsOfficial: boolean;
  modifiedFiles: number;
  untrackedFiles: number;
  channelNote: string;
}

export interface UpdateOutcome {
  success: boolean;
  fromCommit: string | null;
  toCommit: string | null;
  rolledBack: boolean;
  message: string;
  backupRef: string | null;
  compatibility: CompatibilityReport;
}

export interface UpdateHistoryEntry {
  id: string;
  at: string;
  action: "update" | "rollback";
  fromCommit: string | null;
  toCommit: string | null;
  result: "success" | "rolled_back" | "failed";
  message: string;
  backupRef: string | null;
  adapterVersion: string;
}

export interface BuildRequest {
  stages: string[];
  gamePath: string | null;
  outPath: string;
  workPath: string;
  jobs: number;
  wave64Emulate: "auto" | "on" | "off";
  wav: boolean;
  fullCheck: boolean;
  fallbackFlags: string[];
}

export type JobStatus =
  | "idle"
  | "running"
  | "cancelling"
  | "succeeded"
  | "failed"
  | "cancelled";

export interface JobSnapshot {
  kind: string;
  status: JobStatus;
  id: string | null;
  command: string | null;
  stages: string[];
  stageIndex: number;
  stageCount: number;
  currentStage: string | null;
  percent: number | null;
  eta: string | null;
  startedAt: string | null;
  finishedAt: string | null;
  error: string | null;
  lines: string[];
  pid: number | null;
}

export interface DiagnosticItem {
  id: string;
  group: string;
  label: string;
  status: "ok" | "warning" | "error" | "info";
  detail: string;
}

export interface NetworkCheck {
  reachable: boolean;
  detail: string;
  latencyMs: number | null;
}

export interface DiagnosticReport {
  runAt: string;
  overall: "ok" | "warning" | "error";
  environment: EnvironmentInfo;
  items: DiagnosticItem[];
  dependencies: DependencyReport[];
  compatibility: CompatibilityReport;
  game: GameValidation | null;
  disks: DiskSpace[];
  network: NetworkCheck;
  repositoryPath: string;
  outPath: string;
  workPath: string;
  gamePath: string | null;
}

export interface LauncherRelease {
  configured: boolean;
  available: boolean;
  currentVersion: string;
  version: string | null;
  notes: string | null;
  publishedAt: string | null;
  downloadBytes: number | null;
  endpoint: string;
  message: string;
}

export interface LauncherUpdateProgress {
  phase: "downloading" | "installing" | "restarting" | "error";
  downloaded: number;
  total: number | null;
  message: string;
}

export interface AppMetadata {
  productName: string;
  version: string;
  owner: string;
  repositoryUrl: string;
  upstreamUrl: string;
  license: string;
  channel: string;
  updaterConfigured: boolean;
  updaterEndpoint: string;
  identifier: string;
}

export interface WorkspaceSummary {
  installation: InstallationState;
  compatibility: CompatibilityReport;
  dependencies: DependencyReport[];
  environment: EnvironmentInfo;
  release: LauncherRelease;
  build: JobSnapshot;
  install: JobSnapshot;
  disks: DiskSpace[];
  metadata: AppMetadata;
}

export interface BuildEvent {
  kind: "log" | "stage" | "progress" | "finished" | "state" | "error";
  line?: string;
  stream?: "stdout" | "stderr";
  stage?: string;
  status?: string;
  percent?: number | null;
  eta?: string | null;
  message?: string;
  detail?: string | null;
  snapshot?: JobSnapshot;
}

export interface InstallEvent {
  kind: "step" | "log" | "finished" | "error";
  step?: string;
  status?: string;
  detail?: string;
  line?: string;
  message?: string;
  snapshot?: JobSnapshot;
}

export interface UpdateProgressEvent {
  stage: string;
  label: string;
  detail: string;
}

export interface LogLine {
  timestamp: string;
  category: "Launcher" | "Updater" | "Git" | "Dependencies" | "Build" | "Diagnostics" | string;
  level: "DEBUG" | "INFO" | "WARN" | "ERROR" | string;
  message: string;
}

export interface LogFileInfo {
  name: string;
  sizeBytes: number;
  modified: string;
}
