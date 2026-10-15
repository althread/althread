// Lezer parser compiled with pnpm run prepare

import {
	foldInside,
	foldNodeProp,
	indentNodeProp,
	LRLanguage,
} from "@codemirror/language";

import { languageExtensions, type LanguageOptions } from "../../lsp/extension";

import { althreadHighlight } from "../../../lezer/highlight.js";
import { parser } from "../../../lezer/parser.js";

const parserWithMetadata = parser.configure({
	props: [
		althreadHighlight,
		indentNodeProp.add({
			Application: (context) =>
				context.column(context.node.from) + context.unit,
		}),
		foldNodeProp.add({
			Application: foldInside,
		}),
	],
});

export const exampleLanguage = LRLanguage.define({
	parser: parserWithMetadata,
	languageData: {
		commentTokens: { line: "//" },
	},
});

import { LanguageSupport } from "@codemirror/language";

export default function (options: LanguageOptions) {
	const service = languageExtensions(options);
	return new LanguageSupport(exampleLanguage, [
		service.extension,
		exampleLanguage.data.of({ autocomplete: service.completion }),
	]);
}
