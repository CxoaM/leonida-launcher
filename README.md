# Leonida Launcher

An independent Windows desktop front end for the [Leonida](https://github.com/ubiquitarianism/leonida)
static recompilation project.

It does three things, carefully:

1. **Installs** the official Leonida checkout from its own repository and keeps the
   `third_party/rpf9-explorer` submodule in sync.
2. **Updates** it with a restore point, a compatibility probe and a
   fast-forward-only merge — and rolls back automatically if the probe fails.
3. **Builds** it by calling the upstream `make_game.py` with exactly the stages
   and flags that the installed revision documents.

Everything else in the window exists to make those three actions honest: real
logs, real percentages (or none), real dependency checks, real paths.

## What it will not do

- It never downloads game files, keys, assets or executables. You point it at a
  copy of the game you already own.
- It never bypasses copy protection, anti-cheat or licensing.
- It never invents an upstream capability. If `make_game.py --help` does not list
  a flag, the launcher does not offer it.
- It never shows a **Play** button, because upstream does not document a game
  launcher or run command. That is a deliberate product decision, not an
  unfinished feature.
- It never collects telemetry. Diagnostics are written locally and exported only
  when you click the button.

## Requirements

| Requirement | Notes |
| --- | --- |
| Windows 10 2004+ or Windows 11, x64 | Windows 7/8 and 32-bit are not supported. |
| Git for Windows | Must be on `PATH`. Installation, updates and rollbacks all use it. |
| Python 3.12+ | Needed by the upstream build. The launcher probes it, it does not install it. |
| .NET 8 SDK, LLVM 21+ (AMDGPU), Vulkan SDK, VS 2022 C++ tools, Windows SDK 10.0.26100+ | Needed by different upstream stages. Diagnostics reports what is missing and can run `winget` for you. |
| Free disk space | About 30 GB for the shader stage, up to ~370 GB for the full asset pipeline. |

The launcher itself is a small Tauri 2 application. The installer does **not**
require administrator rights per user account; it installs per user by default.

## Install

Download `LeonidaLauncher-Setup-x64.exe` from the latest release and run it.

The MSI package is a per-machine deployment and therefore asks for administrator
rights; it exists for managed installs. The NSIS setup above stays per-user and
needs no elevation.

The installer lets you choose the folder, offers an optional desktop shortcut,
creates a Start Menu entry and registers an uninstaller. Uninstall asks two
separate questions:

1. Remove launcher settings, logs and cache? (default: keep)
2. Also delete the Leonida checkout and build output? (default: keep)

Your game folder is never touched by the uninstaller.

## First run

1. **Installation** — pick a checkout folder (empty), your game folder, an output
   folder and a work folder. The launcher validates all four, shows free space and
   detects common game install locations.
2. **Updates** — fetch the official branch and read the compatibility report. The
   first install is already probed; updates are probed again before they are kept.
3. **Diagnostics** — run the environment report and install anything that is
   missing. Nothing is installed silently.
4. **Build** — choose stages, output and work folders, then start. The log is the
   upstream output, unmodified. Resume is offered when the work folder already has
   completed upstream stages.
5. **Settings** — language (English/Русский/system), startup behaviour, paths,
   build defaults, restore-point retention and launcher self-updates.

## Two independent update systems

| | Leonida | Launcher |
| --- | --- | --- |
| Source | `git` against `ubiquitarianism/leonida` | signed Tauri release artifacts |
| Trigger | you, explicitly | you, or a start-up check you can disable |
| Verification | live `make_game.py --help` probe + layout + submodules | minisign signature over the installer |
| Rollback | `git reset --keep` to a `launcher-backup/*` ref | reinstall the previous signed version |

Neither system can move the other one silently.

## Safety model

- Every external process is spawned with an argv array. There is no shell string
  concatenation anywhere, so paths with spaces, `&`, `$` or Cyrillic characters
  cannot become commands.
- `git` is only ever run with an explicit working directory, and destructive
  operations require an explicit user action with a modal confirmation.
- Updates require a clean worktree on the official branch with the official
  remote. If the tree is dirty, the update is refused — the launcher never
  discards your work.
- Paths longer than `MAX_PATH` are passed through the `\\?\` extended-length
  prefix, so deep user folders keep working.
- Tokens, keys and secrets are never written to logs. The launcher does not have
  any to begin with; it never handles credentials.

## Development

```powershell
npm install
npm run dev          # Tauri dev window (Vite + Rust)
npm run dev:web      # browser-only preview: shows an honest "no backend" screen
npm run typecheck    # tsc --noEmit
npm test             # vitest (dictionary parity, formatters, UI helpers)
cd src-tauri
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

A local `make_game.py` fixture is available for UI work without real assets:

```powershell
npm run mock:upstream                       # .tmp/upstream-fixture
npm run mock:upstream -- --add-commits 3    # simulate upstream movement
```

Point **Settings → Leonida checkout** at the fixture. The launcher will show the
repository, its submodule and the documented stages, and it will refuse to
*update* from that remote because it is not the official URL — that refusal is
the guard working.

### Repository layout

```
src/                     React 19 + TypeScript UI
  components/            shell, primitives, log console, step list
  pages/                 Home, Installation, Updates, Build, Diagnostics, Settings, About
  state/AppContext.tsx   single source of truth for state, events, toasts
  services/ipc.ts        typed wrapper for every Tauri command
  i18n/                  typed EN/RU dictionary
src-tauri/src/           Rust backend
  commands/              the IPC surface
  leonida/               git, ssh-free repository handling, compatibility, build, deps
  diagnostics/           environment, toolchain, storage and network reporting
  updater/               signed launcher self-update with progress
  process/               argv-only spawning, tree kill, streaming
docs/                    architecture, compatibility, building, releasing, security
tests are colocated: *.test.ts next to the module, #[cfg(test)] in Rust
```

## Configuration

The repository owner is baked into `src-tauri/resources/launcher.json` and the
updater endpoint is derived from it, so no release can point at a repository that
does not exist:

```powershell
npm run configure -- --owner <github-user-or-org>
npm run configure -- --owner <owner> --pubkey "<base64 minisign public key>"
```

See [docs/RELEASING.md](docs/RELEASING.md) for the full release process.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Compatibility model](docs/COMPATIBILITY.md)
- [Building locally](docs/BUILDING.md)
- [Releasing](docs/RELEASING.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Security posture](docs/SECURITY.md)
- [Roadmap](docs/ROADMAP.md)
- [Changelog](CHANGELOG.md)

## License

MIT. See [LICENSE](LICENSE).

Leonida itself is built by the upstream contributors at
[ubiquitarianism/leonida](https://github.com/ubiquitarianism/leonida). This
launcher is an independent front end: it is not affiliated with, endorsed by or
bundled with the upstream project, and it ships none of its code.
