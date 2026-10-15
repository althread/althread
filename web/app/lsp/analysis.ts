import { offsetAt, rangeAt, uriToPath, pathToUri } from "./protocol.ts";
import type {
	CompletionItem,
	Hover,
	Location,
	Position,
	SignatureHelp,
} from "./protocol.ts";

interface Token {
	value: string;
	from: number;
	to: number;
	kind: "word" | "string" | "comment" | "punct" | "number";
}
interface Scope {
	from: number;
	to: number;
	parent: Scope | null;
}
interface Symbol {
	name: string;
	token: Token;
	uri: string;
	scope: Scope;
	kind: number;
	type: string;
	detail: string;
	parameters?: string[];
	private?: boolean;
	available: number;
	module?: string;
}
interface Import {
	name: string;
	path: string;
	token: Token;
}
interface Document {
	uri: string;
	text: string;
	tokens: Token[];
	scopes: Scope[];
	symbols: Symbol[];
	imports: Import[];
}
const identifier = /^[a-zA-Z][\w]*$/;
export const keywords =
	"main shared program fn let const if else while for in loop break continue return run atomic await first seq send receive channel always never check eventually until exists import as private label reaches true false null".split(
		" ",
	);
const types = [
	"int",
	"float",
	"bool",
	"string",
	"void",
	"list",
	"tuple",
	"proc",
];
const builtins: CompletionItem[] = [
	{
		label: "print",
		kind: 3,
		detail: "print(values...) -> void",
		documentation: "Print values to the execution output.",
	},
	{
		label: "assert",
		kind: 3,
		detail: "assert(condition: bool, message: string) -> void",
		documentation: "Stop execution if the condition is false.",
	},
];
const snippets: CompletionItem[] = [
	{
		label: "main",
		kind: 15,
		detail: "Entry point",
		insertText: "main {\n\t$0\n}",
		insertTextFormat: 2,
	},
	{
		label: "program",
		kind: 15,
		detail: "Concurrent program",
		insertText: "program ${1:name}($2) {\n\t$0\n}",
		insertTextFormat: 2,
	},
	{
		label: "fn",
		kind: 15,
		detail: "Function",
		insertText: "fn ${1:name}($2) -> ${3:int} {\n\t$0\n}",
		insertTextFormat: 2,
	},
	{
		label: "for",
		kind: 15,
		detail: "Loop",
		insertText: "for ${1:i} in ${2:0}..${3:10} {\n\t$0\n}",
		insertTextFormat: 2,
	},
	{
		label: "if",
		kind: 15,
		detail: "Conditional",
		insertText: "if ${1:condition} {\n\t$0\n}",
		insertTextFormat: 2,
	},
	{
		label: "await",
		kind: 15,
		detail: "Wait for a condition",
		insertText: "await ${1:condition};$0",
		insertTextFormat: 2,
	},
];

// A recovery-oriented token index: incomplete statements must not disable language
// features. Compiler diagnostics remain authoritative for syntax and type checking.
function tokenize(text: string): Token[] {
	const result: Token[] = [];
	const pattern =
		/\/\/[^\n]*|\/\*[\s\S]*?(?:\*\/|$)|"(?:\\[\s\S]|[^"\\])*(?:"|$)|[a-zA-Z_][\w]*|(?:0[xX][\da-fA-F]+|0[bB][01]+|\d+(?:\.\d+)?|\d+)|\.\.|->|=>|==|!=|<=|>=|&&|\|\||[^\s]/g;
	for (const match of text.matchAll(pattern)) {
		const value = match[0];
		result.push({
			value,
			from: match.index,
			to: match.index + value.length,
			kind:
				value.startsWith("//") || value.startsWith("/*")
					? "comment"
					: value.startsWith('"')
						? "string"
						: /^\d/.test(value)
							? "number"
							: identifier.test(value)
								? "word"
								: "punct",
		});
	}
	return result;
}
const joined = (tokens: Token[]): string => tokens.map((t) => t.value).join("");
function split(tokens: Token[], delimiter = ","): Token[][] {
	const parts: Token[][] = [[]];
	let depth = 0;
	for (const token of tokens) {
		if (token.value === delimiter && depth === 0) parts.push([]);
		else {
			parts[parts.length - 1].push(token);
			if (["(", "["].includes(token.value)) depth++;
			if ([")", "]"].includes(token.value)) depth--;
		}
	}
	return parts;
}
function matching(tokens: Token[], start: number): number {
	const open = tokens[start]?.value;
	const close = open === "(" ? ")" : open === "[" ? "]" : "}";
	let depth = 0;
	for (let i = start; i < tokens.length; i++) {
		if (tokens[i].value === open) depth++;
		if (tokens[i].value === close && --depth === 0) return i;
	}
	return tokens.length;
}
const innerType = (type: string): string =>
	type.slice(type.indexOf("(") + 1, -1);
