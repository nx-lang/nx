import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = fileURLToPath(new URL('.', import.meta.url));
const packageRoot = join(scriptDir, '..');
const repoRoot = join(packageRoot, '..', '..');
const manifestPath = join(packageRoot, 'package.json');

function runNbgv(variableName) {
  return execFileSync('dotnet', ['nbgv', 'get-version', '-v', variableName], {
    cwd: repoRoot,
    encoding: 'utf8'
  }).trim();
}

function getPackageVersion() {
  for (const key of ['VSCODE_EXTENSION_VERSION', 'NBGV_NpmPackageVersion', 'NBGV_NPMPACKAGEVERSION', 'NPM_PACKAGE_VERSION']) {
    const value = process.env[key]?.trim();
    if (value) {
      return value;
    }
  }

  try {
    return runNbgv('NpmPackageVersion');
  } catch (error) {
    throw new Error(`Could not determine VS Code extension version. Run dotnet tool restore or set VSCODE_EXTENSION_VERSION. ${error.message}`);
  }
}

const packageVersion = getPackageVersion();
const vsixVersion = packageVersion.split('-')[0];
if (!/^\d+\.\d+\.\d+$/.test(vsixVersion)) {
  throw new Error(`VS Code extension version '${vsixVersion}' must be major.minor.patch.`);
}

const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
manifest.version = vsixVersion;
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Staged VS Code extension version ${vsixVersion}.`);
