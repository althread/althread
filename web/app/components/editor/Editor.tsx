import {
	autocompletion,
	closeBrackets,
	closeBracketsKeymap,
	completionKeymap,
} from "@codemirror/autocomplete";
import {
	defaultKeymap,
	history,
	historyKeymap,
	indentWithTab,
	toggleComment,
} from "@codemirror/commands";
import {
	bracketMatching,
	foldGutter,
	foldKeymap,
	indentOnInput,
} from "@codemirror/language";
import { lintGutter, lintKeymap } from "@codemirror/lint";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import {
	Compartment,
	EditorState,
	type Extension,
	StateEffect,
	StateField,
} from "@codemirror/state";
import {
	crosshairCursor,
	Decoration,
	type DecorationSet,
	drawSelection,
	dropCursor,
	EditorView,
	highlightActiveLine,
	highlightActiveLineGutter,
	highlightSpecialChars,
	keymap,
	lineNumbers,
	rectangularSelection,
	WidgetType,
} from "@codemirror/view";
import { createCodeMirror } from "solid-codemirror";
import { createEffect } from "solid-js";
import type { LanguageOptions } from "../../lsp/extension";
import {
	type CodeEditorTheme,
	createCustomSyntaxHighlighting,
} from "./custom-style";
import editor_lang from "./editor-lang";

const fileExtension = (filePath: string) =>
	filePath.split(".").pop()?.toLowerCase() ?? "";

const isAlthreadFile = (filePath: string) => fileExtension(filePath) === "alt";

/**
 * Highlighting for non-Althread files. The language packages are loaded on
 * demand so that they are not part of the main bundle.
 */
const loadOtherLanguage = async (filePath: string): Promise<Extension> => {
	const extension = fileExtension(filePath);
	switch (extension) {
		case "js":
		case "jsx":
		case "ts":
		case "tsx":
			return (await import("@codemirror/lang-javascript")).javascript({
				jsx: extension.endsWith("x"),
				typescript: extension.startsWith("ts"),
			});
		case "css":
			return (await import("@codemirror/lang-css")).css();
		case "html":
		case "htm":
			return (await import("@codemirror/lang-html")).html();
		case "json":
			return (await import("@codemirror/lang-json")).json();
		case "md":
		case "markdown":
			return (await import("@codemirror/lang-markdown")).markdown();
		case "py":
			return (await import("@codemirror/lang-python")).python();
		case "rs":
			return (await import("@codemirror/lang-rust")).rust();
		case "cpp":
		case "cc":
		case "cxx":
		case "c":
		case "h":
		case "hpp":
			return (await import("@codemirror/lang-cpp")).cpp();
		default:
			return []; // plain text
	}
};

// Define effect for highlighting lines
const addHighlightEffect =
	StateEffect.define<{ line: number; label?: string }[]>();
const clearHighlightEffect = StateEffect.define();

class ProcessBadgeWidget extends WidgetType {
	constructor(readonly label: string) {
		super();
	}
	toDOM() {
		const span = document.createElement("span");
		span.className = "cm-process-badge";
		span.textContent = this.label;
		return span;
	}
}

// Define state field for line highlights
const highlightField = StateField.define<DecorationSet>({
	create() {
		return Decoration.none;
	},
	update(highlights, tr) {
		highlights = highlights.map(tr.changes);
		for (const effect of tr.effects) {
			if (effect.is(addHighlightEffect)) {
				const specs = effect.value;
				const decorations: any[] = [];

				// Group labels by line
				const lineLabels = new Map<number, string[]>();
				for (const spec of specs) {
					if (!lineLabels.has(spec.line)) lineLabels.set(spec.line, []);
					if (spec.label) lineLabels.get(spec.line)!.push(spec.label);
				}

				// Sort lines
				const sortedLines = Array.from(lineLabels.keys()).sort((a, b) => a - b);

				for (const lineNum of sortedLines) {
					if (lineNum > 0 && lineNum <= tr.state.doc.lines) {
						const line = tr.state.doc.line(lineNum);

						// Background Highlight
						decorations.push(
							Decoration.line({ class: "cm-exec-line" }).range(line.from),
						);

						// Badge Widget (at the end of the line)
						const labels = lineLabels.get(lineNum)!;
						if (labels.length > 0) {
							decorations.push(
								Decoration.widget({
									widget: new ProcessBadgeWidget(labels.join(", ")),
									side: 1, // After content
								}).range(line.to),
							);
						}
					}
				}
				highlights = Decoration.set(
					decorations.sort((a, b) => a.from - b.from),
				);
			} else if (effect.is(clearHighlightEffect)) {
				highlights = Decoration.none;
			}
		}
		return highlights;
	},
	provide: (f) => EditorView.decorations.from(f),
});

