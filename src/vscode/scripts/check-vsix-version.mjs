import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = fileURLToPath(new URL('.', import.meta.url));
const packageRoot = join(scriptDir, '..');
const vsixPath = process.argv[2];
const manifest = vsixPath
  ? JSON.parse(execFileSync('unzip', ['-p', vsixPath, 'extension/package.json'], { encoding: 'utf8' }))
  : JSON.parse(readFileSync(join(packageRoot, 'package.json'), 'utf8'));
const extensionId = `${manifest.publisher}.${manifest.name}`;
const version = manifest.version;

function run(command, args) {
  const executable = process.platform === 'win32' ? `${command}.cmd` : command;
  return execFileSync(join(packageRoot, 'node_modules', '.bin', executable), args, {
    cwd: packageRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  });
}

function versionAppearsInOutput(output) {
  return output.split(/\r?\n/).some((line) => line.trim() === version || line.includes(`"${version}"`));
}

function isNotPublishedError(error) {
  const status = typeof error.status === 'number' ? error.status : 1;
  const output = `${error.stdout?.toString() ?? ''}\n${error.stderr?.toString() ?? ''}`;
  return status !== 0 && /not found|does not exist|doesn't exist|could not find|no extension|404/i.test(output);
}

try {
  const vsceOutput = run('vsce', ['show', extensionId, '--json']);
  if (versionAppearsInOutput(vsceOutput)) {
    throw new Error(`${extensionId} ${version} already exists in the Visual Studio Marketplace.`);
  }
} catch (error) {
  if (isNotPublishedError(error)) {
    console.log(`${extensionId} is not present in the Visual Studio Marketplace yet.`);
  } else {
    throw error;
  }
}

try {
  const ovsxOutput = run('ovsx', ['get', extensionId, '--metadata', '--versionRange', version]);
  if (versionAppearsInOutput(ovsxOutput)) {
    throw new Error(`${extensionId} ${version} already exists in Open VSX.`);
  }
} catch (error) {
  if (isNotPublishedError(error)) {
    console.log(`${extensionId} is not present in Open VSX yet.`);
  } else {
    throw error;
  }
}

console.log(`${extensionId} ${version} is available in VS Code extension registries.`);
