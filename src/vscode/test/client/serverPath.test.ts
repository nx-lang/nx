import * as path from 'node:path';
import { expect } from 'chai';
import {
  createStartupFailureMessage,
  nxDocumentSelector,
  resolveServerPath,
  serverExecutableName,
  serverPlatformId
} from '../../src/serverPath.js';

describe('NX LSP client server path resolution', function () {
  it('prefers configured nx-lsp path', function () {
    const resolution = resolveServerPath({
      extensionRoot: '/extension',
      configuredPath: '/tools/nx-lsp',
      existsSync: (candidate) => candidate === '/tools/nx-lsp'
    });

    expect(resolution).to.deep.equal({
      path: '/tools/nx-lsp',
      source: 'configured',
      exists: true
    });
  });

  it('falls back to packaged platform server path', function () {
    const extensionRoot = '/extension';
    const expected = path.join(extensionRoot, 'server', 'linux-x64', 'nx-lsp');
    const resolution = resolveServerPath({
      extensionRoot,
      platform: 'linux',
      arch: 'x64',
      existsSync: (candidate) => candidate === expected
    });

    expect(resolution).to.deep.equal({
      path: expected,
      source: 'packaged',
      exists: true
    });
  });

  it('uses Windows executable names', function () {
    expect(serverPlatformId('win32', 'x64')).to.equal('win32-x64');
    expect(serverExecutableName('win32')).to.equal('nx-lsp.exe');
  });

  it('selects NX file, untitled, and logical documents', function () {
    expect(nxDocumentSelector).to.deep.include({ language: 'nx', scheme: 'file' });
    expect(nxDocumentSelector).to.deep.include({ language: 'nx', scheme: 'untitled' });
    expect(nxDocumentSelector).to.deep.include({ language: 'nx', scheme: 'nx' });
  });

  it('reports startup failures without blocking grammar-only activation', function () {
    const message = createStartupFailureMessage({
      path: '/missing/nx-lsp',
      source: 'packaged',
      exists: false
    });

    expect(message).to.contain('Unable to start the NX language server');
    expect(message).to.contain('/missing/nx-lsp');
  });
});
