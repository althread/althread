import { createCodeMirror } from "solid-codemirror";
import editor_lang from "./editor-lang";
import {keymap, highlightSpecialChars, drawSelection, highlightActiveLine, dropCursor,
    rectangularSelection, crosshairCursor,
    lineNumbers, highlightActiveLineGutter, EditorView, Decoration, DecorationSet, WidgetType} from "@codemirror/view"
import {Extension, EditorState, Compartment, StateField, StateEffect} from "@codemirror/state"
import {indentOnInput, bracketMatching,
    foldGutter, foldKeymap} from "@codemirror/language"
import {defaultKeymap, history, historyKeymap, toggleComment} from "@codemirror/commands"
import {searchKeymap, highlightSelectionMatches} from "@codemirror/search"
import {autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap} from "@codemirror/autocomplete"
import {lintKeymap, lintGutter} from "@codemirror/lint"
import {indentWithTab} from "@codemirror/commands"
import {linter, Diagnostic} from "@codemirror/lint"
import { createCustomSyntaxHighlighting, type CodeEditorTheme } from "./custom-style";

// Import additional language supports
import { javascript } from "@codemirror/lang-javascript";
import { css } from "@codemirror/lang-css";
import { html } from "@codemirror/lang-html";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { cpp } from "@codemirror/lang-cpp";

// Language detection based on file extension
const getLanguageExtension = (filePath: string): Extension => {
  const extension = filePath.split('.').pop()?.toLowerCase();
  
  switch (extension) {
    case 'js':
    case 'jsx':
    case 'ts':
    case 'tsx':
      return javascript({ jsx: true, typescript: extension.includes('ts') });
    case 'css':
      return css();
    case 'html':
    case 'htm':
      return html();
    case 'json':
      return json();
    case 'md':
    case 'markdown':
      return markdown();
    case 'py':
      return python();
    case 'rs':
      return rust();
    case 'cpp':
    case 'cc':
    case 'cxx':
    case 'c':
    case 'h':
    case 'hpp':
      return cpp();
    case 'alt':
      return editor_lang(); // Your custom language
    default:
      return javascript(); // Default to JavaScript for unknown files
  }
};

// Define effect for highlighting lines
const addHighlightEffect = StateEffect.define<{line: number, label?: string}[]>();
const clearHighlightEffect = StateEffect.define();

class ProcessBadgeWidget extends WidgetType {
  constructor(readonly label: string) { super() }
  toDOM() {
    let span = document.createElement("span")
    span.className = "cm-process-badge"
    span.textContent = this.label
    return span
  }
}

// Define state field for line highlights
const highlightField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(highlights, tr) {
    highlights = highlights.map(tr.changes);
    for (let effect of tr.effects) {
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
                  style: "background-color: rgba(255, 165, 0, 0.2); border-left: 3px solid orange;" 
                }
              }).range(line.from)
            );

            // Badge Widget (at the end of the line)
            const labels = lineLabels.get(lineNum)!;
            if (labels.length > 0) {
              decorations.push(
                Decoration.widget({
                  widget: new ProcessBadgeWidget(labels.join(", ")),
                  side: 1 // After content
                }).range(line.to)
              );
            }
          }
        }
        highlights = Decoration.set(decorations.sort((a, b) => a.from - b.from));
      } else if (effect.is(clearHighlightEffect)) {
        highlights = Decoration.none;
      }
    }
    return highlights;
  },
  provide: f => EditorView.decorations.from(f)
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
    ...lintKeymap // Keep this for all languages, but only add actual linter for .alt
    ])
])()

const createEditorTheme = (theme: CodeEditorTheme) => {
  const isDark = theme === 'dark';

  return EditorView.theme({
    '&': {
      color: isDark ? '#abb2bf' : '#2f3441',
      backgroundColor: isDark ? '#1e1e1e' : '#f7f1e7',
    },
    '.cm-content': {
      caretColor: isDark ? '#528bff' : '#0f6bbd',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeft: `2px solid ${isDark ? '#528bff' : '#0f6bbd'}`
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
      backgroundColor: isDark ? 'rgba(38, 79, 120, 0.70) !important' : 'rgba(245, 184, 96, 0.38) !important'
    },
    '.cm-selectionLayer': {
      mixBlendMode: 'normal'
    },
    '.cm-selectionLayer .cm-selectionBackground': {
      mixBlendMode: 'normal'
    },
    '.cm-content ::selection': {
      backgroundColor: isDark ? 'rgba(38, 79, 120, 1.00)' : 'rgba(245, 184, 96, 0.52)'
    },
    '.cm-gutters': {
      backgroundColor: isDark ? '#1e1e1e' : '#efe5d6',
      color: isDark ? '#5c6370' : '#8b7f72',
      borderRight: `1px solid ${isDark ? '#333' : '#dccfbf'}`,
      minWidth: '60px'
    },
    '.cm-lineNumbers': {
      minWidth: '40px',
      paddingRight: '8px',
      textAlign: 'right'
    },
    '.cm-lineNumbers .cm-gutterElement': {
      minWidth: '32px',
      textAlign: 'right'
    },
    '.cm-activeLineGutter': {
      backgroundColor: isDark ? '#3a3f4b' : '#e5d9c5'
    },
    '.cm-activeLine': {
      backgroundColor: isDark ? 'rgba(58, 63, 75, 0.35)' : 'rgba(196, 148, 88, 0.10)',
      boxShadow: isDark
        ? 'inset 0 0 0 1px rgba(255, 255, 255, 0.06)'
        : 'inset 0 0 0 1px rgba(163, 118, 63, 0.14)'
    },
    '.cm-selectionMatch': {
      backgroundColor: isDark ? 'rgba(255, 217, 102, 0.15)' : 'rgba(209, 136, 38, 0.16)',
      borderRadius: '2px'
    },
    '.cm-selectionMatch.cm-selectionMatch-main': {
      backgroundColor: isDark ? 'rgba(38, 79, 120, 0.30)' : 'rgba(214, 156, 75, 0.22)'
    },
    '.cm-foldGutter': {
      width: '16px',
      paddingLeft: '2px'
    },
    '.cm-lintGutter': {
      width: '16px'
    },
    '.cm-scroller': {
        fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
        fontSize: '13px',
        lineHeight: '1.5'
    },
    '.cm-process-badge': {
      backgroundColor: isDark ? 'orange' : '#c96f1a',
      color: isDark ? 'black' : '#fffaf2',
      borderRadius: '4px',
      padding: '0 4px',
      fontSize: '10px',
      fontWeight: 'bold',
      marginLeft: '10px',
      verticalAlign: 'middle',
      display: 'inline-block'
    }
  }, {dark: isDark});
};