const basicSetup: Extension = (() => [
	lineNumbers(),
	highlightActiveLineGutter(),
	highlightSpecialChars(),
	history(),
	foldGutter(), // This should be available for all languages
	drawSelection(),
	dropCursor(),
	EditorState.allowMultipleSelections.of(true),
	indentOnInput(),
	bracketMatching(),
	closeBrackets(),
	autocompletion(),
	rectangularSelection(),
	crosshairCursor(),
	highlightActiveLine(),
	highlightSelectionMatches(),
	keymap.of([
		...closeBracketsKeymap,
		...defaultKeymap,
		...searchKeymap,
		...historyKeymap,
		...foldKeymap,
		...completionKeymap,
		...lintKeymap, // Keep this for all languages, but only add actual linter for .alt
	]),
])();

const editorPalettes = {
	dark: {
		text: "#c9d1d9",
		background: "#1e1e1e",
		gutterText: "#5a5f69",
		gutterActiveText: "#c9d1d9",
		activeLine: "rgba(255, 255, 255, 0.035)",
		caret: "#f0a24b",
		selection: "rgba(64, 120, 192, 0.45)",
		selectionFocused: "rgba(64, 120, 192, 0.55)",
		selectionMatch: "rgba(255, 217, 102, 0.12)",
		tooltipBg: "#252526",
		tooltipText: "#dddddd",
		tooltipBorder: "#3c3c3c",
		completionSelected: "#04395e",
		completionSelectedText: "#ffffff",
		execLine: "rgba(232, 146, 52, 0.16)",
		execBorder: "#e89234",
		badgeBg: "rgba(232, 146, 52, 0.22)",
		badgeText: "#f5b56a",
		foldMarker: "#6b707a",
	},
	light: {
		text: "#2f3441",
		background: "#fbf6ee",
		gutterText: "#b0a493",
		gutterActiveText: "#4a3d2f",
		activeLine: "rgba(160, 120, 70, 0.07)",
		caret: "#c96f1a",
		selection: "rgba(245, 184, 96, 0.35)",
		selectionFocused: "rgba(245, 184, 96, 0.45)",
		selectionMatch: "rgba(209, 136, 38, 0.15)",
		tooltipBg: "#fffaf2",
		tooltipText: "#2f3441",
		tooltipBorder: "#dccfbf",
		completionSelected: "#f1dfc4",
		completionSelectedText: "#2a1f14",
		execLine: "rgba(214, 125, 30, 0.14)",
		execBorder: "#d67d1e",
		badgeBg: "rgba(201, 111, 26, 0.14)",
		badgeText: "#9a4f0c",
		foldMarker: "#b0a493",
	},
} as const;

