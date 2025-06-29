/** @jsxImportSource solid-js */
import { For, Show, createSignal, createEffect } from 'solid-js';
import './FileExplorer.css';

// Define a type for our file system entries
export type FileSystemEntry = {
  id: string; // Add this line
  name: string;
  type: 'file' | 'directory';
  children?: FileSystemEntry[];
};

// Props for the component
type FileExplorerProps = {
  files: FileSystemEntry[];
  onFileSelect: (path: string, isMultiSelect?: boolean) => void;
  onNewFile: (name: string) => void;
  onNewFolder: (name: string) => void;
  onMoveEntry: (sourcePath: string, destPath: string) => void;
  onFileUpload: (files: File[], destPath: string) => void;
  getFilePath: (entry: FileSystemEntry) => string;
  activeFile: FileSystemEntry | null;
  selectedFiles: string[];
  onSelectionChange: (selected: string[]) => void;
  creationError?: string | null;
  setCreationError?: (msg: string | null) => void;
};

const FileEntry = (props: { 
  entry: FileSystemEntry; 
  path: string; 
  onFileSelect: (path: string, isMultiSelect?: boolean) => void; 
  getFilePath: (entry: FileSystemEntry) => string; 
  activeFile: FileSystemEntry | null; 
  onMoveEntry: (source: string, dest: string) => void;
  selectedFiles: string[];
  onSelectionChange: (selected: string[]) => void;
  onFileUpload: (files: File[], destPath: string) => void;
}) => {
  const currentPath = props.path ? `${props.path}/${props.entry.name}` : props.entry.name;
  const [isDragOver, setIsDragOver] = createSignal(false);
  const [isDragging, setIsDragging] = createSignal(false);

  const isSelected = () => props.selectedFiles.includes(currentPath);

  const handleClick = (_e: MouseEvent) => {
    props.onSelectionChange([currentPath]);
    props.onFileSelect(currentPath);
  };

  const handleDragStart = (e: DragEvent) => {
    e.dataTransfer!.effectAllowed = 'move';
    setIsDragging(true);

    if (isSelected()) {
      e.dataTransfer!.setData('text/plain', JSON.stringify(props.selectedFiles));
    } else {
      e.dataTransfer!.setData('text/plain', JSON.stringify([currentPath]));
    }

    e.stopPropagation();
  };

  const handleDragEnd = () => {
    setIsDragging(false);
  };

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer?.files.length) {
      e.dataTransfer!.dropEffect = 'copy';
    } else {
      e.dataTransfer!.dropEffect = 'move';
    }
    setIsDragOver(true);
  };

  const handleDragLeave = (e: DragEvent) => {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const isInsideX = e.clientX >= rect.left && e.clientX <= rect.right;
    const isInsideY = e.clientY >= rect.top && e.clientY <= rect.bottom;

    if (!isInsideX || !isInsideY) {
      setIsDragOver(false);
    }
  };

  const handleDrop = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);

    console.log('Drop event on:', currentPath, 'with data:', e.dataTransfer?.getData('text/plain'));

    if (e.dataTransfer?.files.length) {
      if (props.entry.type === 'directory') {
        props.onFileUpload(Array.from(e.dataTransfer.files), currentPath);
      }
      return;
    }

    const draggedData = e.dataTransfer!.getData('text/plain');
    if (draggedData && props.entry.type === 'directory') {
      try {
        const draggedPaths = JSON.parse(draggedData) as string[];
        console.log('Moving paths:', draggedPaths, 'to:', currentPath);
        draggedPaths.forEach(sourcePath => {
          if (sourcePath !== currentPath && !currentPath.startsWith(sourcePath + '/')) {
            props.onMoveEntry(sourcePath, currentPath);
          }
        });
      } catch (error) {
        console.log('Fallback: moving single item:', draggedData, 'to:', currentPath);
        if (draggedData !== currentPath && !currentPath.startsWith(draggedData + '/')) {
          props.onMoveEntry(draggedData, currentPath);
        }
      }
    }
  };

  if (props.entry.type === 'directory') {
    const [isOpen, setIsOpen] = createSignal(true);

    const handleDirectoryClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      handleChevronClick(e);
      if (target.classList.contains('codicon-chevron-down') || target.classList.contains('codicon-chevron-right')) {
        e.stopPropagation();
        setIsOpen(!isOpen());
      } else {
        handleClick(e);
      }
    };

    const handleChevronClick = (e: MouseEvent) => {
      e.stopPropagation();
      setIsOpen(!isOpen());
    };

    return (
      <div 
        class="directory"
        classList={{ 
          'drag-over': isDragOver(),
          'dragging': isDragging(),
          'selected': isSelected()
        }}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <div 
          class="directory-header" 
          onClick={handleDirectoryClick}
          draggable="true" 
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
        >
          <i 
            class={`codicon codicon-chevron-${isOpen() ? 'down' : 'right'}`}
            onClick={handleChevronClick}
          ></i>
          <i class="codicon codicon-folder"></i>
          <span>{props.entry.name}</span>
        </div>
        <Show when={isOpen()}>
            <div class="directory-children">
            <For each={props.entry.children}>
                {(child) => (
                  <FileEntry 
                    entry={child} 
                    path={currentPath} 
                    onFileSelect={props.onFileSelect} 
                    getFilePath={props.getFilePath} 
                    activeFile={props.activeFile} 
                    onMoveEntry={props.onMoveEntry}
                    selectedFiles={props.selectedFiles}
                    onSelectionChange={props.onSelectionChange}
                    onFileUpload={props.onFileUpload}
                  />
                )}
            </For>
            </div>
        </Show>
      </div>
    );
  }

  return (
    <div 
      class="file" 
      draggable="true"
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      classList={{ 
        active: props.activeFile !== null && props.getFilePath(props.activeFile) === currentPath,
        selected: isSelected(),
        dragging: isDragging()
      }}
      onClick={handleClick}
    >
      <i class="codicon codicon-file"></i>
      <span>{props.entry.name}</span>
    </div>
  );
};

