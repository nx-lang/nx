/**
 * Proves a workspace package is publishable as it stands: packs it with
 * `pnpm pack`, checks the packed manifest carries no `workspace:` specifier and the entry points it
 * declares are in the tarball, installs the tarball into a scratch project, and imports every
 * export. One script for every package, run from each one's `verify:package`.
 *
 * The same proof `@nx-lang/language` requires before release, so publishing is a pipeline change
 * rather than a discovery.
 */
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// The package to verify: the directory given, or the one this is run from, which is the package
// itself when pnpm runs its `verify:package` script.
const packageRoot = resolve(process.argv[2] ?? process.cwd());
const repositoryRoot = resolve(packageRoot, "..", "..");
const manifest = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8"));
const scratch = mkdtempSync(join(tmpdir(), "nx-verify-package-"));

/** Every file an export target names, through nested conditions such as `node` → `import`. */
function exportTargets(target) {
  if (typeof target === "string") {
    return [target];
  }
  return Object.values(target ?? {}).flatMap(exportTargets);
}

/** Whether an export target is a non-JavaScript asset, judged by every file it can resolve to. */
function isAsset(target) {
  return exportTargets(target).every((path) => !/\.(?:m?js|cjs|d\.ts)$/.test(path));
}

function run(command, args, options = {}) {
  return execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"], ...options });
}

try {
  const packDir = join(scratch, "pack");
  run("pnpm", ["pack", "--pack-destination", packDir], { cwd: packageRoot });
  const tarball = readdirSync(packDir).find((name) => name.endsWith(".tgz"));
  if (!tarball) {
    throw new Error(`pnpm pack produced no tarball in ${packDir}`);
  }
  const tarballPath = join(packDir, tarball);

  const entries = run("tar", ["-tzf", tarballPath]).trim().split("\n");
  const required = ["package/package.json", "package/README.md"];
  for (const target of Object.values(manifest.exports ?? {})) {
    for (const path of exportTargets(target)) {
      required.push(`package/${path.replace(/^\.\//, "")}`);
    }
  }
  for (const entry of required) {
    if (!entries.includes(entry)) {
      throw new Error(`Packed ${manifest.name} is missing ${entry}. Was it built?`);
    }
  }
  if (entries.some((entry) => entry.startsWith("package/dist/test/") || entry.startsWith("package/src/"))) {
    throw new Error(`Packed ${manifest.name} includes sources or tests.`);
  }

  const extracted = join(scratch, "extracted");
  mkdirSync(extracted, { recursive: true });
  run("tar", ["-xzf", tarballPath, "-C", extracted]);
  const packed = JSON.parse(readFileSync(join(extracted, "package", "package.json"), "utf8"));
  for (const field of ["dependencies", "peerDependencies", "optionalDependencies"]) {
    for (const [name, range] of Object.entries(packed[field] ?? {})) {
      if (String(range).startsWith("workspace:")) {
        throw new Error(`Packed ${manifest.name} still depends on ${name}@${range}; pnpm pack did not rewrite it.`);
      }
    }
  }
  if (packed.private) {
    throw new Error(`Packed ${manifest.name} is marked private.`);
  }
  if (packed.publishConfig?.access !== "public") {
    throw new Error(`Packed ${manifest.name} does not declare public access.`);
  }

  // Install into a scratch project; everything not linked resolves from the registry the way a
  // consumer's would.
  const consumer = join(scratch, "consumer");
  mkdirSync(consumer, { recursive: true });
  // Workspace siblings are not on any registry yet, so the consumer links them — as overrides, so
  // the tarball's own dependency on them resolves to the link rather than to the registry.
  const overrides = {};
  for (const field of ["dependencies", "peerDependencies"]) {
    for (const name of Object.keys(packed[field] ?? {})) {
      if (name.startsWith("@nx-lang/")) {
        const sibling = name === "@nx-lang/language" ? join(repositoryRoot, "src", "vscode")
          : name === "@nx-lang/sdk-node" ? join(repositoryRoot, "bindings", "node")
          : name === "@nx-lang/sdk-wasm" ? join(repositoryRoot, "bindings", "wasm")
          : name === "@nx-lang/ir-runtime" ? join(repositoryRoot, "runtime", "typescript")
          : join(repositoryRoot, "packages", name.replace("@nx-lang/", ""));
        overrides[name] = `link:${sibling}`;
      }
    }
  }
  writeFileSync(
    join(consumer, "package.json"),
    JSON.stringify(
      {
        name: "nx-verify-consumer",
        private: true,
        type: "module",
        dependencies: {
          [manifest.name]: `file:${tarballPath}`,
          ...overrides,
          ...Object.fromEntries(Object.entries(packed.peerDependencies ?? {}).filter(([name]) => !name.startsWith("@nx-lang/"))),
        },
        pnpm: { overrides },
      },
      null,
      2,
    ),
  );
  writeFileSync(join(consumer, "pnpm-workspace.yaml"), "packages: []\n");
  run("pnpm", ["install", "--ignore-workspace", "--config.confirmModulesPurge=false"], { cwd: consumer, stdio: ["ignore", "ignore", "inherit"] });

  // Every JavaScript entry point is imported; an asset entry point such as `./nx.wasm` is resolved
  // to a file instead, since Node cannot import a module of that type without a flag.
  const probe = join(consumer, "probe.mjs");
  const subpaths = Object.keys(manifest.exports ?? { ".": true }).filter((subpath) => !isAsset(manifest.exports?.[subpath]));
  const assets = Object.keys(manifest.exports ?? {}).filter((subpath) => isAsset(manifest.exports[subpath]));
  // The asset helpers are declared once, ahead of everything: a package with two asset subpaths
  // would otherwise redeclare them and the probe would not parse.
  const lines = assets.length === 0
    ? []
    : ['import { statSync } from "node:fs";', 'import { createRequire } from "node:module";', "const packageRequire = createRequire(import.meta.url);"];
  lines.push(
    ...subpaths.map(
      (subpath, index) =>
        `import * as m${index} from ${JSON.stringify(manifest.name + subpath.slice(1))}; if (Object.keys(m${index}).length === 0) throw new Error("no exports from ${subpath}");`,
    ),
    ...assets.map((subpath) => `statSync(packageRequire.resolve(${JSON.stringify(manifest.name + subpath.slice(1))}));`),
    `console.log('imported', ${subpaths.length}, 'entry point(s)');`,
  );
  writeFileSync(probe, `${lines.join("\n")}\n`);
  run("node", [probe], { cwd: consumer, stdio: "inherit" });
  console.log(`Verified ${manifest.name} package: ${tarball}`);
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
