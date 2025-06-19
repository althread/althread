import { createCodeMirror } from "solid-codemirror";
import { createSignal, onMount } from "solid-js";
import editor_lang from "./editor-lang";
import {tags} from "@lezer/highlight"
import {HighlightStyle} from "@codemirror/language"
import { Tag } from "@lezer/highlight";
import {keymap, highlightSpecialChars, drawSelection, highlightActiveLine, dropCursor,
    rectangularSelection, crosshairCursor,
    lineNumbers, highlightActiveLineGutter, EditorView} from "@codemirror/view"
import {Extension, EditorState} from "@codemirror/state"
import {defaultHighlightStyle, syntaxHighlighting, indentOnInput, bracketMatching,
    foldGutter, foldKeymap} from "@codemirror/language"
import {defaultKeymap, history, historyKeymap, toggleComment} from "@codemirror/commands"
import {searchKeymap, highlightSelectionMatches} from "@codemirror/search"
import {autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap} from "@codemirror/autocomplete"
import {lintKeymap, lintGutter} from "@codemirror/lint"
import {indentWithTab} from "@codemirror/commands"
import { tags as t } from '@lezer/highlight';

import {linter, Diagnostic} from "@codemirror/lint"

// (The superfluous function calls around the list of extensions work
// around current limitations in tree-shaking software.)

/// This is an extension value that just pulls together a number of
/// extensions that you might want in a basic editor. It is meant as a
/// convenient helper to quickly set up CodeMirror without installing
/// and importing a lot of separate packages.
///
/// Specifically, it includes...
///
///  - [the default command bindings](#commands.defaultKeymap)
///  - [line numbers](#view.lineNumbers)
///  - [special character highlighting](#view.highlightSpecialChars)
///  - [the undo history](#commands.history)
///  - [a fold gutter](#language.foldGutter)
///  - [custom selection drawing](#view.drawSelection)
///  - [drop cursor](#view.dropCursor)
///  - [multiple selections](#state.EditorState^allowMultipleSelections)
///  - [reindentation on input](#language.indentOnInput)
///  - [the default highlight style](#language.defaultHighlightStyle) (as fallback)
///  - [bracket matching](#language.bracketMatching)
///  - [bracket closing](#autocomplete.closeBrackets)
///  - [autocompletion](#autocomplete.autocompletion)
///  - [rectangular selection](#view.rectangularSelection) and [crosshair cursor](#view.crosshairCursor)
///  - [active line highlighting](#view.highlightActiveLine)
///  - [active line gutter highlighting](#view.highlightActiveLineGutter)
///  - [selection match highlighting](#search.highlightSelectionMatches)
///  - [search](#search.searchKeymap)
///  - [linting](#lint.lintKeymap)
///
/// (You'll probably want to add some language package to your setup
/// too.)
///
/// This extension does not allow customization. The idea is that,
/// once you decide you want to configure your editor more precisely,
/// you take this package's source (which is just a bunch of imports
/// and an array literal), copy it into your own code, and adjust it
/// as desired.
const basicSetup: Extension = (() => [
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightSpecialChars(),
    history(),
    foldGutter(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    indentOnInput(),
    //syntaxHighlighting(defaultHighlightStyle, {fallback: true}),
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
    ...lintKeymap
    ])
])()


