import { execFileSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = fileURLToPath(new URL('.', import.meta.url));
const packageRoot = join(scriptDir, '..');
const manifest = JSON.parse(readFileSync(join(packageRoot, 'package.json'), 'utf8'));
const distRoot = join(packageRoot, 'dist');
const stagingRoot = mkdtempSync(join(tmpdir(), 'nx-language-assets-'));

const assetPaths = [
  'README.md',
  'CHANGELOG.md',
  'LICENSE',
  'language-configuration.json',
  'syntaxes',
  'snippets'
];

function getPackageVersion() {
  for (const key of ['NPM_PACKAGE_VERSION', 'PACKAGE_VERSION', 'RELEASE_VERSION']) {
    const value = process.env[key]?.trim();
    if (value) {
      return value;
    }
  }

  throw new Error('Could not determine @nx-lang/language package version. Run tools/versions/Get-ReleaseVersion.ps1 or set NPM_PACKAGE_VERSION.');
}

try {
  for (const path of assetPaths) {
    cpSync(join(packageRoot, path), join(stagingRoot, path), { recursive: true });
  }

  const packageVersion = getPackageVersion();
  const packageManifest = {
    name: '@nx-lang/language',
    displayName: manifest.displayName,
    version: packageVersion,
    description: 'Reusable TextMate grammar, language configuration, and snippets for the NX language.',
    license: manifest.license,
    keywords: manifest.keywords,
    repository: manifest.repository,
    bugs: manifest.bugs,
    homepage: manifest.homepage,
    type: manifest.type,
    exports: {
      './grammar': './syntaxes/nx.tmLanguage.json',
      './markdown-codeblock-grammar': './syntaxes/nx.markdown.codeblock.tmLanguage.json',
      './language-configuration': './language-configuration.json',
      './snippets': './snippets/nx.json',
      './package.json': './package.json'
    },
    files: [
      'package.json',
      'README.md',
      'CHANGELOG.md',
      'LICENSE',
      'language-configuration.json',
      'syntaxes/**',
      'snippets/**'
    ]
  };

  writeFileSync(join(stagingRoot, 'package.json'), `${JSON.stringify(packageManifest, null, 2)}\n`);
  rmSync(distRoot, { recursive: true, force: true });
  mkdirSync(distRoot, { recursive: true });

  execFileSync('pnpm', ['pack', '--pack-destination', distRoot], {
    cwd: stagingRoot,
    stdio: 'inherit'
  });

  console.log(`Packaged @nx-lang/language ${packageVersion} in ${basename(distRoot)}/`);
} finally {
  rmSync(stagingRoot, { recursive: true, force: true });
}
