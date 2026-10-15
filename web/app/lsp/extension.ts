import { snippet, type CompletionContext } from "@codemirror/autocomplete";
import { linter, lintGutter } from "@codemirror/lint";
import {
	EditorSelection,
	StateEffect,
	StateField,
	type Extension,
} from "@codemirror/state";
import {
	EditorView,
	ViewPlugin,
	hoverTooltip,
	keymap,
	showPanel,
	showTooltip,
	type Panel,
	type Tooltip,
	type ViewUpdate,
} from "@codemirror/view";
import { LanguageClient } from "./client.ts";
import { offsetAt, pathToUri, positionAt, uriToPath } from "./protocol.ts";
import type {
	Location,
	RequestParams,
	RequestResults,
	TextEdit,
} from "./protocol.ts";

export interface LanguageOptions {
	filePath: string;
	getWorkspace?: () => Record<string, string>;
	onNavigate?: (
		path: string,
		position: { line: number; character: number },
	) => void;
	onWorkspaceChange?: (files: Record<string, string>) => void;
}
const kinds: Record<number, string> = {
	2: "method",
	3: "function",
	6: "variable",
	7: "class",
	9: "namespace",
	14: "keyword",
	15: "keyword",
	21: "constant",
	25: "type",
};
const signatureEffect = StateEffect.define<Tooltip | null>();
const createSignatureField = () =>
	StateField.define<Tooltip | null>({
		create: () => null,
		update(value, tr) {
			if (tr.docChanged || tr.selection) value = null;
			for (const effect of tr.effects)
				if (effect.is(signatureEffect)) value = effect.value;
			return value;
		},
		provide: (field) => showTooltip.from(field),
	});
const panelEffect = StateEffect.define<((view: EditorView) => Panel) | null>();
const createPanelField = () =>
	StateField.define<((view: EditorView) => Panel) | null>({
		create: () => null,
		update(value, tr) {
			if (tr.docChanged) value = null;
			for (const effect of tr.effects)
				if (effect.is(panelEffect)) value = effect.value;
			return value;
		},
		provide: (field) => showPanel.from(field),
	});
function textDom(text: string, className = "cm-lsp-info"): HTMLElement {
	const dom = document.createElement("div");
	dom.className = className;
	dom.textContent = text;
	return dom;
}
export function applyTextEdits(text: string, edits: TextEdit[]): string {
	for (const edit of [...edits].sort(
		(a, b) => offsetAt(text, b.range.start) - offsetAt(text, a.range.start),
	)) {
		text =
			text.slice(0, offsetAt(text, edit.range.start)) +
			edit.newText +
			text.slice(offsetAt(text, edit.range.end));
	}
	return text;
}

