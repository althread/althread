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
  onNewFile: (name: string, targetPath?: string) => void;
  onNewFolder: (name: string, targetPath?: string) => void;
  onMoveEntry: (sourcePath: string, destPath: string) => void;
  onFileUpload: (files: File[], destPath: string) => void;
  onRenameEntry: (oldPath: string, newName: string) => void;
  onDeleteEntry: (path: string) => void;
  getFilePath: (entry: FileSystemEntry) => string;
  activeFile: FileSystemEntry | null;
  selectedFiles: string[];
  onSelectionChange: (selected: string[]) => void;
  creationError?: string | null;
  setCreationError?: (msg: string | null) => void;
  checkNameConflict?: (destPath: string, movingName: string) => boolean;
  showConfirmDialog?: (sourcePaths: string[], destPath: string, conflictingName: string) => void;
  showDeleteConfirmDialog?: (paths: string[]) => void;
};

const FileEntry = (props: { 
  entry: FileSystemEntry; 
  path: string; 
  onFileSelect: (path: string, isMultiSelect?: boolean) => void; 
  getFilePath: (entry: FileSystemEntry) => string; 
  activeFile: FileSystemEntry | null; 
  onMoveEntry: (source: string, dest: string) => void;
  onRenameEntry: (oldPath: string, newName: string) => void;
  onDeleteEntry: (path: string) => void;
  selectedFiles: string[];
  onSelectionChange: (selected: string[]) => void;
  onFileUpload: (files: File[], destPath: string) => void;
  checkNameConflict: (destPath: string, movingName: string) => boolean;
  showConfirmDialog: (sourcePaths: string[], destPath: string, conflictingName: string) => void;
  showDeleteConfirmDialog: (paths: string[]) => void;
  allVisibleFiles: string[];
  creating: { type: 'file' | 'folder', parentPath: string } | null;
  onCreateCommit: (name: string) => void;
  onCreateCancel: () => void;
  creationError: string | null;
  setCreationError?: (msg: string | null) => void;
  createCommitInProgress?: () => boolean;
  // New props for edit state management
  currentlyRenaming: string | null;
  startRename: (path: string) => void;
}) => {
  const currentPath = props.path ? `${props.path}/${props.entry.name}` : props.entry.name;
  const [isDragOver, setIsDragOver] = createSignal(false);
  const [isDragging, setIsDragging] = createSignal(false);
  const [showContextMenu, setShowContextMenu] = createSignal(false);
  const [contextMenuPos, setContextMenuPos] = createSignal({ x: 0, y: 0 });
  const [renameError, setRenameError] = createSignal<string | null>(null);

  // Check if this entry is currently being renamed
  const isRenaming = () => props.currentlyRenaming === currentPath;

  // Clear rename error when this entry is no longer being renamed
  createEffect(() => {
    if (!isRenaming() && renameError()) {
      setRenameError(null);
    }
  });

  const isSelected = () => props.selectedFiles.includes(currentPath);

  const handleClick = (e: MouseEvent) => {
    // Handle multi-selection with Cmd/Ctrl and Shift
    if (e.metaKey || e.ctrlKey) {
      // Cmd/Ctrl click: toggle selection
      const newSelection = isSelected() 
        ? props.selectedFiles.filter(path => path !== currentPath)
        : [...props.selectedFiles, currentPath];
      props.onSelectionChange(newSelection);
      
      // Only call onFileSelect if this item is now selected
      if (!isSelected()) {
        props.onFileSelect(currentPath);
      }
    } else if (e.shiftKey && props.selectedFiles.length > 0) {
      // Shift click: range selection
      const lastSelected = props.selectedFiles[props.selectedFiles.length - 1];
      const rangeSelection = getFileRange(lastSelected, currentPath);
      props.onSelectionChange(rangeSelection);
      props.onFileSelect(currentPath);
    } else {
      // Normal click: single selection
      props.onSelectionChange([currentPath]);
      props.onFileSelect(currentPath);
    }
  };

  const handleRightClick = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    
    // Select the item if it's not already selected
    if (!isSelected()) {
      props.onSelectionChange([currentPath]);
    }
    
    // Show context menu
    setContextMenuPos({ x: e.clientX, y: e.clientY });
    setShowContextMenu(true);
  };

  const handleRename = () => {
    setShowContextMenu(false);
    props.startRename(currentPath);
  };

  const handleDelete = () => {
    setShowContextMenu(false);
    if (props.selectedFiles.length > 1) {
      // Multiple files selected
      props.showDeleteConfirmDialog(props.selectedFiles);
    } else {
      // Single file
      props.showDeleteConfirmDialog([currentPath]);
    }
  };

  const handleRenameCommit = (newName: string) => {
    newName = newName.trim();
    if (newName && newName !== props.entry.name) {
      // Check for name conflicts first
      const pathParts = currentPath.split('/');
      pathParts.pop(); // Remove current name
      const parentPath = pathParts.join('/');
      
      if (props.checkNameConflict && props.checkNameConflict(parentPath, newName)) {
        setRenameError("A file or folder with this name already exists.");
        return;
      }
      
      setRenameError(null);
      props.onRenameEntry(currentPath, newName);
    }
    // Clear the rename state
    props.startRename(''); // Empty string clears the rename state
  };

  const handleRenameCancel = () => {
    props.startRename(''); // Empty string clears the rename state
    setRenameError(null);
  };

  // Close context menu when clicking elsewhere
  createEffect(() => {
    const handleGlobalClick = () => setShowContextMenu(false);
    if (showContextMenu()) {
      document.addEventListener('click', handleGlobalClick);
      return () => document.removeEventListener('click', handleGlobalClick);
    }
    return undefined;
  });

  // Helper function to get range of files between two paths
  const getFileRange = (startPath: string, endPath: string): string[] => {
    // This is a simplified range selection - in a real implementation,
    // you'd want to traverse the file tree in display order
    const allFiles = getAllVisibleFiles();
    const startIndex = allFiles.indexOf(startPath);
    const endIndex = allFiles.indexOf(endPath);
    
    if (startIndex === -1 || endIndex === -1) {
      return [endPath]; // Fallback to single selection
    }
    
    const minIndex = Math.min(startIndex, endIndex);
    const maxIndex = Math.max(startIndex, endIndex);
    return allFiles.slice(minIndex, maxIndex + 1);
  };

  // Helper function to get all visible file paths in display order
  const getAllVisibleFiles = (): string[] => {
    return props.allVisibleFiles;
  };

  const handleDragStart = (e: DragEvent) => {
    e.dataTransfer!.effectAllowed = 'move';
    setIsDragging(true);

    if (isSelected() && props.selectedFiles.length > 1) {
      // If this file is part of a multi-selection, drag all selected files
      e.dataTransfer!.setData('text/plain', JSON.stringify(props.selectedFiles));
    } else {
      // Single file drag (either not selected or only one file selected)
      e.dataTransfer!.setData('text/plain', JSON.stringify([currentPath]));
      // Update selection to this single file
      props.onSelectionChange([currentPath]);
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
        
        // Check for conflicts
        for (const sourcePath of draggedPaths) {
          // Skip if moving to the same destination (no actual move needed)
          const sourceParent = sourcePath.split('/').slice(0, -1).join('/');
          if (sourceParent === currentPath) {
            console.log('Skipping move - already in destination:', sourcePath);
            continue;
          }
          
          const movingName = sourcePath.split('/').pop()!;
          if (props.checkNameConflict && props.checkNameConflict(currentPath, movingName)) {
            // Show confirmation dialog
            if (props.showConfirmDialog) {
              props.showConfirmDialog(draggedPaths, currentPath, movingName);
            }
            return;
          }
        }
        
        // No conflicts, proceed with move (only for items that actually need to move)
        draggedPaths.forEach(sourcePath => {
          const sourceParent = sourcePath.split('/').slice(0, -1).join('/');
          if (sourceParent !== currentPath) {
            props.onMoveEntry(sourcePath, currentPath);
          }
        });
      } catch (error) {
        console.log('Fallback: moving single item:', draggedData, 'to:', currentPath);
        
        // Skip if moving to the same destination
        const sourceParent = draggedData.split('/').slice(0, -1).join('/');
        if (sourceParent === currentPath) {
          console.log('Skipping move - already in destination:', draggedData);
          return;
        }
        
        const movingName = draggedData.split('/').pop()!;
        if (props.checkNameConflict && props.checkNameConflict(currentPath, movingName)) {
          // Show confirmation dialog
          if (props.showConfirmDialog) {
            props.showConfirmDialog([draggedData], currentPath, movingName);
          }
          return;
        }
        props.onMoveEntry(draggedData, currentPath);
      }
    }
  };

  if (props.entry.type === 'directory') {
    const [isOpen, setIsOpen] = createSignal(true);

    // Auto-expand when creating inside this directory
    createEffect(() => {
      if (props.creating && props.creating.parentPath === currentPath) {
        setIsOpen(true);
      }
    });

    // Helper function to sort files and folders alphabetically (same as main component)
    const sortEntries = (entries: FileSystemEntry[]): FileSystemEntry[] => {
      return [...entries].sort((a, b) => {
        // Directories first, then files
        if (a.type !== b.type) {
          return a.type === 'directory' ? -1 : 1;
        }
        // Within same type, sort alphabetically (case-insensitive)
        return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
      });
    };

    const handleDirectoryClick = (e: MouseEvent) => {
      // Always toggle the folder open/closed state when clicking anywhere on the directory row
      setIsOpen(!isOpen());
      // Also handle selection
      handleClick(e);
    };

    return (
      <div 
        class="directory"
        classList={{ 
          'drag-over': isDragOver(),
          'dragging': isDragging(),
          'selected': isSelected() && !isRenaming()
        }}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <div 
          class="directory-header" 
          onClick={handleDirectoryClick}
          onContextMenu={handleRightClick}
          draggable="true" 
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
        >
          <i 
            class={`codicon codicon-chevron-${isOpen() ? 'down' : 'right'}`}
          ></i>
          <i class="codicon codicon-folder"></i>
          <Show when={isRenaming()} fallback={<span>{props.entry.name}</span>}>
            <div class="file-entry-input-wrapper">
              <div class="file-entry-rename-input">
                <i class="codicon codicon-folder"></i>
                <input
                  class="rename-input"
                  type="text"
                  value={props.entry.name}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      handleRenameCommit(e.currentTarget.value);
                    } else if (e.key === 'Escape') {
                      e.preventDefault();
                      handleRenameCancel();
                    }
                  }}
                  onBlur={(e) => handleRenameCommit(e.currentTarget.value)}
                  ref={(el) => {
                    if (el) {
                      setTimeout(() => {
                        el.focus();
                        el.select();
                      }, 10);
                    }
                  }}
                />
              </div>
              <Show when={renameError()}>
                <div class="file-entry-error">{renameError()}</div>
              </Show>
            </div>
          </Show>
        </div>
        <Show when={isOpen()}>
            <div class="directory-children">
            <For each={sortEntries(props.entry.children || [])}>
                {(child) => (
                  <FileEntry 
                    entry={child} 
                    path={currentPath} 
                    onFileSelect={props.onFileSelect} 
                    getFilePath={props.getFilePath} 
                    activeFile={props.activeFile} 
                    onMoveEntry={props.onMoveEntry}
                    onRenameEntry={props.onRenameEntry}
                    onDeleteEntry={props.onDeleteEntry}
                    selectedFiles={props.selectedFiles}
                    onSelectionChange={props.onSelectionChange}
                    onFileUpload={props.onFileUpload}
                    checkNameConflict={props.checkNameConflict}
                    showConfirmDialog={props.showConfirmDialog}
                    showDeleteConfirmDialog={props.showDeleteConfirmDialog}
                    allVisibleFiles={props.allVisibleFiles}
                    creating={props.creating}
                    onCreateCommit={props.onCreateCommit}
                    onCreateCancel={props.onCreateCancel}
                    creationError={props.creationError}
                    setCreationError={props.setCreationError}
                    createCommitInProgress={props.createCommitInProgress}
                    currentlyRenaming={props.currentlyRenaming}
                    startRename={props.startRename}
                  />
                )}
            </For>
            {/* Show creation input if this directory is the target */}
            <Show when={props.creating && props.creating.parentPath === currentPath}>
              <div class="file-entry-input-wrapper">
                <div class="file-entry-input">
                  <i class={`codicon codicon-${props.creating!.type === 'file' ? 'file' : 'folder'}`}></i>
                  <input
                    ref={(el) => {
                      if (el && props.creating && props.creating.parentPath === currentPath) {
                        // Auto-focus this input when it's created for this directory
                        setTimeout(() => el.focus(), 10);
                      }
                    }}
                    type="text"
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        e.preventDefault();
                        const name = e.currentTarget.value.trim();
                        props.onCreateCommit(name);
                      } else if (e.key === 'Escape') {
                        e.preventDefault();
                        props.onCreateCancel();
                      }
                    }}
                    onBlur={(e) => {
                      // Only commit on blur if not already processing a commit
                      if (!props.createCommitInProgress || !props.createCommitInProgress()) {
                        const name = e.currentTarget.value.trim();
                        props.onCreateCommit(name);
                      }
                    }}
                    onInput={() => {
                      // Clear error when user starts typing
                      if (props.setCreationError) {
                        props.setCreationError(null);
                      }
                    }}
                  />
                </div>
                <Show when={props.creationError}>
                  <div class="file-entry-error">{props.creationError}</div>
                </Show>
              </div>
            </Show>
            </div>
        </Show>
        
        {/* Context Menu */}
        <Show when={showContextMenu()}>
          <div 
            class="context-menu"
            style={{
              position: 'fixed',
              top: `${contextMenuPos().y}px`,
              left: `${contextMenuPos().x}px`,
              'z-index': 1000
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <button onClick={handleRename}>
              <i class="codicon codicon-edit"></i>
              Rename
            </button>
            <button onClick={handleDelete}>
              <i class="codicon codicon-trash"></i>
              Delete
            </button>
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
      onContextMenu={handleRightClick}
      classList={{ 
        active: props.activeFile !== null && props.getFilePath(props.activeFile) === currentPath,
        selected: isSelected() && !isRenaming(),
        dragging: isDragging()
      }}
      onClick={handleClick}
    >
      <Show when={isRenaming()} fallback={
        <>
          <i class="codicon codicon-file"></i>
          <span>{props.entry.name}</span>
        </>
      }>
        <div class="file-entry-input-wrapper">
          <div class="file-entry-rename-input">
            <i class="codicon codicon-file"></i>
            <input
              class="rename-input"
              type="text"
              value={props.entry.name}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  handleRenameCommit(e.currentTarget.value);
                } else if (e.key === 'Escape') {
                  e.preventDefault();
                  handleRenameCancel();
                }
              }}
              onBlur={(e) => handleRenameCommit(e.currentTarget.value)}
              ref={(el) => {
                if (el) {
                  setTimeout(() => {
                    el.focus();
                    el.select();
                  }, 10);
                }
              }}
            />
          </div>
          <Show when={renameError()}>
            <div class="file-entry-error">{renameError()}</div>
          </Show>
        </div>
      </Show>
      
      {/* Context Menu */}
      <Show when={showContextMenu()}>
        <div 
          class="context-menu"
          style={{
            position: 'fixed',
            top: `${contextMenuPos().y}px`,
            left: `${contextMenuPos().x}px`,
            'z-index': 1000
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <button onClick={handleRename}>
            <i class="codicon codicon-edit"></i>
            Rename
          </button>
          <button onClick={handleDelete}>
            <i class="codicon codicon-trash"></i>
            Delete
          </button>
        </div>
      </Show>
    </div>
  );
};

