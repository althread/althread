import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

export type CodeEditorTheme = 'dark' | 'light';

const palette = {
    dark: {
        text: '#abb2bf',
        keyword: '#c678dd',
        string: '#98c379',
        number: '#d19a66',
        comment: '#5c6370',
        typeName: '#e5c07b',
        functionName: '#61afef',
        property: '#e06c75',
        namespace: '#61afef',
    },
    light: {
        text: '#2f3441',
        keyword: '#8b38c9',
        string: '#2f7d32',
        number: '#b65c1b',
        comment: '#8b94a7',
        typeName: '#9f6a00',
        functionName: '#0f6bbd',
        property: '#bc4b63',
        namespace: '#0f6bbd',
    }
} as const;

export const createUiHighlightStyle = (theme: CodeEditorTheme) => {
    const colors = palette[theme];

    return HighlightStyle.define([
        { tag: t.content, color: colors.text },
        { tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.operatorKeyword, t.modifier], color: colors.keyword },
        { tag: [t.moduleKeyword], color: colors.keyword, fontWeight: 'bold' },
        { tag: [t.special(t.keyword)], color: colors.keyword, fontStyle: 'italic' },
        { tag: [t.string, t.special(t.string)], color: colors.string },
        { tag: [t.number, t.bool, t.null], color: colors.number },
        { tag: t.lineComment, color: colors.comment, fontStyle: 'italic' },
        { tag: t.blockComment, color: colors.comment, fontStyle: 'italic' },
        { tag: [t.className, t.typeName], color: colors.typeName },
        { tag: t.variableName, color: colors.text },
        { tag: t.function(t.variableName), color: colors.functionName },
        { tag: t.propertyName, color: colors.property },
        { tag: t.namespace, color: colors.namespace },
        { tag: [t.separator, t.punctuation], color: colors.text },
        { tag: t.derefOperator, color: colors.text },
    ]);
};

export const createCustomSyntaxHighlighting = (theme: CodeEditorTheme) =>
    syntaxHighlighting(createUiHighlightStyle(theme), { fallback: true });

export const customSyntaxHighlighting = createCustomSyntaxHighlighting('dark');