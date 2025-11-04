/** @jsxImportSource solid-js */
import { createSignal, Show, createEffect, onCleanup } from 'solid-js';
import FileExplorer from '@components/fileexplorer/FileExplorer';
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import PackageManagerView from './PackageManagerView';
import HelpView from './HelpView';
import SearchView from '@components/search/SearchView';
import type { SearchResult } from '@components/search/SearchView';
import { loadFileContent } from '@utils/storage';
import './Sidebar.css';

export type SidebarView = 'explorer' | 'search' | 'packages' | 'help';

interface SidebarProps {
  // File Explorer props
  files: FileSystemEntry[];
  onFileSelect: (path: string, isMultiSelect?: boolean) => void;
  onNewFile: (name: string, targetPath?: string) => void;
  onNewFolder: (name: string, targetPath?: string) => void;
  onMoveEntry: (sourcePath: string, destPath: string) => void;
  onFileUpload: (files: File[], destPath: string) => void;
  onRenameEntry: (oldPath: string, newName: string) => void;
  onDeleteEntry: (path: string) => void;
  onCopyEntry: (sourcePath: string, destPath: string, newName: string) => void;
  getFilePath: (entry: FileSystemEntry) => string;
  activeFile: FileSystemEntry | null;
  selectedFiles: string[];
  onSelectionChange: (selected: string[]) => void;
  creationError?: string | null;
  setCreationError?: (msg: string | null) => void;
  checkNameConflict?: (destPath: string, movingName: string) => boolean;
  showConfirmDialog?: (sourcePaths: string[], destPath: string, conflictingName: string) => void;
  showDeleteConfirmDialog?: (paths: string[]) => void;
  globalFileCreation?: { type: 'file' | 'folder', parentPath: string } | null;
  setGlobalFileCreation?: (creation: { type: 'file' | 'folder', parentPath: string } | null) => void;
  
  // Package Manager props
  setFileSystem: (fs: FileSystemEntry[]) => void;
  
  // Help View props
  onLoadExample?: (content: string, fileName: string) => void;
  
  // Sidebar control
  activeView?: SidebarView;
  onViewChange?: (view: SidebarView) => void;
  isCollapsed?: boolean;
  onToggleCollapse?: () => void;
}

