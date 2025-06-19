// Lezer parser compiled with pnpm run prepare
import {parser} from "../lezer/parser.js"
import {foldNodeProp, foldInside, indentNodeProp} from "@codemirror/language"
import {styleTags, tags as t} from "@lezer/highlight"

import {LRLanguage} from "@codemirror/language"
import {completeFromList} from "@codemirror/autocomplete"
  
import {althreadHighlight} from "../lezer/highlight.js"

let parserWithMetadata = parser.configure(//{}); /*
    {
props: [
    althreadHighlight,
    indentNodeProp.add({
    Application: context => context.column(context.node.from) + context.unit
    }),
    foldNodeProp.add({
    Application: foldInside
    })
]
}); // */


export const exampleLanguage = LRLanguage.define({
    parser: parserWithMetadata,
    languageData: {
      commentTokens: {line: "//"}
    }
  })
  
  export const exampleCompletion = exampleLanguage.data.of({
      autocomplete: completeFromList([
        {label: "defun", type: "keyword"},
        {label: "defvar", type: "keyword"},
        {label: "let", type: "keyword"},
        {label: "cons", type: "function"},
        {label: "car", type: "function"},
        {label: "cdr", type: "function"}
      ])
    })
  

import {LanguageSupport} from "@codemirror/language"

export default function() {
  return new LanguageSupport(exampleLanguage, [exampleCompletion])
}
