import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
// `pnpm run publish:all -- <vsix>` passes the `--` through, so it is not an argument.
const [vsixPath] = process.argv.slice(2).filter((arg) => arg !== '--');

if (!vsixPath) {
  console.error('Usage: pnpm run publish:all -- <extension.vsix>');
  process.exit(1);
}

// Fail before either registry is written. The Marketplace signs in through `az login` (see
// publish-vsix.mjs), so only Open VSX has a token to check.
if (!process.env.OVSX_PAT) {
  console.error('Missing required environment variable: OVSX_PAT');
  process.exit(1);
}

// publish-vsix.mjs checks the VSIX and each registry's credentials, and skips a version a registry
// already has, so a rerun after a partial failure completes the other registry.
for (const registry of ['vsce', 'ovsx']) {
  const result = spawnSync(process.execPath, [join(scriptDir, 'publish-vsix.mjs'), registry, vsixPath], {
    stdio: 'inherit',
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
