import { createCodeMirror } from "solid-codemirror";
import { createSignal, onMount } from "solid-js";
import editor_lang from "./editor-lang";
import {tags} from "@lezer/highlight"
import {HighlightStyle} from "@codemirror/language"
import { Tag } from "@lezer/highlight";
import {keymap, highlightSpecialChars, drawSelection, highlightActiveLine, dropCursor,
    rectangularSelection, crosshairCursor,
    lineNumbers, highlightActiveLineGutter, EditorView} from "@codemirror/view"
import {Extension, EditorState, Compartment} from "@codemirror/state"
import {defaultHighlightStyle, syntaxHighlighting, indentOnInput, bracketMatching,
    foldGutter, foldKeymap} from "@codemirror/language"
import {defaultKeymap, history, historyKeymap, toggleComment} from "@codemirror/commands"
import {searchKeymap, highlightSelectionMatches} from "@codemirror/search"
import {autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap} from "@codemirror/autocomplete"
import {lintKeymap, lintGutter} from "@codemirror/lint"
import {indentWithTab} from "@codemirror/commands"
import { tags as t } from '@lezer/highlight';
import {linter, Diagnostic} from "@codemirror/lint"

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
const getLanguageExtension = (fileName: string): Extension => {
  const extension = fileName.split('.').pop()?.toLowerCase();
  
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

const createEditor = ({
    compile, 
    defaultValue,
    onValueChange,
    fileName = 'main.alt'
  }: {
    compile: (code: string) => any,
    defaultValue: string | undefined, 
    onValueChange: undefined | ((value:string) => void),
    fileName?: string
  }) => {
  const editor = createCodeMirror({
    value: defaultValue,
    onValueChange: onValueChange,
  });

  // Store current filename for language detection
  let currentFileName = fileName;

  // Create compartments for dynamic extensions
  const languageCompartment = new Compartment();
  const linterCompartment = new Compartment();

  // Theme definitions with consistent line number width
  const uiTheme = EditorView.theme({
    '&': {
      color: '#abb2bf',
      backgroundColor: '#1e1e1e',
    },
    '.cm-content': {
      caretColor: '#528bff',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeft: '2px solid #528bff'
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: '#3e4451'
    },
    '.cm-gutters': {
      backgroundColor: '#1e1e1e',
      color: '#5c6370',
      borderRight: '1px solid #333',
      minWidth: '60px' // Ensure minimum width for gutters
    },
    '.cm-lineNumbers': {
      minWidth: '40px', // Reserve space for up to 4 digits (9999 lines)
      paddingRight: '8px',
      textAlign: 'right'
    },
    '.cm-lineNumbers .cm-gutterElement': {
      minWidth: '32px', // Consistent width for line numbers
      textAlign: 'right'
    },
    '.cm-activeLineGutter': {
      backgroundColor: '#282c34'
    },
    '.cm-activeLine': {
      backgroundColor: '#282c34'
    },
    '.cm-foldGutter': {
      width: '16px', // Fixed width for fold gutter
      paddingLeft: '2px'
    },
    '.cm-lintGutter': {
      width: '16px' // Fixed width for lint gutter when present
    },
    '.cm-scroller': {
        fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
        fontSize: '13px',
        lineHeight: '1.5'
    }
  }, {dark: true});

  const uiHighlightStyle = HighlightStyle.define([
    { tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.moduleKeyword, t.operatorKeyword], color: '#c678dd' },
    { tag: [t.string, t.special(t.string)], color: '#98c379' },
    { tag: [t.number, t.bool, t.null], color: '#d19a66' },
    { tag: t.comment, color: '#5c6370', fontStyle: 'italic' },
    { tag: [t.className, t.typeName], color: '#e5c07b' },
    { tag: t.macroName, color: '#61afef' },
    { tag: t.variableName, color: '#abb2bf' },
    { tag: t.propertyName, color: '#e06c75' },
    { tag: [t.separator, t.punctuation], color: '#abb2bf' },
    { tag: t.invalid, color: '#f44747', borderBottom: '1px dotted #f44747' }
  ]);

  // Create linter only for .alt files, but always include lintGutter for consistent spacing
  const createLinterExtension = (fileName: string): Extension => {
    if (fileName.endsWith('.alt')) {
      const regexpLinter = linter(view => {
        console.log('linting .alt file');
        let diagnostics: Diagnostic[] = []
        
        const code = view.state.doc.toString();
        
        try {
            compile(code)
        } catch(e) {
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
  editor.createExtension(uiTheme);
  editor.createExtension(syntaxHighlighting(uiHighlightStyle, { fallback: true }));

  // Add compartments for dynamic extensions
  editor.createExtension(languageCompartment.of(getLanguageExtension(fileName)));
  editor.createExtension(linterCompartment.of(createLinterExtension(fileName)));

  // Return editor with updateLanguage method
  return {
    ...editor,
    updateLanguage,
    getCurrentFileName: () => currentFileName
  };
}

export default createEditor;