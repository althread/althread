/** @jsxImportSource solid-js */
import { For, createSignal } from 'solid-js';
import type { FileSystemEntry } from './FileExplorer';
import './FileTabs.css';

type FileTabsProps = {
  openFiles: FileSystemEntry[];
  activeFile: FileSystemEntry | null;
  onTabClick: (file: FileSystemEntry) => void;
  onTabClose: (file: FileSystemEntry) => void;
};

const FileTabs = (props: FileTabsProps) => {
  // A helper to get a unique key for each file, assuming paths are unique
  const getFileId = (file: FileSystemEntry) => {
    // This is a placeholder. In a real app, you'd have unique IDs.
    // For now, we'll assume file names are unique at the root for simplicity.
    return file.name;
  }

  const [hovered, setHovered] = createSignal(false);

  return (
    <div
      class={`file-tabs-container${hovered() ? ' scrollbar-hover' : ''}`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <For each={props.openFiles}>
        {(file) => (
          <div
            class="file-tab"
            classList={{ active: props.activeFile && getFileId(props.activeFile) === getFileId(file) }}
            onClick={() => props.onTabClick(file)}
          >
            <i class="codicon codicon-file"></i>
            <span class="tab-label">{file.name}</span>
            <button
              class="tab-close-button"
              title="Close"
              onClick={(e) => {
                e.stopPropagation();
                props.onTabClose(file);
              }}
            >
              <i class="codicon codicon-close"></i>
            </button>
          </div>
        )}
      </For>
    </div>
  );
};

export default FileTabs;