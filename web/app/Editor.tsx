
import { createCodeMirror } from "solid-codemirror";
import { createSignal, onMount } from "solid-js";
import editor_lang from "./editor-lang";
import {tags} from "@lezer/highlight"
import {HighlightStyle} from "@codemirror/language"

import {keymap, highlightSpecialChars, drawSelection, highlightActiveLine, dropCursor,
    rectangularSelection, crosshairCursor,
    lineNumbers, highlightActiveLineGutter, EditorView} from "@codemirror/view"
import {Extension, EditorState} from "@codemirror/state"
import {defaultHighlightStyle, syntaxHighlighting, indentOnInput, bracketMatching,
    foldGutter, foldKeymap} from "@codemirror/language"
import {defaultKeymap, history, historyKeymap} from "@codemirror/commands"
import {searchKeymap, highlightSelectionMatches} from "@codemirror/search"
import {autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap} from "@codemirror/autocomplete"
import {lintKeymap, lintGutter} from "@codemirror/lint"
import {indentWithTab} from "@codemirror/commands"
import {oneDark} from "@codemirror/theme-one-dark"
import { tags as t } from '@lezer/highlight';
import customStyle from "./custom-style";

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


const Editor =  (props) => {
  const { editorView, ref: editorRef, createExtension } = createCodeMirror({
    /**
     * The initial value of the editor
     */
    value: `

shared {
  let A: bool = false;
  let B: bool = true;
  let Done = 0;
}

program A() {
  print("starting A");
  A = false;
  B = true;
  Done += 1;
  send out(42,true);
}

program B() {
  print("starting B");
  A = true;
  B = false;
  Done += 1;
}

always {
  A || B;
}

main {
  let a = run A();
  run B();
  wait Done == 2;

  channel a.out (int, bool)> self.in;

  wait receive in(x,y) => {
    print("Receive", x, y);
  };
  print("DONE");
}
    
    `,
    /**
     * Fired whenever the editor code value changes.
     */
    onValueChange: (value) => props.onValueChange(value),
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

  const baseTheme = EditorView.theme({
    '&': {
      textAlign: 'left',
      fontSize: '18px',
    },
  });
  createExtension(baseTheme);
  createExtension(basicSetup);
  createExtension(keymap.of([indentWithTab]));
  createExtension(editor_lang());


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
      return { tag: value, "--tags": `"tag.${key}"` };
    })
  );
  
  const debug = [debugTheme, syntaxHighlighting(debugHighlightStyle)];
  //createExtension(debug);



  
  const highlightStyle = HighlightStyle.define(customStyle);  
  const modifiedTheme = [EditorView.theme({
    ".cm-gutters": {
      borderRight: "#383e48 solid 2px",
    }
  }), syntaxHighlighting(highlightStyle)];

  createExtension(modifiedTheme);
  createExtension(oneDark);


  const regexpLinter = linter(view => {
    console.log('linting');
    let diagnostics: Diagnostic[] = []
    try {
        props.compile(view.state.doc.toString())
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
  createExtension(regexpLinter);
  createExtension(lintGutter());

  // if we want to highlight specific tags
  //const myHighlightStyle = HighlightStyle.define([
    //{tag: tags.keyword, color: "#0c6"},
    //{tag: tags.comment, color: "#05d", fontStyle: "italic"}
  //]);

  let [out, setOut] = createSignal("");

  return <>
  <div>
    <button onClick={(e) => {
      console.log(editorView)
      try {
        setOut(props.run(editorView().state.doc.toString()));
      } catch(e) {
        setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
      }
    }}>Run</button>
  </div>
  <div ref={editorRef} />
<pre>
{out()}
</pre>
  </>;
};

export default Editor;