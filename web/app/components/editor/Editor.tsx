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
import { cpp } from "@codemirror/lang-cpp";
import { css } from "@codemirror/lang-css";
import { html } from "@codemirror/lang-html";
// Import additional language supports
import { javascript } from "@codemirror/lang-javascript";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import {
	bracketMatching,
	defaultHighlightStyle,
	foldGutter,
	foldKeymap,
	HighlightStyle,
	indentOnInput,
	syntaxHighlighting,
} from "@codemirror/language";
import {
	type Diagnostic,
	linter,
	lintGutter,
	lintKeymap,
} from "@codemirror/lint";
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
import { Tag, tags as t, tags } from "@lezer/highlight";
import { createCodeMirror } from "solid-codemirror";
import { createSignal, onMount } from "solid-js";
import { customSyntaxHighlighting } from "./custom-style";
import editor_lang from "./editor-lang";

// Language detection based on file extension
const getLanguageExtension = (filePath: string): Extension => {
	const extension = filePath.split(".").pop()?.toLowerCase();

	switch (extension) {
		case "js":
		case "jsx":
		case "ts":
		case "tsx":
			return javascript({ jsx: true, typescript: extension.includes("ts") });
		case "css":
			return css();
		case "html":
		case "htm":
			return html();
		case "json":
			return json();
		case "md":
		case "markdown":
			return markdown();
		case "py":
			return python();
		case "rs":
			return rust();
		case "cpp":
		case "cc":
		case "cxx":
		case "c":
		case "h":
		case "hpp":
			return cpp();
		case "alt":
			return editor_lang(); // Your custom language
		default:
			return javascript(); // Default to JavaScript for unknown files
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
							Decoration.line({
								attributes: {
									style:
										"background-color: rgba(255, 165, 0, 0.2); border-left: 3px solid orange;",
								},
							}).range(line.from),
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

const createEditor = ({
	compile,
	defaultValue,
	onValueChange,
	filePath = "main.alt",
}: {
	compile: (code: string) => Promise<any>;
	defaultValue: string | undefined;
	onValueChange: undefined | ((value: string) => void);
	filePath?: string;
}) => {
	const editor = createCodeMirror({
		value: defaultValue,
		onValueChange: onValueChange,
	});

	// Store current filename for language detection
	let currentFileName = filePath;

	// Create compartments for dynamic extensions
	const languageCompartment = new Compartment();
	const linterCompartment = new Compartment();
	const readOnlyCompartment = new Compartment();

	// Theme definitions with consistent line number width
	const uiTheme = EditorView.theme(
		{
			"&": {
				color: "#abb2bf",
				backgroundColor: "#1e1e1e",
			},
			".cm-content": {
				caretColor: "#528bff",
			},
			".cm-cursor, .cm-dropCursor": {
				borderLeft: "2px solid #528bff",
			},
			// Selection: force visible highlight (some browsers + mix-blend-mode can make it look invisible).
			"&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
				backgroundColor: "rgba(38, 79, 120, 0.70) !important",
			},
			".cm-selectionLayer": {
				mixBlendMode: "normal",
			},
			".cm-selectionLayer .cm-selectionBackground": {
				mixBlendMode: "normal",
			},
			".cm-content ::selection": {
				backgroundColor: "rgba(38, 79, 120, 1.00)",
			},
			".cm-gutters": {
				backgroundColor: "#1e1e1e",
				color: "#5c6370",
				borderRight: "1px solid #333",
				minWidth: "60px", // Ensure minimum width for gutters
			},
			".cm-lineNumbers": {
				minWidth: "40px", // Reserve space for up to 4 digits (9999 lines)
				paddingRight: "8px",
				textAlign: "right",
			},
			".cm-lineNumbers .cm-gutterElement": {
				minWidth: "32px", // Consistent width for line numbers
				textAlign: "right",
			},
			".cm-activeLineGutter": {
				backgroundColor: "#3a3f4b", // More noticeable active line gutter
			},
			".cm-activeLine": {
				// Avoid obscuring text selections on the active line.
				backgroundColor: "rgba(58, 63, 75, 0.35)",
				boxShadow: "inset 0 0 0 1px rgba(255, 255, 255, 0.06)",
			},
			// highlightSelectionMatches(): make "other occurrences" subtle.
			".cm-selectionMatch": {
				backgroundColor: "rgba(255, 217, 102, 0.15)",
				borderRadius: "2px",
			},
			// The match currently under the cursor (if any) can be slightly stronger.
			".cm-selectionMatch.cm-selectionMatch-main": {
				backgroundColor: "rgba(38, 79, 120, 0.30)",
			},
			".cm-foldGutter": {
				width: "16px", // Fixed width for fold gutter
				paddingLeft: "2px",
			},
			".cm-lintGutter": {
				width: "16px", // Fixed width for lint gutter when present
			},
			".cm-scroller": {
				fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
				fontSize: "13px",
				lineHeight: "1.5",
			},
			".cm-process-badge": {
				backgroundColor: "orange",
				color: "black",
				borderRadius: "4px",
				padding: "0 4px",
				fontSize: "10px",
				fontWeight: "bold",
				marginLeft: "10px",
				verticalAlign: "middle",
				display: "inline-block",
			},
		},
		{ dark: true },
	);

	// Create linter only for .alt files, but always include lintGutter for consistent spacing
	const createLinterExtension = (filePath: string): Extension => {
		if (filePath.endsWith(".alt")) {
			const regexpLinter = linter(async (view) => {
				console.log("linting .alt file");
				const diagnostics: Diagnostic[] = [];

				const code = view.state.doc.toString();

				try {
					await compile(code);
				} catch (e: any) {
					console.log("Lint error:", e);

					// Check if error has position information
					if (
						e.pos &&
						typeof e.pos.start === "number" &&
						typeof e.pos.end === "number"
					) {
						diagnostics.push({
							from: e.pos.start,
							to: e.pos.end,
							severity: "error",
							message: e.message || "Compilation error",
						});
					} else {
						// If no position info, highlight the entire document or first line
						const docLength = view.state.doc.length;
						if (docLength > 0) {
							const firstLineEnd = view.state.doc.line(1).to;
							diagnostics.push({
								from: 0,
								to: Math.min(firstLineEnd, docLength),
								severity: "error",
								message: e.message || "Compilation error",
							});
						}
					}
				}
				return diagnostics;
			});
			return [regexpLinter, lintGutter()];
		}
		// For non-.alt files, still include lintGutter for consistent spacing, but no actual linter
		return [lintGutter()];
	};

	// Function to update language and linter
	const updateLanguage = (newFileName: string) => {
		console.log(`Switching to language for: ${newFileName}`);
		currentFileName = newFileName;

		// Update language extension
		const newLanguageExtension = getLanguageExtension(newFileName);
		editor.editorView().dispatch({
			effects: languageCompartment.reconfigure(newLanguageExtension),
		});

		// Update linter extension (only for .alt files, but always include gutter)
		const newLinterExtension = createLinterExtension(newFileName);
		editor.editorView().dispatch({
			effects: linterCompartment.reconfigure(newLinterExtension),
		});
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
	editor.createExtension(uiTheme);
	// This is the crucial line we are adding back.
	// It tells the editor to use our custom color palette for syntax highlighting.
	editor.createExtension(customSyntaxHighlighting);

	// Add compartments for dynamic extensions
	editor.createExtension(
		languageCompartment.of(getLanguageExtension(filePath)),
	);
	editor.createExtension(linterCompartment.of(createLinterExtension(filePath)));
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

	// Return editor with updateLanguage method and safe wrappers
	return {
		...editor,
		updateLanguage,
		getCurrentFileName: () => currentFileName,
		safeEditorView,
		safeUpdateContent,
		setReadOnly,
		highlightLines,
		clearHighlights,
	};
};

export default createEditor;
