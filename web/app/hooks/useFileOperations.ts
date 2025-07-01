import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import { STORAGE_KEYS, saveFileSystem, saveFileContent, getDefaultContentForFile } from '@utils/storage';
import { findTargetDirectory, findEntryAndParent, collectDeletedFileIds, findEntryById, getPathFromId } from '@utils/fileSystemUtils';

export const createFileOperationsHandlers = (
  mockFileSystem: () => FileSystemEntry[],
  setMockFileSystem: (fs: FileSystemEntry[]) => void,
  setCreationError: (error: string | null) => void,
  openFiles: () => FileSystemEntry[],
  setOpenFiles: (files: FileSystemEntry[]) => void,
  activeFile: () => FileSystemEntry | null,
  setActiveFile: (file: FileSystemEntry | null) => void,
  selectedFiles: () => string[],
  setSelectedFiles: (files: string[]) => void,
  editor: any,
  loadFileContent: (path: string) => string
) => {
  const handleNewFile = (name: string, targetPath?: string) => {
    const newFile: FileSystemEntry = { id: crypto.randomUUID(), name, type: 'file' };

    // Determine where to create the file
    const createInPath = targetPath || '';
    
    // Deep copy to avoid mutation
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    const targetDir = findTargetDirectory(newFileSystem, createInPath);
    if (!targetDir) {
      setCreationError("Target directory not found.");
      return;
    }

    // Check if file already exists in the target directory
    const existingFile = targetDir.find((f: FileSystemEntry) => f.name === name);
    if (existingFile) {
      setCreationError("A file or folder with this name already exists.");
      return;
    }
    setCreationError(null);

    // Add the new file to the target directory
    targetDir.push(newFile);
    setMockFileSystem(newFileSystem);
    saveFileSystem(newFileSystem);
    
    // Save empty content for new file with full path
    const fullPath = createInPath === '' ? name : `${createInPath}/${name}`;
    const defaultContent = getDefaultContentForFile(name);
    saveFileContent(fullPath, defaultContent);
    
    // Automatically open the new file
    setOpenFiles([...openFiles(), newFile]);
    setActiveFile(newFile);
    
    // Load content using safe method
    if (editor && editor.safeUpdateContent) {
      editor.safeUpdateContent(defaultContent);
      
      // Then update language
      setTimeout(() => {
        if (editor && editor.updateLanguage) {
          editor.updateLanguage(name);
        }
      }, 10);
    }
  };

  const handleNewFolder = (name: string, targetPath?: string) => {
    const newFolder: FileSystemEntry = { id: crypto.randomUUID(), name, type: 'directory', children: [] };

    // Determine where to create the folder
    const createInPath = targetPath || '';
    
    // Deep copy to avoid mutation
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    const targetDir = findTargetDirectory(newFileSystem, createInPath);
    if (!targetDir) {
      setCreationError("Target directory not found.");
      return;
    }

    // Check if folder already exists in the target directory
    const existingFolder = targetDir.find((f: FileSystemEntry) => f.name === name);
    if (existingFolder) {
      setCreationError("A file or folder with this name already exists.");
      return;
    }
    setCreationError(null);

    // Add the new folder to the target directory
    targetDir.push(newFolder);
    setMockFileSystem(newFileSystem);
    saveFileSystem(newFileSystem);
  };

  const moveFileContent = (entry: FileSystemEntry, oldPath: string, newPath: string) => {
    if (entry.type === 'file') {
      const oldKey = STORAGE_KEYS.FILE_CONTENT_PREFIX + oldPath;
      const newKey = STORAGE_KEYS.FILE_CONTENT_PREFIX + newPath;
      const content = localStorage.getItem(oldKey);
      if (content !== null) {
        localStorage.setItem(newKey, content);
        localStorage.removeItem(oldKey);
      }
    } else if (entry.type === 'directory' && entry.children) {
      entry.children.forEach(child => {
        const childOldPath = oldPath + '/' + child.name;
        const childNewPath = newPath + '/' + child.name;
        moveFileContent(child, childOldPath, childNewPath);
      });
    }
  };

  const handleMoveEntry = (sourcePath: string, destPath: string) => {
    console.log(`Moving ${sourcePath} to ${destPath}`);
    
    // Prevent moving into itself
    if (sourcePath === destPath) {
      console.warn('Cannot move into itself');
      return;
    }

    // Prevent moving into a child directory (would create a cycle)
    if (destPath.startsWith(sourcePath + '/')) {
      console.warn('Cannot move into child directory');
      return;
    }
    
    // Deep copy the file system to avoid mutation issues
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    // Find and remove the source entry
    const sourceInfo = findEntryAndParent(newFileSystem, sourcePath);
    if (!sourceInfo) {
      console.error("Source not found:", sourcePath);
      return;
    }

    const [movedEntry] = sourceInfo.parent.splice(sourceInfo.index, 1);

    // Find destination and add the entry
    if (destPath === '') { 
      // Moving to root
      newFileSystem.push(movedEntry);
    } else {
      const destInfo = findEntryAndParent(newFileSystem, destPath);
      if (!destInfo || destInfo.entry.type !== 'directory') {
        console.error("Destination not found or is not a directory:", destPath);
        // Re-add the entry to its original position if dest is invalid
        sourceInfo.parent.splice(sourceInfo.index, 0, movedEntry);
        return;
      }
      
      if (!destInfo.entry.children) {
        destInfo.entry.children = [];
      }
      destInfo.entry.children.push(movedEntry);
    }

    // Only move content if the path actually changed
    const newPath = destPath === '' ? movedEntry.name : `${destPath}/${movedEntry.name}`;
    if (sourcePath !== newPath) {
      moveFileContent(movedEntry, sourcePath, newPath);
    }

    console.log("File system after move:", newFileSystem);
    saveFileSystem(newFileSystem);
    
    // Get IDs of open files and active file before the move
    const openFileIds = openFiles().map(f => f.id);
    const activeFileId = activeFile()?.id;

    // Update the file system state
    setMockFileSystem(newFileSystem);

    // Re-find open files in the new file system using their IDs
    const newOpenFiles = openFileIds
      .map(id => findEntryById(newFileSystem, id))
      .filter(Boolean) as FileSystemEntry[];
    
    setOpenFiles(newOpenFiles);

    // Re-find active file
    if (activeFileId) {
      const newActiveFile = findEntryById(newFileSystem, activeFileId);
      setActiveFile(newActiveFile);
    }
  };

  const handleFileUpload = async (files: File[], destPath: string) => {
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    for (const file of files) {
      const reader = new FileReader();
      reader.onload = (e) => {
        const content = e.target?.result as string;
        const newFile: FileSystemEntry = { id: crypto.randomUUID(), name: file.name, type: 'file' };
        
        // Save content with full path as key
        const fullPath = destPath === '' ? file.name : `${destPath}/${file.name}`;
        saveFileContent(fullPath, content);

        // Helper to find destination directory recursively
        const findDestinationDir = (fs: FileSystemEntry[], path: string): FileSystemEntry[] | null => {
          if (path === '') return fs; // Root directory
          
          const parts = path.split('/');
          let currentLevel = fs;
          
          for (const part of parts) {
            const dir = currentLevel.find(e => e.name === part && e.type === 'directory');
            if (!dir || !dir.children) return null;
            currentLevel = dir.children;
          }
          return currentLevel;
        };

        const destDir = findDestinationDir(newFileSystem, destPath);
        if (destDir) {
          destDir.push(newFile);
          setMockFileSystem([...newFileSystem]);
          saveFileSystem(newFileSystem);
        }
      };
      reader.readAsText(file);
    }
  };

  const handleRenameEntry = (oldPath: string, newName: string) => {
    console.log(`Renaming ${oldPath} to ${newName}`);
    
    // Deep copy the file system to avoid mutation issues
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    // Find the entry to rename
    const entryInfo = findEntryAndParent(newFileSystem, oldPath);
    if (!entryInfo) {
      console.error("Entry not found for rename:", oldPath);
      return;
    }

    // Update the name (conflict checking is done in FileExplorer)
    entryInfo.entry.name = newName;

    // Calculate new path
    const pathParts = oldPath.split('/');
    pathParts[pathParts.length - 1] = newName;
    const newPath = pathParts.join('/');

    // Move content if path changed
    if (oldPath !== newPath) {
      moveFileContent(entryInfo.entry, oldPath, newPath);
    }

    // Update the file system
    setMockFileSystem(newFileSystem);
    saveFileSystem(newFileSystem);

    // Update open files and active file using IDs
    const openFileIds = openFiles().map(f => f.id);
    const activeFileId = activeFile()?.id;

    // Re-find open files in the new file system using their IDs
    const newOpenFiles = openFileIds
      .map(id => findEntryById(newFileSystem, id))
      .filter(Boolean) as FileSystemEntry[];
    
    setOpenFiles(newOpenFiles);

    // Re-find active file
    if (activeFileId) {
      const newActiveFile = findEntryById(newFileSystem, activeFileId);
      setActiveFile(newActiveFile);
    }
  };

  const removeFileContent = (entry: FileSystemEntry, path: string) => {
    if (entry.type === 'file') {
      const key = STORAGE_KEYS.FILE_CONTENT_PREFIX + path;
      localStorage.removeItem(key);
    } else if (entry.type === 'directory' && entry.children) {
      entry.children.forEach(child => {
        const childPath = path + '/' + child.name;
        removeFileContent(child, childPath);
      });
    }
  };

  const handleDeleteEntry = (path: string) => {
    console.log(`Deleting ${path}`);
    
    // Deep copy the file system to avoid mutation issues
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    // Find and remove the entry
    const entryInfo = findEntryAndParent(newFileSystem, path);
    if (!entryInfo) {
      console.error("Entry not found for deletion:", path);
      return;
    }

    // Remove from file system
    entryInfo.parent.splice(entryInfo.index, 1);

    // Remove file content from localStorage
    removeFileContent(entryInfo.entry, path);

    // Update the file system
    setMockFileSystem(newFileSystem);
    saveFileSystem(newFileSystem);

    // Remove deleted files from open files and update active file
    const deletedFileIds = collectDeletedFileIds(entryInfo.entry);
    const newOpenFiles = openFiles().filter(f => !deletedFileIds.includes(f.id!));
    setOpenFiles(newOpenFiles);

    // If the deleted file (or any file in a deleted directory) was active, switch to another file or null
    const wasActiveFileDeleted = activeFile() && deletedFileIds.includes(activeFile()!.id!);
    if (wasActiveFileDeleted) {
      const newActiveFile = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null;
      setActiveFile(newActiveFile);
      
      if (newActiveFile && editor && editor.safeUpdateContent) {
        // Load the new active file's content
        const newFilePath = getPathFromId(newFileSystem, newActiveFile.id) || newActiveFile.name;
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

    // Clear selection if deleted items were selected
    const newSelection = selectedFiles().filter(selectedPath => selectedPath !== path);
    setSelectedFiles(newSelection);
  };

  const handleMoveWithReplacement = (sourcePath: string, destPath: string, conflictingName: string) => {
    console.log(`Moving ${sourcePath} to ${destPath} with replacement of ${conflictingName}`);
    
    // Prevent moving into itself
    if (sourcePath === destPath) {
      console.warn('Cannot move into itself');
      return;
    }

    // Prevent moving into a child directory (would create a cycle)
    if (destPath.startsWith(sourcePath + '/')) {
      console.warn('Cannot move into child directory');
      return;
    }
    
    // Deep copy the file system to avoid mutation issues
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    // Find and remove the source entry
    const sourceInfo = findEntryAndParent(newFileSystem, sourcePath);
    if (!sourceInfo) {
      console.error("Source not found:", sourcePath);
      return;
    }

    const [movedEntry] = sourceInfo.parent.splice(sourceInfo.index, 1);

    let removedFileIds: string[] = [];

    // Find destination and handle replacement
    if (destPath === '') { 
      // Moving to root - remove ALL existing conflicting items first
      for (let i = newFileSystem.length - 1; i >= 0; i--) {
        if (newFileSystem[i].name === conflictingName) {
          // Collect IDs of files that will be removed
          removedFileIds.push(...collectDeletedFileIds(newFileSystem[i]));
          newFileSystem.splice(i, 1);
          // Remove conflicting item's content from localStorage
          localStorage.removeItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + conflictingName);
        }
      }
      newFileSystem.push(movedEntry);
    } else {
      const destInfo = findEntryAndParent(newFileSystem, destPath);
      if (!destInfo || destInfo.entry.type !== 'directory') {
        console.error("Destination not found or is not a directory:", destPath);
        // Re-add the entry to its original position if dest is invalid
        sourceInfo.parent.splice(sourceInfo.index, 0, movedEntry);
        return;
      }
      
      if (!destInfo.entry.children) {
        destInfo.entry.children = [];
      }
      
      // Remove ALL existing conflicting items from destination directory
      for (let i = destInfo.entry.children.length - 1; i >= 0; i--) {
        if (destInfo.entry.children[i].name === conflictingName) {
          // Collect IDs of files that will be removed
          removedFileIds.push(...collectDeletedFileIds(destInfo.entry.children[i]));
          destInfo.entry.children.splice(i, 1);
          // Remove conflicting item's content from localStorage
          const conflictingPath = `${destPath}/${conflictingName}`;
          localStorage.removeItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + conflictingPath);
        }
      }
      
      destInfo.entry.children.push(movedEntry);
    }

    // Only move content if the path actually changed
    const newPath = destPath === '' ? movedEntry.name : `${destPath}/${movedEntry.name}`;
    if (sourcePath !== newPath) {
      moveFileContent(movedEntry, sourcePath, newPath);
    }

    console.log("File system after move with replacement:", newFileSystem);
    saveFileSystem(newFileSystem);
    
    // Get IDs of open files and active file before the move
    const openFileIds = openFiles().map(f => f.id);
    const activeFileId = activeFile()?.id;

    // Update the file system state
    setMockFileSystem(newFileSystem);

    // Re-find open files in the new file system using their IDs, excluding removed files
    const newOpenFiles = openFileIds
      .filter(id => !removedFileIds.includes(id!)) // Filter out removed files
      .map(id => findEntryById(newFileSystem, id))
      .filter(Boolean) as FileSystemEntry[];
    
    setOpenFiles(newOpenFiles);

    // Re-find active file, or switch to another if the active file was removed
    if (activeFileId) {
      if (removedFileIds.includes(activeFileId)) {
        // Active file was removed, switch to another file or null
        const newActiveFile = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null;
        setActiveFile(newActiveFile);
        
        if (newActiveFile && editor && editor.safeUpdateContent) {
          // Load the new active file's content
          const newFilePath = getPathFromId(newFileSystem, newActiveFile.id) || newActiveFile.name;
          const content = loadFileContent(newFilePath);
          editor.safeUpdateContent(content);
          
          // Update language
          setTimeout(() => {
            if (editor && editor.updateLanguage) {
              editor.updateLanguage(newActiveFile.name);
            }
          }, 10);
        }
      } else {
        const newActiveFile = findEntryById(newFileSystem, activeFileId);
        setActiveFile(newActiveFile);
      }
    }
  };

  return {
    handleNewFile,
    handleNewFolder,
    handleMoveEntry,
    handleFileUpload,
    handleRenameEntry,
    handleDeleteEntry,
    handleMoveWithReplacement
  };
};
