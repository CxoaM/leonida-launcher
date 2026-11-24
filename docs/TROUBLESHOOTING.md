# Troubleshooting

Start with **Diagnostics → Run report**. It records the OS build, WebView2
version, Git, Python, .NET SDKs, LLVM target, Vulkan, Visual Studio components,
free space on every relevant drive, the repository state and the compatibility
probe. **Export** writes `diagnostics-<timestamp>.json` and `.txt` into the logs
folder and tells you the path.

Log files live in `%APPDATA%\dev.leonida.launcher\logs\launcher-YYYY-MM-DD.log`,
one JSON object per line. **Diagnostics → Logs** shows the same stream with a
search field, and **Open folder** reveals it.

## The launcher will not start

**Blank window.** The renderer crashed before mount. Open
`%APPDATA%\dev.leonida.launcher\logs` and read the last lines. If the window is
still open, the error boundary offers a reload; a reload does not change anything
on disk.

**"Desktop backend is not connected."** You are looking at `npm run dev:web` in a
browser. Only `npm run dev` (or an installed build) has a Rust core behind it.

**Instantly exits after install.** WebView2 is missing. Windows 11 ships it;
Windows 10 2004 usually does but a stripped image may not. Install the Evergreen
Bootstrapper from Microsoft, then start the launcher again.

**Windows SmartScreen warns about the installer.** The build is not
code-signed. There is no EV certificate behind this project, so SmartScreen has no
reputation to go on. Verify the SHA-256 against `SHA256SUMS.txt` from the release,
then choose **More info → Run anyway**. Do not disable SmartScreen globally.

## Installation

**`folder_not_empty`** — the checkout folder must be empty. Pick a new folder
rather than letting the launcher write into an existing project.

**`spawn_failed` while cloning** — `git` could not be started at all: Git for
Windows is not on `PATH`. Check with `git --version` in the environment the
launcher runs from.

**`git_ls_remote_failed` / `git_fetch_failed`** — Git ran but could not reach
`github.com`. A corporate proxy needs `HTTP_PROXY`/`HTTPS_PROXY` in the
environment; the launcher passes the environment through unchanged.

**`clone_failed`** — the clone started but did not finish. The most common causes
are a dropped connection and a full disk. The partial folder is left in place so
you can inspect it; delete it and retry.

**`submodule_failed`** — `third_party/rpf9-explorer` could not be fetched. Run
**Installation → Update submodules** once the connection is back. If the submodule
URL changed upstream, the compatibility report will say the path is no longer
declared in `.gitmodules`.

**`invalid_game_path`** — the folder you picked does not look like a game
install. The launcher checks for the package files it expects; it never scans your
whole drive.

**`disk_space`** — not enough room. The build needs roughly 30 GB for the shader
stage and up to ~370 GB for the full asset pipeline. The message states the
requirement and the free space it measured.

## Updates

**`remote_mismatch`** — the checkout's `origin` is not
`https://github.com/ubiquitarianism/leonida`. This happens with a local fixture,
a mirror, or a fork. Updates are refused by design; if you really want to track a
different remote, point the checkout at that remote yourself and accept that the
compatibility baseline no longer applies.

**`local_changes`** — there are uncommitted changes. The launcher never stashes
or discards them. Commit, or move them aside:

```powershell
cd <checkout>
git status
git stash push -u -m "before update"
```

**`detached_head`** — the checkout is not on a branch. `git switch main` first.

**`diverged_history`** — local `main` and `origin/main` have both moved. The
update is fast-forward-only, so it cannot be applied. Rebase or reset manually;
the launcher will not force it.

**`verification_required`** — the update merged, the probe passed, and the new
revision is ahead of the verified baseline. The previous revision was restored.
This is the compatibility gate, not a failure. Re-run the update with the
"install anyway" confirmation once you accept the risk, or wait for a launcher
release that verifies the new revision.

