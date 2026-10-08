/**
 * What the crate pack, verify and publish scripts share: which crates go to crates.io, how the
 * workspace is put at a release version while they are packaged, and what a packed crate must
 * look like.
 */
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/**
 * The crates a Rust host needs to run compiled NX, in dependency order: a crate after every crate
 * of the list it depends on. Every other member of the workspace is `publish = false`.
 */
export const runtimeCrates = ["nx-ir", "nx-value", "nx-ir-runtime"];

/** The crates of the list each crate depends on, which its packed manifest must pin exactly. */
const internalDependencies = {
  "nx-ir": [],
  "nx-value": [],
  "nx-ir-runtime": ["nx-ir", "nx-value"],
};

export function isSemanticVersion(version) {
  return /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version);
}

/** The file a crate is packaged as. */
export function crateFile(name, version) {
  return `${name}-${version}.crate`;
}

export function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

/** Replaces the one match of `pattern` in `text`, and fails when there is not exactly one. */
function replaceOnce(text, pattern, replacement, what) {
  const matches = text.match(new RegExp(pattern.source, `${pattern.flags.replace("g", "")}g`)) ?? [];
  if (matches.length !== 1) {
    throw new Error(`Cargo.toml: expected one ${what}, found ${matches.length}`);
  }
  return text.replace(pattern, replacement);
}

/**
 * Runs `work`, which may be asynchronous, with the workspace at `version`: `[workspace.package]
 * version` set to it and the requirements the runtime crates have on each other set to exactly it.
 * The root manifest and the lock file, which Cargo rewrites for the new version, are put back
 * afterwards, so a local run leaves the tree as it found it.
 */
export async function atReleaseVersion(repositoryRoot, version, work) {
  const manifestPath = join(repositoryRoot, "Cargo.toml");
  const lockPath = join(repositoryRoot, "Cargo.lock");
  const manifest = readFileSync(manifestPath, "utf8");
  const lock = readFileSync(lockPath, "utf8");

  let text = replaceOnce(
    manifest,
    /(\[workspace\.package\]\r?\nversion = ")[^"]+(")/,
    `$1${version}$2`,
    "[workspace.package] version",
  );
  for (const name of runtimeCrates) {
    if (!runtimeCrates.some((other) => internalDependencies[other].includes(name))) {
      continue;
    }
    text = replaceOnce(
      text,
      new RegExp(`^(${name} = \\{ path = "crates/${name}", version = ")[^"]+(" \\})$`, "m"),
      `$1=${version}$2`,
      `[workspace.dependencies] entry for ${name}`,
    );
  }

  try {
    writeFileSync(manifestPath, text);
    return await work();
  } finally {
    writeFileSync(manifestPath, manifest);
    writeFileSync(lockPath, lock);
  }
}

function targetDirectory(repositoryRoot) {
  const metadata = execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
    cwd: repositoryRoot,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  return JSON.parse(metadata).target_directory;
}

/**
 * Packages the runtime crates of a workspace that is already at `version` and copies the `.crate`
 * files to `outputDir`. `cargo package` builds each crate from its packaged contents alone, with
 * the others taken from their packages, so a crate that reads a file outside its own directory or
 * depends on an unpublished crate fails here. Returns each as `{ name, file, path }`.
 *
 * The tree is not clean, since the version was just written into it, so Cargo is told to allow
 * that; what goes into a package is decided by each crate's `include`.
 */
export function packageCrates(repositoryRoot, version, outputDir) {
  execFileSync(
    "cargo",
    ["package", ...runtimeCrates.flatMap((name) => ["-p", name]), "--allow-dirty"],
    { cwd: repositoryRoot, stdio: "inherit" },
  );
  mkdirSync(outputDir, { recursive: true });
  const packaged = join(targetDirectory(repositoryRoot), "package");
  return runtimeCrates.map((name) => {
    const file = crateFile(name, version);
    const source = join(packaged, file);
    if (!existsSync(source)) {
      throw new Error(`cargo package did not produce ${file}`);
    }
    const path = join(outputDir, file);
    copyFileSync(source, path);
    return { name, file, path };
  });
}

/** Packs the runtime crates at `version` into `outputDir`, leaving the tree as it was. */
export function packCrates(repositoryRoot, version, outputDir) {
  return atReleaseVersion(repositoryRoot, version, () => packageCrates(repositoryRoot, version, outputDir));
}

/** A table of a normalized manifest, as its `key = "value"` string entries. */
function table(manifest, header) {
  const start = manifest.indexOf(`[${header}]\n`);
  if (start < 0) {
    return undefined;
  }
  const body = manifest.slice(start + header.length + 3);
  const end = body.search(/^\[/m);
  const entries = {};
  for (const [, key, value] of (end < 0 ? body : body.slice(0, end)).matchAll(/^([\w-]+) = "([^"]*)"$/gm)) {
    entries[key] = value;
  }
  return entries;
}

/**
 * The problems with a packed crate that must be `name` at `version`: an empty list when there are
 * none. The manifest read is the one Cargo wrote into the package, which is what the registry and
 * a consumer see.
 */
export function packedCrateProblems(path, name, version) {
  const problems = [];
  let manifest;
  try {
    manifest = execFileSync("tar", ["-xzOf", path, `${name}-${version}/Cargo.toml`], { encoding: "utf8" });
  } catch {
    return [`holds no ${name}-${version}/Cargo.toml`];
  }
  const packageTable = table(manifest, "package") ?? {};
  if (packageTable.name !== name) {
    problems.push(`is named ${packageTable.name}, not ${name}`);
  }
  if (packageTable.version !== version) {
    problems.push(`version is ${packageTable.version}, not ${version}`);
  }
  for (const field of ["description", "license", "repository", "readme"]) {
    if (!packageTable[field]) {
      problems.push(`declares no ${field}`);
    }
  }
  for (const dependency of internalDependencies[name]) {
    const entry = table(manifest, `dependencies.${dependency}`);
    if (entry === undefined) {
      problems.push(`does not depend on ${dependency}`);
    } else if (entry.version !== `=${version}`) {
      problems.push(`requires ${dependency} ${entry.version} rather than exactly the release version =${version}`);
    }
  }
  // Any other crate of ours in the manifest is one that is not published.
  for (const [, kind, dependency] of manifest.matchAll(/^\[((?:dev-|build-)?dependencies)\.(nx-[\w-]+)\]$/gm)) {
    if (!internalDependencies[name].includes(dependency) || kind !== "dependencies") {
      problems.push(`${kind} names ${dependency}, which is not a published dependency of ${name}`);
    }
  }
  return problems;
}
