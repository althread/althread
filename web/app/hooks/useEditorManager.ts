import { createSignal } from 'solid-js';
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import { findFileByPath, getPathFromId } from '@utils/fileSystemUtils';
import { loadFileContent, saveFileContent } from '@utils/storage';

export const createEditorManager = (editor: any) => {
  const [openFiles, setOpenFiles] = createSignal<FileSystemEntry[]>([]);
  const [activeFile, setActiveFile] = createSignal<FileSystemEntry | null>(null);

  const handleFileSelect = (path: string, mockFileSystem: FileSystemEntry[]) => {
    console.log("File selected:", path);
    
    const file = findFileByPath(mockFileSystem, path);
    if (file) {
      // Check if file is already open using ID instead of path
      const isAlreadyOpen = openFiles().some(f => f.id === file.id);
      if (!isAlreadyOpen) {
        setOpenFiles([...openFiles(), file]);
      }
      setActiveFile(file);
      
      // Check if file is in deps directory (read-only)
      const isInDeps = path === 'deps' || path.startsWith('deps/');
      
      // Use safe content update
      if (editor && editor.safeUpdateContent) {
        const content = loadFileContent(path);
        editor.safeUpdateContent(content);
        
        // Set read-only mode for deps files
        if (editor.setReadOnly) {
          editor.setReadOnly(isInDeps);
        }
        
        // Then update language (after content is loaded)
        setTimeout(() => {
          if (editor && editor.updateLanguage) {
            editor.updateLanguage(file.name);
          }
        }, 10);
      }
    }
  };

  const handleFileTabClick = (file: FileSystemEntry, mockFileSystem: FileSystemEntry[]) => {
    setActiveFile(file);
    
    // Use safe content update
    if (editor && editor.safeUpdateContent) {
      const filePath = getPathFromId(mockFileSystem, file.id) || file.name;
      const content = loadFileContent(filePath);
      editor.safeUpdateContent(content);
      
      // Check if file is in deps directory (read-only)
      const isInDeps = filePath === 'deps' || filePath.startsWith('deps/');
      
      // Set read-only mode for deps files
      if (editor.setReadOnly) {
        editor.setReadOnly(isInDeps);
      }
      
      // Then update language (after content is loaded)
      setTimeout(() => {
        if (editor && editor.updateLanguage) {
          editor.updateLanguage(file.name);
        }
      }, 10);
    }
  };

  const handleTabClose = (file: FileSystemEntry, mockFileSystem: FileSystemEntry[]) => {
    // Use ID-based filtering instead of path-based
    const newOpenFiles = openFiles().filter(f => f.id !== file.id);
    setOpenFiles(newOpenFiles);
    
    // If we closed the active file, switch to another open file or null
    if (activeFile() && activeFile()!.id === file.id) {
      const newActiveFile = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null;
      setActiveFile(newActiveFile);
      
      if (newActiveFile && editor && editor.safeUpdateContent) {
        // Load the new active file's content
        const newFilePath = getPathFromId(mockFileSystem, newActiveFile.id) || newActiveFile.name;
        const content = loadFileContent(newFilePath);
        editor.safeUpdateContent(content);
        
        // Update language
        setTimeout(() => {
          if (editor && editor.updateLanguage) {
            editor.updateLanguage(newActiveFile.name);
          }
        }, 10);
      }
    }
  };

  const createNewFileWithContent = (fileName: string, content: string, onFileOperations: any, mockFileSystem?: () => FileSystemEntry[]) => {
    // Auto-add .alt extension if not provided
    if (!fileName.includes('.')) {
      fileName = fileName + '.alt';
    }
    
    // Create the file using file operations
    onFileOperations.handleNewFile(fileName);
    
    // Wait a bit for the file to be created and opened, then update content
    setTimeout(() => {
      if (editor && editor.safeUpdateContent) {
        editor.safeUpdateContent(content);
        
        // Also save the content to localStorage immediately after a small delay
        // to ensure the file system has been updated
        setTimeout(() => {
          const currentFile = activeFile();
          if (currentFile && mockFileSystem) {
            // Get the proper file path using the file system
            const filePath = getPathFromId(mockFileSystem(), currentFile.id) || currentFile.name;
            saveFileContent(filePath, content);
          } else {
            // Fallback to just the filename (for root directory files)
            saveFileContent(fileName, content);
          }
        }, 10);
      }
    }, 50);
  };

  return {
    openFiles,
    setOpenFiles,
    activeFile,
    setActiveFile,
    handleFileSelect,
    handleFileTabClick,
    handleTabClose,
    createNewFileWithContent
  };
};
