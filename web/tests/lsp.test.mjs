import assert from "node:assert/strict";
import { test } from "node:test";
import { LanguageWorkspace } from "../app/lsp/analysis.ts";
import { LanguageServer, byteOffsetToUtf16 } from "../app/lsp/server.ts";
import {
	pathToUri,
	uriToPath,
	positionAt,
	offsetAt,
} from "../app/lsp/protocol.ts";
const uri = pathToUri("main.alt");
const position = (source, needle, delta = needle.length) =>
	positionAt(source, source.indexOf(needle) + delta);
function workspace(source, files = {}) {
	const ws = new LanguageWorkspace();
	for (const [path, text] of Object.entries(files))
		ws.set(pathToUri(path), text);
	ws.set(uri, source);
	return ws;
}
const labels = (items) => items.map((i) => i.label);

test("completion observes lexical scope, declaration order and parameters", () => {
	const source = `shared { let Count: int = 0; }
fn total(n: int) -> int { let local = 3; return n; }
main { let outer = 1; if true { let inner = 2; inn } out let later = 4; }`;
	const ws = workspace(source);
	const inside = labels(ws.completion(uri, position(source, "inn }", 3)));
	assert.ok(inside.includes("inner"));
	assert.ok(inside.includes("outer"));
	assert.ok(inside.includes("Count"));
	assert.ok(!inside.includes("later"));
	assert.ok(!inside.includes("n"));
	assert.ok(!inside.includes("local"));
	assert.ok(
		!labels(ws.completion(uri, position(source, "out let", 3))).includes(
			"inner",
		),
	);
	assert.ok(
		labels(ws.completion(uri, position(source, "return n", 8))).includes("n"),
	);
});

test("receiver completion uses inferred and annotated types, including tuples", () => {
	const source =
		"main { let values = [1, 2]; values. let pair: tuple(int,string); pair. let value = 2; value. }";
	const ws = workspace(source);
	assert.deepEqual(
		labels(ws.completion(uri, position(source, "values."))).sort(),
		["at", "len", "push", "remove", "set"],
	);
	assert.equal(
		ws
			.completion(uri, position(source, "values."))
			.find((i) => i.label === "push").detail,
		"push(value: int) -> void",
	);
	assert.deepEqual(
		labels(ws.completion(uri, position(source, "pair."))).sort(),
		["0", "1", "len", "set0", "set1"],
	);
	assert.deepEqual(ws.completion(uri, position(source, "value.")), []);
});

test("nested tuple destructuring, range loops and receive patterns create bindings", () => {
	const source =
		'main { let (a, (b, c)) = (1, (true, "hello")); for i in 0..10 { print(i); } await receive inbox(item) => { print(item); } }';
	const ws = workspace(source);
	assert.match(
		ws.hover(uri, position(source, "print(i", 7)).contents.value,
		/i: int/,
	);
	assert.match(
		ws.hover(uri, position(source, "print(item", 10)).contents.value,
		/received value/,
	);
	assert.match(
		ws.hover(uri, position(source, "b, c", 1)).contents.value,
		/b: bool/,
	);
});

test("imports respect aliases, nested modules, privacy and file paths", () => {
	const source =
		"import { lib/math as m } main { let answer = m.add(1, 2); m. }";
	const files = {
		"lib/math.alt":
			"import { helpers as h } fn add(a: int, b: int) -> int { return a + b; } @private fn hidden() -> int { return 0; }",
		"lib/helpers.alt": "fn twice(x: int) -> int { return x * 2; }",
		"unrelated.alt": "fn leak() -> int { return 0; }",
	};
	const ws = workspace(source, files);
	const completion = labels(ws.completion(uri, position(source, "m. }", 2)));
	assert.ok(completion.includes("add"));
	assert.ok(completion.includes("h"));
	assert.ok(!completion.includes("hidden"));
	assert.ok(!completion.includes("leak"));
	const target = ws.definition(uri, position(source, "m.add", 5));
	assert.equal(uriToPath(target.uri), "lib/math.alt");
	assert.match(
		ws.hover(uri, position(source, "answer", 3)).contents.value,
		/int/,
	);
});

test("completion only suggests programs after run and types after colon", () => {
	const source =
		"program Worker(id: int) {} fn work() -> int { return 0; } main { run Wo; let x: in }";
	const ws = workspace(source);
	assert.deepEqual(labels(ws.completion(uri, position(source, "run Wo"))), [
		"Worker",
	]);
	const items = labels(ws.completion(uri, position(source, "x: in")));
	assert.ok(items.includes("int"));
	assert.ok(!items.includes("Worker"));
});

