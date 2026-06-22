import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const packagePath = resolve(process.argv[2] ?? '');
if (!packagePath) {
  throw new Error('Usage: node scripts/smoke-editor-package.mjs <package.tgz>');
}

const tempRoot = mkdtempSync(join(tmpdir(), 'nx-language-smoke-'));
try {
  writeFileSync(join(tempRoot, 'package.json'), '{"type":"module"}\n');
  execFileSync('npm', ['install', packagePath], {
    cwd: tempRoot,
    stdio: 'inherit'
  });

  const requireFromSmokeProject = createRequire(join(tempRoot, 'smoke.mjs'));
  const grammar = requireFromSmokeProject('@nx-lang/language/grammar');
  const markdownGrammar = requireFromSmokeProject('@nx-lang/language/markdown-codeblock-grammar');
  const languageConfiguration = requireFromSmokeProject('@nx-lang/language/language-configuration');
  const snippets = requireFromSmokeProject('@nx-lang/language/snippets');

  if (grammar.scopeName !== 'source.nx') {
    throw new Error('Unexpected NX grammar export.');
  }

  if (markdownGrammar.scopeName !== 'source.nx.embedded.markdown') {
    throw new Error('Unexpected NX markdown code-block grammar export.');
  }

  if (!Array.isArray(languageConfiguration.brackets)) {
    throw new Error('Unexpected NX language configuration export.');
  }

  if (!snippets || Object.keys(snippets).length === 0) {
    throw new Error('Unexpected NX snippets export.');
  }
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

console.log(`Smoke-tested editor-assets package imports: ${packagePath}`);