const createEditorTheme = (theme: CodeEditorTheme) => {
	const isDark = theme === "dark";
	const c = editorPalettes[theme];

	return EditorView.theme(
		{
			"&": {
				color: c.text,
				backgroundColor: c.background,
				fontSize: "13.5px",
			},
			"&.cm-focused": {
				outline: "none",
			},
			".cm-scroller": {
				fontFamily: "var(--app-font-mono)",
				lineHeight: "1.65",
				fontVariantLigatures: "contextual",
			},
			".cm-content": {
				caretColor: c.caret,
				padding: "10px 0 40px",
			},
			".cm-line": {
				padding: "0 16px 0 6px",
			},
			".cm-cursor, .cm-dropCursor": {
				borderLeft: `2px solid ${c.caret}`,
			},
			"&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground":
				{
					backgroundColor: `${c.selectionFocused} !important`,
				},
			".cm-selectionBackground": {
				backgroundColor: `${c.selection} !important`,
			},
			".cm-content ::selection": {
				backgroundColor: c.selectionFocused,
			},
			".cm-activeLine": {
				backgroundColor: c.activeLine,
			},
			".cm-selectionMatch": {
				backgroundColor: c.selectionMatch,
				borderRadius: "2px",
			},
			".cm-selectionMatch.cm-selectionMatch-main": {
				backgroundColor: "transparent",
			},
			".cm-matchingBracket, &.cm-focused .cm-matchingBracket": {
				backgroundColor: "transparent",
				outline: `1px solid ${isDark ? "rgba(255,255,255,0.25)" : "rgba(90,60,30,0.3)"}`,
				borderRadius: "2px",
			},
			".cm-gutters": {
				backgroundColor: c.background,
				color: c.gutterText,
				border: "none",
				paddingLeft: "8px",
			},
			".cm-lineNumbers .cm-gutterElement": {
				minWidth: "28px",
				padding: "0 4px 0 0",
				textAlign: "right",
				fontVariantNumeric: "tabular-nums",
			},
			".cm-activeLineGutter": {
				backgroundColor: "transparent",
				color: c.gutterActiveText,
			},
			".cm-foldGutter .cm-gutterElement": {
				width: "14px",
				color: c.foldMarker,
				opacity: "0",
				transition: "opacity 0.15s",
				cursor: "pointer",
			},
			".cm-gutters:hover .cm-foldGutter .cm-gutterElement": {
				opacity: "1",
			},
			".cm-lintGutter": {
				width: "12px",
			},
			".cm-tooltip": {
				backgroundColor: c.tooltipBg,
				color: c.tooltipText,
				border: `1px solid ${c.tooltipBorder}`,
				borderRadius: "6px",
				boxShadow: isDark
					? "0 6px 20px rgba(0, 0, 0, 0.45)"
					: "0 6px 20px rgba(121, 89, 46, 0.16)",
				overflow: "hidden",
			},
			".cm-tooltip-autocomplete > ul": {
				fontFamily: "var(--app-font-mono)",
				fontSize: "12.5px",
				padding: "4px",
			},
			".cm-tooltip-autocomplete > ul > li": {
				borderRadius: "4px",
				padding: "2px 8px !important",
			},
			".cm-tooltip-autocomplete ul li[aria-selected]": {
				backgroundColor: c.completionSelected,
				color: c.completionSelectedText,
			},
			".cm-panels": {
				backgroundColor: c.tooltipBg,
				color: c.tooltipText,
			},
			".cm-panels.cm-panels-bottom": {
				borderTop: `1px solid ${c.tooltipBorder}`,
			},
			// Lines executed by the selected step / state.
			".cm-exec-line": {
				backgroundColor: c.execLine,
				boxShadow: `inset 3px 0 0 ${c.execBorder}`,
			},
			".cm-process-badge": {
				backgroundColor: c.badgeBg,
				color: c.badgeText,
				borderRadius: "999px",
				padding: "0 7px",
				fontFamily: "var(--app-font-ui)",
				fontSize: "10.5px",
				fontWeight: "600",
				lineHeight: "16px",
				marginLeft: "12px",
				verticalAlign: "1px",
				display: "inline-block",
			},
		},
		{ dark: isDark },
	);
};

