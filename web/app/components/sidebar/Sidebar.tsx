/** @jsxImportSource solid-js */
import { createSignal, Show } from 'solid-js';
import FileExplorer from '@components/fileexplorer/FileExplorer';
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import PackageManagerView from './PackageManagerView';
import HelpView from './HelpView';
import './Sidebar.css';

export type SidebarView = 'explorer' | 'packages' | 'help';

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
  onLoadExample?: () => void;
}

export default function Sidebar(props: SidebarProps) {
  const [activeView, setActiveView] = createSignal<SidebarView>('explorer');

  return (
    <div class="sidebar">
      <div class="sidebar-tabs">
        <button 
          class={`sidebar-tab ${activeView() === 'explorer' ? 'active' : ''}`}
          onClick={() => setActiveView('explorer')}
          title="File Explorer (Ctrl+Shift+E)"
        >
          <i class="codicon codicon-files"></i>
        </button>
        <button 
          class={`sidebar-tab ${activeView() === 'packages' ? 'active' : ''}`}
          onClick={() => setActiveView('packages')}
          title="Package Manager (Ctrl+Shift+P)"
        >
          <i class="codicon codicon-package"></i>
        </button>
        <button
          class={`sidebar-tab ${activeView() === 'help' ? 'active' : ''}`}
          onClick={() => setActiveView('help')}
          title="Help & Resources"
        >
          <i class="codicon codicon-question"></i>
        </button>
      </div>
      
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
    </div>
  );
}