const createEditor = ({
    compile, 
    defaultValue,
    onValueChange,
    filePath = 'main.alt',
    theme = 'dark',
  }: {
    compile: (code: string) => Promise<any>,
    defaultValue: string | undefined, 
    onValueChange: undefined | ((value:string) => void),
    filePath?: string,
    theme?: CodeEditorTheme,
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
  const themeCompartment = new Compartment();
  const syntaxCompartment = new Compartment();

  // Create linter only for .alt files, but always include lintGutter for consistent spacing
  const createLinterExtension = (filePath: string): Extension => {
    if (filePath.endsWith('.alt')) {
      const regexpLinter = linter(async (view) => {
        console.log('linting .alt file');
        let diagnostics: Diagnostic[] = []
        
        const code = view.state.doc.toString();
        
        try {
            await compile(code)
        } catch(e: any) {
            console.log('Lint error:', e);
            
            // Check if error has position information
            if (e.pos && typeof e.pos.start === 'number' && typeof e.pos.end === 'number') {
              diagnostics.push({
                from: e.pos.start,
                to: e.pos.end,
                severity: "error",
                message: e.message || 'Compilation error'
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
                  message: e.message || 'Compilation error'
                });
              }
            }
        }
        return diagnostics
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
      effects: languageCompartment.reconfigure(newLanguageExtension)
    });
    
    // Update linter extension (only for .alt files, but always include gutter)
    const newLinterExtension = createLinterExtension(newFileName);
    editor.editorView().dispatch({
      effects: linterCompartment.reconfigure(newLinterExtension)
    });
  };

  // Initialize with basic setup and theme
  editor.createExtension(basicSetup);
  editor.createExtension(keymap.of([
    indentWithTab,
    { key: "Mod-/", run: toggleComment },
    {
      key: "Mod-s",
      preventDefault: true,
      run: () => {
        return true;
      }
    }
  ]));

  // Add theme and syntax highlighting
  editor.createExtension(themeCompartment.of(createEditorTheme(theme)));
  editor.createExtension(syntaxCompartment.of(createCustomSyntaxHighlighting(theme)));

  // Add compartments for dynamic extensions
  editor.createExtension(languageCompartment.of(getLanguageExtension(filePath)));
  editor.createExtension(linterCompartment.of(createLinterExtension(filePath)));
  editor.createExtension(readOnlyCompartment.of([]));
  editor.createExtension(highlightField);

  // Safe wrapper for editor view operations
  const safeEditorView = () => {
    try {
      return editor.editorView && editor.editorView() ? editor.editorView() : null;
    } catch (e) {
      console.warn('Editor view not ready:', e);
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
            insert: content
          }
        });
        view.update([update]);
        return true;
      } catch (e) {
        console.warn('Failed to update editor content:', e);
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
          effects: readOnlyCompartment.reconfigure(extension)
        });
        return true;
      } catch (e) {
        console.warn('Failed to set read-only mode:', e);
        return false;
      }
    }
    return false;
  };

  // Function to highlight specific lines
  const highlightLines = (specs: number[] | {line: number, label?: string}[]) => {
    const view = safeEditorView();
    if (view) {
      try {
        const formattedSpecs = specs.map(s => typeof s === 'number' ? {line: s} : s);
        view.dispatch({
          effects: addHighlightEffect.of(formattedSpecs)
        });
        return true;
      } catch (e) {
        console.warn('Failed to highlight lines:', e);
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
          effects: clearHighlightEffect.of(null)
        });
        return true;
      } catch (e) {
        console.warn('Failed to clear highlights:', e);
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
            syntaxCompartment.reconfigure(createCustomSyntaxHighlighting(nextTheme))
          ]
        });
        return true;
      } catch (e) {
        console.warn('Failed to update editor theme:', e);
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
    clearHighlights
  };
}

export default createEditor;