const createEditor = ({
	defaultValue,
	onValueChange,
	filePath = "main.alt",
	theme = "dark",
	getWorkspace,
	onNavigate,
	onWorkspaceChange,
}: {
	defaultValue: string | undefined;
	onValueChange: undefined | ((value: string) => void);
	filePath?: string;
	theme?: CodeEditorTheme;
	getWorkspace?: LanguageOptions["getWorkspace"];
	onNavigate?: LanguageOptions["onNavigate"];
	onWorkspaceChange?: LanguageOptions["onWorkspaceChange"];
}) => {
	let currentContent = defaultValue ?? "";
	let requestedReadOnly = false;
	const editor = createCodeMirror({
		value: defaultValue,
		onValueChange: (value) => {
			currentContent = value;
			onValueChange?.(value);
		},
	});

	// Store current filename for language detection
	let currentFileName = filePath;

	// Create compartments for dynamic extensions
	const languageCompartment = new Compartment();
	const readOnlyCompartment = new Compartment();
	const themeCompartment = new Compartment();
	const syntaxCompartment = new Compartment();

	const languageOptions = (path: string): LanguageOptions => ({
		filePath: path,
		getWorkspace,
		onNavigate,
		onWorkspaceChange,
	});
	const updateLanguage = (newFileName: string) => {
		currentFileName = newFileName;
		const view = editor.editorView();
		if (!view || !languageCompartment.get(view.state)) return;
		if (isAlthreadFile(newFileName)) {
			view.dispatch({
				effects: languageCompartment.reconfigure(
					editor_lang(languageOptions(newFileName)),
				),
			});
			return;
		}
		// Plain text until the language package is loaded.
		view.dispatch({ effects: languageCompartment.reconfigure([]) });
		loadOtherLanguage(newFileName)
			.then((language) => {
				const current = editor.editorView();
				if (current && currentFileName === newFileName) {
					current.dispatch({
						effects: languageCompartment.reconfigure(language),
					});
				}
			})
			.catch((error) =>
				console.warn(`Could not load highlighting for ${newFileName}`, error),
			);
	};

	// Initialize with basic setup and theme
	editor.createExtension(basicSetup);
	editor.createExtension(
		keymap.of([
			indentWithTab,
			{ key: "Mod-/", run: toggleComment },
			{
				key: "Mod-s",
				preventDefault: true,
				run: () => {
					return true;
				},
			},
		]),
	);

	// Add theme and syntax highlighting
	editor.createExtension(themeCompartment.of(createEditorTheme(theme)));
	editor.createExtension(
		syntaxCompartment.of(createCustomSyntaxHighlighting(theme)),
	);

	// Add compartments for dynamic extensions
	editor.createExtension(
		languageCompartment.of(
			isAlthreadFile(filePath) ? editor_lang(languageOptions(filePath)) : [],
		),
	);
	editor.createExtension(lintGutter());
	editor.createExtension(readOnlyCompartment.of([]));
	editor.createExtension(highlightField);

	// Safe wrapper for editor view operations
	const safeEditorView = () => {
		try {
			return editor.editorView && editor.editorView()
				? editor.editorView()
				: null;
		} catch (e) {
			console.warn("Editor view not ready:", e);
			return null;
		}
	};

	// Safe content update function
	const safeUpdateContent = (content: string) => {
		currentContent = content;
		const view = safeEditorView();
		if (view) {
			try {
				const update = view.state.update({
					changes: {
						from: 0,
						to: view.state.doc.length,
						insert: content,
					},
				});
				view.update([update]);
				return true;
			} catch (e) {
				console.warn("Failed to update editor content:", e);
				return false;
			}
		}
		return false;
	};

	// Function to toggle read-only mode
	const setReadOnly = (isReadOnly: boolean) => {
		requestedReadOnly = isReadOnly;
		const view = safeEditorView();
		if (view) {
			try {
				const extension = isReadOnly ? EditorState.readOnly.of(true) : [];
				view.dispatch({
					effects: readOnlyCompartment.reconfigure(extension),
				});
				return true;
			} catch (e) {
				console.warn("Failed to set read-only mode:", e);
				return false;
			}
		}
		return false;
	};

	// A file can be selected before Solid mounts the editor. Apply the latest
	// requested buffer, URI and access mode as soon as its view is available.
	createEffect(() => {
		const view = editor.editorView();
		if (!view) return;
		safeUpdateContent(currentContent);
		updateLanguage(currentFileName);
		setReadOnly(requestedReadOnly);
	});

	// Function to highlight specific lines
	const highlightLines = (
		specs: number[] | { line: number; label?: string }[],
	) => {
		const view = safeEditorView();
		if (view) {
			try {
				const formattedSpecs = specs.map((s) =>
					typeof s === "number" ? { line: s } : s,
				);
				view.dispatch({
					effects: addHighlightEffect.of(formattedSpecs),
				});
				return true;
			} catch (e) {
				console.warn("Failed to highlight lines:", e);
				return false;
			}
		}
		return false;
	};

	// Function to clear line highlights
	const clearHighlights = () => {
		const view = safeEditorView();
		if (view) {
			try {
				view.dispatch({
					effects: clearHighlightEffect.of(null),
				});
				return true;
			} catch (e) {
				console.warn("Failed to clear highlights:", e);
				return false;
			}
		}
		return false;
	};

	const updateTheme = (nextTheme: CodeEditorTheme) => {
		const view = safeEditorView();
		if (view) {
			try {
				view.dispatch({
					effects: [
						themeCompartment.reconfigure(createEditorTheme(nextTheme)),
						syntaxCompartment.reconfigure(
							createCustomSyntaxHighlighting(nextTheme),
						),
					],
				});
				return true;
			} catch (e) {
				console.warn("Failed to update editor theme:", e);
				return false;
			}
		}
		return false;
	};

	// Return editor with updateLanguage method and safe wrappers
	return {
		...editor,
		updateLanguage,
		updateTheme,
		getCurrentFileName: () => currentFileName,
		safeEditorView,
		safeUpdateContent,
		setReadOnly,
		highlightLines,
		clearHighlights,
	};
};

export default createEditor;
