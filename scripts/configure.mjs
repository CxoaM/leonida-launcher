#!/usr/bin/env node
/**
 * Bakes the publishing identity into the build.
 *
 * The launcher never guesses who owns it: the repository owner lives in
 * src-tauri/resources/launcher.json and the updater endpoint is derived from it
 * at runtime. This script is the only supported way to set that value, so a
 * release can never point at a repository that does not exist.
 *
 *   npm run configure -- --owner <github-user-or-org>
 *   npm run configure -- --owner acme --repository leonida-launcher
 *   npm run configure -- --owner acme --pubkey "<base64 public key>"
 *
 * Nothing is published here. It only edits files.
 */
import { readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function parseArgs(argv) {
  const args = { owner: null, repository: null, pubkey: null, dryRun: false };
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    switch (token) {
      case "--owner":
        args.owner = argv[++index] ?? null;
        break;
      case "--repository":
      case "--repo":
        args.repository = argv[++index] ?? null;
        break;
      case "--pubkey":
        args.pubkey = argv[++index] ?? null;
        break;
      case "--dry-run":
        args.dryRun = true;
        break;
      case "--help":
      case "-h":
        printUsage();
        process.exit(0);
      default:
        fail(`Unknown option: ${token}`);
    }
  }
  return args;
}

function printUsage() {
  console.log(
    [
      "Usage: npm run configure -- --owner <github-user-or-org> [options]",
      "",
      "Options:",
      "  --owner <name>        GitHub user or organisation that owns the launcher repository",
      "  --repository <name>   Repository name (defaults to the current value)",
      "  --pubkey <base64>     Tauri updater public key (defaults to the current value)",
      "  --dry-run             Show what would change without writing files",
      "",
      "The owner is baked into src-tauri/resources/launcher.json. Releases and",
      "self-updates stay disabled until it is set to a real account.",
    ].join("\n"),
  );
}

function fail(message) {
  console.error(`configure: ${message}`);
  process.exit(1);
}

const OWNER_PATTERN = /^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})$/;
const REPOSITORY_PATTERN = /^[A-Za-z0-9._-]{1,100}$/;
const PUBKEY_PATTERN = /^[A-Za-z0-9+/=]{40,}$/;

async function readText(relative) {
  return readFile(path.join(root, relative), "utf8");
}

async function writeText(relative, content) {
  const target = path.join(root, relative);
  await writeFile(target, content, "utf8");
  console.log(`  updated ${relative}`);
}

function replaceValue(text, pattern, replacement, relative) {
  if (!pattern.test(text)) {
    fail(`Could not find the value to replace in ${relative}`);
  }
  return text.replace(pattern, replacement);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (!args.owner) {
    printUsage();
    fail("--owner is required");
  }
  if (!OWNER_PATTERN.test(args.owner)) {
    fail(`"${args.owner}" is not a valid GitHub account name`);
  }

  const metaPath = "src-tauri/resources/launcher.json";
  const metaRaw = await readText(metaPath);
  const meta = JSON.parse(metaRaw);

  const repository = args.repository ?? meta.repository ?? "leonida-launcher";
  if (!REPOSITORY_PATTERN.test(repository)) {
    fail(`"${repository}" is not a valid repository name`);
  }

  meta.owner = args.owner;
  meta.repository = repository;
  meta.notes =
    "Owner and repository are baked in by scripts/configure.mjs. The updater endpoint is derived from them at build time.";

  const pkgPath = "package.json";
  const pkgRaw = await readText(pkgPath);
  const pkg = JSON.parse(pkgRaw);
  const repoUrl = `https://github.com/${args.owner}/${repository}`;

  console.log(
    `${args.dryRun ? "Would configure" : "Configuring"} ${repoUrl}${args.dryRun ? " (dry run)" : ""}`,
  );

  if (!args.dryRun) {
    await writeText(metaPath, `${JSON.stringify(meta, null, 2)}\n`);

    pkg.repository = { type: "git", url: `${repoUrl}.git` };
    pkg.homepage = repoUrl;
    await writeText(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);

    const cargoPath = "src-tauri/Cargo.toml";
    const cargo = await readText(cargoPath);
    await writeText(
      cargoPath,
      replaceValue(cargo, /^repository = ".*"$/m, `repository = "${repoUrl}"`, cargoPath),
    );

    if (args.pubkey) {
      if (!PUBKEY_PATTERN.test(args.pubkey)) {
        fail("The updater public key does not look like a base64 minisign key");
      }
      const confPath = "src-tauri/tauri.conf.json";
      const conf = await readText(confPath);
      await writeText(
        confPath,
        replaceValue(
          conf,
          /("updater"\s*:\s*\{[^}]*?"pubkey"\s*:\s*")[^"]*(")/s,
          `$1${args.pubkey}$2`,
          confPath,
        ),
      );
    }

    const keyPath = path.join(root, "src-tauri", "resources", "launcher.json");
    if (!existsSync(keyPath)) {
      fail("launcher.json disappeared while writing; check file permissions");
    }
  }

  console.log(
    [
      "",
      "Next steps:",
      "  1. Create the GitHub repository if it does not exist yet:",
      `       https://github.com/new  ->  name: ${repository}  ->  do not add a README`,
      "  2. Push this working tree:",
      "       git init -b main",
      "       git add .",
      '       git commit -m "Leonida Launcher 1.0.0"',
      `       git remote add origin ${repoUrl}.git`,
      "       git push -u origin main",
      "  3. Add the release secrets in GitHub (Settings -> Secrets and variables -> Actions):",
      "       TAURI_SIGNING_PRIVATE_KEY           contents of your private key file",
      "       TAURI_SIGNING_PRIVATE_KEY_PASSWORD  its password, if it has one",
      "  4. Publish the first release by pushing a tag:",
      "       git tag v1.0.0 && git push origin v1.0.0",
      "",
      "The updater public key is already baked into src-tauri/tauri.conf.json.",
      "Keep the private key out of this repository and out of chat logs.",
    ].join("\n"),
  );
}

main().catch((error) => {
  fail(error instanceof Error ? error.message : String(error));
});
