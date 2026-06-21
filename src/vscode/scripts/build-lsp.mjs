import { copyFileSync, chmodSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(scriptDir, '..');
const repoRoot = join(packageRoot, '..', '..');
const platformId = process.env.NX_LSP_PLATFORM ?? `${process.platform}-${process.arch}`;
const executableName = platformId.startsWith('win32-') ? 'nx-lsp.exe' : 'nx-lsp';
const cargoTarget = process.env.CARGO_BUILD_TARGET;
const profile = process.env.NX_LSP_PROFILE ?? 'release';
const cargoArgs = ['build', '-p', 'nx-lsp'];

if (profile === 'release') {
  cargoArgs.push('--release');
} else if (profile !== 'debug') {
  console.error(`Unsupported NX_LSP_PROFILE: ${profile}`);
  process.exit(1);
}

if (cargoTarget) {
  cargoArgs.push('--target', cargoTarget);
}

const build = spawnSync('cargo', cargoArgs, {
  cwd: repoRoot,
  env: process.env,
  stdio: 'inherit'
});

if (build.error) {
  console.error(build.error.message);
  process.exit(1);
}

if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

const source = cargoTarget
  ? join(repoRoot, 'target', cargoTarget, profile, executableName)
  : join(repoRoot, 'target', profile, executableName);
const serverDir = join(packageRoot, 'server', platformId);
const target = join(serverDir, executableName);

mkdirSync(serverDir, { recursive: true });
copyFileSync(source, target);

if (process.platform !== 'win32') {
  chmodSync(target, 0o755);
}

console.log(`Copied ${source} -> ${target}`);
