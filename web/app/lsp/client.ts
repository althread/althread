import { pathToUri } from "./protocol.ts";
import type {
	Notification,
	RequestParams,
	RequestResults,
} from "./protocol.ts";

export class LanguageClient {
	private worker: Worker;
	private nextId = 0;
	private pending = new Map<
		number,
		{
			resolve: (result: unknown) => void;
			reject: (error: Error) => void;
			timer: ReturnType<typeof setTimeout>;
		}
	>();
	private documents = new Map<string, { text: string; version: number }>();
	private ready: Promise<unknown>;
	private closed = false;

	constructor() {
		this.worker = new Worker(
			new URL("../workers/language-server.worker.ts", import.meta.url),
			{ type: "module" },
		);
		this.worker.onmessage = (event) => {
			const { id, result, error } = event.data;
			const pending = this.pending.get(id);
			if (!pending) return;
			clearTimeout(pending.timer);
			this.pending.delete(id);
			if (error) pending.reject(new Error(error.message));
			else pending.resolve(result);
		};
		this.worker.onerror = () =>
			this.dispose(new Error("Althread language worker failed to start"));
		this.ready = this.send("initialize", {
			capabilities: {
				general: { positionEncodings: ["utf-16"] },
				textDocument: {
					completion: { completionItem: { snippetSupport: true } },
				},
			},
			rootUri: null,
		}).then(() => this.notify({ jsonrpc: "2.0", method: "initialized" }));
		// A failed startup is also reported to requests, without an unhandled rejection.
		void this.ready.catch(() => {});
	}
	private notify(message: Notification): void {
		if (!this.closed) this.worker.postMessage(message);
	}
	private send<K extends keyof RequestParams>(
		method: K,
		params: RequestParams[K],
	): Promise<RequestResults[K]> {
		if (this.closed) return Promise.reject(new Error("Language client closed"));
		const id = this.nextId++;
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				this.pending.delete(id);
				reject(new Error(`Language request timed out: ${method}`));
			}, 15000);
			this.pending.set(id, {
				resolve: (result) => resolve(result as RequestResults[K]),
				reject,
				timer,
			});
			this.worker.postMessage({ jsonrpc: "2.0", id, method, params });
		});
	}
	async request<K extends keyof RequestParams>(
		method: K,
		params: RequestParams[K],
		files: Record<string, string>,
	): Promise<RequestResults[K]> {
		await this.ready;
		// Send only changed documents. The active buffer is overlaid by the caller,
		// so requests never depend on the localStorage save timer.
		const currentUris = new Set(Object.keys(files).map(pathToUri));
		for (const uri of this.documents.keys())
			if (!currentUris.has(uri)) {
				this.notify({
					jsonrpc: "2.0",
					method: "textDocument/didClose",
					params: { textDocument: { uri } },
				});
				this.documents.delete(uri);
			}
		for (const [path, text] of Object.entries(files)) {
			const uri = pathToUri(path);
			const previous = this.documents.get(uri);
			if (previous?.text === text) continue;
			const version = (previous?.version ?? 0) + 1;
			this.documents.set(uri, { text, version });
			if (previous)
				this.notify({
					jsonrpc: "2.0",
					method: "textDocument/didChange",
					params: {
						textDocument: { uri, version },
						contentChanges: [{ text }],
					},
				});
			else
				this.notify({
					jsonrpc: "2.0",
					method: "textDocument/didOpen",
					params: {
						textDocument: {
							uri,
							version,
							text,
							languageId: path.endsWith(".alt") ? "althread" : "plaintext",
						},
					},
				});
		}
		return this.send(method, params);
	}
	dispose(error = new Error("Language client closed")): void {
		if (this.closed) return;
		this.closed = true;
		this.worker.terminate();
		for (const pending of this.pending.values()) {
			clearTimeout(pending.timer);
			pending.reject(error);
		}
		this.pending.clear();
	}
}
