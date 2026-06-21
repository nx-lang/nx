import * as fs from 'node:fs';
import * as path from 'node:path';

export const nxDocumentSelector = [
  { language: 'nx', scheme: 'file' },
  { language: 'nx', scheme: 'untitled' },
  { language: 'nx', scheme: 'nx' }
];

export type ServerPathSource = 'configured' | 'packaged';

export interface ServerPathResolution {
  readonly path: string;
  readonly source: ServerPathSource;
  readonly exists: boolean;
}

export interface ResolveServerPathOptions {
  readonly extensionRoot: string;
  readonly configuredPath?: string;
  readonly platform?: NodeJS.Platform | string;
  readonly arch?: string;
  readonly existsSync?: (candidate: string) => boolean;
}

export function resolveServerPath(options: ResolveServerPathOptions): ServerPathResolution {
  const existsSync = options.existsSync ?? fs.existsSync;
  const configuredPath = options.configuredPath?.trim();
  if (configuredPath) {
    return {
      path: configuredPath,
      source: 'configured',
      exists: existsSync(configuredPath)
    };
  }

  const platform = options.platform ?? process.platform;
  const arch = options.arch ?? process.arch;
  const candidate = path.join(
    options.extensionRoot,
    'server',
    serverPlatformId(platform, arch),
    serverExecutableName(platform)
  );

  return {
    path: candidate,
    source: 'packaged',
    exists: existsSync(candidate)
  };
}

export function serverPlatformId(platform: NodeJS.Platform | string, arch: string): string {
  return `${platform}-${arch}`;
}

export function serverExecutableName(platform: NodeJS.Platform | string): string {
  return platform === 'win32' ? 'nx-lsp.exe' : 'nx-lsp';
}

export function createStartupFailureMessage(
  resolution: ServerPathResolution,
  error?: unknown
): string {
  const source =
    resolution.source === 'configured' ? 'configured nx.server.path' : 'packaged nx-lsp binary';
  const suffix = error instanceof Error ? ` ${error.message}` : '';
  return `Unable to start the NX language server from the ${source}: ${resolution.path}.${suffix}`;
}
