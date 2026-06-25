import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = fileURLToPath(new URL('.', import.meta.url));
const packageRoot = join(scriptDir, '..');
const manifestPath = join(packageRoot, 'package.json');

function getPackageVersion() {
  for (const key of ['VSCODE_EXTENSION_VERSION', 'PACKAGE_VERSION', 'NPM_PACKAGE_VERSION', 'RELEASE_VERSION']) {
    const value = process.env[key]?.trim();
    if (value) {
      return value;
    }
  }

  throw new Error('Could not determine VS Code extension version. Run tools/versions/Get-ReleaseVersion.ps1 or set VSCODE_EXTENSION_VERSION.');
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
