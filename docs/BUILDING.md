# Building locally

Everything below is what CI runs, in the same order. If it works here and
`npm run typecheck` / `cargo test` are green, the Windows workflow will be green
too.

## Toolchain

| Tool | Version used | Needed for |
| --- | --- | --- |
| Node.js | 22 (20.19 minimum) | Vite, Vitest, Tauri CLI |
| Rust | 1.82+ (stable) | the core, via the `x86_64-pc-windows-msvc` target |
| Visual Studio 2022 Build Tools | C++ workload, Windows 10/11 SDK | linking the Rust core and the WebView2 host |
| WebView2 | Evergreen runtime | running the app (present on Windows 11, usually on Windows 10) |

The MSI target needs the WiX toolset. Tauri downloads a pinned WiX 3.14 archive on
the first `--bundles msi` build and caches it under
`%LOCALAPPDATA%\tauri`. That download is why the first MSI build is slower and why
an offline machine cannot produce one — `--bundles nsis` needs only the NSIS
archive, which is also cached, and `npm run build` needs neither.

## Install and run

```powershell
npm install
npm run dev            # Tauri window: Vite on 1420 + cargo dev build
```

`npm run dev` rebuilds the Rust side on change and hot-reloads the renderer. The
first run compiles ~400 crates and takes a few minutes; later runs are seconds.

`npm run dev:web` serves the renderer in a normal browser. There is no backend
there, so the app deliberately shows a "desktop backend is not connected" screen
instead of inventing data. Use it for layout work only.

## Checks

```powershell
npm run typecheck                    # tsc --noEmit, no emit, strict
npm test                             # vitest run
npm run build:web                    # production bundle into dist/
cd src-tauri
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

All six are enforced in CI. `clippy` is run with `-D warnings`, so a style lint
fails the build; run `cargo clippy --fix --allow-dirty --all-targets` only for
mechanical suggestions, and re-read the diff.

The Rust tests are the interesting ones. They cover the parser for upstream log
output, version comparison, the compatibility evaluator (against a temporary
repository built on the fly), the restore-point round trip, settings migration,
path validation and the update pipeline's git helpers. None of them need the
network, a real Leonida checkout, or a game copy.

## The mock upstream

`make_game.py` is a real script that expects a real game. To exercise the UI
without either, generate a fixture:

```powershell
npm run mock:upstream                        # .tmp/upstream-fixture, one commit
npm run mock:upstream -- --add-commits 3     # three more commits on top
npm run mock:upstream -- --path D:\tmp\fake  # somewhere else
```

The fixture is a real git repository with the real entry points (`make_game.py`,
`.gitmodules`, `tools/`, `runtime/`), a real `--help` that lists every required
flag and the `from:` stage list, and an initialised submodule at
`third_party/rpf9-explorer`. `--add-commits` produces genuine new commits, so
fetch/compare/rollback behave exactly as they would with upstream.

Point **Settings → Leonida checkout** at the fixture. Two things are worth
knowing:

- The fixture's remote is the local path, not `ubiquitarianism/leonida`, so
  updates are refused with `The remote does not match the official repository`.
  That is the guard working, not a bug.
- Deleting the fixture between runs resets the app's idea of the checkout; the
  restore points under `%APPDATA%\dev.leonida.launcher` will then point at a
  revision that no longer exists. Clear them from **Updates → Restore points**.

## Installers

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = "C:\Users\<you>\.tauri\leonida-launcher.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
npx tauri build --bundles nsis,msi
```

Without the key the build still succeeds, but it produces no updater artifacts:

```powershell
npx tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

Output, under `src-tauri/target/release/bundle/`:

```
nsis\Leonida Launcher_1.0.0_x64-setup.exe        (~2.5 MB)
nsis\Leonida Launcher_1.0.0_x64-setup.exe.sig
msi\Leonida Launcher_1.0.0_x64_en-US.msi         (~3.4 MB)
msi\Leonida Launcher_1.0.0_x64_en-US.msi.sig
```

The names carry the product name and version because the signed `latest.json`
manifest refers to them verbatim. Release workflows add a copy called
`LeonidaLauncher-Setup-x64.exe` with the same bytes and the same `.sig`; that is
the name the README links to. Never rename the original — the updater would fetch
a file that does not exist.

Install with `/S` for a silent per-user install (`...setup.exe /S`), or
`...setup.exe /S /D=C:\Tools\LeonidaLauncher` to choose the folder. The MSI is a
per-machine package, so it elevates; install it with `msiexec /i <file> /qn` from
an elevated shell (or keep the NSIS setup if you want a no-admin install).

## Icons

`src-tauri/icons/*` and the installer bitmaps are generated, not hand-drawn:

```powershell
npm run icons
```

The script rasterises the mark at every size Tauri wants, writes
`installer/header.bmp` (150×57) and `installer/sidebar.bmp` (164×314) with the
palette from `src/styles/globals.css`, and leaves `icon.ico` with the standard
sizes. Change a colour there and re-run it; do not edit the bitmaps.

## Version bumps

`package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` all carry
the version, and `release.yml` refuses to publish when they disagree with the tag.
Bump all three, run the checks, commit, then tag.

## Cleaning

```powershell
Remove-Item -Recurse -Force dist, src-tauri\target   # full rebuild
cargo clean                                          # inside src-tauri
```

`src-tauri/gen/schemas` is generated by the Tauri build and is git-ignored — it
reappears on the next `dev` run.
