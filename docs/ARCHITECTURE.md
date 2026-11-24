# Architecture

Leonida Launcher is a Tauri 2 desktop application: a React 19 renderer inside
WebView2, and a Rust core that owns every filesystem, process and network
operation. The renderer never shells out, never reads files and never talks to
the network; it asks the core through typed commands.

```
┌──────────────────────────── WebView2 ────────────────────────────┐
│  React 19 + TypeScript                                            │
│  AppContext ── pages ── components ── typed i18n dictionary       │
│        │                                                          │
│        └── services/ipc.ts  (invoke + typed errors)               │
└───────────────────────────────┬───────────────────────────────────┘
                                │  Tauri IPC (commands + events)
┌───────────────────────────────┴───────────────────────────────────┐
│  Rust core                                                        │
│  commands/   ─ thin command layer, no business logic              │
│  leonida/    ─ git, compatibility, build, dependencies            │
│  diagnostics/─ environment, storage, network report               │
│  updater/    ─ signed launcher self-update                        │
│  process/    ─ argv spawning, streaming, tree kill                │
│  logging/    ─ on-disk JSONL logs + event stream                  │
│  settings/   ─ persisted preferences, autostart, path probing     │
└───────────────────────────────────────────────────────────────────┘
```

## Why the core owns everything

- **One place for safety.** Every process launch goes through
  `process::CommandSpec`, which takes an executable and an argv vector. There is no
  `cmd /c` anywhere in the codebase, so a path cannot become a command.
- **One place for truth.** The renderer cannot fabricate a percentage, an ETA or a
  capability, because it only renders snapshots the core produced.
- **One place for recovery.** Interrupted updates are detected and repaired on the
  next start, without the UI being open.

## IPC surface

Commands are registered in `src-tauri/src/lib.rs` and wrapped in
`src/services/ipc.ts`. The names are stable; the TypeScript types mirror the Rust
`Serialize` structs (`serde(rename_all = "camelCase")` everywhere).

| Group | Commands |
| --- | --- |
| Settings and environment | `settings_load`, `settings_save`, `settings_reset`, `paths_info`, `environment_info`, `app_metadata`, `probe_path`, `disk_space`, `reveal_path`, `workspace_summary` |
| Installation | `installation_state`, `detect_game_folders`, `validate_game_dir`, `install_leonida`, `cancel_install`, `update_submodules`, `install_state` |
| Updates | `compatibility_report`, `upstream_check`, `apply_upstream_update`, `rollback_upstream_update`, `restore_points`, `update_history`, `clear_update_history` |
| Dependencies | `check_dependencies`, `install_dependency`, `dependency_install_state` |
| Build | `build_stages`, `build_work_state`, `start_build`, `cancel_build`, `build_state` |
| Diagnostics and logs | `run_diagnostics`, `export_diagnostics`, `logs_list`, `logs_read`, `logs_clear`, `logs_recent` |
| Launcher self-update | `launcher_update_info`, `launcher_update_check`, `launcher_update_install` |

Errors cross the boundary as `{ code, message, detail }`. `code` is a stable
machine token (`spawn_failed`, `unsupported_stage`, `local_changes`,
`verification_required`, `updater_not_configured`, …), `message` is a short human
sentence and `detail` is optional context. The UI shows `message` and puts
`detail` behind a disclosure.

### Events

| Event | Payload | Emitted by |
| --- | --- | --- |
| `settings://changed` | `Settings` | save/reset, so every open view converges |
| `log://line` | `LogLine` | the log manager, drives Home's activity list |
| `install://event` | `InstallEvent` (`step`, `log`, `finished`, `error`) | installation job |
| `build://event` | `BuildEvent` (`log`, `stage`, `progress`, `state`, `finished`, `error`) | build job |
| `update://progress` | `UpdateProgressEvent` (`stage`, `label`, `detail`) | upstream update pipeline |
| `launcher://update-progress` | `LauncherUpdateProgress` (phase, bytes) | launcher self-update |

