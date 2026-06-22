import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const packagePath = resolve(process.argv[2] ?? '');
if (!packagePath || !existsSync(packagePath)) {
  throw new Error('Usage: node scripts/verify-editor-package.mjs <package.tgz>');
}

const entries = execFileSync('tar', ['-tf', packagePath], { encoding: 'utf8' })
  .split(/\r?\n/)
  .filter(Boolean);

const requiredEntries = [
  'package/package.json',
  'package/README.md',
  'package/language-configuration.json',
  'package/syntaxes/nx.tmLanguage.json',
  'package/syntaxes/nx.markdown.codeblock.tmLanguage.json',
  'package/snippets/nx.json'
];

for (const entry of requiredEntries) {
  if (!entries.includes(entry)) {
    throw new Error(`Packed editor-assets package is missing ${entry}.`);
  }
}

const forbiddenEntryPrefixes = [
  'package/out/',
  'package/server/',
  'package/src/'
];

for (const entry of entries) {
  if (forbiddenEntryPrefixes.some((prefix) => entry.startsWith(prefix))) {
    throw new Error(`Packed editor-assets package includes VS Code extension-only file ${entry}.`);
  }
}

const tempRoot = mkdtempSync(join(tmpdir(), 'nx-language-package-'));
try {
  execFileSync('tar', ['-xzf', packagePath, '-C', tempRoot]);
  const manifestPath = join(tempRoot, 'package', 'package.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));

  if (manifest.name !== '@nx-lang/language') {
    throw new Error(`Packed editor-assets package has unexpected name ${manifest.name}.`);
  }

  if (manifest.dependencies || manifest.devDependencies) {
    throw new Error('Packed editor-assets package should not include runtime or development dependencies.');
  }

  const requiredExports = [
    './grammar',
    './markdown-codeblock-grammar',
    './language-configuration',
    './snippets'
  ];

  for (const exportPath of requiredExports) {
    if (!manifest.exports?.[exportPath]) {
      throw new Error(`Packed editor-assets package is missing export ${exportPath}.`);
    }
  }
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

console.log(`Verified editor-assets package: ${packagePath}`);