const createEditor = ({
    compile, 
    defaultValue,
    onValueChange
  }: {
    compile: (code: string) => any,
    defaultValue: string | undefined, 
    onValueChange: undefined | ((value:string) => void)
  }) => {
  const editor = createCodeMirror({
    /**
     * The initial value of the editor
     */
    value: defaultValue,
    /**
     * Fired whenever the editor code value changes.
     */
    onValueChange: onValueChange,
    /**
     * Fired whenever a change occurs to the document, every time the view updates.
     */
    //onModelViewUpdate: (modelView) => console.log("modelView updated", modelView),
    /**
     * Fired whenever a transaction has been dispatched to the view.
     * Used to add external behavior to the transaction [dispatch function](https://codemirror.net/6/docs/ref/#view.EditorView.dispatch) for this editor view, which is the way updates get routed to the view
     */
    //onTransactionDispatched: (tr: Transaction, view: EditorView) => console.log("Transaction", tr)
  });

  // --- START: "One Dark Pro Night Flat" inspired theme ---

  const uiTheme = EditorView.theme({
    '&': {
      color: '#abb2bf', // Default text color
      backgroundColor: '#1e1e1e', // One Dark Pro background
    },
    '.cm-content': {
      caretColor: '#528bff', // Blinking cursor color
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeft: '2px solid #528bff'
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: '#3e4451' // A more visible selection color
    },
    '.cm-gutters': {
      backgroundColor: '#1e1e1e',
      color: '#5c6370', // Gutter text color
      borderRight: '1px solid #333'
    },
    '.cm-activeLineGutter': {
      backgroundColor: '#282c34' // Subtle dark highlight from One Dark Pro
    },
    '.cm-activeLine': {
      backgroundColor: '#282c34' // Matching the gutter for a unified look
    },
    '.cm-scroller': {
        fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
        fontSize: '13px',
        lineHeight: '1.5'
    }
  }, {dark: true});

  const uiHighlightStyle = HighlightStyle.define([
    { tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.moduleKeyword, t.operatorKeyword], color: '#c678dd' }, // Magenta for keywords
    { tag: [t.string, t.special(t.string)], color: '#98c379' }, // Green for strings
    { tag: [t.number, t.bool, t.null], color: '#d19a66' }, // Orange for literals
    { tag: t.comment, color: '#5c6370', fontStyle: 'italic' }, // Gray for comments
    { tag: [t.className, t.typeName], color: '#e5c07b' }, // Yellow for types/classes
    { tag: t.macroName, color: '#61afef' }, // Blue for functions/macros
    { tag: t.variableName, color: '#abb2bf' }, // Light gray for variables
    { tag: t.propertyName, color: '#e06c75' }, // Red for properties
    { tag: [t.separator, t.punctuation], color: '#abb2bf' },
    { tag: t.invalid, color: '#f44747', borderBottom: '1px dotted #f44747' }
  ]);

  // --- END: Theme ---

  // Remove or comment out the old theme extensions
  // editor.createExtension(baseTheme);
  // editor.createExtension(modifiedTheme);
  // editor.createExtension(oneDark);

  editor.createExtension(basicSetup);
  editor.createExtension(keymap.of([
    indentWithTab,
    { key: "Mod-/", run: toggleComment }
  ]));
  editor.createExtension(editor_lang());

  // Add the new theme and syntax highlighting
  editor.createExtension(uiTheme);
  editor.createExtension(syntaxHighlighting(uiHighlightStyle, { fallback: true }));

  const debugTheme = EditorView.theme({
    ".cm-gutters": {
      borderRight: "#383e48 solid 2px",
    },
    ".cm-line span": {
      position: "relative",
    },
    ".cm-line span:hover::after": {
      position: "absolute",
      bottom: "100%",
      left: 0,
      background: "black",
      color: "white",
      border: "solid 2px",
      borderRadius: "5px",
      content: "var(--tags)",
      width: `max-content`,
      padding: "1px 4px",
      zIndex: 10,
      pointerEvents: "none",
    },
  });
  const debugHighlightStyle = HighlightStyle.define(
    Object.entries(tags).map(([key, value]) => {
      return { tag: value as Tag, "--tags": `"tag.${key}"` };
    })
  );
  
  const debug = [debugTheme, syntaxHighlighting(debugHighlightStyle)];
  //editor.createExtension(debug);



  
  const regexpLinter = linter(view => {
    console.log('linting');
    let diagnostics: Diagnostic[] = []
    try {
        compile(view.state.doc.toString())
    } catch(e) {
        console.log(e);
        console.log(Object.keys(e));
        diagnostics.push({
            from: e.pos.start,
            to: e.pos.end,
            severity: "error",
            message: e.message
        })
    }
    return diagnostics
  })
  editor.createExtension(regexpLinter);
  editor.createExtension(lintGutter());

  // if we want to highlight specific tags
  //const myHighlightStyle = HighlightStyle.define([
    //{tag: tags.keyword, color: "#0c6"},
    //{tag: tags.comment, color: "#05d", fontStyle: "italic"}
  //]);

  return editor;
}

export default createEditor;