test("comments and strings do not leak declarations or offer completion", () => {
	const source =
		'main { // let fake = 2;\n let s = "let nope = 3"; /* hidden */ print(s); }';
	const ws = workspace(source);
	assert.deepEqual(ws.completion(uri, position(source, "fake", 2)), []);
	assert.deepEqual(ws.completion(uri, position(source, "nope", 2)), []);
	assert.deepEqual(ws.completion(uri, position(source, "hidden", 3)), []);
	assert.ok(
		!labels(ws.completion(uri, position(source, "print(s", 7))).includes(
			"fake",
		),
	);
});

test("incomplete code retains declarations, types and function signatures", () => {
	const source =
		"fn sum(a: int, b: int) -> int { return a + b; } main { let items: list(int); let count = 1; sum(count, ";
	const ws = workspace(source);
	assert.ok(
		labels(ws.completion(uri, positionAt(source, source.length))).includes(
			"count",
		),
	);
	const help = ws.signatureHelp(uri, positionAt(source, source.length));
	assert.equal(help.activeParameter, 1);
	assert.equal(help.signatures[0].parameters[1].label, "b: int");
});

test("references and rename distinguish shadowed names across scopes and files", () => {
	const source =
		"fn compute(x: int) -> int { return x; } main { let x = 1; if true { let x = 2; print(x); } print(x); compute(x); }";
	const ws = workspace(source, {
		"other.alt": "import { main as m } main { print(m.compute(2)); }",
	});
	const outer = ws.references(uri, position(source, "let x", 5));
	assert.equal(outer.length, 3);
	const parameter = ws.references(uri, position(source, "x: int", 1));
	assert.equal(parameter.length, 2);
	const edit = ws.rename(uri, position(source, "compute", 4), "calculate");
	assert.equal(edit.changes[uri].length, 2);
	assert.equal(edit.changes[pathToUri("other.alt")].length, 1);
	assert.equal(ws.rename(uri, position(source, "let x", 5), "if"), null);
});

test("rename rejects capture and never edits dependencies", () => {
	const source = "main { let x = 1; if true { let y = 2; print(x); } }";
	const ws = workspace(source);
	assert.equal(ws.rename(uri, position(source, "let x", 5), "y"), null);
	const imported = "import { deps/lib as m } main { m.call(); }";
	const dep = workspace(imported, { "deps/lib.alt": "fn call() -> void {}" });
	assert.equal(dep.prepareRename(uri, position(imported, "m.call")), null);
});

test("LSP positions round-trip UTF-16 and compiler byte spans convert correctly", () => {
	const text = '// é😀\r\nmain { print("été"); }';
	for (let offset = 0; offset <= text.length; offset++)
		assert.equal(offsetAt(text, positionAt(text, offset)), offset);
	const prefix = "// é😀\r\n";
	assert.equal(
		byteOffsetToUtf16(text, new TextEncoder().encode(prefix).length),
		prefix.length,
	);
	const path = "lib/my file #é.alt";
	assert.equal(uriToPath(pathToUri(path)), path);
});

test("server speaks JSON-RPC LSP, discards old versions, and closes documents", async () => {
	const server = new LanguageServer(async () => {});
	const call = (method, params) =>
		server.request({ jsonrpc: "2.0", id: 1, method, params });
	const capabilities = await call("initialize", {
		capabilities: {},
		rootUri: null,
	});
	assert.ok(capabilities.capabilities.completionProvider);
	const open = (text, version) =>
		server.notify({
			jsonrpc: "2.0",
			method: "textDocument/didOpen",
			params: { textDocument: { uri, languageId: "althread", version, text } },
		});
	open("main { let fresh = 2; fresh }", 2);
	server.notify({
		jsonrpc: "2.0",
		method: "textDocument/didChange",
		params: {
			textDocument: { uri, version: 1 },
			contentChanges: [{ text: "old" }],
		},
	});
	assert.match(server.workspace.getText(uri), /fresh/);
	const result = await call("textDocument/completion", {
		textDocument: { uri },
		position: { line: 0, character: 25 },
	});
	assert.ok(result.items.some((i) => i.label === "fresh"));
	server.notify({
		jsonrpc: "2.0",
		method: "textDocument/didClose",
		params: { textDocument: { uri } },
	});
	assert.equal(server.workspace.getText(uri), "");
});

