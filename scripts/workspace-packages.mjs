/**
 * What the pack and publish scripts share: which workspace packages are publishable, what a packed
 * manifest must look like, and the order the packages go to the registry in.
 */
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

/**
 * The publishable workspace packages, each as `{ root, manifest }`, in the order
 * `pnpm-workspace.yaml` lists their globs.
 *
 * A package is publishable when it is a workspace member and not `private`. Sites are members so
 * that they can depend on the packages, but are never packages themselves.
 */
export function publishablePackages(repositoryRoot) {
  const workspace = readFileSync(join(repositoryRoot, "pnpm-workspace.yaml"), "utf8");
  const globs = [...workspace.matchAll(/^\s*-\s*(\S+)\s*$/gm)].map((match) => match[1]);
  const roots = [];
  for (const glob of globs) {
    if (glob.endsWith("/*")) {
      const parent = join(repositoryRoot, glob.slice(0, -2));
      for (const name of readdirSync(parent).sort()) {
        const root = join(parent, name);
        if (statSync(root).isDirectory()) {
          roots.push(root);
        }
      }
    } else {
      roots.push(join(repositoryRoot, glob));
    }
  }
  const packages = [];
  for (const root of roots) {
    let manifest;
    try {
      manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
    } catch {
      continue;
    }
    if (manifest.private || !manifest.name) {
      continue;
    }
    packages.push({ root, manifest });
  }
  return packages;
}

/**
 * The problems with a packed manifest that must carry `version`: an empty list when there are
 * none.
 */
export function packedManifest(manifest, version) {
  const problems = [];
  if (manifest.version !== version) {
    problems.push(`version is ${manifest.version}, not ${version}`);
  }
  if (manifest.private) {
    problems.push("is marked private");
  }
  if (manifest.publishConfig?.access !== "public") {
    problems.push("does not declare public access");
  }
  for (const field of ["dependencies", "peerDependencies", "optionalDependencies"]) {
    for (const [name, range] of Object.entries(manifest[field] ?? {})) {
      if (String(range).startsWith("workspace:")) {
        problems.push(`${field} still names ${name}@${range}`);
      } else if (name.startsWith("@nx-lang/") && field !== "peerDependencies" && range !== version) {
        problems.push(`${field} pins ${name}@${range} rather than the release version ${version}`);
      }
    }
  }
  return problems;
}

/** Every `.tgz` in `dir` with its `package/package.json`, as `{ file, path, manifest }`. */
export function readTarballManifests(dir) {
  return readdirSync(dir)
    .filter((name) => name.endsWith(".tgz"))
    .sort()
    .map((file) => {
      const path = join(dir, file);
      const text = execFileSync("tar", ["-xOf", path, "package/package.json"], { encoding: "utf8" });
      return { file, path, manifest: JSON.parse(text) };
    });
}

/**
 * The tarballs in dependency order: a package after every `@nx-lang/*` package it depends on that
 * is in the same set, so that a consumer installing a just-published package finds its
 * dependencies on the registry already. Peer dependencies count too: `@nx-lang/monaco` needs
 * `@nx-lang/language` published first, even though it only peers on it.
 */
export function inDependencyOrder(tarballs) {
  const byName = new Map(tarballs.map((tarball) => [tarball.manifest.name, tarball]));
  const ordered = [];
  const visiting = new Set();
  const done = new Set();
  const visit = (tarball) => {
    const { name } = tarball.manifest;
    if (done.has(name)) {
      return;
    }
    if (visiting.has(name)) {
      throw new Error(`dependency cycle through ${name}`);
    }
    visiting.add(name);
    const dependencies = [
      ...Object.keys(tarball.manifest.dependencies ?? {}),
      ...Object.keys(tarball.manifest.peerDependencies ?? {}),
      ...Object.keys(tarball.manifest.optionalDependencies ?? {}),
    ];
    for (const dependency of dependencies.sort()) {
      const other = byName.get(dependency);
      if (other !== undefined) {
        visit(other);
      }
    }
    visiting.delete(name);
    done.add(name);
    ordered.push(tarball);
  };
  for (const tarball of [...tarballs].sort((left, right) => left.manifest.name.localeCompare(right.manifest.name))) {
    visit(tarball);
  }
  return ordered;
}
