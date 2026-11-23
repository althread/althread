// LSP 3.17 types for the capabilities implemented by the browser server.
// Positions are zero-based UTF-16, as used by CodeMirror and JavaScript strings.
export interface Position {
	line: number;
	character: number;
}
export interface Range {
	start: Position;
	end: Position;
}
export interface Location {
	uri: string;
	range: Range;
}
export interface TextEdit {
	range: Range;
	newText: string;
}
export interface CompletionItem {
	label: string;
	kind: number;
	detail: string;
	documentation?: string;
	insertText?: string;
	insertTextFormat?: number;
}
export interface Hover {
	contents: { kind: "plaintext"; value: string };
	range: Range;
}
export interface SignatureHelp {
	signatures: { label: string; parameters: { label: string }[] }[];
	activeSignature: number;
	activeParameter: number;
}
export interface Diagnostic {
	range: Range;
	severity: number;
	message: string;
	source: string;
}
export interface TextDocumentPositionParams {
	textDocument: { uri: string };
	position: Position;
}
export interface RequestParams {
	initialize: { capabilities: object; rootUri: string | null };
	"textDocument/completion": TextDocumentPositionParams;
	"textDocument/hover": TextDocumentPositionParams;
	"textDocument/definition": TextDocumentPositionParams;
	"textDocument/references": TextDocumentPositionParams & {
		context: { includeDeclaration: boolean };
	};
	"textDocument/prepareRename": TextDocumentPositionParams;
	"textDocument/rename": TextDocumentPositionParams & { newName: string };
	"textDocument/signatureHelp": TextDocumentPositionParams;
	"textDocument/diagnostic": { textDocument: { uri: string } };
	shutdown: null;
}
export interface RequestResults {
	initialize: {
		capabilities: object;
		serverInfo: { name: string; version: string };
	};
	"textDocument/completion": { isIncomplete: boolean; items: CompletionItem[] };
	"textDocument/hover": Hover | null;
	"textDocument/definition": Location | null;
	"textDocument/references": Location[];
	"textDocument/prepareRename": { range: Range; placeholder: string } | null;
	"textDocument/rename": { changes: Record<string, TextEdit[]> } | null;
	"textDocument/signatureHelp": SignatureHelp | null;
	"textDocument/diagnostic": { kind: "full"; items: Diagnostic[] };
	shutdown: null;
}
export type Request = {
	[K in keyof RequestParams]: {
		jsonrpc: "2.0";
		id: number;
		method: K;
		params: RequestParams[K];
	};
}[keyof RequestParams];
export type Notification =
	| { jsonrpc: "2.0"; method: "initialized" | "exit" }
	| {
			jsonrpc: "2.0";
			method: "textDocument/didOpen";
			params: {
				textDocument: {
					uri: string;
					languageId: string;
					version: number;
					text: string;
				};
			};
	  }
	| {
			jsonrpc: "2.0";
			method: "textDocument/didChange";
			params: {
				textDocument: { uri: string; version: number };
				contentChanges: { text: string }[];
			};
	  }
	| {
			jsonrpc: "2.0";
			method: "textDocument/didClose";
			params: { textDocument: { uri: string } };
	  };

export const pathToUri = (path: string): string =>
	`file:///${path.split("/").map(encodeURIComponent).join("/")}`;
export const uriToPath = (uri: string): string =>
	decodeURIComponent(uri.replace(/^file:\/\/\//, ""));
export function positionAt(text: string, offset: number): Position {
	const before = text.slice(0, Math.max(0, Math.min(offset, text.length)));
	const line = before.split("\n").length - 1;
	return { line, character: before.length - before.lastIndexOf("\n") - 1 };
}
export function offsetAt(text: string, position: Position): number {
	let start = 0;
	for (let line = 0; line < position.line; line++) {
		const end = text.indexOf("\n", start);
		if (end < 0) return text.length;
		start = end + 1;
	}
	const end = text.indexOf("\n", start);
	return Math.min(start + position.character, end < 0 ? text.length : end);
}
export const rangeAt = (text: string, from: number, to: number): Range => ({
	start: positionAt(text, from),
	end: positionAt(text, to),
});
