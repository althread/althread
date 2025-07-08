/** @jsxImportSource solid-js */
import { createSignal, createMemo, For, Show } from 'solid-js';
import './SearchView.css';

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
  searchFields?: ('title' | 'subtitle' | 'description' | 'content' | 'path' | 'tags')[];
  emptyMessage?: string;
  noResultsMessage?: string;
  showAllByDefault?: boolean; // Whether to show all items when query is empty
}

export default function SearchView(props: SearchViewProps) {
  const [query, setQuery] = createSignal('');
  
  const searchFields = () => props.searchFields || ['title', 'subtitle', 'description', 'content', 'path'];
  
  // Helper function to create VS Code-style content preview
  const createContentPreview = (item: SearchResult, searchQuery: string): string => {
    const content = item.content || '';
    const description = item.description || '';
    
    if (!searchQuery || !content) {
      // No search query or content, show description with first line or first 100 chars
      const firstLine = content.split('\n')[0] || '';
      const preview = firstLine.length > 100 ? `${firstLine.substring(0, 100)}...` : firstLine;
      return preview ? `${description} | Content: ${preview}` : description;
    }
    
    const queryLower = searchQuery.toLowerCase();
    const lines = content.split('\n');
    
    // Find the first line that contains the search query
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (line.toLowerCase().includes(queryLower)) {
        // Found a matching line, show it with context
        const trimmedLine = line.trim();
        if (trimmedLine.length > 100) {
          // If the line is too long, try to center the match
          const matchIndex = line.toLowerCase().indexOf(queryLower);
          const start = Math.max(0, matchIndex - 30);
          const end = Math.min(line.length, start + 100);
          const snippet = line.substring(start, end);
          return `${description}\n${start > 0 ? '...' : ''}${snippet}${end < line.length ? '...' : ''}`;
        }
        return `${description}\n${trimmedLine}`;
      }
    }
    
    // No match found in content, show description with first line
    const firstLine = lines[0] || '';
    const preview = firstLine.length > 100 ? `${firstLine.substring(0, 100)}...` : firstLine;
    return preview ? `${description} | Content: ${preview}` : description;
  };
  
  // Helper function to highlight search matches
  const highlightMatch = (text: string, searchQuery: string): string => {
    if (!searchQuery) return text;
    
    const queryLower = searchQuery.toLowerCase();
    const textLower = text.toLowerCase();
    const index = textLower.indexOf(queryLower);
    
    if (index === -1) return text;
    
    const before = text.substring(0, index);
    const match = text.substring(index, index + searchQuery.length);
    const after = text.substring(index + searchQuery.length);
    
    return `${before}<span class="search-highlight">${match}</span>${after}`;
  };
  
  const filteredItems = createMemo(() => {
    const searchQuery = query().toLowerCase().trim();
    
    if (!searchQuery) {
      return props.showAllByDefault !== false ? props.items : [];
    }
    
    return props.items.filter(item => {
      return searchFields().some(field => {
        const value = item[field];
        if (!value) return false;
        
        // Handle string arrays (like tags)
        if (Array.isArray(value)) {
          return value.some(v => v.toLowerCase().includes(searchQuery));
        }
        
        // Handle strings
        return value.toLowerCase().includes(searchQuery);
      });
    });
  });

  const clearSearch = () => {
    setQuery('');
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
                if (e.key === 'Escape') {
                  clearSearch();
                  e.preventDefault();
                }
              }}
            />
            <Show when={query()}>
              <button class="search-clear" onClick={clearSearch}>
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
          
          <Show when={props.items.length > 0 && !query() && props.showAllByDefault === false}>
            <div class="search-empty">
              <i class="codicon codicon-search"></i>
              <p>Start typing to search files...</p>
            </div>
          </Show>
          
          <Show when={props.items.length > 0 && filteredItems().length === 0 && query()}>
            <div class="search-no-results">
              <i class="codicon codicon-search-stop"></i>
              <p>{props.noResultsMessage || "No results found"}</p>
              <button class="search-clear-btn" onClick={clearSearch}>
                Clear search
              </button>
            </div>
          </Show>
          
          <Show when={filteredItems().length > 0}>
            <For each={filteredItems()}>
              {(item) => (
                <button class="search-result-item" onClick={item.onClick}>
                  <Show when={item.icon}>
                    <i class={`codicon codicon-${item.icon} search-result-icon`}></i>
                  </Show>
                  <div class="search-result-content">
                    <div class="search-result-title">{item.title}</div>
                    <Show when={item.subtitle}>
                      <div class="search-result-subtitle">
                        {item.subtitle === 'Root' ? 'root' : item.subtitle}
                      </div>
                    </Show>
                    <Show when={item.description || item.content}>
                      <div 
                        class="search-result-description"
                        innerHTML={highlightMatch(createContentPreview(item, query()), query())}
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
