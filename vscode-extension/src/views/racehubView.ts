import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

import {
  deleteArtifact,
  fetchCapabilities,
  listArtifacts,
  updateArtifactVisibility,
  uploadArtifact
} from '../api';
import { clearToken, readToken } from '../auth';
import { buildBinary } from '../build';
import { defaultArtifactTarget } from '../config';
import { ArtifactSummary } from '../types';
import {
  artifactOutputPath,
  getWorkspaceRoot,
  hasCargoToml,
  listLocalBinaries,
  LocalBinary
} from '../workspace';

type ViewState = 'loggedOut' | 'needsWorkspace' | 'ready';
type ViewStateDetail = 'none' | 'notLoggedIn' | 'sessionExpired' | 'workspaceMissing' | 'noBinaries' | 'requestError';

type RootNodeKind = 'localRoot' | 'remoteRoot';

type Node =
  | { kind: RootNodeKind }
  | { kind: 'localBin'; bin: LocalBinary }
  | { kind: 'remoteArtifact'; artifact: ArtifactSummary }
  | { kind: 'message'; message: string };

export class RaceHubItem extends vscode.TreeItem {
  constructor(public readonly node: Node) {
    super(itemLabel(node), collapsibleState(node));
    this.contextValue = contextValue(node);

    if (node.kind === 'localBin') {
      this.description = node.bin.rootPath;
      this.tooltip = `${node.bin.name} (${node.bin.rootPath})`;
      this.iconPath = new vscode.ThemeIcon('symbol-method');
    }

    if (node.kind === 'remoteArtifact') {
      const artifact = node.artifact;
      this.description = `${artifact.owner_username} · ${artifact.is_public ? 'public' : 'private'}`;
      this.tooltip = `${artifact.name} (#${artifact.id})`;
      this.iconPath = new vscode.ThemeIcon('package');
    }

    if (node.kind === 'message') {
      this.iconPath = new vscode.ThemeIcon('info');
      this.tooltip = node.message;
    }

    if (node.kind === 'localRoot' || node.kind === 'remoteRoot') {
      this.iconPath = new vscode.ThemeIcon('list-tree');
    }
  }
}

function itemLabel(node: Node): string {
  switch (node.kind) {
    case 'localRoot':
      return 'Local Binaries';
    case 'remoteRoot':
      return 'Remote Artifacts';
    case 'localBin':
      return node.bin.name;
    case 'remoteArtifact':
      return node.artifact.name;
    case 'message':
      return node.message;
  }
}

function collapsibleState(node: Node): vscode.TreeItemCollapsibleState {
  if (node.kind === 'localRoot' || node.kind === 'remoteRoot') {
    return vscode.TreeItemCollapsibleState.Expanded;
  }
  return vscode.TreeItemCollapsibleState.None;
}

function contextValue(node: Node): string | undefined {
  switch (node.kind) {
    case 'localRoot':
      return 'localRoot';
    case 'remoteRoot':
      return 'remoteRoot';
    case 'localBin':
      return 'localBin';
    case 'remoteArtifact':
      return node.artifact.owned_by_me ? 'remoteArtifactOwned' : 'remoteArtifact';
    case 'message':
      return 'message';
  }
}

export class RaceHubViewProvider implements vscode.TreeDataProvider<RaceHubItem> {
  private readonly onDidChangeTreeDataEmitter = new vscode.EventEmitter<RaceHubItem | undefined>();
  readonly onDidChangeTreeData = this.onDidChangeTreeDataEmitter.event;

  private state: ViewState = 'loggedOut';
  private stateDetail: ViewStateDetail = 'notLoggedIn';
  private token: string | undefined;
  private workspaceRoot: string | undefined;
  private localBinaries: LocalBinary[] = [];
  private artifacts: ArtifactSummary[] = [];

  constructor(private readonly context: vscode.ExtensionContext) {}

