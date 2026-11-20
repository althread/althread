import type { SearchResult } from "@components/search/SearchView";
import SearchView from "@components/search/SearchView";
import { EXAMPLES } from "@examples/examples";
import { createMemo } from "solid-js";

interface ExampleSearchProps {
	onLoadExample: (content: string, fileName: string) => void;
}

const ACRONYMS = new Set(["ltl", "tp1", "tp2", "tp3"]);

/** "ltl-safety-violation" → "LTL safety violation" */
const prettifyTitle = (title: string) =>
	title
		.split(/[-_\s]+/)
		.filter(Boolean)
		.map((word, i) => {
			const lower = word.toLowerCase();
			if (ACRONYMS.has(lower)) return lower.toUpperCase();
			return i === 0 ? lower.charAt(0).toUpperCase() + lower.slice(1) : lower;
		})
		.join(" ");

/** Use the leading comment of the example as its description when none was provided. */
const describeExample = (description: string, content: string) => {
	if (description && !description.startsWith("Example program:")) {
		return description;
	}
	const comment: string[] = [];
	for (const line of content.split("\n")) {
		const match = line.trim().match(/^\/\/\s?(.*)$/);
		if (!match) break;
		const text = match[1].replace(/^Example\s+\d+\s*:\s*/i, "").trim();
		if (text) comment.push(text);
	}
	return comment.join(" — ");
};

export default function ExampleSearch(props: ExampleSearchProps) {
	// Use a custom search results function that handles content search
	const allSearchResults = createMemo((): SearchResult[] => {
		return EXAMPLES.map((example) => ({
			id: example.fileName,
			title: prettifyTitle(example.title),
			subtitle: example.fileName,
			description: describeExample(example.description, example.content),
			icon: "file-code",
			content: example.content,
			tags: example.tags,
			onClick: () => {
				props.onLoadExample(example.content, example.fileName);
			},
		}));
	});

	return (
		<SearchView
			placeholder="Search examples by name, description, or content..."
			items={allSearchResults()}
			searchFields={["title", "subtitle", "description", "content", "tags"]}
			emptyMessage="No examples available"
			noResultsMessage="No examples match your search"
			showAllByDefault={true}
		/>
	);
}