export default function Sidebar(props: SidebarProps) {
  const [internalActiveView, setInternalActiveView] = createSignal<SidebarView>('explorer');
  
  // Use controlled or uncontrolled mode
  const activeView = () => props.activeView !== undefined ? props.activeView : internalActiveView();
  const setActiveView = (view: SidebarView) => {
    if (props.onViewChange) {
      props.onViewChange(view);
    } else {
      setInternalActiveView(view);
    }
  };

  const isMac = (() => {
    const userAgentData = (navigator as any).userAgentData;
    return userAgentData?.platform?.toLowerCase().includes('mac') || 
           navigator.userAgent.toLowerCase().includes('mac');
  })();

  const getModifierKey = () => isMac ? 'Cmd' : 'Ctrl';

  // Handle tab clicks with collapse logic
  const handleTabClick = (view: SidebarView) => {
    if (props.isCollapsed && props.onToggleCollapse) {
      // When collapsed, any tab click expands and sets that view
      props.onToggleCollapse();
      setActiveView(view);
    } else if (activeView() === view && props.onToggleCollapse) {
      // Clicking on active tab when expanded - collapse
      props.onToggleCollapse();
    } else {
      // Normal tab switching when expanded
      setActiveView(view);
    }
  };

  // Handle keyboard shortcuts for sidebar tabs
  createEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Use Cmd on macOS, Ctrl on other platforms
      const modifier = isMac ? e.metaKey : e.ctrlKey;
      
      // Check for Cmd+Shift (macOS) or Ctrl+Shift (other platforms) combinations
      if (modifier && e.shiftKey) {
        switch (e.key.toLowerCase()) {
          case 'e':
            e.preventDefault();
            handleTabClick('explorer');
            break;
          case 'f':
            e.preventDefault();
            handleTabClick('search');
            break;
          case 'p':
            e.preventDefault();
            handleTabClick('packages');
            break;
          case 'i':
            e.preventDefault();
            handleTabClick('help');
            break;
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    
    onCleanup(() => {
      document.removeEventListener('keydown', handleKeyDown);
    });
  });

  // Create search results for files with content
  const fileSearchResults = (): SearchResult[] => {
    const results: SearchResult[] = [];
    
    const addFileToResults = (entry: FileSystemEntry, path: string = '') => {
      const fullPath = path ? `${path}/${entry.name}` : entry.name;
      
      if (entry.type === 'file') {
        // Load file content from localStorage for search
        const fileContent = loadFileContent(fullPath);
        
        results.push({
          id: entry.id,
          title: entry.name,
          subtitle: path || 'Root',
          description: `File: ${fullPath}`, // Base description without content preview
          content: fileContent, // Include content for search
          path: fullPath,
          icon: entry.name.endsWith('.alt') ? 'file-code' : 'file',
          onClick: () => props.onFileSelect(fullPath)
        });
      }
      
      if (entry.children) {
        entry.children.forEach(child => addFileToResults(child, fullPath));
      }
    };
    
    props.files.forEach(entry => addFileToResults(entry));
    return results;
  };

  return (
    <div class={`sidebar ${props.isCollapsed ? 'collapsed' : ''}`}>
      <div class="sidebar-tabs">
        <button 
          class={`sidebar-tab ${activeView() === 'explorer' ? 'active' : ''}`}
          onClick={() => handleTabClick('explorer')}
          title={`File Explorer (${getModifierKey()}+Shift+E)`}
        >
          <i class="codicon codicon-files"></i>
        </button>
        <button 
          class={`sidebar-tab ${activeView() === 'search' ? 'active' : ''}`}
          onClick={() => handleTabClick('search')}
          title={`Search (${getModifierKey()}+Shift+F)`}
        >
          <i class="codicon codicon-search"></i>
        </button>
        <button 
          class={`sidebar-tab ${activeView() === 'packages' ? 'active' : ''}`}
          onClick={() => handleTabClick('packages')}
          title={`Package Manager (${getModifierKey()}+Shift+P)`}
        >
          <i class="codicon codicon-package"></i>
        </button>
        <button
          class={`sidebar-tab ${activeView() === 'help' ? 'active' : ''}`}
          onClick={() => handleTabClick('help')}
          title={`Help & Resources (${getModifierKey()}+Shift+I)`}
        >
          <i class="codicon codicon-question"></i>
        </button>
      </div>
      
      <Show when={!props.isCollapsed}>
        <div class="sidebar-content">
          <Show when={activeView() === 'explorer'}>
            <FileExplorer 
              files={props.files}
              onFileSelect={props.onFileSelect}
              onNewFile={props.onNewFile}
              onNewFolder={props.onNewFolder}
              onMoveEntry={props.onMoveEntry}
              onRenameEntry={props.onRenameEntry}
              onDeleteEntry={props.onDeleteEntry}
              onCopyEntry={props.onCopyEntry}
              onFileUpload={props.onFileUpload}
              activeFile={props.activeFile}
              getFilePath={props.getFilePath}
              selectedFiles={props.selectedFiles}
              onSelectionChange={props.onSelectionChange}
              creationError={props.creationError}
              setCreationError={props.setCreationError}
              checkNameConflict={props.checkNameConflict}
              showConfirmDialog={props.showConfirmDialog}
              showDeleteConfirmDialog={props.showDeleteConfirmDialog}
              globalFileCreation={props.globalFileCreation}
              setGlobalFileCreation={props.setGlobalFileCreation}
            />
          </Show>
          
          <Show when={activeView() === 'search'}>
            <SearchView 
              placeholder="Search files..."
              items={fileSearchResults()}
              searchFields={['title', 'subtitle', 'description', 'content', 'path']}
              emptyMessage="No files to search"
              noResultsMessage="No files match your search"
              showAllByDefault={false}
            />
          </Show>
          
          <Show when={activeView() === 'packages'}>
            <PackageManagerView
              fileSystem={props.files}
              setFileSystem={props.setFileSystem}
            />
          </Show>

          <Show when={activeView() === 'help'}>
            <HelpView onLoadExample={props.onLoadExample} />
          </Show>
        </div>
      </Show>
    </div>
  );
}
