import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const packageRoot = join(scriptDir, '..');
const executable = process.platform === 'win32' ? 'esbuild.cmd' : 'esbuild';
const result = spawnSync(
  join(packageRoot, 'node_modules', '.bin', executable),
  [
    'src/extension.ts',
    '--bundle',
    '--platform=node',
    '--format=cjs',
    '--target=node18',
    '--external:vscode',
    '--outfile=out/extension.cjs'
  ],
  {
    cwd: packageRoot,
    env: process.env,
    stdio: 'inherit'
  }
);

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
