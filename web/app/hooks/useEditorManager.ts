import { createSignal } from 'solid-js';
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import { findFileByPath, getPathFromId } from '@utils/fileSystemUtils';
import { loadFileContent } from '@utils/storage';

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
      
      // Use safe content update
      if (editor && editor.safeUpdateContent) {
        const content = loadFileContent(path);
        editor.safeUpdateContent(content);
        
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

  return {
    openFiles,
    setOpenFiles,
    activeFile,
    setActiveFile,
    handleFileSelect,
    handleFileTabClick,
    handleTabClose
  };
};