  async refreshArtifacts(): Promise<void> {
    this.artifacts = [];
    this.stateDetail = 'none';
    this.localBinaries = [];
    this.workspaceRoot = getWorkspaceRoot();

    try {
      const caps = await fetchCapabilities();
      if (caps.auth_required) {
        this.token = await readToken(this.context);
        if (!this.token) {
          this.state = 'loggedOut';
          this.stateDetail = 'notLoggedIn';
          await this.pushContext();
          this.onDidChangeTreeDataEmitter.fire(undefined);
          return;
        }
      } else {
        this.token = undefined;
      }

      if (!this.workspaceRoot || !hasCargoToml(this.workspaceRoot)) {
        this.state = 'needsWorkspace';
        this.stateDetail = 'workspaceMissing';
        await this.pushContext();
        this.onDidChangeTreeDataEmitter.fire(undefined);
        return;
      }

      this.localBinaries = listLocalBinaries(this.workspaceRoot);
      if (this.localBinaries.length === 0) {
        this.state = 'needsWorkspace';
        this.stateDetail = 'noBinaries';
        await this.pushContext();
        this.onDidChangeTreeDataEmitter.fire(undefined);
        return;
      }

      this.state = 'ready';
      this.stateDetail = 'none';
      this.artifacts = await listArtifacts(this.token);
      await this.pushContext();
      this.onDidChangeTreeDataEmitter.fire(undefined);
    } catch (error) {
      const message = String(error);
      if (message.includes('401')) {
        await clearToken(this.context);
        this.state = 'loggedOut';
        this.stateDetail = 'sessionExpired';
      } else {
        this.state = 'loggedOut';
        this.stateDetail = 'requestError';
      }
      await this.pushContext();
      this.onDidChangeTreeDataEmitter.fire(undefined);
    }
  }

  private async pushContext(): Promise<void> {
    await vscode.commands.executeCommand('setContext', 'racehub.state', this.state);
    await vscode.commands.executeCommand('setContext', 'racehub.stateDetail', this.stateDetail);
    await this.pushBinSourcePathsContext();
  }

  private async pushBinSourcePathsContext(): Promise<void> {
    const allowed: Record<string, boolean> = {};
    for (const bin of this.localBinaries) {
      if (!bin.sourcePath) {
        continue;
      }
      const normalizedFsPath = path.normalize(bin.sourcePath);
      const uri = vscode.Uri.file(normalizedFsPath);
      allowed[normalizedFsPath] = true;
      allowed[uri.fsPath] = true;
      allowed[uri.path] = true;
    }
    await vscode.commands.executeCommand('setContext', 'racehub.binSourcePaths', allowed);
  }

  getTreeItem(element: RaceHubItem): vscode.TreeItem {
    return element;
  }

  getChildren(element?: RaceHubItem): vscode.ProviderResult<RaceHubItem[]> {
    if (!element) {
      if (this.state === 'loggedOut' || this.state === 'needsWorkspace') {
        return [];
      }

      return [new RaceHubItem({ kind: 'localRoot' }), new RaceHubItem({ kind: 'remoteRoot' })];
    }

    const node = element.node;
    if (node.kind === 'localRoot') {
      return this.localBinaries.map((bin) => new RaceHubItem({ kind: 'localBin', bin }));
    }

    if (node.kind === 'remoteRoot') {
      if (this.artifacts.length === 0) {
        return [new RaceHubItem({ kind: 'message', message: 'No artifacts found' })];
      }
      return this.artifacts.map((artifact) => new RaceHubItem({ kind: 'remoteArtifact', artifact }));
    }

    return [];
  }

  async buildBinaryItem(item?: RaceHubItem): Promise<void> {
    const node = item?.node;
    if (!node || node.kind !== 'localBin') {
      return;
    }
    await buildBinary(node.bin.rootPath, node.bin.name);
    void vscode.window.showInformationMessage(`Built binary '${node.bin.name}'`);
  }

  async revealElfPath(item?: RaceHubItem): Promise<void> {
    const node = item?.node;
    if (!node || node.kind !== 'localBin') {
      return;
    }

    const elfPath = artifactOutputPath(node.bin.rootPath, node.bin.name);
    if (!fs.existsSync(elfPath)) {
      throw new Error(`ELF not found: ${elfPath}. Build the binary first.`);
    }

    await vscode.commands.executeCommand('revealFileInOS', vscode.Uri.file(elfPath));
  }

  async buildAndUploadBinary(item?: RaceHubItem): Promise<void> {
    const node = item?.node;
    if (!node || node.kind !== 'localBin') {
      return;
    }

    await this.uploadFromLocalBinary(node.bin, node.bin.name);
    await this.refreshArtifacts();
  }