const FileExplorer = (props: FileExplorerProps) => {
  const [creating, setCreating] = createSignal<{ type: 'file' | 'folder' } | null>(null);
  let inputRef: HTMLInputElement | undefined;
  const [isDragOver, setIsDragOver] = createSignal(false);

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
      // Only close input if no error
      if (!props.creationError) {
        setCreating(null);
        props.setCreationError && props.setCreationError(null); // Reset error on successful commit
      }
    } else {
      setCreating(null);
      props.setCreationError && props.setCreationError(null); // Reset error on cancel
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleCreateCommit();
    } else if (e.key === 'Escape') {
      setCreating(null);
      props.setCreationError && props.setCreationError(null); // Reset error on cancel
    }
  };

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    // Show copy cursor for files, move for internal drags
    if (e.dataTransfer?.files.length) {
      e.dataTransfer!.dropEffect = 'copy';
    } else {
      e.dataTransfer!.dropEffect = 'move';
    }
    setIsDragOver(true);
  };

  const handleDragLeave = () => {
    setIsDragOver(false);
  };

  const handleDrop = (e: DragEvent) => {
    e.preventDefault();
    setIsDragOver(false);

    console.log('Drop event on root explorer with data:', e.dataTransfer?.getData('text/plain'));

    // Handle files from local machine
    if (e.dataTransfer?.files.length) {
      props.onFileUpload(Array.from(e.dataTransfer.files), ''); // Empty path for root
      return;
    }

    // Handle internal moves to root
    const draggedData = e.dataTransfer!.getData('text/plain');
    if (draggedData) {
      try {
        const draggedPaths = JSON.parse(draggedData) as string[];
        console.log('Moving paths to root:', draggedPaths);
        draggedPaths.forEach(sourcePath => {
          // Move to root (empty string destination)
          console.log('Moving:', sourcePath, 'to root');
          props.onMoveEntry(sourcePath, '');
        });
      } catch (error) {
        console.log(error);
        // Fallback for plain text (single item)
        console.log('Fallback: moving single item to root:', draggedData);
        props.onMoveEntry(draggedData, '');
      }
    }
  };

  return (
    <div 
      class="file-explorer"
      classList={{ 'drag-over': isDragOver() }}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
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
          {(entry) => (
            <FileEntry 
              entry={entry} 
              path="" 
              onFileSelect={props.onFileSelect} 
              activeFile={props.activeFile} 
              getFilePath={props.getFilePath} 
              onMoveEntry={props.onMoveEntry}
              selectedFiles={props.selectedFiles}
              onSelectionChange={props.onSelectionChange}
              onFileUpload={props.onFileUpload}
            />
          )}
        </For>
        <Show when={creating()}>
          <div class="file-entry-input-wrapper">
            <div class="file-entry-input">
              <i class={`codicon codicon-${creating()!.type === 'file' ? 'file' : 'folder'}`}></i>
              <input
                ref={inputRef}
                type="text"
                onKeyDown={handleKeyDown}
                onBlur={handleCreateCommit}
                onInput={() => props.setCreationError && props.setCreationError(null)}
              />
            </div>
            <Show when={props.creationError}>
              <div class="file-entry-error">{props.creationError}</div>
            </Show>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default FileExplorer;