const typeParts = (type: string): string[] =>
	split(tokenize(innerType(type))).map(joined);
const isWithin = (scope: Scope, offset: number): boolean =>
	scope.from <= offset && offset <= scope.to;
const isAncestor = (ancestor: Scope, scope: Scope): boolean =>
	ancestor === scope || (!!scope.parent && isAncestor(ancestor, scope.parent));
const normalizePath = (path: string): string => {
	const parts: string[] = [];
	for (const part of path.split("/")) {
		if (part === "..") parts.pop();
		else if (part && part !== ".") parts.push(part);
	}
	return parts.join("/");
};

function expressionBefore(tokens: Token[], end: number): Token[] {
	let start = end - 1,
		depth = 0;
	for (; start >= 0; start--) {
		const token = tokens[start];
		if ([")", "]"].includes(token.value)) depth++;
		else if (["(", "["].includes(token.value)) {
			if (depth === 0) break;
			depth--;
		} else if (
			depth === 0 &&
			([";", "{", "}", "=", ",", "+", "-", "*", "/"].includes(token.value) ||
				keywords.includes(token.value))
		)
			break;
	}
	return tokens.slice(start + 1, end);
}

export class LanguageWorkspace {
	private documents = new Map<string, Document>();
	private dirty = false;

	set(uri: string, text: string): void {
		if (this.documents.get(uri)?.text === text) return;
		this.dirty = true;
		const allTokens = tokenize(text);
		const tokens = allTokens.filter((t) => t.kind !== "comment");
		const root: Scope = { from: 0, to: text.length, parent: null };
		const scopes = [root];
		let current = root;
		const scopeAtToken = new Map<number, Scope>();
		for (const token of tokens) {
			scopeAtToken.set(token.from, current);
			if (token.value === "{") {
				current = { from: token.to, to: text.length, parent: current };
				scopes.push(current);
			} else if (token.value === "}" && current.parent) {
				current.to = token.from;
				current = current.parent;
			}
		}
		const doc: Document = {
			uri,
			text,
			tokens: allTokens,
			scopes,
			symbols: [],
			imports: [],
		};
		this.documents.set(uri, doc);
		const add = (
			token: Token,
			scope: Scope,
			kind: number,
			type: string,
			detail: string,
			available = token.to,
		): Symbol => {
			const symbol = {
				name: token.value,
				token,
				uri,
				scope,
				kind,
				type,
				detail,
				available,
			};
			doc.symbols.push(symbol);
			return symbol;
		};
		// Function headers and parameters are indexed before variable initializers.
		for (let i = 0; i < tokens.length; i++) {
			const token = tokens[i];
			if (token.value === "import") {
				const end = matching(tokens, i + 1);
				let j = i + 2;
				while (j < end) {
					if (tokens[j]?.kind !== "word") {
						j++;
						continue;
					}
					const first = tokens[j];
					let path = tokens[j++].value;
					// Paths may contain dots and hyphens (remote dependencies).
					while (j < end && ["/", ".", "-"].includes(tokens[j].value)) {
						path += tokens[j++].value;
						if (tokens[j]) path += tokens[j++].value;
					}
					let name = path.split("/").pop()!;
					if (tokens[j]?.value === "as" && tokens[j + 1]) {
						name = tokens[j + 1].value;
						j += 2;
					}
					doc.imports.push({ name, path, token: first });
				}
			}
			if (
				!["fn", "program"].includes(token.value) ||
				tokens[i + 1]?.kind !== "word" ||
				tokens[i + 2]?.value !== "("
			)
				continue;
			const close = matching(tokens, i + 2);
			const parameters = split(tokens.slice(i + 3, close)).filter(
				(p) => p.length,
			);
			let body = close + 1;
			while (
				body < tokens.length &&
				!["{", ";", "fn", "program", "main"].includes(tokens[body].value)
			)
				body++;
			const type =
				token.value === "program"
					? `proc(${tokens[i + 1].value})`
					: tokens[close + 1]?.value === "->"
						? joined(tokens.slice(close + 2, body)) || "void"
						: "void";
			const labels = parameters.map(
				(p) => `${p[0].value}: ${joined(p.slice(2))}`,
			);
			const symbol = add(
				tokens[i + 1],
				root,
				token.value === "fn" ? 3 : 7,
				type,
				`${token.value} ${tokens[i + 1].value}(${labels.join(", ")})${token.value === "fn" ? ` -> ${type}` : ""}`,
				0,
			);
			symbol.parameters = labels;
			symbol.private =
				tokens[i - 1]?.value === "private" && tokens[i - 2]?.value === "@";
			const bodyScope = scopes.find((s) => s.from === tokens[body]?.to);
			if (bodyScope)
				for (const parameter of parameters) {
					if (parameter[0]?.kind === "word")
						add(
							parameter[0],
							bodyScope,
							6,
							joined(parameter.slice(2)),
							labels[parameters.indexOf(parameter)],
							bodyScope.from,
						);
				}
		}
		for (let i = 0; i < tokens.length; i++) {
			const token = tokens[i];
			const scope = scopeAtToken.get(token.from) ?? root;
			if (["let", "const"].includes(token.value)) {
				let end = i + 1;
				let depth = 0;
				for (; end < tokens.length; end++) {
					const value = tokens[end].value;
					if (depth === 0 && [";", "}", "{", "let", "const"].includes(value))
						break;
					if (["(", "["].includes(value)) depth++;
					if ([")", "]"].includes(value)) depth--;
				}
				const declaration = tokens.slice(i + 1, end);
				let patternEnd =
					declaration[0]?.value === "(" ? matching(declaration, 0) + 1 : 1;
				patternEnd = Math.min(patternEnd, declaration.length);
				const equal = declaration.findIndex((t) => t.value === "=");
				const annotation =
					declaration[patternEnd]?.value === ":"
						? joined(
								declaration.slice(
									patternEnd + 1,
									equal < 0 ? undefined : equal,
								),
							)
						: "";
				const type =
					annotation ||
					this.infer(
						doc,
						declaration.slice(equal < 0 ? declaration.length : equal + 1),
						token.from,
					);
				// The shared block's bindings belong to the document scope.
				const shared = tokens.findIndex((t) => t.to === scope.from);
				const bindingScope =
					tokens[shared - 1]?.value === "shared" ? root : scope;
				const bind = (pattern: Token[], bindingType: string) => {
					if (pattern[0]?.value === "(") {
						const parts = bindingType.startsWith("tuple(")
							? typeParts(bindingType)
							: [];
						split(pattern.slice(1, -1)).forEach((p, n) =>
							bind(p, parts[n] || "unknown"),
						);
					} else if (pattern[0]?.kind === "word" && pattern[0].value !== "_") {
						add(
							pattern[0],
							bindingScope,
							token.value === "const" ? 21 : 6,
							bindingType,
							`${token.value} ${pattern[0].value}: ${bindingType}`,
							bindingScope === root ? 0 : (tokens[end]?.from ?? text.length),
						);
					}
				};
				bind(declaration.slice(0, patternEnd), type);
			}
			if (
				token.value === "for" &&
				tokens[i + 1]?.kind === "word" &&
				tokens[i + 2]?.value === "in"
			) {
				let body = i + 3;
				while (body < tokens.length && !["{", ";"].includes(tokens[body].value))
					body++;
				const bodyScope = scopes.find((s) => s.from === tokens[body]?.to);
				const iterable = this.infer(doc, tokens.slice(i + 3, body), token.from);
				if (bodyScope) {
					const type = iterable.startsWith("list(")
						? innerType(iterable)
						: "unknown";
					add(
						tokens[i + 1],
						bodyScope,
						6,
						type,
						`${tokens[i + 1].value}: ${type}`,
						bodyScope.from,
					);
				}
			}
			if (token.value === "receive") {
				let open = i + 1;
				while (
					open < tokens.length &&
					!["(", ";", "}"].includes(tokens[open].value)
				)
					open++;
				if (tokens[open]?.value !== "(") continue;
				const close = matching(tokens, open);
				const bodyScope =
					tokens[close + 1]?.value === "=>" && tokens[close + 2]?.value === "{"
						? scopes.find((s) => s.from === tokens[close + 2].to)
						: scope;
				for (const p of split(tokens.slice(open + 1, close)))
					if (
						p.length === 1 &&
						p[0].kind === "word" &&
						!keywords.includes(p[0].value) &&
						bodyScope
					)
						add(
							p[0],
							bodyScope,
							6,
							"unknown",
							`${p[0].value}: received value`,
							tokens[close]?.to ?? text.length,
						);
			}
		}
	}
	delete(uri: string): void {
		this.documents.delete(uri);
		this.dirty = true;
	}
	private refresh(): void {
		if (!this.dirty) return;
		// Recompute inferred bindings against the latest dependency headers too.
		for (const [uri, doc] of [...this.documents]) {
			this.documents.delete(uri);
			this.set(uri, doc.text);
		}
		this.dirty = false;
	}
	getText(uri: string): string {
		return this.documents.get(uri)?.text ?? "";
	}
	files(): Record<string, string> {
		return Object.fromEntries(
			[...this.documents].map(([uri, doc]) => [uriToPath(uri), doc.text]),
		);
	}