  async replaceArtifact(item?: RaceHubItem): Promise<void> {
    const node = item?.node;
    if (!node || node.kind !== 'remoteArtifact') {
      return;
    }
    const artifact = node.artifact;
    if (!artifact.owned_by_me) {
      throw new Error('You can only replace artifacts you own.');
    }

    const bins = this.getCurrentLocalBinaries();
    if (bins.length === 0) {
      throw new Error('No local binaries available in the current bot workspace.');
    }

    const picked = await vscode.window.showQuickPick(
      bins.map((bin) => ({
        label: bin.name,
        description: bin.sourcePath ?? bin.rootPath,
        bin
      })),
      { title: `Replace artifact '${artifact.name}' with local binary` }
    );
    if (!picked) {
      return;
    }

    await this.uploadFromLocalBinary(picked.bin, artifact.name);

    try {
      await deleteArtifact(artifact.id, this.token);
      void vscode.window.showInformationMessage(
        `Replaced artifact '${artifact.name}' by uploading new build and deleting #${artifact.id}`
      );
    } catch (error) {
      void vscode.window.showWarningMessage(
        `Uploaded replacement, but failed to delete old artifact #${artifact.id}: ${String(error)}`
      );
    }

    await this.refreshArtifacts();
  }

  async deleteArtifact(item?: RaceHubItem): Promise<void> {
    const node = item?.node;
    if (!node || node.kind !== 'remoteArtifact') {
      return;
    }

    const artifact = node.artifact;
    if (!artifact.owned_by_me) {
      throw new Error('You can only delete artifacts you own.');
    }

    const confirmed = await vscode.window.showWarningMessage(
      `Delete artifact '${artifact.name}' (#${artifact.id})?`,
      { modal: true },
      'Delete'
    );

    if (confirmed !== 'Delete') {
      return;
    }

    await deleteArtifact(artifact.id, this.token);
    void vscode.window.showInformationMessage(`Deleted artifact '${artifact.name}' (#${artifact.id})`);
    await this.refreshArtifacts();
  }

  async toggleVisibility(item?: RaceHubItem): Promise<void> {
    const node = item?.node;
    if (!node || node.kind !== 'remoteArtifact') {
      return;
    }

    const artifact = node.artifact;
    if (!artifact.owned_by_me) {
      throw new Error('You can only change visibility of artifacts you own.');
    }

    const nextPublic = !artifact.is_public;
    await updateArtifactVisibility(artifact.id, nextPublic, this.token);
    void vscode.window.showInformationMessage(
      `Artifact '${artifact.name}' is now ${nextPublic ? 'public' : 'private'}`
    );
    await this.refreshArtifacts();
  }

  private async uploadFromLocalBinary(bin: LocalBinary, defaultName: string): Promise<void> {
    await buildBinary(bin.rootPath, bin.name);

    const elfPath = artifactOutputPath(bin.rootPath, bin.name);
    if (!fs.existsSync(elfPath)) {
      throw new Error(`ELF not found after build: ${elfPath}`);
    }

    const name = await vscode.window.showInputBox({
      title: 'Artifact Name',
      value: defaultName
    });
    if (!name) {
      return;
    }

    const note = await vscode.window.showInputBox({
      title: 'Artifact Note (optional)',
      value: ''
    });

    const target = defaultArtifactTarget();

    const bytes = fs.readFileSync(elfPath);

    const data = await uploadArtifact(
      {
        name,
        note: note && note.trim().length > 0 ? note.trim() : null,
        target,
        elf_base64: bytes.toString('base64')
      },
      this.token
    );

    void vscode.window.showInformationMessage(`Artifact uploaded: #${data.artifact_id} from '${bin.name}'`);
  }

  private getCurrentLocalBinaries(): LocalBinary[] {
    const workspaceRoot = getWorkspaceRoot();
    if (!workspaceRoot || !hasCargoToml(workspaceRoot)) {
      return [];
    }

    this.workspaceRoot = workspaceRoot;
    this.localBinaries = listLocalBinaries(workspaceRoot);
    return this.localBinaries;
  }
}
