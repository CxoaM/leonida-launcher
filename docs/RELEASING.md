# Releasing

Two identities have to be in place before a release is meaningful: **who owns the
repository** and **which key signs the updates**. Both are baked into the binary,
so both are decided before the build, not during it.

## 1. Claim the repository

```powershell
npm run configure -- --owner <github-user-or-org>
```

That one command writes the owner into `src-tauri/resources/launcher.json` and the
matching URLs into `package.json`, `src-tauri/Cargo.toml` and the README links. It
never publishes anything and never touches the network.

The updater endpoint is not configured anywhere:

```text
https://github.com/<owner>/leonida-launcher/releases/latest/download/latest.json
```

It is derived from `owner` + `repository` at build time, so a binary can only ever
look for updates in the repository it was configured for. While the owner is still
`YOUR_GITHUB_OWNER`, every self-update path returns `updater_not_configured` with
an explanation instead of a broken request — that is what the current local build
does.

## 2. Signing key

Generate the update keypair once, outside the repository:

```powershell
npx tauri signer generate -w $env:USERPROFILE\.tauri\leonida-launcher.key
```

Two files appear: `leonida-launcher.key` (private) and `leonida-launcher.key.pub`
(public). The public key belongs in the build:

```powershell
npm run configure -- --owner <owner> --pubkey "<contents of leonida-launcher.key.pub>"
```

That writes it into `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`. It is
already set in this working tree; only regenerate it if you are rotating keys.

Rules that are not negotiable:

- The private key never enters the repository, a workflow file, a log, a chat or
  an issue. `*.key` is git-ignored for a reason.
- Lose the key and you cannot update existing installs — they will reject a new
  signature. Keep an offline copy.
- Rotating the key means every installed client must be reinstalled manually,
  because it only trusts the key compiled into it. Ship the new key in a new
  installer first, then start signing with the new key.

## 3. Repository secrets

In **Settings → Secrets and variables → Actions**:

| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | the full contents of `leonida-launcher.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the key password, empty if it has none |

Nothing else. There is no token, no `GITHUB_TOKEN` override, no vendor account.
`release.yml` fails before building if the signing key secret is missing, so an
unsigned updater artifact can never be published by accident.

## 4. Push and tag

The working tree is not a git repository yet. When the owner is known:

```powershell
git init -b main
git add .
git commit -m "Leonida Launcher 1.0.0"
git remote add origin https://github.com/<owner>/leonida-launcher.git
git push -u origin main
```

Then, for each release:

1. Bump the version in `package.json`, `src-tauri/Cargo.toml` and
   `src-tauri/tauri.conf.json` — all three, they must agree.
2. Update `CHANGELOG.md` with a section for that version.
3. Commit, then tag and push the tag:

```powershell
git tag v1.0.0
git push origin v1.0.0
```

The tag is the only release trigger. There is no manual "publish" button and no
workflow that releases on a branch push.

## 5. What the workflow does

`release.yml` runs in two stages:

**Verify** (no secrets used beyond the key check): checkout, `npm ci`,
`npm run typecheck`, `npm test`, `cargo fmt --all --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, then two
gates:

- the tag must be `vX.Y.Z` and must equal the version in all three files;
- `TAURI_SIGNING_PRIVATE_KEY` must be non-empty.

**Release**: builds `--bundles nsis,msi` on `windows-latest` through
`tauri-action`, which creates the GitHub release, uploads the installers and their
`.sig` files, and generates `latest.json` with the signed URLs and the release
notes.

Then two small steps finish the job:

- `LeonidaLauncher-Setup-x64.exe` (and its `.sig`) is copied from the versioned
  NSIS artifact and uploaded. Same bytes, predictable name for humans. The
  versioned file stays, because `latest.json` points at it.
- `SHA256SUMS.txt` is generated over every installer and uploaded.

The last step asserts that `latest.json` and at least one `.exe` are attached,
which is the check that catches a silently disabled updater.

## 6. Verifying a release

```powershell
gh release view v1.0.0 --json assets --jq '.assets[].name'
```

Expected:

```text
latest.json
Leonida Launcher_1.0.0_x64-setup.exe
Leonida Launcher_1.0.0_x64-setup.exe.sig
Leonida Launcher_1.0.0_x64_en-US.msi
Leonida Launcher_1.0.0_x64_en-US.msi.sig
LeonidaLauncher-Setup-x64.exe
LeonidaLauncher-Setup-x64.exe.sig
SHA256SUMS.txt
```

Download the installer, confirm the hash against `SHA256SUMS.txt`, install it, then
open **Settings → Launcher updates → Check now**. It should report the current
version with no update available. Bump the version by hand in a test repository,
publish, and confirm the older build offers the update and installs it — that is
the only real test of the updater.

## 7. When something is wrong

**Wrong asset name in `latest.json`.** `latest.json` is generated from the bundler
output. If you renamed the artifact in a step *before* `tauri-action`, the manifest
points at a file that does not exist and every client fails with a signature or
404 error. Rename after, never before, and keep the original.

**A broken release is already live.** Do not delete the tag. Publish a new patch
release with a fixed build: `latest.json` always serves the *latest* release, so a
newer version automatically supersedes the broken one. Users who already installed
the broken build can still update, because updates compare versions, not release
dates.

**Signature rejected.** The public key in `tauri.conf.json` and the private key in
the secret do not belong together. Compare with
`npx tauri signer sign --help` output format, or regenerate the pair and accept
that existing installs need a manual reinstall.

**The release workflow refuses the tag.** That is the version gate. `v1.0.1` with
`1.0.0` in `Cargo.toml` is exactly the mistake it exists to catch.

## 8. Never do this

- Do not commit `leonida-launcher.key`, even "temporarily".
- Do not put the key in a workflow `env:` literal.
- Do not publish with `createUpdaterArtifacts: false` and then hand-write
  `latest.json` — a hand-written manifest cannot be signed correctly, and the
  launcher will reject or fail to verify it.
- Do not point `launcher.json` at a fork. Updates are verified against
  `ubiquitarianism/leonida` by URL, and the guard is deliberate.
