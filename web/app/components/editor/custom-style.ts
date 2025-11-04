import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

// This creates the "color palette" that maps the tags from your parser to actual CSS colors.
export const uiHighlightStyle = HighlightStyle.define([
    // Default text color
    { tag: t.content, color: '#abb2bf' },

    // Keywords
    { tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.moduleKeyword, t.operatorKeyword], color: '#c678dd' },
    
    // Literals: strings, numbers, booleans
    { tag: [t.string, t.special(t.string)], color: '#98c379' },
    { tag: [t.number, t.bool, t.null], color: '#d19a66' },
    
    // Comments - use lower specificity to avoid overriding function names
    { tag: t.lineComment, color: '#5c6370', fontStyle: 'italic' },
    { tag: t.blockComment, color: '#5c6370', fontStyle: 'italic' },
    
    // Names: variables, functions, classes
    { tag: [t.className, t.typeName], color: '#e5c07b' },
    { tag: t.variableName, color: '#abb2bf' }, // Default variable color
    { tag: t.function(t.variableName), color: '#61afef' }, // Functions should be blue
    { tag: t.propertyName, color: '#e06c75' }, // Properties (like in `obj.prop`) in red
    
    // Special highlighting for your language
    { tag: t.namespace, color: '#61afef' }, // For module names like `math` in `math.add`
    
    // Punctuation and Operators
    { tag: [t.separator, t.punctuation], color: '#abb2bf' },
    { tag: t.derefOperator, color: '#abb2bf' }, // The '.' in `math.add`
]);

// This exports the HighlightStyle as a ready-to-use CodeMirror extension.
export const customSyntaxHighlighting = syntaxHighlighting(uiHighlightStyle, { fallback: true });