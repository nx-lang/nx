import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect } from 'chai';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const packageRoot = path.join(__dirname, '..', '..');

describe('NX extension activation metadata', function () {
  it('declares the compiled extension entry point and language activation', function () {
    const packageJson = JSON.parse(
      fs.readFileSync(path.join(packageRoot, 'package.json'), 'utf8')
    );

    expect(packageJson.main).to.equal('./out/extension.cjs');
    expect(packageJson.activationEvents).to.include('onLanguage:nx');
    expect(packageJson.files).to.include('out/**');
    expect(packageJson.files).to.include('server/**');
  });

  it('preserves grammar and snippet contributions', function () {
    const packageJson = JSON.parse(
      fs.readFileSync(path.join(packageRoot, 'package.json'), 'utf8')
    );

    expect(packageJson.contributes.grammars).to.have.length.greaterThan(0);
    expect(packageJson.contributes.snippets).to.have.length.greaterThan(0);
  });
});
