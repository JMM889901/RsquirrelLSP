// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as path from 'path';
import { workspace, ExtensionContext, commands, window } from 'vscode';
import * as vscode from 'vscode';

import {
	Executable,
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
export function activate(context: vscode.ExtensionContext) {

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

	let filename = "LSP"
	if (process.platform == 'win32') {
		filename = filename + ".exe"
	}
	const command = path.resolve(context.extensionPath, "./bin/", filename)
  	const run: Executable = {
  	  command,
  	  options: {
  	    env: {
  	      ...process.env,
  	      // eslint-disable-next-line @typescript-eslint/naming-convention
  	      RUST_LOG: "debug",
  	    },
  	  },
  	};

	const debugrun: Executable = {
	command,
	options: {
		env: {
				...process.env,
				RUST_BACKTRACE: 1,
				RUST_LOG: "debug"
			}
		}
	}

	
	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used
	const serverOptions: ServerOptions = {
		run: run,
		debug: debugrun
	};

	// Options to control the language client
	const clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [{ scheme: 'file', language: 'squirrel' }],
		synchronize: {
			// Notify the server about file changes to '.clientrc files contained in the workspace
			fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
		}
	};

	

	// Create the language client and start the client.
	client = new LanguageClient(
		'SquirrelLSP',
		'SquirrelLSP',
		serverOptions,
		clientOptions
	);

	// Start the client. This will also launch the server
	client.start();
	window.showInformationMessage("done")

	context.subscriptions.push(disposable);
}

// This method is called when your extension is deactivated
export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}
