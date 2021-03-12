import * as path from 'path';
import * as vscode from 'vscode';
import * as lsp from 'vscode-languageclient/node';

import { getServerOrDownload } from './download';

const TAG = 'v0.1.0';

let client: lsp.LanguageClient;

export async function activate(context: vscode.ExtensionContext) {
  const serverPath = /*context.extensionMode === vscode.ExtensionMode.Production*/ true
    ? await getServerOrDownload(context, TAG)
    : path.resolve(__dirname, '../../target/release/prosemd-lsp');

  const serverExecutable: lsp.Executable = {
    command: serverPath,
    args: ['--stdio'],
    options: {
      env: { RUST_LOG: 'warn' },
    },
  };

  const serverOptions: lsp.ServerOptions = {
    run: serverExecutable,
    debug: serverExecutable,
  };

  const clientOptions: lsp.LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'markdown' }],
  };

  client = new lsp.LanguageClient(
    'prosemd-lsp',
    'prosemd-lsp',
    serverOptions,
    clientOptions
  );
  client.start();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}