	private scope(doc: Document, offset: number): Scope {
		return doc.scopes
			.filter((s) => isWithin(s, offset))
			.sort((a, b) => b.from - a.from)[0];
	}
	private visible(doc: Document, offset: number): Symbol[] {
		const scope = this.scope(doc, offset);
		const symbols = doc.symbols.filter(
			(s) => s.available <= offset && isAncestor(s.scope, scope),
		);
		// The nearest binding wins when nested scopes shadow a name.
		return [
			...new Map(
				symbols
					.sort(
						(a, b) => a.scope.from - b.scope.from || a.available - b.available,
					)
					.map((s) => [s.name, s]),
			).values(),
		];
	}
	private importUri(doc: Document, item: Import): string | null {
		const path = uriToPath(doc.uri);
		const dir = path.includes("/")
			? path.slice(0, path.lastIndexOf("/") + 1)
			: "";
		const candidates = [
			`${dir}${item.path}/mod.alt`,
			`${dir}${item.path}.alt`,
			`deps/${item.path}/mod.alt`,
			`deps/${item.path}.alt`,
		];
		const segments = item.path.split("/");
		if (segments.length >= 3 && segments[0].includes(".")) {
			const packagePath = `deps/${segments
				.slice(0, 3)
				.join("/")
				.replace(/[^a-zA-Z0-9]/g, "_")}`;
			const target =
				segments.length > 3
					? `${packagePath}/${segments.slice(3).join("/")}`
					: packagePath;
			candidates.push(`${target}/mod.alt`, `${target}.alt`);
		}
		return (
			candidates
				.map((p) => pathToUri(normalizePath(p)))
				.find((uri) => this.documents.has(uri)) ?? null
		);
	}
	private exports(
		doc: Document,
		item: Import,
		seen = new Set<string>(),
	): Symbol[] {
		const uri = this.importUri(doc, item);
		if (!uri || seen.has(uri)) return [];
		seen.add(uri);
		const imported = this.documents.get(uri)!;
		const symbols = imported.symbols
			.filter((s) => !s.scope.parent && !s.private && s.name !== "main")
			.map((s) => ({ ...s, name: `${item.name}.${s.name}` }));
		for (const nested of imported.imports)
			symbols.push(
				...this.exports(imported, nested, new Set(seen)).map((s) => ({
					...s,
					name: `${item.name}.${s.name}`,
				})),
			);
		return symbols;
	}
	private candidates(doc: Document, offset: number): Symbol[] {
		return [
			...this.visible(doc, offset),
			...doc.imports.flatMap((item) => this.exports(doc, item)),
		];
	}
	private resolve(
		doc: Document,
		name: string,
		offset: number,
	): Symbol | undefined {
		return this.candidates(doc, offset).find((s) => s.name === name);
	}
	private infer(
		doc: Document,
		tokens: Token[],
		offset: number,
		depth = 0,
	): string {
		if (!tokens.length || depth > 12) return "unknown";
		const first = tokens[0].value;
		let nesting = 0;
		const top = tokens.filter((t) => {
			if (["(", "["].includes(t.value)) nesting++;
			if ([")", "]"].includes(t.value)) nesting--;
			return nesting === 0;
		});
		const dot = top.findLast((t) => t.value === ".");
		if (dot) {
			const index = tokens.indexOf(dot);
			const member = tokens[index + 1];
			const open = tokens[index + 2]?.value === "(" ? index + 2 : -1;
			if (
				member &&
				(index + 2 === tokens.length ||
					(open >= 0 && matching(tokens, open) === tokens.length - 1))
			) {
				const qualified = this.resolve(
					doc,
					joined(tokens.slice(0, open < 0 ? undefined : open)),
					offset,
				);
				if (qualified) return qualified.type;
				const receiver = this.infer(
					doc,
					tokens.slice(0, index),
					offset,
					depth + 1,
				);
				const method = this.methods(receiver).find(
					(m) => m.label === member.value,
				);
				if (method) return method.detail.split(" -> ")[1] || "unknown";
			}
		}
		if (top.some((t) => t.value === "..")) return "list(int)";
		if (tokens[0].kind === "string") return "string";
		if (
			["true", "false", "!"].includes(first) ||
			top.some((t) =>
				["==", "!=", "<", ">", "<=", ">=", "&&", "||"].includes(t.value),
			)
		)
			return "bool";
		if (first === "[")
			return `list(${this.infer(doc, split(tokens.slice(1, tokens.at(-1)?.value === "]" ? -1 : undefined))[0], offset, depth + 1)})`;
		if (first === "(") {
			const parts = split(tokens.slice(1, -1));
			return parts.length > 1
				? `tuple(${parts.map((p) => this.infer(doc, p, offset, depth + 1)).join(",")})`
				: this.infer(doc, parts[0], offset, depth + 1);
		}
		if (tokens[0].kind === "number" || ["-", "+"].includes(first))
			return tokens.some((t) => t.kind === "number" && t.value.includes("."))
				? "float"
				: "int";
		let end = first === "run" ? 1 : 0;
		const start = end;
		while (
			end < tokens.length &&
			(tokens[end].kind === "word" ||
				tokens[end].value === "." ||
				tokens[end].kind === "number")
		)
			end++;
		const name = joined(tokens.slice(start, end));
		if (first === "run") return `proc(${name})`;
		const symbol = this.resolve(doc, name, offset);
		if (symbol) return symbol.type;
		return "unknown";
	}
	private methods(type: string): CompletionItem[] {
		const method = (
			label: string,
			params: string[],
			ret: string,
		): CompletionItem => ({
			label,
			kind: 2,
			detail: `${label}(${params.join(", ")}) -> ${ret}`,
		});
		if (type.startsWith("list(")) {
			const element = innerType(type);
			return [
				method("len", [], "int"),
				method("push", [`value: ${element}`], "void"),
				method("remove", ["index: int"], element),
				method("set", ["index: int", `value: ${element}`], "void"),
				method("at", ["index: int"], element),
			];
		}
		if (type.startsWith("tuple("))
			return [
				method("len", [], "int"),
				...typeParts(type).flatMap((t, i) => [
					method(String(i), [], t),
					method(`set${i}`, [`value: ${t}`], "void"),
				]),
			];
		if (type.startsWith("proc("))
			return [method("reaches", ["label: label"], "bool")];
		return [];
	}
	private ignored(doc: Document, offset: number): boolean {
		return doc.tokens.some(
			(t) =>
				(t.kind === "comment" || t.kind === "string") &&
				t.from < offset &&
				(offset < t.to ||
					(offset === t.to &&
						(t.kind === "comment"
							? !t.value.endsWith("*/")
							: !t.value.endsWith('"')))),
		);
	}
	completion(uri: string, position: Position): CompletionItem[] {
		this.refresh();
		const doc = this.documents.get(uri);
		if (!doc) return [];
		const offset = offsetAt(doc.text, position);
		if (this.ignored(doc, offset)) return [];
		const before = doc.text.slice(0, offset);
		const member = /([a-zA-Z][\w]*(?:\.[\w]+)*)\.\w*$/.exec(before);
		const toItem = (s: Symbol): CompletionItem => ({
			label: s.name,
			kind: s.kind,
			detail: s.detail,
		});
		const beforeTokens = doc.tokens.filter(
			(t) => t.to <= offset && t.kind !== "comment",
		);
		const last = beforeTokens.at(-1);
		const dotIndex =
			last?.value === "."
				? beforeTokens.length - 1
				: beforeTokens.at(-2)?.value === "."
					? beforeTokens.length - 2
					: -1;
		if (dotIndex >= 0) {
			const receiverType = this.infer(
				doc,
				expressionBefore(beforeTokens, dotIndex),
				offset,
			);
			const prefix = member ? `${member[1]}.` : "\0";
			const exported = this.candidates(doc, offset)
				.filter((s) => s.name.startsWith(prefix))
				.map((s) => ({ ...toItem(s), label: s.name.slice(prefix.length) }))
				.filter((s) => !s.label.includes("."));
			for (const item of doc.imports)
				for (const s of this.exports(doc, item)) {
					if (s.name.startsWith(prefix)) {
						const next = s.name.slice(prefix.length).split(".")[0];
						if (!exported.some((e) => e.label === next))
							exported.push({ label: next, kind: 9, detail: "module" });
					}
				}
			const members = [...exported, ...this.methods(receiverType)];
			return /\brun\s+[\w.]*$/.test(before)
				? members.filter((item) => item.kind === 7 || item.kind === 9)
				: members;
		}
		if (/:\s*\w*$/.test(before) || /->\s*\w*$/.test(before))
			return types.map((label) => ({ label, kind: 25, detail: "type" }));
		const candidates = this.candidates(doc, offset).filter(
			(s) => !s.name.includes("."),
		);
		if (/\brun\s+\w*$/.test(before))
			return candidates
				.filter((s) => s.kind === 7)
				.map(toItem)
				.concat(
					doc.imports.map((i) => ({
						label: i.name,
						kind: 9,
						detail: "module",
					})),
				);
		const items = [
			...candidates.map(toItem),
			...doc.imports.map((i) => ({ label: i.name, kind: 9, detail: "module" })),
			...builtins,
			...keywords.map((label) => ({ label, kind: 14, detail: "keyword" })),
			...types.map((label) => ({ label, kind: 25, detail: "type" })),
		];
		for (const item of snippets)
			if (!candidates.some((s) => s.name === item.label)) {
				const index = items.findIndex((i) => i.label === item.label);
				if (index >= 0) items[index] = item;
			}
		return items;
	}
	private word(
		doc: Document,
		offset: number,
	): {
		name: string;
		token: Token;
		symbol?: Symbol;
		receiverType?: string;
	} | null {
		if (this.ignored(doc, offset)) return null;
		const tokens = doc.tokens.filter((t) => t.kind !== "comment");
		const index = tokens.findIndex(
			(t) =>
				t.from <= offset &&
				offset <= t.to &&
				(t.kind === "word" || t.kind === "number"),
		);
		if (index < 0) return null;
		const token = tokens[index];
		let start = index;
		while (start > 1 && tokens[start - 1].value === ".") start -= 2;
		const name = joined(tokens.slice(start, index + 1));
		const declaration = doc.symbols.find((s) => s.token.from === token.from);
		return {
			name,
			token,
			symbol: declaration ?? this.resolve(doc, name, offset),
			receiverType:
				tokens[index - 1]?.value === "."
					? this.infer(doc, expressionBefore(tokens, index - 1), offset)
					: undefined,
		};
	}
	hover(uri: string, position: Position): Hover | null {
		this.refresh();
		const doc = this.documents.get(uri);
		if (!doc) return null;
		const offset = offsetAt(doc.text, position);
		const word = this.word(doc, offset);
		if (!word) return null;
		let value =
			word.symbol?.detail ??
			builtins.find((b) => b.label === word.name)?.detail;
		if (!value && word.receiverType)
			value = this.methods(word.receiverType).find(
				(m) => m.label === word.token.value,
			)?.detail;
		if (!value && word.name.includes(".")) {
			const dot = word.name.lastIndexOf(".");
			const receiver = this.resolve(doc, word.name.slice(0, dot), offset);
			value =
				receiver &&
				this.methods(receiver.type).find(
					(m) => m.label === word.name.slice(dot + 1),
				)?.detail;
		}
		return value
			? {
					contents: { kind: "plaintext", value },
					range: rangeAt(doc.text, word.token.from, word.token.to),
				}
			: null;
	}
	definition(uri: string, position: Position): Location | null {
		this.refresh();
		const doc = this.documents.get(uri);
		if (!doc) return null;
		const word = this.word(doc, offsetAt(doc.text, position));
		const symbol = word?.symbol;
		if (symbol)
			return {
				uri: symbol.uri,
				range: rangeAt(
					this.getText(symbol.uri),
					symbol.token.from,
					symbol.token.to,
				),
			};
		const item = doc.imports.find((i) => i.name === word?.name);
		const target = item && this.importUri(doc, item);
		return target
			? { uri: target, range: rangeAt(this.getText(target), 0, 0) }
			: null;
	}
	references(
		uri: string,
		position: Position,
		includeDeclaration = true,
	): Location[] {
		this.refresh();
		const doc = this.documents.get(uri);
		if (!doc) return [];
		const target = this.word(doc, offsetAt(doc.text, position))?.symbol;
		if (!target) return [];
		const locations: Location[] = [];
		for (const other of this.documents.values())
			for (const token of other.tokens) {
				if (!["word", "number"].includes(token.kind)) continue;
				const symbol = this.word(other, token.from)?.symbol;
				if (
					symbol?.uri === target.uri &&
					symbol.token.from === target.token.from &&
					(includeDeclaration ||
						other.uri !== target.uri ||
						token.from !== target.token.from)
				)
					locations.push({
						uri: other.uri,
						range: rangeAt(other.text, token.from, token.to),
					});
			}
		return locations;
	}
	prepareRename(
		uri: string,
		position: Position,
	): { range: ReturnType<typeof rangeAt>; placeholder: string } | null {
		this.refresh();
		const doc = this.documents.get(uri);
		if (!doc) return null;
		const word = this.word(doc, offsetAt(doc.text, position));
		if (!word?.symbol || uriToPath(word.symbol.uri).startsWith("deps/"))
			return null;
		return {
			range: rangeAt(doc.text, word.token.from, word.token.to),
			placeholder: word.symbol.token.value,
		};
	}
	rename(
		uri: string,
		position: Position,
		newName: string,
	): {
		changes: Record<
			string,
			{ range: ReturnType<typeof rangeAt>; newText: string }[]
		>;
	} | null {
		if (
			!identifier.test(newName) ||
			keywords.includes(newName) ||
			types.includes(newName) ||
			!this.prepareRename(uri, position)
		)
			return null;
		this.refresh();
		const doc = this.documents.get(uri)!;
		const target = this.word(doc, offsetAt(doc.text, position))!.symbol!;
		const references = this.references(uri, position);
		// Reject collisions at the declaration and at every use (including capture
		// by a nested binding), rather than silently changing what a name means.
		for (const location of references) {
			const other = this.documents.get(location.uri)!;
			const offset = offsetAt(other.text, location.range.start);
			const collision = this.visible(other, offset).find(
				(s) =>
					s.name === newName &&
					!(s.uri === target.uri && s.token.from === target.token.from),
			);
			if (collision || uriToPath(location.uri).startsWith("deps/")) return null;
		}
		if (
			doc.symbols.some(
				(s) => s.scope === target.scope && s.name === newName && s !== target,
			)
		)
			return null;
		const changes: Record<
			string,
			{ range: ReturnType<typeof rangeAt>; newText: string }[]
		> = {};
		for (const location of references)
			(changes[location.uri] ??= []).push({
				range: location.range,
				newText: newName,
			});
		return { changes };
	}
	signatureHelp(uri: string, position: Position): SignatureHelp | null {
		this.refresh();
		const doc = this.documents.get(uri);
		if (!doc) return null;
		const offset = offsetAt(doc.text, position);
		if (this.ignored(doc, offset)) return null;
		const tokens = doc.tokens.filter(
			(t) => t.from < offset && t.kind !== "comment",
		);
		let depth = 0,
			activeParameter = 0;
		for (let i = tokens.length - 1; i >= 0; i--) {
			const value = tokens[i].value;
			if ([")", "]"].includes(value)) depth++;
			if (value === "[" && depth > 0) depth--;
			if (value === "," && depth === 0) activeParameter++;
			if (value !== "(") continue;
			if (depth > 0) {
				depth--;
				continue;
			}
			const word = this.word(doc, tokens[i - 1]?.from ?? 0);
			if (!word) return null;
			const hover = this.hover(
				uri,
				rangeAt(doc.text, word.token.from, word.token.to).start,
			);
			const label = hover?.contents.value;
			if (!label || !label.includes("(")) return null;
			const signatureTokens = tokenize(label);
			const open = signatureTokens.findIndex((t) => t.value === "(");
			const params =
				word.symbol?.parameters ??
				split(signatureTokens.slice(open + 1, matching(signatureTokens, open)))
					.map(joined)
					.filter(Boolean);
			return {
				signatures: [{ label, parameters: params.map((label) => ({ label })) }],
				activeSignature: 0,
				activeParameter: Math.min(
					activeParameter,
					Math.max(0, params.length - 1),
				),
			};
		}
		return null;
	}
}
