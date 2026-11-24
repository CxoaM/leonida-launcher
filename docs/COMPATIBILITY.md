# Compatibility model

Leonida has no API version and no release tags. Its public surface is a folder
layout and a command line. That means the launcher cannot ask "which version are
you?" — it has to look and listen every time.

`src-tauri/resources/compatibility.json` is the launcher's written expectation of
that surface. It is **not** a copy of upstream code, a fork, or a vendored
contract. It is a description of what the launcher probes, and a single baseline
commit that a human verified end to end.

```json
{
  "adapterVersion": "1.0.0",
  "upstream": {
    "repository": "https://github.com/ubiquitarianism/leonida",
    "defaultBranch": "main",
    "lastVerifiedCommit": "a28927849adb4e40dfdec8c32fb6758dcd205a37",
    "gameVersion": "1.0.2.1"
  }
}
```

## Baseline, not version

`lastVerifiedCommit` records the last revision at which the whole flow was
exercised: install, compatibility check, a real build. It is a **baseline**, never
a version comparison:

- A revision equal to the baseline is `verified`.
- A descendant of the baseline is `newer` and gets a warning.
- An ancestor is `older` and is reported as such — it is not silently upgraded.
- Anything else (shallow clone, rewritten history, different branch) is
  `unknown`. Only the live probe counts in that case.

The live probe always wins. A revision 40 commits ahead of the baseline with a
clean probe is `degraded`, not broken. A revision equal to the baseline whose
`make_game.py` no longer accepts `--stages` is `incompatible`, baseline or not.

## What is probed

`compatibility::evaluate` runs against the checkout and produces a
`CompatibilityReport`.

| Check id | What it looks at | Severity when it fails |
| --- | --- | --- |
| `launcher` | adapter version range against this launcher build | warning |
| `files` | `requiredFiles`: `make_game.py`, `.gitmodules` | error |
| `dir-<name>` | each `requiredDirectories` entry (`tools`, `runtime`) | error |
| `submodule-<path>` | declared in `.gitmodules` **and** checked out at a SHA | error if `required`, else warning |
| `cli` | `py -3 -I make_game.py --help` exits 0 with output | warning |
| `flags` | every `requiredFlags` entry still appears in `--help` | error |
| `stages` | every `requiredStages` entry still appears in the stage list | error |
| `revision` | relation between installed commit and the baseline | warning when `newer` |

The probe runs `python -I make_game.py --help` with the checkout as the working
directory and `PYTHONUTF8=1`, under a 25 second timeout. `-I` keeps a user
`sitecustomize.py` from changing the answer. If `LEONIDA_PYTHON` is set, that
interpreter is used; otherwise `py -3`, then `python`, then any discovered
install under `%LOCALAPPDATA%\Programs\Python`.

Flags and stages are extracted from the help text, not assumed:

- flags are every `--token` the text mentions;
- stages come from the `from:` list (`comma list from: check,shaders,assets,runtime,pipelines`),
  falling back to the documented set with an explicit warning if the list cannot
  be read.

The last 120 lines of help text are kept in `probedHelp` so the UI can show the
exact evidence instead of a summary of it.

## Statuses

`compatible` — no errors, no warnings. `update` and `build` are allowed.

`degraded` — at least one warning. This is the normal state for a revision ahead
of the baseline, for missing optional files, and for a failed `--help` probe. The
report carries the warnings verbatim. Update and build stay available, but the UI
shows the revision warning banner.

`incompatible` — at least one error. A required file, folder, submodule, flag or
stage is gone. The launcher refuses to **install** an incompatible revision
unless the user explicitly opts in, and it always refuses to **build** one.

`unknown` — nothing usable was probed: no repository, Python missing, help text
unreadable. The launcher does not claim compatibility it could not demonstrate.

## Capabilities are derived, never assumed

```rust
Capabilities {
    install, update, build, assets, runtime, pipelines,
    resume, wav, full_check, wave64_emulate,
    game_launch, game_launch_reason,
}
```

`assets`, `runtime`, `pipelines` are true only when the corresponding stage is in
the probed stage list **and** the build is otherwise ready. `wav`, `full_check`
and `wave64_emulate` follow the presence of `--wav`, `--full-check` and
`--wave64-emulate`. `resume` follows the documented resume phrase in `--help`;
if it disappears, the launcher warns but still lets upstream continue an existing
work folder.

`game_launch` is hard-coded `false` with a reason string. Upstream documents no
launcher or run command, so the launcher exposes no Play button. This is a product
decision and it will stay until the upstream help text says otherwise.

## The update gate

`apply_upstream_update` accepts two explicit consents:

| Option | Default | Meaning |
| --- | --- | --- |
| `verifyCompatibility` | `true` | re-probe after the merge |
| `requireVerified` | `true` | refuse a revision that is `newer` than the baseline |

With defaults, an update that lands on a revision ahead of the baseline is
**rolled back** and reported as `verification_required`, with the message the UI
shows:

> New Leonida version detected. Compatibility verification is required.

The user can then re-run the same update with `requireVerified: false`. That is a
deliberate second click: the live probe passed, the user accepts that the baseline
is behind. Every rejection is recorded in `update-history.json` as `rolled_back`
with the restore ref, so the decision is auditable later.

If the probe finds an actual error, no consent flag helps. The merge is undone and
the error is `incompatible_upstream`, with the failing checks in `detail`.

```text
preflight ─ fetch ─ restore ref ─ merge --ff-only ─ submodules ─ probe
                                             │            │
                                             └─ fail ─────┴─ reset --keep <backup>
```

## Installation consent

`install_leonida` takes `skipCompatibility`. When the freshly cloned revision is
`incompatible`, the launcher stops. `skipCompatibility: true` lets the user keep
the checkout anyway (for inspection, or to wait for an adapter update), but the
Build page keeps refusing a build on that checkout until the probe is clean.
Installing cannot be used to bypass the build gate.

## Bumping the baseline

Do this only after a real end-to-end run, never to silence a warning:

1. Update the checkout to the target revision, with submodules initialised.
2. Run Diagnostics and confirm every required dependency is present.
3. Run a build with the stages you intend to claim (`check` at minimum).
4. Edit `src-tauri/resources/compatibility.json`:
   - `upstream.lastVerifiedCommit` → the full 40-character SHA;
   - `upstream.lastVerifiedAt` → the date;
   - `upstream.gameVersion` → the game copy you used;
   - `adapterVersion` → bump if you changed probe logic or required flags;
   - add or remove `requiredFlags` / `optionalFlags` only from the observed
     `--help` output.
5. Re-run `cargo test --all-targets`. The compatibility tests parse this file, so
   a malformed baseline fails the build.

The file is embedded in the binary at compile time (`include_str!`), so a shipped
launcher and its baseline can never drift apart. That is also why updating the
baseline requires a launcher release — the two moving parts are versioned
together on purpose.