## Jobs

There are three independent single-slot jobs: `install`, `build` and
`dependency_install`. Each is a `JobRuntime` holding a `JobSnapshot`:

```ts
{ kind, status, id, command, stages, stageIndex, stageCount,
  currentStage, percent, eta, startedAt, finishedAt, error, lines, pid }
```

- `status` is `idle | running | cancelling | succeeded | failed | cancelled`.
- `percent` and `eta` are `null` unless upstream printed them. The parser accepts
  `NN%` in the range 0–100 and `eta 1h 20m`, and nothing else.
- `lines` is the raw stream, capped, so the log view is always the real output.
- Cancelling kills the process tree (`taskkill /PID <pid> /T /F`) because MSBuild
  and .NET leave children behind.

## Upstream update pipeline

`leonida/updater.rs` runs the same sequence for every update, reporting through
`update://progress`:

1. **Preflight** — repository exists, `HEAD` is the configured branch, the remote
   matches the official URL, the worktree has no tracked changes.
2. **Fetch** — `git fetch origin <branch>` with streamed output.
3. **Restore point** — `git update-ref refs/launcher-backup/<timestamp> <head>` and
   `restore-point.json` in the config folder.
4. **Merge** — `git merge --ff-only origin/<branch>`; divergence is refused, never
   forced.
5. **Submodules** — `git submodule sync --recursive` then `update --init --recursive`.
6. **Verify** — the same live compatibility probe as installation.
7. **Rollback** — on failure: `git reset --keep <backup>`, resync submodules, clear
   the restore point, append a `rolled_back` history entry and return the error.

If the process dies mid-update, `restore-point.json` survives. On the next start
`recover_interrupted` restores the recorded ref before the UI appears.

## Storage layout

| What | Where |
| --- | --- |
| Settings, logs, `restore-point.json`, `update-history.json` | `%APPDATA%\dev.leonida.launcher` |
| Cache | `%LOCALAPPDATA%\dev.leonida.launcher\cache` |
| Default checkout | `%LOCALAPPDATA%\dev.leonida.launcher\workspace\repository` |
| Default work folder | `%LOCALAPPDATA%\dev.leonida.launcher\workspace\work` |
| Default build output | `%USERPROFILE%\Documents\Leonida` |

Nothing is written inside the git working copy. The uninstaller offers to remove
the launcher's own folders and, as a separate question, the checkout and output.

## Renderer structure

- `state/AppContext.tsx` is the only state container. It bootstraps settings,
  paths, metadata, environment, job snapshots, then subscribes to the events
  above. Pages read from `useApp()`; nothing else stores global state.
- Settings edits are optimistic and persisted with a 320 ms debounce. Path fields
  commit on blur or <kbd>Enter</kbd> so a half-typed path never reaches disk.
- `i18n/dictionary.ts` is typed: English defines the key union, Russian is a
  `Record<MessageKey, string>`, so a missing translation is a compile error. A test
  also enforces identical interpolation variables in both languages.
- In a plain browser (`npm run dev:web`) there is no backend. Instead of fake
  data, the app shows a fatal screen explaining that the desktop backend is not
  connected.
- `components/ErrorBoundary.tsx` catches renderer crashes and offers a reload
  without claiming that anything on disk changed.

## Window and shell

- 1200×800 default, 980×660 minimum, custom title bar (`decorations: false`) with
  drag regions and window controls.
- Layout: title bar, sidebar (workspace + system groups), scrolling content,
  status bar with git state, active job and compatibility, toast stack.
- Design tokens live in `styles/globals.css`: base `#0B0D10`, surface `#10141A`,
  panel `#151A21`, raised `#1B222B`, line `#252D38`, ink `#F2F4F7`, muted
  `#929BA8`, accent `#8EA9FF`.
- Motion is limited to fades, a scale-in for modals and an indeterminate bar; all
  of it collapses under `prefers-reduced-motion`.
