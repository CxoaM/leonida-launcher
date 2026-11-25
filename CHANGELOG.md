# Changelog

All notable changes to Leonida Launcher are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [1.0.1] – 2026-11-25

Interface pass. No behaviour, data or feature changes.

### Changed

- The whole window now follows one "night workshop" design system: warm graphite
  surfaces, an off-white primary action, a single safety-orange accent,
  Bahnschrift display type and measured tick rails.
- Overview, Installation, Updates, Build, Diagnostics, Settings and About were
  rebuilt on the same tokens and primitives. Asymmetric columns, flat divided
  lists and borderless status sections replace gradients, icon tiles and uniform
  card grids.
- Overview shows the five workspace stages as a clickable measured rail instead
  of a separate next-steps card.
- Updates groups upstream, restore points and history beside the update pipeline
  and compatibility check; while idle the pipeline lists the stage sequence
  instead of seven waiting rows.
- Build shows Time, ETA and stage count as a mono readout, and Diagnostics draws
  disk usage as bars.
- The not-installed cards on Build and Updates use the same inner padding as
  every other card.

### Fixed

- Job states in the update pipeline are localized instead of printing a raw
  status enum.
- Build and Updates no longer present "No ETA reported" as if it were a
  percentage.
- Dependency kinds are no longer rendered as uppercase micro-labels, and the
  pipeline card no longer borrows the "Restore point" title.

## [1.0.0] – 2026-11-24

First stable release. Verified against upstream Leonida `a289278` on `main`.

### Added

- **Installation** — clones the official Leonida repository into an empty folder,
  initialises the `third_party/rpf9-explorer` submodule, probes the checkout and
  registers the paths in settings. Detects common game folders and validates the
  one you pick without touching it.
- **Updates** — fetches `origin/main`, compares commits, then applies a
  fast-forward-only merge with a `launcher-backup/<timestamp>` ref, a
  `restore-point.json` marker and a post-merge compatibility probe. A failed probe
  rolls back with `git reset --keep` and records the outcome in history.
- **Compatibility layer** — `compatibility.json` describes the adapter, the
  verified baseline commit, required/optional files, directories, submodules and
  CLI flags. The live `make_game.py --help` probe is the source of truth; the
  baseline only classifies the revision as verified/newer/older.
- **Build** — stage selection limited to stages the installed revision actually
  documents, argv-only invocation of `py -3 -I make_game.py`, streamed output,
  percentage and ETA parsed only when upstream prints them, tree-kill cancel,
  resume detection from work-folder markers.
- **Diagnostics** — one report over system, toolchain, git, compatibility, game
  files, storage and GitHub reachability, with a `winget` install action per
  dependency, log viewer, log retention and JSON export.
- **Settings** — English/Russian/system language, startup registration, upstream
  auto-check interval, all workspace folders, build defaults, restore-point
  retention, verbose logs and a reset that restores preferences while keeping your
  workspace paths and never touching files on disk.
- **Launcher self-update** — signed Tauri releases with progress, size and release
  notes, gated behind an explicit repository owner and updater public key.
- **Installer** — per-user NSIS setup with folder choice, optional desktop
  shortcut, Start Menu entry and an uninstaller that asks separately about user
  data and about the Leonida sources.
- **CI** — `build.yml` (typecheck, tests, clippy, fmt, Windows x64 installers) and
  `release.yml` (version/tag verification, signing-key enforcement, GitHub
  Release with updater manifest and SHA-256 checksums).

### Security

- Every external command runs from an argv array; no shell concatenation.
- Updates refuse a dirty worktree, a non-official remote or a detached HEAD.
- Destructive git operations require an explicit confirmation dialog.
- Long paths use the `\\?\` prefix; Cyrillic and space-containing paths are passed
  as single argv values.
- No telemetry. Logs stay on disk and never contain credentials.

### Known limitations

- Upstream publishes only `main` and no tags, so the launcher cannot offer a
  stable/latest channel switch. The UI says so instead of pretending otherwise.
- There is no game launch feature, because upstream documents no run command.
- The repository is prepared for publication but not published from this build:
  set the owner with `npm run configure -- --owner <name>` and push.
