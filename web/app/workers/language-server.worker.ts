import { LanguageServer } from "../lsp/server.ts";
import type { Notification, Request } from "../lsp/protocol.ts";

// Independent of the execution/model-checking worker so an exhaustive check
// cannot block completions or navigation. WASM is only needed for diagnostics.
let compiler: Promise<typeof import("../../pkg/althread_web")> | undefined;
const server = new LanguageServer(async (source, path, files) => {
	compiler ??= import("../../pkg/althread_web").then(async (wasm) => {
		await wasm.default();
		wasm.initialize();
		return wasm;
	});
	return (await compiler).compile(source, path, files);
});
self.onmessage = async (event: MessageEvent<Request | Notification>) => {
	const message = event.data;
	if (!("id" in message)) {
		server.notify(message);
		return;
	}
	try {
		const result = await server.request(message);
		self.postMessage({ jsonrpc: "2.0", id: message.id, result });
	} catch (error) {
		const err = error as { code?: number; message?: string };
		self.postMessage({
			jsonrpc: "2.0",
			id: message.id,
			error: {
				code: err.code ?? -32603,
				message: err.message ?? String(error),
			},
		});
	}
};