test("compiler diagnostics use byte ranges and identify errors in imported files", async () => {
	const text = "// é😀\nmain { bad; }";
	const start = new TextEncoder().encode(
		text.slice(0, text.indexOf("bad")),
	).length;
	let compilerError = {
		message: "Unknown variable",
		pos: { start, end: start + 3, file_path: "main.alt" },
	};
	const server = new LanguageServer(async () => {
		throw compilerError;
	});
	await server.request({
		jsonrpc: "2.0",
		id: 1,
		method: "initialize",
		params: { capabilities: {}, rootUri: null },
	});
	server.workspace.set(uri, text);
	const request = {
		jsonrpc: "2.0",
		id: 2,
		method: "textDocument/diagnostic",
		params: { textDocument: { uri } },
	};
	const result = await server.request(request);
	assert.deepEqual(result.items[0].range, {
		start: { line: 1, character: 7 },
		end: { line: 1, character: 10 },
	});
	compilerError = {
		message: "Bad type",
		pos: { start: 900, end: 999, file_path: "lib.alt" },
	};
	const imported = await server.request(request);
	assert.match(imported.items[0].message, /lib.alt: Bad type/);
	assert.equal(imported.items[0].range.start.line, 0);
});

test("chained receivers preserve element types for completion and signature help", () => {
	const source =
		"main { let matrix: list(list(int)); matrix.at(0).push(1); matrix.at(0). }";
	const ws = workspace(source);
	assert.equal(
		ws
			.completion(uri, position(source, "matrix.at(0). }", 13))
			.find((i) => i.label === "push").detail,
		"push(value: int) -> void",
	);
	assert.match(
		ws.hover(uri, position(source, "push", 2)).contents.value,
		/push\(value: int\)/,
	);
	const help = ws.signatureHelp(uri, position(source, "push(", 5));
	assert.match(help.signatures[0].label, /push\(value: int\)/);
});

test("remote package paths match the package manager and respect privacy", () => {
	const source = "import { github.com/team/math/algebra as m } main { m. }";
	const ws = workspace(source, {
		"deps/github_com_team_math/algebra.alt":
			"fn sum(a: int) -> int { return a; } @private fn hidden() -> int { return 0; }",
	});
	assert.deepEqual(labels(ws.completion(uri, position(source, "m. }", 2))), [
		"sum",
	]);
});

test("dependency changes refresh inferred types without changing the active buffer", () => {
	const source =
		"import { math as m } main { let values = m.values(); values. }";
	const ws = workspace(source, {
		"math.alt": "fn values() -> list(int) { return [1]; }",
	});
	assert.ok(
		labels(ws.completion(uri, position(source, "values. }", 7))).includes(
			"push",
		),
	);
	ws.set(pathToUri("math.alt"), "fn values() -> int { return 1; }");
	assert.deepEqual(ws.completion(uri, position(source, "values. }", 7)), []);
});

test("browser client synchronizes only changed buffers and rejects requests on disposal", async () => {
	const { LanguageClient } = await import("../app/lsp/client.ts");
	const originalWorker = globalThis.Worker;
	const messages = [];
	let paused = false;
	class TestWorker {
		constructor() {
			this.server = new LanguageServer(async () => {});
		}
		postMessage(message) {
			messages.push(message);
			if (!("id" in message)) {
				this.server.notify(message);
				return;
			}
			if (!paused)
				queueMicrotask(async () =>
					this.onmessage({
						data: {
							id: message.id,
							result: await this.server.request(message),
						},
					}),
				);
		}
		terminate() {
			this.terminated = true;
		}
	}
	globalThis.Worker = TestWorker;
	try {
		const client = new LanguageClient();
		const source = "main { let item = 1; item }";
		const params = {
			textDocument: { uri },
			position: position(source, "item }", 3),
		};
		const files = { "main.alt": source, "other.alt": "main {}" };
		await client.request("textDocument/completion", params, files);
		await client.request("textDocument/hover", params, files);
		assert.equal(
			messages.filter((m) => m.method === "textDocument/didOpen").length,
			2,
		);
		assert.equal(
			messages.filter((m) => m.method === "textDocument/didChange").length,
			0,
		);
		await client.request("textDocument/completion", params, {
			"main.alt": `${source}\n`,
		});
		assert.equal(
			messages.filter((m) => m.method === "textDocument/didChange").length,
			1,
		);
		assert.equal(
			messages.filter((m) => m.method === "textDocument/didClose").length,
			1,
		);
		paused = true;
		const pending = client.request("textDocument/hover", params, files);
		await Promise.resolve();
		client.dispose();
		await assert.rejects(pending, /closed/);
	} finally {
		globalThis.Worker = originalWorker;
	}
});