const FileExplorer = (props: FileExplorerProps) => {
  const [creating, setCreating] = createSignal<{ type: 'file' | 'folder', parentPath: string } | null>(null);
  const [isDragOver, setIsDragOver] = createSignal(false);
  
  // Global edit state management - only one edit operation allowed at a time
  const [currentlyRenaming, setCurrentlyRenaming] = createSignal<string | null>(null);

  // Function to cancel all current edit operations
  const cancelAllEdits = () => {
    setCreating(null);
    setCurrentlyRenaming(null);
    props.setCreationError && props.setCreationError(null);
  };

  // Function to start a rename operation (cancels any existing edits)
  const startRename = (path: string) => {
    if (path === '') {
      // Empty string means cancel rename
      setCurrentlyRenaming(null);
    } else {
      cancelAllEdits();
      setCurrentlyRenaming(path);
    }
  };

  // Function to start creation (cancels any existing edits)
  const startCreation = (type: 'file' | 'folder', parentPath: string) => {
    cancelAllEdits();
    setCreating({ type, parentPath });
  };

  // Helper function to sort files and folders alphabetically
  const sortEntries = (entries: FileSystemEntry[]): FileSystemEntry[] => {
    return [...entries].sort((a, b) => {
      // Directories first, then files
      if (a.type !== b.type) {
        return a.type === 'directory' ? -1 : 1;
      }
      // Within same type, sort alphabetically (case-insensitive)
      return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
    });
  };

  // Generate list of all visible files in display order for range selection
  const getAllVisibleFiles = (): string[] => {
    const files: string[] = [];
    
    const addFiles = (entries: FileSystemEntry[], currentPath: string = '') => {
      const sorted = sortEntries(entries);
      for (const entry of sorted) {
        const entryPath = currentPath ? `${currentPath}/${entry.name}` : entry.name;
        files.push(entryPath);
        if (entry.type === 'directory' && entry.children) {
          // For now, assume all directories are open for simplicity
          // In a more sophisticated implementation, you'd track open/closed state
          addFiles(entry.children, entryPath);
        }
      }
    };
    
    addFiles(props.files);
    return files;
  };

  // Handle keyboard shortcuts
  createEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Handle escape key to cancel edits and clear selection
      if (e.key === 'Escape') {
        if (creating() || currentlyRenaming()) {
          // Cancel any active edit operations
          cancelAllEdits();
        } else {
          // Clear selection if no edits are active
          props.onSelectionChange([]);
        }
        return;
      }
      
      // Only handle other shortcuts when not creating a new file/folder or renaming
      if (creating() || currentlyRenaming()) return;
      
      // Cmd/Ctrl + A: Select all files
      if ((e.metaKey || e.ctrlKey) && e.key === 'a') {
        e.preventDefault();
        const allFiles = getAllVisibleFiles();
        props.onSelectionChange(allFiles);
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    
    // Cleanup
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  });

  // Helper function to get create button title based on selection
  const getCreateButtonTitle = (type: 'file' | 'folder'): string => {
    const baseTitle = type === 'file' ? 'New File' : 'New Folder';
    
    // If there's a single selected item, show where it will be created
    if (props.selectedFiles.length === 1) {
      const selectedPath = props.selectedFiles[0];
      const selectedEntry = findEntryByPath(props.files, selectedPath);
      if (selectedEntry && selectedEntry.type === 'directory') {
        return `${baseTitle} in ${selectedPath}`;
      } else if (selectedEntry && selectedEntry.type === 'file') {
        const pathParts = selectedPath.split('/');
        if (pathParts.length > 1) {
          pathParts.pop(); // Remove filename
          const parentPath = pathParts.join('/');
          return `${baseTitle} in ${parentPath}`;
        } else {
          return `${baseTitle} in root (same level as ${selectedEntry.name})`;
        }
      }
    }
    
    return `${baseTitle} in root`;
  };

  const [createCommitInProgress, setCreateCommitInProgress] = createSignal(false);

  const handleCreateCommit = (name: string) => {
    if (!creating() || createCommitInProgress()) return;

    name = name.trim();
    if (name) {
      const targetPath = creating()!.parentPath;
      const creationType = creating()!.type;
      
      // Set flag to prevent double commits
      setCreateCommitInProgress(true);
      
      if (creationType === 'file') {
        props.onNewFile(name, targetPath);
      } else {
        props.onNewFolder(name, targetPath);
      }
      
      // Close the input and reset flag after a short delay to allow error processing
      setTimeout(() => {
        if (!props.creationError) {
          cancelAllEdits();
        }
        setCreateCommitInProgress(false);
      }, 50);
    } else {
      cancelAllEdits();
    }
  };

  // Helper function to find an entry by path
  const findEntryByPath = (files: FileSystemEntry[], targetPath: string): FileSystemEntry | null => {
    for (const file of files) {
      if (file.name === targetPath && file.type === 'file') {
        return file;
      }
      if (file.name === targetPath && file.type === 'directory') {
        return file;
      }
      if (file.type === 'directory' && file.children) {
        const pathParts = targetPath.split('/');
        if (pathParts[0] === file.name && pathParts.length > 1) {
          const remainingPath = pathParts.slice(1).join('/');
          const found = findEntryByPath(file.children, remainingPath);
          if (found) return found;
        }
      }
    }
    return null;
  };

  const handleCreateCancel = () => {
    cancelAllEdits();
    setCreateCommitInProgress(false); // Reset the commit flag
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
        
        // Check for conflicts
        for (const sourcePath of draggedPaths) {
          // Skip if already at root (no parent path)
          if (!sourcePath.includes('/')) {
            console.log('Skipping move - already at root:', sourcePath);
            continue;
          }
          
          const movingName = sourcePath.split('/').pop()!;
          if (props.checkNameConflict && props.checkNameConflict('', movingName)) {
            // Show confirmation dialog
            if (props.showConfirmDialog) {
              props.showConfirmDialog(draggedPaths, '', movingName);
            }
            return;
          }
        }
        
        // No conflicts, proceed with move (only for items that actually need to move)
        draggedPaths.forEach(sourcePath => {
          // Only move if not already at root
          if (sourcePath.includes('/')) {
            props.onMoveEntry(sourcePath, '');
          }
        });
      } catch (error) {
        // Fallback for plain text (single item) that might be a path
        // but not a JSON array. This case should ideally not happen with
        // the current drag logic, but it's good practice to handle it.
        console.log('Fallback: moving single item to root:', draggedData);
        
        // Skip if already at root
        if (!draggedData.includes('/')) {
          console.log('Skipping move - already at root:', draggedData);
          return;
        }
        
        const movingName = draggedData.split('/').pop()!;
        if (props.checkNameConflict && props.checkNameConflict('', movingName)) {
          // Show confirmation dialog
          if (props.showConfirmDialog) {
            props.showConfirmDialog([draggedData], '', movingName);
          }
          return;
        }
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
            <button 
              onClick={() => {
                // Determine parent path based on selection
                let parentPath = '';
                if (props.selectedFiles.length === 1) {
                  const selectedPath = props.selectedFiles[0];
                  const selectedEntry = findEntryByPath(props.files, selectedPath);
                  if (selectedEntry && selectedEntry.type === 'directory') {
                    // Selected item is a directory, create inside it
                    parentPath = selectedPath;
                  } else if (selectedEntry && selectedEntry.type === 'file') {
                    // Selected item is a file, create in its parent directory
                    const pathParts = selectedPath.split('/');
                    if (pathParts.length > 1) {
                      // Remove the last part (filename) to get parent directory
                      pathParts.pop();
                      parentPath = pathParts.join('/');
                    }
                    // If pathParts.length === 1, it's a root file, so parentPath stays ''
                  }
                }
                startCreation('file', parentPath);
              }} 
              title={getCreateButtonTitle('file')} 
              disabled={!!creating() || !!currentlyRenaming()}
            >
                <i class="codicon codicon-new-file"></i>
            </button>
            <button 
              onClick={() => {
                // Determine parent path based on selection
                let parentPath = '';
                if (props.selectedFiles.length === 1) {
                  const selectedPath = props.selectedFiles[0];
                  const selectedEntry = findEntryByPath(props.files, selectedPath);
                  if (selectedEntry && selectedEntry.type === 'directory') {
                    // Selected item is a directory, create inside it
                    parentPath = selectedPath;
                  } else if (selectedEntry && selectedEntry.type === 'file') {
                    // Selected item is a file, create in its parent directory
                    const pathParts = selectedPath.split('/');
                    if (pathParts.length > 1) {
                      // Remove the last part (filename) to get parent directory
                      pathParts.pop();
                      parentPath = pathParts.join('/');
                    }
                    // If pathParts.length === 1, it's a root file, so parentPath stays ''
                  }
                }
                startCreation('folder', parentPath);
              }} 
              title={getCreateButtonTitle('folder')} 
              disabled={!!creating() || !!currentlyRenaming()}
            >
                <i class="codicon codicon-new-folder"></i>
            </button>
        </div>
      </div>
      <div 
        class="file-explorer-content"
        onClick={(e) => {
          // Only clear selection if clicking on the content area itself, not on child elements
          if (e.target === e.currentTarget) {
            props.onSelectionChange([]);
          }
        }}
      >
        <For each={sortEntries(props.files)}>
          {(entry) => (
            <FileEntry 
              entry={entry} 
              path="" 
              onFileSelect={props.onFileSelect} 
              activeFile={props.activeFile} 
              getFilePath={props.getFilePath} 
              onMoveEntry={props.onMoveEntry}
              onRenameEntry={props.onRenameEntry}
              onDeleteEntry={props.onDeleteEntry}
              selectedFiles={props.selectedFiles}
              onSelectionChange={props.onSelectionChange}
              onFileUpload={props.onFileUpload}
              checkNameConflict={props.checkNameConflict || (() => false)}
              showConfirmDialog={props.showConfirmDialog || (() => {})}
              showDeleteConfirmDialog={props.showDeleteConfirmDialog || (() => {})}
              allVisibleFiles={getAllVisibleFiles()}
              creating={creating()}
              onCreateCommit={handleCreateCommit}
              onCreateCancel={handleCreateCancel}
              creationError={props.creationError || null}
              setCreationError={props.setCreationError}
              createCommitInProgress={createCommitInProgress}
              currentlyRenaming={currentlyRenaming()}
              startRename={startRename}
            />
          )}
        </For>
        {/* Show creation input at root if no parent path specified */}
        <Show when={creating() && creating()!.parentPath === ''}>
          <div class="file-entry-input-wrapper">
            <div class="file-entry-input">
              <i class={`codicon codicon-${creating()!.type === 'file' ? 'file' : 'folder'}`}></i>
              <input
                ref={(el) => {
                  if (el && creating() && creating()!.parentPath === '') {
                    // Auto-focus this input when it's created at root
                    setTimeout(() => el.focus(), 10);
                  }
                }}
                type="text"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    const name = e.currentTarget.value.trim();
                    handleCreateCommit(name);
                  } else if (e.key === 'Escape') {
                    e.preventDefault();
                    handleCreateCancel();
                  }
                }}
                onBlur={(e) => {
                  // Only commit on blur if not already processing a commit
                  if (!createCommitInProgress()) {
                    const name = e.currentTarget.value.trim();
                    handleCreateCommit(name);
                  }
                }}
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