import { createMemo, createSignal, For, Show } from "solid-js";
import "./SearchView.css";

export interface SearchResult {
	id: string;
	title: string;
	subtitle?: string;
	description?: string;
	icon?: string;
	onClick: () => void;
	// Additional searchable fields
	content?: string;
	path?: string;
	tags?: string[];
}

interface SearchViewProps {
	placeholder?: string;
	items: SearchResult[];
	searchFields?: (
		| "title"
		| "subtitle"
		| "description"
		| "content"
		| "path"
		| "tags"
	)[];
	emptyMessage?: string;
	noResultsMessage?: string;
	showAllByDefault?: boolean; // Whether to show all items when query is empty
}

export default function SearchView(props: SearchViewProps) {
	const [query, setQuery] = createSignal("");

	const searchFields = () =>
		props.searchFields || [
			"title",
			"subtitle",
			"description",
			"content",
			"path",
		];

	/** Description, plus the first content line matching the query (VS Code style). */
	const createContentPreview = (
		item: SearchResult,
		searchQuery: string,
	): string => {
		const description = item.description || "";
		const content = item.content || "";
		const queryLower = searchQuery.trim().toLowerCase();
		if (!queryLower || !content) return description;

		const line = content
			.split("\n")
			.find((l) => l.toLowerCase().includes(queryLower));
		if (!line) return description;

		let snippet = line.trim();
		if (snippet.length > 100) {
			const matchIndex = snippet.toLowerCase().indexOf(queryLower);
			const start = Math.max(0, matchIndex - 30);
			const end = Math.min(snippet.length, start + 100);
			snippet = `${start > 0 ? "…" : ""}${snippet.substring(start, end)}${end < snippet.length ? "…" : ""}`;
		}
		return description ? `${description}\n${snippet}` : snippet;
	};

	const escapeHtml = (text: string) =>
		text
			.replace(/&/g, "&amp;")
			.replace(/</g, "&lt;")
			.replace(/>/g, "&gt;")
			.replace(/"/g, "&quot;");

	/** Escapes the text (it can come from user files) and wraps the first match in a highlight span. */
	const highlightMatch = (text: string, searchQuery: string): string => {
		const query = searchQuery.trim();
		const index = query ? text.toLowerCase().indexOf(query.toLowerCase()) : -1;
		if (index === -1) return escapeHtml(text);

		const before = text.substring(0, index);
		const match = text.substring(index, index + query.length);
		const after = text.substring(index + query.length);
		return `${escapeHtml(before)}<span class="search-highlight">${escapeHtml(match)}</span>${escapeHtml(after)}`;
	};

	const filteredItems = createMemo(() => {
		const searchQuery = query().toLowerCase().trim();

		if (!searchQuery) {
			return props.showAllByDefault !== false ? props.items : [];
		}

		return props.items.filter((item) => {
			return searchFields().some((field) => {
				const value = item[field];
				if (!value) return false;

				// Handle string arrays (like tags)
				if (Array.isArray(value)) {
					return value.some((v) => v.toLowerCase().includes(searchQuery));
				}

				// Handle strings
				return value.toLowerCase().includes(searchQuery);
			});
		});
	});

	const clearSearch = () => {
		setQuery("");
	};

	return (
		<div class="search-view">
			<div class="search-view-header">
				<h3>Search</h3>
			</div>

			<div class="search-view-content">
				<div class="search-input-container">
					<div class="search-input-wrapper">
						<i class="codicon codicon-search search-icon"></i>
						<input
							type="text"
							class="search-input"
							placeholder={props.placeholder || "Search..."}
							value={query()}
							onInput={(e) => setQuery(e.currentTarget.value)}
							onKeyDown={(e) => {
								if (e.key === "Escape") {
									clearSearch();
									e.preventDefault();
								}
							}}
						/>
						<Show when={query()}>
							<button type="button" class="search-clear" onClick={clearSearch}>
								<i class="codicon codicon-close"></i>
							</button>
						</Show>
					</div>
				</div>

				<div class="search-results">
					<Show when={props.items.length === 0}>
						<div class="search-empty">
							<i class="codicon codicon-search"></i>
							<p>{props.emptyMessage || "No items available"}</p>
						</div>
					</Show>

					<Show
						when={
							props.items.length > 0 &&
							!query() &&
							props.showAllByDefault === false
						}
					>
						<div class="search-empty">
							<i class="codicon codicon-search"></i>
							<p>Start typing to search files...</p>
						</div>
					</Show>

					<Show
						when={
							props.items.length > 0 && filteredItems().length === 0 && query()
						}
					>
						<div class="search-no-results">
							<i class="codicon codicon-search-stop"></i>
							<p>{props.noResultsMessage || "No results found"}</p>
							<button type="button" class="search-clear-btn" onClick={clearSearch}>
								Clear search
							</button>
						</div>
					</Show>

					<Show when={filteredItems().length > 0}>
						<For each={filteredItems()}>
							{(item) => (
								<button type="button" class="search-result-item" onClick={item.onClick}>
									<Show when={item.icon}>
										<i
											class={`codicon codicon-${item.icon} search-result-icon`}
										></i>
									</Show>
									<div class="search-result-content">
										<div class="search-result-title">{item.title}</div>
										<Show when={item.subtitle}>
											<div class="search-result-subtitle">
												{item.subtitle === "Root" ? "root" : item.subtitle}
											</div>
										</Show>
										<Show when={createContentPreview(item, query())}>
											<div
												class="search-result-description"
												innerHTML={highlightMatch(
													createContentPreview(item, query()),
													query(),
												)}
											></div>
										</Show>
									</div>
								</button>
							)}
						</For>
					</Show>
				</div>
			</div>
		</div>
	);
}
