#!/usr/bin/env node
// Fetches the pinned WASI sysroot the wasm SDK's C sources (tree-sitter's runtime and the NX
// grammar) compile against, into a gitignored cache under the repository, and prints its path.
//
// The sysroot is the `wasi-sysroot` tarball of a wasi-sdk release. Only the version and checksum
// below need to change to move to a newer one.

import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rename, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { execFile } from "node:child_process";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

const wasiSdkVersion = "25.0";
const wasiSdkMajorVersion = "25";
const tarballSha256 = "d09c62c18efcddffe4b2fdd8c5830109cc8e36130cdbc9acdc0bd1b204c942bb";
const tarballUrl =
  `https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${wasiSdkMajorVersion}` +
  `/wasi-sysroot-${wasiSdkVersion}.tar.gz`;

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cacheRoot = path.join(repositoryRoot, ".cache", "wasi-sysroot");
const sysrootPath = path.join(cacheRoot, `wasi-sysroot-${wasiSdkVersion}`);
// Written only after a verified tarball is fully extracted, so an interrupted run is not mistaken
// for a usable sysroot on the next one.
const stampPath = path.join(sysrootPath, ".nx-fetch-stamp");

/**
 * Resolves the pinned WASI sysroot, downloading and verifying it when the cache does not hold it.
 *
 * @returns {Promise<string>} Absolute path to the sysroot directory.
 */
export async function fetchWasiSysroot() {
  if (await isCached()) {
    return sysrootPath;
  }

  await rm(sysrootPath, { recursive: true, force: true });
  await mkdir(cacheRoot, { recursive: true });

  const stagingDirectory = await mkdtemp(path.join(tmpdir(), "nx-wasi-sysroot-"));
  try {
    const tarballPath = path.join(stagingDirectory, "wasi-sysroot.tar.gz");
    await download(tarballUrl, tarballPath);
    await verifyChecksum(tarballPath);

    const extractedRoot = path.join(stagingDirectory, "extracted");
    await mkdir(extractedRoot, { recursive: true });
    await execFileAsync("tar", ["-xzf", tarballPath, "-C", extractedRoot]);

    const extractedSysroot = path.join(extractedRoot, `wasi-sysroot-${wasiSdkVersion}`);
    if (!existsSync(path.join(extractedSysroot, "include"))) {
      throw new Error(
        `The wasi-sysroot tarball did not contain wasi-sysroot-${wasiSdkVersion}/include.`
      );
    }

    await rename(extractedSysroot, sysrootPath);
    await writeFile(stampPath, `${tarballSha256}\n`, "utf8");
  } finally {
    await rm(stagingDirectory, { recursive: true, force: true });
  }

  return sysrootPath;
}

async function isCached() {
  if (!existsSync(stampPath)) {
    return false;
  }

  const stamp = await readFile(stampPath, "utf8");
  return stamp.trim() === tarballSha256;
}

async function download(url, destinationPath) {
  const response = await fetch(url, { redirect: "follow" });
  if (!response.ok) {
    throw new Error(`Downloading ${url} failed with HTTP ${response.status}.`);
  }

  await writeFile(destinationPath, Buffer.from(await response.arrayBuffer()));
}

async function verifyChecksum(tarballPath) {
  const actual = createHash("sha256").update(await readFile(tarballPath)).digest("hex");
  if (actual !== tarballSha256) {
    throw new Error(
      `The wasi-sysroot download does not match its pinned checksum.\n` +
        `  expected: ${tarballSha256}\n` +
        `  actual:   ${actual}\n` +
        `Delete ${cacheRoot} and retry, or update the checksum in scripts/fetch-wasi-sysroot.mjs.`
    );
  }
}

if (process.argv[1] !== undefined && import.meta.url === `file://${path.resolve(process.argv[1])}`) {
  try {
    process.stdout.write(`${await fetchWasiSysroot()}\n`);
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
