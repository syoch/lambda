import * as path from 'path';
import { workspace, ExtensionContext, window, ConfigurationChangeEvent, OutputChannel } from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind, Trace } from 'vscode-languageclient/node';

let client: LanguageClient;
let restarting = false;
let serverOutputChannel: OutputChannel;
let traceOutputChannel: OutputChannel;

export function activate(context: ExtensionContext) {
    console.log('Lambda Language Support extension activated');

    serverOutputChannel = window.createOutputChannel('Lambda Language Server');
    traceOutputChannel = window.createOutputChannel('Lambda LSP Trace');

    // LSP の有効化確認
    const lspConfig = workspace.getConfiguration('lambda.lsp');
    const lspEnabled = lspConfig.get<boolean>('enable') ?? true;

    if (!lspEnabled) {
        console.log('Lambda LSP server is disabled');
        return;
    }

    // LSP サーバーを起動
    if (!startLanguageServer(context)) {
        window.showErrorMessage('Failed to start Lambda LSP server. Please check your configuration.');
        return;
    }

    // 設定変更時の監視
    const configChangeListener = workspace.onDidChangeConfiguration(async (event: ConfigurationChangeEvent) => {
        if (event.affectsConfiguration('lambda.lsp.serverPath') ||
            event.affectsConfiguration('lambda.lsp.enable') ||
            event.affectsConfiguration('lambda.lsp.debug.enable') ||
            event.affectsConfiguration('lambda.lsp.debug.trace')) {
            if (restarting) {
                return;
            }

            restarting = true;
            try {
                const enabled = workspace.getConfiguration('lambda.lsp').get<boolean>('enable') ?? true;
                await safeStopClient();
                if (enabled) {
                    startLanguageServer(context);
                } else {
                    console.log('Lambda LSP server disabled by configuration');
                }
            } finally {
                restarting = false;
            }
        }
    });

    context.subscriptions.push(configChangeListener);

    // コマンドの登録
    registerCommands(context);
}

function startLanguageServer(context: ExtensionContext): boolean {
    try {
        const lspConfig = workspace.getConfiguration('lambda.lsp');
        const serverPath = lspConfig.get<string>('serverPath') ?? 'lambda';
        const serverArgs = lspConfig.get<string[]>('serverArgs') ?? ['lsp'];
        const debugEnable = lspConfig.get<boolean>('debug.enable') ?? false;
        const traceSetting = lspConfig.get<string>('debug.trace') ?? 'off';
        const traceLevel = parseTrace(traceSetting);

        console.log(`Starting Lambda LSP server: ${serverPath} ${serverArgs.join(' ')}`);
        console.log(`Lambda LSP debug: enabled=${debugEnable}, trace=${traceSetting}`);

        // サーバーオプション
        const serverEnv = {
            ...process.env,
            LAMBDA_LSP_DEBUG: debugEnable ? '1' : '0',
            LAMBDA_LSP_TRACE: traceSetting,
        };

        const serverOptions: ServerOptions = {
            run: {
                command: serverPath,
                args: serverArgs,
                transport: TransportKind.stdio,
                options: { env: serverEnv }
            },
            debug: {
                command: serverPath,
                args: serverArgs,
                transport: TransportKind.stdio,
                options: { detached: false, env: serverEnv }
            },
        };

        // クライアントオプション
        const clientOptions: LanguageClientOptions = {
            documentSelector: [
                { scheme: 'file', language: 'lambda' },
                { scheme: 'file', pattern: '**/*.lambda' }
            ],
            synchronize: {
                fileEvents: workspace.createFileSystemWatcher('**/*.lambda'),
                configurationSection: ['lambda'],
            },
            traceOutputChannel,
            initializationOptions: {
                capabilities: {
                    completion: true,
                    inlayHint: true,
                    hover: true,
                    diagnostics: true,
                },
            },
            outputChannel: serverOutputChannel,
        };
        // vscode-languageclient の型定義差分に対応するため、trace は動的に設定する
        (clientOptions as any).trace = traceLevel;

        // 既存のクライアントを停止
        // 既存クライアント停止は外側で行う（レース回避）

        // LSP クライアントを作成
        client = new LanguageClient(
            'lambda',
            'Lambda Language Support',
            serverOptions,
            clientOptions
        );

        // クライアント開始イベントをリッスン
        client.onDidChangeState((event) => {
            console.log(`Lambda LSP client state changed: ${event.newState}`);
            if (event.newState === 2) { // State.Running = 2
                console.log('Lambda LSP client is ready');
                if (debugEnable) {
                    traceOutputChannel.show(true);
                }
            }
        });

        // エラーハンドリング
        client.onNotification('$/logMessage', (message: any) => {
            console.log(`LSP Log: ${message.message}`);
        });

        // クライアントを開始
        client.start();

        // クライアントをサブスクリプションに追加
        return true;
    } catch (error) {
        console.error('Error starting Lambda LSP server:', error);
        window.showErrorMessage(`Failed to start Lambda LSP server: ${error}`);
        return false;
    }
}

function registerCommands(context: ExtensionContext) {
    const vscode = require('vscode');

    // reduce コマンド
    const reduceCommand = () => {
        const editor = vscode.window.activeTextEditor;
        if (editor) {
            const document = editor.document;
            const selection = editor.selection;
            const selectedText = document.getText(selection);
            console.log(`Reducing: ${selectedText}`);
            window.showInformationMessage(`Reducing expression: ${selectedText}`);
        } else {
            window.showWarningMessage('No active editor');
        }
    };

    context.subscriptions.push(
        vscode.commands.registerCommand('lambda.reduce', reduceCommand)
    );

    // LSP サーバーパス設定コマンド
    const configureLspCommand = async () => {
        const lspConfig = workspace.getConfiguration('lambda.lsp');
        const currentPath = lspConfig.get<string>('serverPath') ?? 'lambda';

        const newPath = await window.showInputBox({
            prompt: 'Enter the path to lambda CLI',
            value: currentPath,
            placeHolder: 'e.g., lambda, /usr/local/bin/lambda',
            validateInput: (input) => {
                if (!input.trim()) {
                    return 'Path cannot be empty';
                }
                return null;
            }
        });

        if (newPath !== undefined && newPath !== currentPath) {
            await lspConfig.update('serverPath', newPath, true);
            window.showInformationMessage(`Lambda CLI path updated to: ${newPath}`);
        }
    };

    context.subscriptions.push(
        vscode.commands.registerCommand('lambda.configureLsp', configureLspCommand)
    );

    // Inlay Hints トグルコマンド
    const toggleInlayHints = async () => {
        const config = workspace.getConfiguration('lambda.inlayHints');
        const enabled = config.get<boolean>('enable') ?? true;
        await config.update('enable', !enabled, true);
        window.showInformationMessage(`Inlay Hints ${!enabled ? 'enabled' : 'disabled'}`);
    };

    context.subscriptions.push(
        vscode.commands.registerCommand('lambda.showInlayHints', toggleInlayHints)
    );
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return safeStopClient();
}

async function safeStopClient(): Promise<void> {
    if (!client) {
        return;
    }

    try {
        await client.stop();
    } catch (error) {
        // client が starting/stopped のタイミングで stop が失敗しても無視する
        console.warn('Ignoring client stop error:', error);
    }
}

function parseTrace(value: string): Trace {
    switch (value) {
        case 'messages':
            return Trace.Messages;
        case 'verbose':
            return Trace.Verbose;
        default:
            return Trace.Off;
    }
}