export function languageExtensions(options: LanguageOptions): {
	extension: Extension;
	completion: (
		context: CompletionContext,
	) => Promise<import("@codemirror/autocomplete").CompletionResult | null>;
} {
	const signatureField = createSignatureField();
	const panelField = createPanelField();
	let client: LanguageClient | undefined;
	let disposed = false;
	const uri = pathToUri(options.filePath);
	const files = (text: string) => ({
		...options.getWorkspace?.(),
		[options.filePath]: text,
	});
	const request = <K extends keyof RequestParams>(
		method: K,
		params: RequestParams[K],
		text: string,
	): Promise<RequestResults[K]> => {
		if (disposed)
			return Promise.reject(new Error("Editor language session closed"));
		client ??= new LanguageClient();
		return client.request(method, params, files(text));
	};
	const at = (text: string, offset: number) => ({
		textDocument: { uri },
		position: positionAt(text, offset),
	});
	const stillCurrent = (view: EditorView, text: string) =>
		!disposed && view.state.doc.toString() === text;
	const navigate = (view: EditorView, location: Location) => {
		if (location.uri === uri) {
			const anchor = offsetAt(view.state.doc.toString(), location.range.start);
			view.dispatch({
				selection: EditorSelection.cursor(anchor),
				effects: EditorView.scrollIntoView(anchor, { y: "center" }),
			});
			view.focus();
		} else options.onNavigate?.(uriToPath(location.uri), location.range.start);
	};
	const definition = (
		view: EditorView,
		offset = view.state.selection.main.head,
	) => {
		const text = view.state.doc.toString();
		void request("textDocument/definition", at(text, offset), text)
			.then((location) => {
				if (location && stillCurrent(view, text)) navigate(view, location);
			})
			.catch(console.warn);
		return true;
	};
	const closePanel = (view: EditorView) => {
		view.dispatch({ effects: panelEffect.of(null) });
		view.focus();
	};
	const references = (view: EditorView) => {
		const text = view.state.doc.toString();
		void request(
			"textDocument/references",
			{
				...at(text, view.state.selection.main.head),
				context: { includeDeclaration: true },
			},
			text,
		)
			.then((locations) => {
				if (!stillCurrent(view, text)) return;
				view.dispatch({
					effects: panelEffect.of(() => {
						const dom = textDom(
							`${locations.length} reference(s)`,
							"cm-lsp-panel",
						);
						for (const location of locations) {
							const button = document.createElement("button");
							button.textContent = `${uriToPath(location.uri)}:${location.range.start.line + 1}:${location.range.start.character + 1}`;
							button.onclick = () => navigate(view, location);
							dom.append(button);
						}
						const close = document.createElement("button");
						close.textContent = "Close";
						close.onclick = () => closePanel(view);
						dom.append(close);
						return { dom, top: false };
					}),
				});
			})
			.catch(console.warn);
		return true;
	};
	const rename = (view: EditorView) => {
		if (view.state.readOnly) return false;
		const text = view.state.doc.toString();
		const params = at(text, view.state.selection.main.head);
		void request("textDocument/prepareRename", params, text)
			.then((prepared) => {
				if (!prepared || !stillCurrent(view, text)) return;
				view.dispatch({
					effects: panelEffect.of(() => {
						const dom = document.createElement("form");
						dom.className = "cm-lsp-panel";
						const label = document.createElement("label");
						label.textContent = "Rename symbol: ";
						const input = document.createElement("input");
						input.value = prepared.placeholder;
						input.required = true;
						input.pattern = "[a-zA-Z][a-zA-Z0-9_]*";
						label.append(input);
						dom.append(label);
						const submit = document.createElement("button");
						submit.type = "submit";
						submit.textContent = "Rename";
						dom.append(submit);
						const cancel = document.createElement("button");
						cancel.type = "button";
						cancel.textContent = "Cancel";
						cancel.onclick = () => closePanel(view);
						dom.append(cancel);
						const status = textDom("");
						status.setAttribute("role", "status");
						dom.append(status);
						dom.onkeydown = (event) => {
							if (event.key === "Escape") {
								event.preventDefault();
								closePanel(view);
							}
						};
						dom.onsubmit = (event) => {
							event.preventDefault();
							const snapshot = files(text);
							submit.disabled = true;
							void request(
								"textDocument/rename",
								{ ...params, newName: input.value },
								text,
							)
								.then((edit) => {
									if (!stillCurrent(view, text) || view.state.readOnly) return;
									if (!edit) {
										status.textContent =
											"Cannot rename: invalid name, name conflict, or read-only reference.";
										return;
									}
									const current = files(text);
									const updated: Record<string, string> = {};
									for (const [target, edits] of Object.entries(edit.changes)) {
										const path = uriToPath(target);
										if (
											snapshot[path] === undefined ||
											snapshot[path] !== current[path]
										) {
											status.textContent = "A file changed. Retry the rename.";
											return;
										}
										if (target !== uri && !options.onWorkspaceChange) {
											status.textContent =
												"Cross-file rename is unavailable in this editor.";
											return;
										}
										updated[path] = applyTextEdits(snapshot[path], edits);
									}
									options.onWorkspaceChange?.(updated);
									if (updated[options.filePath] !== undefined)
										view.dispatch({
											changes: {
												from: 0,
												to: view.state.doc.length,
												insert: updated[options.filePath],
											},
											userEvent: "input.rename",
										});
									closePanel(view);
								})
								.catch((error) => {
									status.textContent = String(error);
								})
								.finally(() => {
									submit.disabled = false;
								});
						};
						return {
							dom,
							top: true,
							mount: () => {
								input.focus();
								input.select();
							},
						};
					}),
				});
			})
			.catch(console.warn);
		return true;
	};

	const completion = async (context: CompletionContext) => {
		if (context.state.readOnly) return null;
		const word = context.matchBefore(/\w*/);
		const preceding = context.state.sliceDoc(
			Math.max(0, context.pos - 1),
			context.pos,
		);
		if (!context.explicit && !word?.text && preceding !== ".") return null;
		const text = context.state.doc.toString();
		try {
			const result = await request(
				"textDocument/completion",
				at(text, context.pos),
				text,
			);
			if (context.aborted || disposed) return null;
			return {
				from: word?.from ?? context.pos,
				options: result.items.map((item) => ({
					label: item.label,
					type: kinds[item.kind] ?? "variable",
					detail: item.detail,
					info: item.documentation,
					apply:
						item.insertTextFormat === 2
							? snippet(
									(item.insertText ?? item.label).replace(
										/\$(\d+)/g,
										(_match, index) => `\${${index === "0" ? "99" : index}:}`,
									),
								)
							: item.insertText,
				})),
			};
		} catch (error) {
			if (!disposed) console.warn("Althread completion unavailable", error);
			return null;
		}
	};
	const lifecycle = ViewPlugin.fromClass(
		class {
			private timer: ReturnType<typeof setTimeout> | undefined;
			constructor(private view: EditorView) {}
			update(update: ViewUpdate) {
				if (!update.docChanged && !update.selectionSet && !update.focusChanged)
					return;
				clearTimeout(this.timer);
				if (!update.view.hasFocus || update.view.state.readOnly) return;
				this.timer = setTimeout(() => this.signature(), 150);
			}
			private signature() {
				const view = this.view;
				const text = view.state.doc.toString(),
					offset = view.state.selection.main.head;
				void request("textDocument/signatureHelp", at(text, offset), text)
					.then((help) => {
						if (
							!stillCurrent(view, text) ||
							view.state.selection.main.head !== offset ||
							!view.hasFocus
						)
							return;
						const signature = help?.signatures[help.activeSignature];
						view.dispatch({
							effects: signatureEffect.of(
								signature
									? {
											pos: offset,
											above: true,
											create: () => {
												const dom = textDom(signature.label);
												const parameter =
													signature.parameters[help!.activeParameter];
												if (parameter)
													dom.append(
														textDom(
															`Parameter ${help!.activeParameter + 1}: ${parameter.label}`,
														),
													);
												return { dom };
											},
										}
									: null,
							),
						});
					})
					.catch((error) => {
						if (!disposed) console.warn(error);
					});
			}
			destroy() {
				disposed = true;
				clearTimeout(this.timer);
				client?.dispose();
			}
		},
	);
	return {
		completion,
		extension: [
			lifecycle,
			signatureField,
			panelField,
			lintGutter(),
			linter(
				async (view) => {
					if (view.state.readOnly) return [];
					const text = view.state.doc.toString();
					try {
						const result = await request(
							"textDocument/diagnostic",
							{ textDocument: { uri } },
							text,
						);
						if (!stillCurrent(view, text)) return [];
						return result.items.map((d) => ({
							from: offsetAt(text, d.range.start),
							to: offsetAt(text, d.range.end),
							severity: "error" as const,
							message: d.message,
							source: d.source,
						}));
					} catch (error) {
						if (!disposed)
							console.warn("Althread diagnostics unavailable", error);
						return [];
					}
				},
				{ delay: 450 },
			),
			hoverTooltip(async (view, offset) => {
				const text = view.state.doc.toString();
				try {
					const result = await request(
						"textDocument/hover",
						at(text, offset),
						text,
					);
					if (!result || !stillCurrent(view, text)) return null;
					return {
						pos: offsetAt(text, result.range.start),
						end: offsetAt(text, result.range.end),
						above: true,
						create: () => ({ dom: textDom(result.contents.value) }),
					};
				} catch {
					return null;
				}
			}),
			keymap.of([
				{ key: "F12", run: definition },
				{ key: "Shift-F12", run: references },
				{ key: "F2", run: rename },
				{
					key: "Escape",
					run: (view) => {
						if (!view.state.field(panelField)) return false;
						closePanel(view);
						return true;
					},
				},
			]),
			EditorView.domEventHandlers({
				mousedown(event, view) {
					if (!(event.metaKey || event.ctrlKey) || event.button !== 0)
						return false;
					const offset = view.posAtCoords({
						x: event.clientX,
						y: event.clientY,
					});
					if (offset === null) return false;
					event.preventDefault();
					return definition(view, offset);
				},
			}),
			EditorView.baseTheme({
				".cm-lsp-info": {
					padding: "6px 10px",
					fontFamily: "monospace",
					whiteSpace: "pre-wrap",
					maxWidth: "600px",
				},
				".cm-lsp-panel": {
					padding: "8px",
					display: "flex",
					gap: "8px",
					alignItems: "center",
					flexWrap: "wrap",
					maxHeight: "180px",
					overflow: "auto",
				},
				".cm-lsp-panel button, .cm-lsp-panel input": {
					font: "inherit",
					color: "inherit",
					background: "transparent",
					border: "1px solid #888",
					borderRadius: "3px",
					padding: "3px 6px",
				},
				".cm-lsp-panel button": { cursor: "pointer" },
			}),
		],
	};
}