**`incompatible_upstream`** — the new revision fails a required check (missing
entry point, flag or stage). The previous revision was restored and the failing
checks are listed in the error detail. Nothing you click will install it; the
adapter has to be updated.

**`incompatible_revision`** on install with `skipCompatibility` — the checkout was
kept but is not buildable. **Build** stays disabled for that checkout until the
probe is clean.

**Update was interrupted (power loss, kill).** The next start detects
`restore-point.json` and restores the recorded ref before the window appears.
`recover_interrupted` is idempotent: if the restore already happened, it clears
the file and moves on.

**Rollback refused with `no_backup`.** Restore points are pruned by the retention
setting. A rollback target that was pruned cannot be restored. The current
revision is untouched.

## Builds

**`not_installed`** — no checkout is configured. Finish **Installation** first.

**`build_not_supported`** — the checkout is structurally incompatible. Read the
compatibility checks on the Updates page; the build will not start on a revision
that fails a required check.

**`unsupported_stage`** — the selected stage is not in the installed revision's
`--help` stage list. Reselect the steps; the list is rebuilt from the probe.

**`python_missing`** — no Python 3.12+ found. Run **Diagnostics → Python →
Install** (uses `winget`), or set `LEONIDA_PYTHON` to a full path to `python.exe`.

**`winget_missing`** — dependency installation needs App Installer. Install it
from the Microsoft Store, or install the dependency manually — every dependency
row states the `winget` package id it would use.

**Build fails with MSBuild or clang errors.** That output is upstream's. The
launcher forwards it unmodified, including the failing stage. The usual causes are
a missing Visual Studio C++ workload, a Windows SDK older than 10.0.26100, or an
LLVM build without the AMDGPU target (`llvm-mc --version` must list it).

**The build looks stuck.** Check the log: if the last line is upstream's own
progress and the process is alive, it is working. Cancel kills the whole process
tree, including the MSBuild children.

**Percentage stays indeterminate.** Upstream only prints percentages for some
stages. The launcher shows an indeterminate bar instead of a fabricated number;
that is intended behaviour.

## Paths

**Cyrillic, spaces, `&`, `$` in a path.** Supported everywhere: every process is
spawned with an argv array and the working directory set explicitly, so no path is
ever parsed as a shell command.

**Paths longer than 260 characters.** The launcher prefixes them with `\\?\` when
it hands them to the filesystem. Upstream stages that construct their own paths
can still hit the classic limit; keeping the checkout near the drive root is the
pragmatic fix.

**A folder disappeared.** `path_missing` / `invalid_path` means the configured
path no longer exists. Re-pick it in **Settings**.

## Launcher updates

**"Launcher updates are not configured for this build."** — `launcher.json` still
contains `YOUR_GITHUB_OWNER`. This is a source build, not a release. Run
`npm run configure -- --owner <name>` and rebuild.

**`updater_no_release`** — the endpoint answered but no release is published yet.

**`updater_check`** — the request failed. A proxy or firewall blocking
`github.com` and `objects.githubusercontent.com` is the usual culprit.

**`Signature verification failed`** — the release was signed with a different
key than the one compiled into the launcher. Reinstall from the current release;
trust is pinned at build time.

**Update downloaded but does not restart.** The installer is launched from the
launcher's own folder; if that folder is read-only (installed for all users
without elevation) the replacement fails. Install per-user (the default) or run
the update from an elevated session once.

## Settings and reset

Settings live in `%APPDATA%\dev.leonida.launcher\settings.json`. **Settings →
Reset** restores defaults and keeps your checkout, output and game paths out of
harm's way — it only rewrites preferences. For a full clean state, close the app
and delete that folder; the checkout and build output are separate and untouched.

## Reporting a problem

Include the exported diagnostics file, the last 100 log lines, the exact error
code from the UI (the small monospace token under the message), and the output of
`git -C <checkout> rev-parse HEAD`. Without the commit, a compatibility report
cannot be interpreted.
