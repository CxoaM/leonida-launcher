# Roadmap

1.0.0 is shipped as a complete product, not as a milestone toward one. This file
exists so that "not implemented yet" and "will never exist" are distinguishable.

Status legend: **Done** — in 1.0.0. **Next** — planned, with a concrete blocker.
**Not planned** — a deliberate product decision.

## Phases 1–9 (1.0.0)

### Phase 1 — Foundation · Done

Tauri 2 + React 19 + TypeScript + Vite + Tailwind 4 project, custom title bar,
sidebar shell, status bar, design tokens, `prefers-reduced-motion` handling, and
an error boundary that never pretends a crash did not happen. The renderer talks
to the core only through `services/ipc.ts`.

### Phase 2 — Upstream integration layer · Done

Everything that touches the Leonida repository lives behind
`src-tauri/src/leonida`: repository detection, git wrappers, remote validation,
submodule state, the `compatibility.json` spec, the CLI probe, and the build
runner. No page calls git or a process directly. Nothing is forked, vendored or
copied from upstream; the launcher owns zero Leonida code.

### Phase 3 — Installation · Done

Clone into an empty folder, initialise the submodule, probe the result, register
the paths, detect common game locations, validate the chosen game folder without
modifying it, report free space against the documented requirements.

### Phase 4 — Compatibility and updates · Done

Live `--help` probe, revision classification against a verified baseline,
capability derivation, fetch/compare/merge with restore point, automatic rollback,
update history, restore-point listing, and crash recovery via
`restore-point.json`. Destructive paths require explicit consent.

### Phase 5 — Build control · Done

Stage list derived from the installed revision, per-stage dependency hints, argv
invocation of `py -3 -I make_game.py`, streamed logs, real percentages only when
upstream prints them, tree-kill cancellation, resume detection from work-folder
markers, progress events with no fabricated estimates.

### Phase 6 — Diagnostics and dependencies · Done

Environment report (Windows build, WebView2, Git, Python, .NET SDKs, LLVM target,
Vulkan, VS components, storage, network reachability), one-click `winget` install
per dependency, log viewer with search, retention and JSON/TXT export.

### Phase 7 — Launcher self-update and packaging · Done

Signed Tauri updater with an endpoint derived from the baked owner/repository,
download progress with real byte counts, release notes, and a refusal path when
the build is not configured. NSIS (per-user, folder choice, optional desktop
shortcut, Start Menu) plus an MSI, a two-question uninstaller, and a
`LeonidaLauncher-Setup-x64.exe` alias for humans that never replaces the signed
filename.

### Phase 8 — Language and interface polish · Done

Typed EN/RU dictionary with compile-time key parity and an interpolation-parity
test, system-language detection, skeletons instead of spinners where content is
known to be coming, honest empty and error states on every page, keyboard focus
handling and consistent density across all seven pages.

### Phase 9 — Release engineering and documentation · Done

`build.yml` and `release.yml` with version/tag/signing gates, SHA-256 sums,
checksum-verified artifacts, the seven documents in `docs/`, `CHANGELOG.md`,
`LICENSE`, and a configuration script that makes "who owns this build" a
reproducible fact instead of an assumption.

## Next

### Publish the repository — blocked on the owner

The tree is ready; there is no `gh` CLI and no authenticated account in this
environment, and inventing an owner would bake a dead updater endpoint into the
binary. Once the owner is known:

```powershell
npm run configure -- --owner <owner>
git init -b main && git add . && git commit -m "Leonida Launcher 1.0.0"
git remote add origin https://github.com/<owner>/leonida-launcher.git
git push -u origin main
git tag v1.0.0 && git push origin v1.0.0
```

### Play button — blocked on upstream

There is no run command in `make_game.py --help` and no documented launcher, so
there is nothing to invoke. When upstream documents one, this becomes a small
adapter change: a new capability flag, a typed command, one button. Not before.

### Code signing — needs a certificate

SmartScreen warnings are the main friction in distribution. An OV/EV certificate
(money, an organisation, a review process) would fix it. Nothing in the code
changes; the workflow would sign both the app and the installer.

### More upstream channels — blocked on upstream

Upstream publishes `main` only, with no tags and no releases. A stable/latest
switch would require upstream to start tagging. The launcher says so in the UI
instead of pretending a channel exists.

### Adapter updates without a launcher release — possible, not scheduled

`compatibility.json` is compiled in, so a new baseline needs a new launcher build.
Serving it from the release instead would decouple the two, at the cost of trusting
an unsigned remote file. The current model is stricter on purpose; revisit only if
upstream starts moving faster than launcher releases.

### End-to-end UI tests — worthwhile

Vitest covers the pure logic and Rust covers the pipeline, but no automated test
drives the actual WebView. A Playwright harness against `tauri dev` with the mock
upstream fixture would catch layout and state wiring regressions. It needs a
Windows runner with a display, which is why it is not in this release.

## Not planned

- **Downloading the game, keys or assets.** Illegal in most jurisdictions and
  against the upstream project's own terms.
- **Bypassing protection, anti-cheat or licensing.** Out of scope, permanently.
- **Bundling upstream code.** The launcher remains an independent front end.
- **Telemetry or "anonymous usage statistics".** There is nothing to collect that
  would improve this tool enough to justify it.
- **Silent updates of the Leonida checkout.** Every upstream change is a
  deliberate action with a compatibility report attached.
- **An account system, cloud sync or a launcher marketplace.** This is a local
  tool for one machine.
