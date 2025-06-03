"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
const path = __importStar(require("path"));
const vscode_1 = require("vscode");
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
let client;
// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
function activate(context) {
    // Use the console to output diagnostic information (console.log) and errors (console.error)
    // This line of code will only be executed once when your extension is activated
    console.log('Congratulations, your extension "RSqLSP" is now active!');
    // The command has been defined in the package.json file
    // Now provide the implementation of the command with registerCommand
    // The commandId parameter must match the command field in package.json
    const disposable = vscode.commands.registerCommand('RSqLSP.helloWorld', () => {
        // The code you place here will be executed every time your command is executed
        // Display a message box to the user
        vscode.window.showInformationMessage('Hello World from RSquirrel Language Support!');
    });
    let filename = "LSP";
    if (process.platform == 'win32') {
        filename = filename + ".exe";
    }
    const command = path.resolve(context.extensionPath, "./bin/", filename);
    const run = {
        command,
        options: {
            env: {
                ...process.env,
                // eslint-disable-next-line @typescript-eslint/naming-convention
                RUST_LOG: "debug",
            },
        },
    };
    const debugrun = {
        command,
        options: {
            env: {
                ...process.env,
                RUST_BACKTRACE: 1,
                RUST_LOG: "debug"
            }
        }
    };
    // If the extension is launched in debug mode then the debug server options are used
    // Otherwise the run options are used
    const serverOptions = {
        run: run,
        debug: debugrun
    };
    // Options to control the language client
    const clientOptions = {
        // Register the server for plain text documents
        documentSelector: [{ scheme: 'file', language: 'squirrel' }],
        synchronize: {
            // Notify the server about file changes to '.clientrc files contained in the workspace
            fileEvents: vscode_1.workspace.createFileSystemWatcher('**/.clientrc')
        }
    };
    // Create the language client and start the client.
    client = new node_1.LanguageClient('SquirrelLSP', 'SquirrelLSP', serverOptions, clientOptions);
    // Start the client. This will also launch the server
    client.start();
    vscode_1.window.showInformationMessage("done");
    context.subscriptions.push(disposable);
}
// This method is called when your extension is deactivated
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
//# sourceMappingURL=extension.js.map