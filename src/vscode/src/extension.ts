import * as vscode from 'vscode';
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions
} from 'vscode-languageclient/node.js';
import {
  createStartupFailureMessage,
  nxDocumentSelector,
  resolveServerPath
} from './serverPath.js';

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const outputChannel = vscode.window.createOutputChannel('NX Language Server');
  context.subscriptions.push(outputChannel);

  const configuredPath = vscode.workspace
    .getConfiguration('nx')
    .get<string>('server.path', '');
  const resolution = resolveServerPath({
    extensionRoot: context.extensionPath,
    configuredPath
  });

  if (!resolution.exists) {
    outputChannel.appendLine(createStartupFailureMessage(resolution));
    return;
  }

  const serverOptions: ServerOptions = {
    command: resolution.path,
    args: ['--stdio']
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: nxDocumentSelector,
    outputChannel
  };

  client = new LanguageClient('nx-lsp', 'NX Language Server', serverOptions, clientOptions);
  context.subscriptions.push(client);

  try {
    await client.start();
    outputChannel.appendLine(`Started NX language server: ${resolution.path}`);
  } catch (error) {
    outputChannel.appendLine(createStartupFailureMessage(resolution, error));
  }
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}
