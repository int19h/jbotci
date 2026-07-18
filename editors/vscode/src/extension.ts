import * as path from 'node:path';

import * as vscode from 'vscode';
import {
  type DocumentSelector,
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
} from 'vscode-languageclient/node';

const CONFIGURATION_SECTION = 'jbotci';
const SERVER_PATH_SETTING = 'serverPath';
const ALL_MARKDOWN_SETTING = 'enableInAllMarkdown';
const INLAYS_SETTING = 'inlays';

interface RawBracketsOptions {
  profile: 'raw-brackets';
  maxNestingDepth?: number;
  constructs?: 'all' | 'sumti-boundaries' | 'bridi-tails';
}

interface InlayConfiguration {
  structureBrackets: boolean | RawBracketsOptions;
  wordBoundaries: boolean;
  rafsiBoundaries: boolean;
}

const DEFAULT_INLAYS: InlayConfiguration = {
  structureBrackets: true,
  wordBoundaries: false,
  rafsiBoundaries: false,
};

const LOJBAN_SELECTOR = { language: 'lojban' };
const JBO_MARKDOWN_SELECTOR = {
  language: 'markdown',
  pattern: '**/*.jbo.md',
};
const ALL_MARKDOWN_SELECTOR = { language: 'markdown' };

let client: LanguageClient | undefined;
let lifecycle = Promise.resolve();

export function activate(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.commands.registerCommand('jbotci.restartServer', () =>
      enqueueLifecycle(async () => {
        await stopClient();
        if (hasEligibleOpenDocument()) {
          await startClient(context);
        } else {
          await vscode.window.showInformationMessage(
            'jbotci will start when a Lojban or eligible Markdown document is opened.',
          );
        }
      }),
    ),
    vscode.workspace.onDidOpenTextDocument(() => {
      void enqueueLifecycle(() => startClientIfNeeded(context));
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration(
          `${CONFIGURATION_SECTION}.${SERVER_PATH_SETTING}`,
        ) ||
        event.affectsConfiguration(
          `${CONFIGURATION_SECTION}.${ALL_MARKDOWN_SETTING}`,
        ) ||
        event.affectsConfiguration(
          `${CONFIGURATION_SECTION}.${INLAYS_SETTING}`,
        )
      ) {
        void enqueueLifecycle(async () => {
          await stopClient();
          await startClientIfNeeded(context);
        });
      }
    }),
  );

  void enqueueLifecycle(() => startClientIfNeeded(context));
}

export function deactivate(): Promise<void> {
  return enqueueLifecycle(stopClient);
}

function enqueueLifecycle(operation: () => Promise<void>): Promise<void> {
  lifecycle = lifecycle.then(operation, operation);
  return lifecycle;
}

async function startClientIfNeeded(
  context: vscode.ExtensionContext,
): Promise<void> {
  if (client === undefined && hasEligibleOpenDocument()) {
    await startClient(context);
  }
}

function hasEligibleOpenDocument(): boolean {
  const includeAllMarkdown = configuration().get<boolean>(
    ALL_MARKDOWN_SETTING,
    false,
  );

  return vscode.workspace.textDocuments.some(
    (document) =>
      vscode.languages.match(LOJBAN_SELECTOR, document) > 0 ||
      vscode.languages.match(JBO_MARKDOWN_SELECTOR, document) > 0 ||
      (includeAllMarkdown &&
        vscode.languages.match(ALL_MARKDOWN_SELECTOR, document) > 0),
  );
}

async function startClient(context: vscode.ExtensionContext): Promise<void> {
  const serverCommand = configuredServerCommand();
  if (serverCommand === undefined) {
    return;
  }

  const documentSelector: DocumentSelector = [
    LOJBAN_SELECTOR,
    JBO_MARKDOWN_SELECTOR,
  ];
  if (configuration().get<boolean>(ALL_MARKDOWN_SETTING, false)) {
    documentSelector.push(ALL_MARKDOWN_SELECTOR);
  }

  const serverOptions: ServerOptions = {
    command: serverCommand,
    args: ['lsp', '--stdio'],
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector,
    initializationOptions: {
      inlays: configuration().get<InlayConfiguration>(
        INLAYS_SETTING,
        DEFAULT_INLAYS,
      ),
    },
  };
  const candidate = new LanguageClient(
    'jbotci',
    'jbotci Language Server',
    serverOptions,
    clientOptions,
  );

  try {
    await candidate.start();
    client = candidate;
    context.subscriptions.push(candidate);
  } catch (error: unknown) {
    await showLaunchError(serverCommand, error);
  }
}

async function stopClient(): Promise<void> {
  const runningClient = client;
  client = undefined;
  if (runningClient !== undefined) {
    await runningClient.stop();
  }
}

function configuredServerCommand(): string | undefined {
  const configuredPath = configuration()
    .get<string>(SERVER_PATH_SETTING, '')
    .trim();
  if (configuredPath.length === 0) {
    return 'jbotci';
  }
  if (!path.isAbsolute(configuredPath)) {
    void vscode.window.showErrorMessage(
      'jbotci.serverPath must be an absolute path to the jbotci executable.',
    );
    return undefined;
  }
  return configuredPath;
}

function configuration(): vscode.WorkspaceConfiguration {
  return vscode.workspace.getConfiguration(CONFIGURATION_SECTION);
}

async function showLaunchError(
  command: string,
  error: unknown,
): Promise<void> {
  const detail = error instanceof Error ? error.message : String(error);
  await vscode.window.showErrorMessage(
    `Could not start the jbotci language server using "${command}". Ensure the executable is installed and runnable, or set "jbotci.serverPath" to its absolute path. Details: ${detail}`,
  );
}
