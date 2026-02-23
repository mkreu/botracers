import * as vscode from 'vscode';

import { initializeBotProject, openBotProject } from './bootstrap';
import { loginViaWebview } from './auth';
import { configureServerProfile } from './config';
import { BotRacersItem, BotRacersViewProvider } from './views/botracersView';

function registerCommand(
  context: vscode.ExtensionContext,
  command: string,
  fn: (...args: any[]) => Promise<void>
): void {
  context.subscriptions.push(
    vscode.commands.registerCommand(command, async (...args: any[]) => {
      try {
        await fn(...args);
      } catch (error) {
        void vscode.window.showErrorMessage(String(error));
      }
    })
  );
}

export function activate(context: vscode.ExtensionContext): void {
  const provider = new BotRacersViewProvider(context);
  void vscode.commands.executeCommand('setContext', 'botracers.state', 'loggedOut');
  void vscode.commands.executeCommand('setContext', 'botracers.stateDetail', 'notLoggedIn');

  const view = vscode.window.createTreeView('botracers.explorer', {
    treeDataProvider: provider,
    showCollapseAll: true
  });
  context.subscriptions.push(view);

  const refresh = (): void => {
    void provider.refreshArtifacts();
  };

  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders(refresh),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration('botracers')) {
        refresh();
      }
    })
  );

  const cargoWatcher = vscode.workspace.createFileSystemWatcher('**/Cargo.toml');
  const binWatcher = vscode.workspace.createFileSystemWatcher('**/src/bin/*.rs');
  context.subscriptions.push(
    cargoWatcher,
    binWatcher,
    cargoWatcher.onDidChange(refresh),
    cargoWatcher.onDidCreate(refresh),
    cargoWatcher.onDidDelete(refresh),
    binWatcher.onDidChange(refresh),
    binWatcher.onDidCreate(refresh),
    binWatcher.onDidDelete(refresh)
  );

  registerCommand(context, 'botracers.configureServer', async () => {
    await configureServerProfile();
    await provider.refreshArtifacts();
  });

  registerCommand(context, 'botracers.login', async () => {
    const changed = await loginViaWebview(context);
    if (changed) {
      await provider.refreshArtifacts();
    }
  });

  registerCommand(context, 'botracers.initializeBotProject', async () => {
    await initializeBotProject(context);
    await provider.refreshArtifacts();
  });

  registerCommand(context, 'botracers.openBotProject', async () => {
    await openBotProject();
  });

  registerCommand(context, 'botracers.view.refresh', async () => {
    await provider.refreshArtifacts();
  });

  registerCommand(context, 'botracers.view.buildAndUpload', async (item?: BotRacersItem) => {
    await provider.buildAndUploadBinary(item);
  });

  registerCommand(context, 'botracers.view.buildBinary', async (item?: BotRacersItem) => {
    await provider.buildBinaryItem(item);
  });

  registerCommand(context, 'botracers.view.revealElfPath', async (item?: BotRacersItem) => {
    await provider.revealElfPath(item);
  });

  registerCommand(context, 'botracers.view.replaceArtifact', async (item?: BotRacersItem) => {
    await provider.replaceArtifact(item);
  });

  registerCommand(context, 'botracers.view.deleteArtifact', async (item?: BotRacersItem) => {
    await provider.deleteArtifact(item);
  });

  registerCommand(context, 'botracers.view.toggleVisibility', async (item?: BotRacersItem) => {
    await provider.toggleVisibility(item);
  });

  refresh();
}

export function deactivate(): void {}
