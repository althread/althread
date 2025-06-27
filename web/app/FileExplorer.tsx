/** @jsxImportSource solid-js */
import { For, Show, createSignal, createEffect } from 'solid-js';
import './FileExplorer.css';

// Define a type for our file system entries
export type FileSystemEntry = {
  name: string;
  type: 'file' | 'directory';
  children?: FileSystemEntry[];
};

// Props for the component
type FileExplorerProps = {
  files: FileSystemEntry[];
  onFileSelect: (path: string) => void;
  onNewFile: (name: string) => void;
  onNewFolder: (name: string) => void;
};

const FileEntry = (props: { entry: FileSystemEntry; path: string; onFileSelect: (path: string) => void }) => {
  const { entry, path, onFileSelect } = props;
  const currentPath = path ? `${path}/${entry.name}` : entry.name;

  if (entry.type === 'directory') {
    const [isOpen, setIsOpen] = createSignal(true);

    return (
      <div class="directory">
        <div class="directory-header" onClick={() => setIsOpen(!isOpen())}>
          <i class={`codicon codicon-chevron-${isOpen() ? 'down' : 'right'}`}></i>
          <i class="codicon codicon-folder"></i>
          <span>{entry.name}</span>
        </div>
        <Show when={isOpen()}>
            <div class="directory-children">
            <For each={entry.children}>
                {(child) => <FileEntry entry={child} path={currentPath} onFileSelect={onFileSelect} />}
            </For>
            </div>
        </Show>
      </div>
    );
  }

  return (
    <div class="file" onClick={() => onFileSelect(currentPath)}>
      <i class="codicon codicon-file"></i>
      <span>{entry.name}</span>
    </div>
  );
};

const FileExplorer = (props: FileExplorerProps) => {
  const [creating, setCreating] = createSignal<{ type: 'file' | 'folder' } | null>(null);
  let inputRef: HTMLInputElement | undefined;

  createEffect(() => {
    if (creating() && inputRef) {
      inputRef.focus();
    }
  });

  const handleCreateCommit = () => {
    if (!inputRef || !creating()) return;
    
    const name = inputRef.value.trim();
    if (name) {
      if (creating()!.type === 'file') {
        props.onNewFile(name);
      } else {
        props.onNewFolder(name);
      }
    }
    setCreating(null);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleCreateCommit();
    } else if (e.key === 'Escape') {
      setCreating(null);
    }
  };

  return (
    <div class="file-explorer">
      <div class="file-explorer-header">
        <h3>Explorer</h3>
        <div class="file-explorer-actions">
            <button onClick={() => setCreating({ type: 'file' })} title="New File" disabled={!!creating()}>
                <i class="codicon codicon-new-file"></i>
            </button>
            <button onClick={() => setCreating({ type: 'folder' })} title="New Folder" disabled={!!creating()}>
                <i class="codicon codicon-new-folder"></i>
            </button>
        </div>
      </div>
      <div class="file-explorer-content">
        <For each={props.files}>
          {(entry) => <FileEntry entry={entry} path="" onFileSelect={props.onFileSelect} />}
        </For>
        <Show when={creating()}>
          <div class="file-entry-input">
            <i class={`codicon codicon-${creating()!.type === 'file' ? 'file' : 'folder'}`}></i>
            <input
              ref={inputRef}
              type="text"
              onKeyDown={handleKeyDown}
              onBlur={handleCreateCommit}
            />
          </div>
        </Show>
      </div>
    </div>
  );
};

export default FileExplorer;