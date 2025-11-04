/** @jsxImportSource solid-js */
import { createMemo } from 'solid-js';
import SearchView from '@components/search/SearchView';
import type { SearchResult } from '@components/search/SearchView';
import { EXAMPLES } from '@examples/examples';

interface ExampleSearchProps {
  onLoadExample: (content: string, fileName: string) => void;
}

export default function ExampleSearch(props: ExampleSearchProps) {
  // Use a custom search results function that handles content search
  const allSearchResults = createMemo((): SearchResult[] => {
    return EXAMPLES.map(example => ({
      id: example.fileName,
      title: example.title,
      subtitle: example.fileName,
      description: example.description, // Base description without content preview
      icon: 'file-code',
      content: example.content,
      tags: example.tags,
      onClick: () => {
        props.onLoadExample(example.content, example.fileName);
      }
    }));
  });

  return (
    <SearchView 
      placeholder="Search examples by name, description, or content..."
      items={allSearchResults()}
      searchFields={['title', 'subtitle', 'description', 'content', 'tags']}
      emptyMessage="No examples available"
      noResultsMessage="No examples match your search"
      showAllByDefault={true}
    />
  );
}
