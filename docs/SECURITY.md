# Security posture

This is a modding companion for a project you run locally. Its security job is
narrow and specific: **run other people's programs without letting a path, a
network response or a log line do anything a human did not ask for.**

## What the launcher can do

- Spawn `git`, `py`/`python`, `winget`, `taskkill`, `explorer.exe` and
  `dotnet`, each with an explicit argv array and working directory.
- Read and write inside its own config, data, cache and log folders, and inside
  the checkout, work and output folders you choose.
- Reach `github.com` to clone, fetch and check for launcher releases.

## What it cannot do

- It cannot download game files, keys, decryption tools or executables. There is
  no code path for it.
- It cannot bypass copy protection, anti-cheat or licensing. It never launches the
  game at all — there is no Play action, because upstream documents no run
  command.
- It cannot elevate itself. The installer is per-user (`installMode:
  currentUser`) and the app never requests a UAC prompt. Nothing it does requires
  administrator rights, so a `winget` install that needs elevation asks Windows,
  not the launcher.
- It cannot run commands you did not approve: dependencies are installed from an
  explicit row with a fixed package id, one at a time.
- It cannot phone home. There is no telemetry, no crash reporter, no analytics
  endpoint, no update ping beyond the launcher release check you can disable.

## Command execution

Every process goes through `process::CommandSpec { program, args, cwd, env }` and
`tokio::process::Command`. There is no `cmd /c`, no `sh -c`, no `format!`-built
command line anywhere in the codebase. A path containing spaces, quotes, `&`,
`$`, `%` or Cyrillic characters is one argv element and cannot become syntax.

Two rules make this hold up over time:

- `git` always runs with an explicit `-C`-equivalent working directory, so an
  inherited cwd cannot change which repository is touched.
- Cancellation uses `taskkill /PID <pid> /T /F` with the pid as its own argument.
  The pid comes from the child handle, never from parsed output.

`reveal_path` exists as a dedicated command that calls `process::reveal_in_explorer`
with the path as one argument, precisely so no feature is tempted to build an
`explorer.exe` command line by hand.

## Git safety

Only these git commands can change state: `clone`, `fetch`, `merge --ff-only`,
`submodule update`, `update-ref`, `reset --keep`. There is no `reset --hard`, no
`clean`, no `stash drop`, no `push`, no `force`.

An update requires, in order: a repository, `HEAD` on the configured branch, an
`origin` matching `https://github.com/ubiquitarianism/leonida`, and a clean
tracked worktree. Any failure aborts before anything moves. Before the merge the
current commit is recorded both as `refs/launcher-backup/<timestamp>` and in
`restore-point.json`; both survive a crash, and the next start restores them.

A dirty tree is refused, not stashed. Uncommitted work is the user's and the
launcher has no business deciding what to do with it.

## Launcher updates

Launcher releases are verified with minisign signatures produced by the Tauri
bundler. The public key is compiled into the binary (`plugins.updater.pubkey`);
the private key exists only as a GitHub Actions secret and the maintainer's
offline copy.

Consequences worth stating plainly:

- A compromised release server cannot serve a malicious update, because the
  signature will not match.
- A compromised *build pipeline* can, because it holds the key. GitHub Actions,
  the repository, and the maintainer's account are therefore the real trust
  anchors.
- The endpoint is derived from `owner`/`repository` baked into the binary, so a
  binary cannot be redirected to another repository at runtime.

Leonida itself is **not** verified this way. It is fetched over HTTPS from the
official repository, which means it inherits GitHub's integrity plus whatever the
upstream maintainers publish. That is the same trust a user would extend by
running `git clone` themselves; the launcher adds the clean-tree and
fast-forward guarantees, not a code signature.

## Logs and diagnostics

Log lines are redacted before they reach disk: GitHub tokens (`ghp_…`,
`github_pat_…`), `access_token=`/`refresh_token=`/`api_key=`/`secret=`/`password=`
assignments, `Bearer …` headers, credentials embedded in URLs, and PEM private-key
blocks are replaced before the line is written. Lines are capped in length and the
log has a retention window (14 days by default, configurable).

Diagnostics are written locally and only leave the machine when you export them
and attach the file yourself. They include machine paths and installed tool
versions — review before sharing; there are no tokens in them.

## Renderer hardening

- `withGlobalTauri: false`: no `window.__TAURI__`, so an injected script has no
  ready-made IPC surface.
- A strict CSP: `default-src 'self'`, `script-src 'self'`, no remote origins, no
  inline script in production. `connect-src` allows only `ipc:` and the local IPC
  origin.
- The renderer receives structured snapshots, never a shell command. It cannot
  ask the core to run something the core does not expose as a typed command.
- No `dangerouslySetInnerHTML` anywhere; log output is rendered as text nodes.

## Paths and filesystem

Paths are expanded (`~`, environment variables), validated, and prefixed with
`\\?\` when they exceed `MAX_PATH`. The launcher writes only into folders it owns
or folders you selected for a specific purpose:

| Folder | Owner |
| --- | --- |
| `%APPDATA%\dev.leonida.launcher` | launcher |
| `%LOCALAPPDATA%\dev.leonida.launcher` | launcher |
| checkout, work, output | you, chosen in Installation/Settings |
| your game folder | read only, and only when an upstream stage reads it |

The uninstaller asks twice — once for launcher data, once for the checkout and
output — and never touches the game folder.

## Uninstaller prompts

`src-tauri/installer/hooks.nsh` adds two `MessageBox` questions and nothing else.
No registry keys beyond the standard uninstall entry, no services, no scheduled
tasks, no startup entries except the optional "launch on startup" the user can
turn off in Settings.

## Supply chain

- Rust and npm dependencies are pinned with a lockfile (`Cargo.lock`,
  `package-lock.json`); CI uses `npm ci`.
- The release workflow builds from a tag, runs the full test suite, and refuses to
  publish when the versions disagree or the signing key is absent.
- No dependency is loaded at runtime. There is no plugin marketplace, no
  auto-updating scripting layer.
- `winget` is invoked by explicit package id, one package at a time, only in
  response to a click on that dependency's row.

## Reporting a vulnerability

Open a private security advisory on the launcher repository
(`Security → Report a vulnerability`) rather than a public issue. Include the
launcher version, the Windows build, the affected command, and reproduction steps.

Please do not test against the upstream Leonida repository, its maintainers, or
any third-party infrastructure. This project has no bug bounty and no servers: the
attack surface worth reviewing is the local IPC layer, the installer, and the
update signature path.

## Known limits

Honest list, not a wishlist:

- The build pipeline executes upstream Python. If upstream is malicious, the
  launcher will happily run it — a user who clones and builds upstream code is
  trusting upstream.
- Code signing for the installer is not in place, so SmartScreen reputation is
  the user's problem until it is. Verify `SHA256SUMS.txt`.
- Compatibility is a heuristic over `--help` text. A revision can pass the probe
  and still fail a build; the log is the source of truth there.
- Local settings and restore points are plain files. There is no encryption,
  because there is nothing secret in them.
