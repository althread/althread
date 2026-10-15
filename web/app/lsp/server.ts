import { LanguageWorkspace } from "./analysis.ts";
import { rangeAt, uriToPath } from "./protocol.ts";
import type {
	Diagnostic,
	Notification,
	Request,
	RequestResults,
} from "./protocol.ts";

export interface CompilerError {
	message?: string;
	pos?: { start: number; end: number; file_path?: string };
}
export type Compile = (
	source: string,
	filePath: string,
	files: Record<string, string>,
) => Promise<unknown>;

// Rust compiler spans use UTF-8 bytes; LSP and CodeMirror use UTF-16.
export function byteOffsetToUtf16(text: string, bytes: number): number {
	let consumed = 0,
		offset = 0;
	for (const character of text) {
		const size = new TextEncoder().encode(character).length;
		if (consumed + size > bytes) break;
		consumed += size;
		offset += character.length;
	}
	return offset;
}

export class LanguageServer {
	readonly workspace = new LanguageWorkspace();
	private versions = new Map<string, number>();
	private initialized = false;
	private stopped = false;
	constructor(private compile: Compile) {}

	notify(message: Notification): void {
		if (message.method === "exit") {
			this.stopped = true;
			return;
		}
		if (!this.initialized || this.stopped) return;
		switch (message.method) {
			case "textDocument/didOpen": {
				const doc = message.params.textDocument;
				this.workspace.set(doc.uri, doc.text);
				this.versions.set(doc.uri, doc.version);
				break;
			}
			case "textDocument/didChange": {
				const doc = message.params.textDocument;
				if (doc.version <= (this.versions.get(doc.uri) ?? -1)) return;
				const change = message.params.contentChanges.at(-1);
				if (change) {
					this.workspace.set(doc.uri, change.text);
					this.versions.set(doc.uri, doc.version);
				}
				break;
			}
			case "textDocument/didClose":
				this.workspace.delete(message.params.textDocument.uri);
				this.versions.delete(message.params.textDocument.uri);
				break;
		}
	}

	async request(
		message: Request,
	): Promise<RequestResults[keyof RequestResults]> {
		if (message.method === "initialize") {
			this.initialized = true;
			return {
				capabilities: {
					positionEncoding: "utf-16",
					textDocumentSync: { openClose: true, change: 1 },
					completionProvider: { triggerCharacters: [".", ":"] },
					hoverProvider: true,
					definitionProvider: true,
					referencesProvider: true,
					renameProvider: { prepareProvider: true },
					signatureHelpProvider: {
						triggerCharacters: ["(", ","],
						retriggerCharacters: [","],
					},
					diagnosticProvider: {
						interFileDependencies: true,
						workspaceDiagnostics: false,
					},
				},
				serverInfo: { name: "althread-browser-lsp", version: "0.1.0" },
			};
		}
		if (!this.initialized)
			throw Object.assign(new Error("Server not initialized"), {
				code: -32002,
			});
		if (this.stopped) throw new Error("Language server has shut down");
		if (message.method === "shutdown") {
			this.stopped = true;
			return null;
		}
		const { textDocument } = message.params;
		const uri = textDocument.uri;
		switch (message.method) {
			case "textDocument/completion":
				return {
					isIncomplete: true,
					items: this.workspace.completion(uri, message.params.position),
				};
			case "textDocument/hover":
				return this.workspace.hover(uri, message.params.position);
			case "textDocument/definition":
				return this.workspace.definition(uri, message.params.position);
			case "textDocument/references":
				return this.workspace.references(
					uri,
					message.params.position,
					message.params.context.includeDeclaration,
				);
			case "textDocument/prepareRename":
				return this.workspace.prepareRename(uri, message.params.position);
			case "textDocument/rename":
				return this.workspace.rename(
					uri,
					message.params.position,
					message.params.newName,
				);
			case "textDocument/signatureHelp":
				return this.workspace.signatureHelp(uri, message.params.position);
			case "textDocument/diagnostic":
				return this.diagnostics(uri);
			default:
				throw Object.assign(new Error("Unsupported LSP method"), {
					code: -32601,
				});
		}
	}

	private async diagnostics(
		uri: string,
	): Promise<{ kind: "full"; items: Diagnostic[] }> {
		const text = this.workspace.getText(uri);
		const path = uriToPath(uri);
		try {
			await this.compile(text, path, this.workspace.files());
			return { kind: "full", items: [] };
		} catch (error) {
			const err = error as CompilerError;
			const otherFile =
				err.pos?.file_path && err.pos.file_path.replace(/^\.\//, "") !== path;
			const from =
				err.pos && !otherFile ? byteOffsetToUtf16(text, err.pos.start) : 0;
			const to =
				err.pos && !otherFile
					? Math.max(from, byteOffsetToUtf16(text, err.pos.end))
					: text.indexOf("\n") < 0
						? text.length
						: text.indexOf("\n");
			const message =
				typeof err.message === "string" ? err.message : String(error);
			return {
				kind: "full",
				items: [
					{
						range: rangeAt(text, from, to),
						severity: 1,
						source: "althread",
						message: otherFile ? `${err.pos!.file_path}: ${message}` : message,
					},
				],
			};
		}
	}
}
