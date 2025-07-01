// @refresh granular
/** @jsxImportSource solid-js */
import { createSignal, createEffect, Show, For } from "solid-js";
import Resizable from '@corvu/resizable'
import { Example1 } from "./examples/example1";
import { useNavigate } from "@solidjs/router";

import init, { compile, run, check } from '../pkg/althread_web';
import createEditor from './Editor';
import Graph from "./Graph";
import { Logo } from "./assets/images/Logo";
import { renderMessageFlowGraph } from "./CommGraph";
import { rendervmStates } from "./vmStatesDisplay";
import { nodeToString } from "./Node";
import FileExplorer from './FileExplorer';
import type { FileSystemEntry } from './FileExplorer';
import './FileExplorer.css';
import FileTabs from "./FileTabs";

init().then(() => {
  console.log('loaded');
});

const animationTimeOut = 100; //ms

const STORAGE_KEYS = {
  FILE_SYSTEM: 'althread-file-system',
  FILE_CONTENT_PREFIX: 'althread-file-content-'
};

const saveFileSystem = (fileSystem: FileSystemEntry[]) => {
  localStorage.setItem(STORAGE_KEYS.FILE_SYSTEM, JSON.stringify(fileSystem));
};

const loadFileSystem = (): FileSystemEntry[] => {
  const stored = localStorage.getItem(STORAGE_KEYS.FILE_SYSTEM);
  
  // Helper to add unique IDs to entries if they don't have them
  const addIds = (entries: any[]): FileSystemEntry[] => {
    return entries.map(entry => ({
      ...entry,
      id: entry.id || crypto.randomUUID(),
      children: entry.children ? addIds(entry.children) : undefined
    }));
  };

  if (stored) {
    return addIds(JSON.parse(stored));
  }
  // Default file system if nothing stored
  return addIds([
    { name: 'main.alt', type: 'file' },
    {
      name: 'utils',
      type: 'directory',
      children: [
        { name: 'helpers.alt', type: 'file' },
        { name: 'math.alt', type: 'file' },
      ],
    },
    { name: 'README.md', type: 'file' }
  ]);
};

const saveFileContent = (fileName: string, content: string) => {
  localStorage.setItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + fileName, content);
};

const loadFileContent = (fileName: string): string => {
  const content = localStorage.getItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + fileName);
  if (content !== null) {
    return content;
  }
  
  // Default content for specific files
  if (fileName === 'main.alt') {
    localStorage.setItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + 'main.alt', Example1);
    return localStorage.getItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + 'main.alt')!;
  }
  if (fileName === 'README.md') {
    localStorage.setItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + 'README.md', '# Project README\n\nThis is your project documentation.');
    return localStorage.getItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + 'README.md')!;
  }
  if (fileName === 'utils/helpers.alt') {
    localStorage.setItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + 'utils/helpers.alt', '// Helper functions\n');
    return localStorage.getItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + 'utils/helpers.alt')!;
  }
  if (fileName === 'utils/math.alt') {
    localStorage.setItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + 'utils/math.alt', '// Math functions\n');
    return localStorage.getItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + 'utils/math.alt')!;
  }
  
  return '// New file\n';
};

export default function App() {
  const navigate = useNavigate();

  // Load file system from localStorage
  let initialFileSystem = loadFileSystem();
  const utilsExists = initialFileSystem.some(entry => entry.name === 'utils' && entry.type === 'directory');

  // If the loaded filesystem from local storage is old, reset it.
  if (!utilsExists) {
    localStorage.removeItem(STORAGE_KEYS.FILE_SYSTEM);
    initialFileSystem = loadFileSystem();
  }

  const [mockFileSystem, setMockFileSystem] = createSignal<FileSystemEntry[]>(initialFileSystem);

  // Initialize editor with main.alt content
  const mainContent = loadFileContent('main.alt');
  let editor = createEditor({
    compile, 
    defaultValue: mainContent,
    fileName: 'main.alt', // Pass the initial filename
    onValueChange: (value) => {
      // Save current file content when editor changes
      if (activeFile()) {
        const filePath = getPathFromId(mockFileSystem(), activeFile()!.id) || activeFile()!.name;
        saveFileContent(filePath, value);
      }
    }
  });

  // Add state for open files and active file
  const [openFiles, setOpenFiles] = createSignal<FileSystemEntry[]>([]);
  const [activeFile, setActiveFile] = createSignal<FileSystemEntry | null>(null);
  const [selectedFiles, setSelectedFiles] = createSignal<string[]>([]);

  // New helper function to find an entry by ID
  const findEntryById = (fs: FileSystemEntry[], id: string): FileSystemEntry | null => {
    for (const entry of fs) {
      if (entry.id === id) return entry;
      if (entry.children) {
        const found = findEntryById(entry.children, id);
        if (found) return found;
      }
    }
    return null;
  };

  // New helper function to get a path from an ID
  const getPathFromId = (fs: FileSystemEntry[], id: string, currentPath: string = ''): string | null => {
    for (const entry of fs) {
      const entryPath = currentPath ? `${currentPath}/${entry.name}` : entry.name;
      if (entry.id === id) return entryPath;
      if (entry.children) {
        const foundPath = getPathFromId(entry.children, id, entryPath);
        if (foundPath) return foundPath;
      }
    }
    return null;
  };

  // Initialize with main.alt file after the file system is loaded
  createEffect(() => {
    if (mockFileSystem().length > 0 && !activeFile()) {
      const mainFile = findFileByPath(mockFileSystem(), 'main.alt');
      if (mainFile) {
        setOpenFiles([mainFile]);
        setActiveFile(mainFile);
      }
    }
  });

  const findFileByPath = (files: FileSystemEntry[], targetPath: string): FileSystemEntry | null => {
    for (const file of files) {
      if (file.name === targetPath && file.type === 'file') {
        return file;
      }
      if (file.type === 'directory' && file.children) {
        const pathParts = targetPath.split('/');
        if (pathParts[0] === file.name && pathParts.length > 1) {
          const remainingPath = pathParts.slice(1).join('/');
          const found = findFileByPath(file.children, remainingPath);
          if (found) return found;
        }
      }
    }
    return null;
  };

  const getFilePathFromEntry = (entry: FileSystemEntry, fileSystem: FileSystemEntry[], currentPath: string = ''): string => {
    for (const item of fileSystem) {
      const itemPath = currentPath ? `${currentPath}/${item.name}` : item.name;
      
      if (item === entry) {
        return itemPath;
      }
      
      if (item.type === 'directory' && item.children) {
        const found = getFilePathFromEntry(entry, item.children, itemPath);
        if (found) return found;
      }
    }
    return entry.name; // fallback
  }


  const handleFileSelect = (path: string) => {
    console.log("File selected:", path);
    
    const file = findFileByPath(mockFileSystem(), path);
    if (file) {
      // Check if file is already open using ID instead of path
      const isAlreadyOpen = openFiles().some(f => f.id === file.id);
      if (!isAlreadyOpen) {
        setOpenFiles([...openFiles(), file]);
      }
      setActiveFile(file);
      
      // Load file content into editor first
      const content = loadFileContent(path);
      const update = editor.editorView().state.update({
        changes: {
          from: 0, 
          to: editor.editorView().state.doc.length,
          insert: content
        }
      });
      editor.editorView().update([update]);
      
      // Then update language (after content is loaded)
      setTimeout(() => {
        editor.updateLanguage(file.name);
      }, 10);
    }
  };

  const handleFileTabClick = (file: FileSystemEntry) => {
    setActiveFile(file);
    
    // Load file content into editor first
    const filePath = getPathFromId(mockFileSystem(), file.id) || file.name;
    const content = loadFileContent(filePath);
    const update = editor.editorView().state.update({
      changes: {
        from: 0, 
        to: editor.editorView().state.doc.length,
        insert: content
      }
    });
    editor.editorView().update([update]);
    
    // Then update language (after content is loaded)
    setTimeout(() => {
      editor.updateLanguage(file.name);
    }, 10);
  };

  const handleTabClose = (file: FileSystemEntry) => {
    // Use ID-based filtering instead of path-based
    const newOpenFiles = openFiles().filter(f => f.id !== file.id);
    setOpenFiles(newOpenFiles);
    
    // If we closed the active file, switch to another open file or null
    if (activeFile() && activeFile()!.id === file.id) {
      const newActiveFile = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null;
      setActiveFile(newActiveFile);
      
      if (newActiveFile) {
        // Load the new active file's content
        const newFilePath = getPathFromId(mockFileSystem(), newActiveFile.id) || newActiveFile.name;
        const content = loadFileContent(newFilePath);
        const update = editor.editorView().state.update({
          changes: {
            from: 0, 
            to: editor.editorView().state.doc.length,
            insert: content
          }
        });
        editor.editorView().update([update]);
        
        // Update language
        setTimeout(() => {
          editor.updateLanguage(newActiveFile.name);
        }, 10);
      }
    }
  };

  const handleNewFile = (name: string, targetPath?: string) => {
    const newFile: FileSystemEntry = { id: crypto.randomUUID(), name, type: 'file' };

    // Determine where to create the file
    const createInPath = targetPath || '';
    
    // Deep copy to avoid mutation
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));
    
    // Helper to find the target directory
    const findTargetDirectory = (fs: FileSystemEntry[], path: string): FileSystemEntry[] | null => {
      if (path === '') return fs; // Root directory
      
      const parts = path.split('/').filter(part => part !== '');
      let currentLevel = fs;
      
      for (const part of parts) {
        const dir = currentLevel.find(e => e.name === part && e.type === 'directory');
        if (!dir || !dir.children) return null;
        currentLevel = dir.children;
      }
      return currentLevel;
    };

    const targetDir = findTargetDirectory(newFileSystem, createInPath);
    if (!targetDir) {
      setCreationError("Target directory not found.");
      return;
    }

    // Check if file already exists in the target directory
    const existingFile = targetDir.find(f => f.name === name);
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
    
    // Load content first
    const update = editor.editorView().state.update({
      changes: {
        from: 0, 
        to: editor.editorView().state.doc.length,
        insert: defaultContent
      }
    });
    editor.editorView().update([update]);
    
    // Then update language
    setTimeout(() => {
      editor.updateLanguage(name);
    }, 10);
  };

  // Helper function to get default content based on file type
  const getDefaultContentForFile = (fileName: string): string => {
    const extension = fileName.split('.').pop()?.toLowerCase();
    
    switch (extension) {
      case 'js':
      case 'jsx':
        return '// JavaScript file\nconsole.log("Hello, World!");';
      case 'ts':
      case 'tsx':
        return '// TypeScript file\nconsole.log("Hello, World!");';
      case 'py':
        return '# Python file\nprint("Hello, World!")';
      case 'html':
        return '<!DOCTYPE html>\n<html>\n<head>\n    <title>Document</title>\n</head>\n<body>\n    \n</body>\n</html>';
      case 'css':
        return '/* CSS file */\nbody {\n    margin: 0;\n    padding: 0;\n}';
      case 'json':
        return '{\n    "name": "example",\n    "version": "1.0.0"\n}';
      case 'md':
        return '# Markdown File\n\nThis is a markdown document.';
      case 'alt':
        return '// Althread file\n';
      default:
        return '// New file\n';
    }
  };

  const handleNewFolder = (name: string, targetPath?: string) => {
    const newFolder: FileSystemEntry = { id: crypto.randomUUID(), name, type: 'directory', children: [] };

    // Determine where to create the folder
    const createInPath = targetPath || '';
    
    // Deep copy to avoid mutation
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));
    
    // Helper to find the target directory
    const findTargetDirectory = (fs: FileSystemEntry[], path: string): FileSystemEntry[] | null => {
      if (path === '') return fs; // Root directory
      
      const parts = path.split('/').filter(part => part !== '');
      let currentLevel = fs;
      
      for (const part of parts) {
        const dir = currentLevel.find(e => e.name === part && e.type === 'directory');
        if (!dir || !dir.children) return null;
        currentLevel = dir.children;
      }
      return currentLevel;
    };

    const targetDir = findTargetDirectory(newFileSystem, createInPath);
    if (!targetDir) {
      setCreationError("Target directory not found.");
      return;
    }

    // Check if folder already exists in the target directory
    const existingFolder = targetDir.find(f => f.name === name);
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

    // Helper to find an entry and its parent recursively
    const findEntryAndParent = (fs: FileSystemEntry[], path: string): { entry: FileSystemEntry, parent: FileSystemEntry[], index: number } | null => {
      const parts = path.split('/').filter(part => part !== ''); // Remove empty parts
      if (parts.length === 0) return null;
      
      let currentLevel = fs;
      let parentLevel = fs;

      // Navigate to the correct level
      for (let i = 0; i < parts.length - 1; i++) {
        const part = parts[i];
        const dirEntry = currentLevel.find(e => e.name === part && e.type === 'directory');
        if (!dirEntry || !dirEntry.children) return null;
        
        parentLevel = currentLevel;
        currentLevel = dirEntry.children;
      }

      // Find the target entry in the current level
      const targetName = parts[parts.length - 1];
      const entryIndex = currentLevel.findIndex(e => e.name === targetName);
      if (entryIndex === -1) return null;

      const entry = currentLevel[entryIndex];
      return { entry, parent: currentLevel, index: entryIndex };
    };

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

    // Move file content in localStorage if it's a file or folder
    function moveFileContent(entry: FileSystemEntry, oldPath: string, newPath: string) {
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

  // Conflict checking functions for file operations
  const checkNameConflict = (destPath: string, movingName: string): boolean => {
    if (destPath === '') {
      // Moving to root
      return mockFileSystem().some(entry => entry.name === movingName);
    }
    
    // Find the destination directory
    const findDirectory = (files: FileSystemEntry[], targetPath: string): FileSystemEntry | null => {
      const parts = targetPath.split('/').filter(part => part !== '');
      let currentLevel = files;
      
      for (const part of parts) {
        const dir = currentLevel.find(e => e.name === part && e.type === 'directory');
        if (!dir || !dir.children) return null;
        currentLevel = dir.children;
      }
      
      // Return a synthetic entry representing the directory
      return { id: 'temp', name: '', type: 'directory', children: currentLevel };
    };
    
    const destDir = findDirectory(mockFileSystem(), destPath);
    return destDir?.children?.some(entry => entry.name === movingName) || false;
  };

  // Confirmation dialog state for file conflicts
  const [moveConfirmation, setMoveConfirmation] = createSignal<{
    isOpen: boolean;
    sourcePaths: string[];
    destPath: string;
    conflictingName: string;
  }>({
    isOpen: false,
    sourcePaths: [],
    destPath: '',
    conflictingName: ''
  });

  // Delete confirmation dialog state
  const [deleteConfirmation, setDeleteConfirmation] = createSignal<{
    isOpen: boolean;
    paths: string[];
  }>({
    isOpen: false,
    paths: []
  });

  const showMoveConfirmDialog = (sourcePaths: string[], destPath: string, conflictingName: string) => {
    setMoveConfirmation({
      isOpen: true,
      sourcePaths,
      destPath,
      conflictingName
    });
  };

  const handleConfirmedMove = () => {
    const confirmation = moveConfirmation();
    
    // Create a modified version of handleMoveEntry that handles replacement
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

      // Helper to find an entry and its parent recursively
      const findEntryAndParent = (fs: FileSystemEntry[], path: string): { entry: FileSystemEntry, parent: FileSystemEntry[], index: number } | null => {
        const parts = path.split('/').filter(part => part !== ''); // Remove empty parts
        if (parts.length === 0) return null;
        
        let currentLevel = fs;
        let parentLevel = fs;

        // Navigate to the correct level
        for (let i = 0; i < parts.length - 1; i++) {
          const part = parts[i];
          const dirEntry = currentLevel.find(e => e.name === part && e.type === 'directory');
          if (!dirEntry || !dirEntry.children) return null;
          
          parentLevel = currentLevel;
          currentLevel = dirEntry.children;
        }

        // Find the target entry in the current level
        const targetName = parts[parts.length - 1];
        const entryIndex = currentLevel.findIndex(e => e.name === targetName);
        if (entryIndex === -1) return null;

        const entry = currentLevel[entryIndex];
        return { entry, parent: currentLevel, index: entryIndex };
      };

      // Find and remove the source entry
      const sourceInfo = findEntryAndParent(newFileSystem, sourcePath);
      if (!sourceInfo) {
        console.error("Source not found:", sourcePath);
        return;
      }

      const [movedEntry] = sourceInfo.parent.splice(sourceInfo.index, 1);

      // Find destination and handle replacement
      if (destPath === '') { 
        // Moving to root - remove ALL existing conflicting items first
        for (let i = newFileSystem.length - 1; i >= 0; i--) {
          if (newFileSystem[i].name === conflictingName) {
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
            destInfo.entry.children.splice(i, 1);
            // Remove conflicting item's content from localStorage
            const conflictingPath = `${destPath}/${conflictingName}`;
            localStorage.removeItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + conflictingPath);
          }
        }
        
        destInfo.entry.children.push(movedEntry);
      }

      // Move file content in localStorage if it's a file or folder
      function moveFileContent(entry: FileSystemEntry, oldPath: string, newPath: string) {
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
    
    // Execute the move with replacement for each source path
    confirmation.sourcePaths.forEach(sourcePath => {
      handleMoveWithReplacement(sourcePath, confirmation.destPath, confirmation.conflictingName);
    });
    
    setMoveConfirmation({ isOpen: false, sourcePaths: [], destPath: '', conflictingName: '' });
  };

  const handleCanceledMove = () => {
    setMoveConfirmation({ isOpen: false, sourcePaths: [], destPath: '', conflictingName: '' });
  };

  const handleRenameEntry = (oldPath: string, newName: string) => {
    console.log(`Renaming ${oldPath} to ${newName}`);
    
    // Deep copy the file system to avoid mutation issues
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    // Helper to find an entry and its parent recursively
    const findEntryAndParent = (fs: FileSystemEntry[], path: string): { entry: FileSystemEntry, parent: FileSystemEntry[], index: number } | null => {
      const parts = path.split('/').filter(part => part !== '');
      if (parts.length === 0) return null;
      
      let currentLevel = fs;

      // Navigate to the correct level
      for (let i = 0; i < parts.length - 1; i++) {
        const part = parts[i];
        const dirEntry = currentLevel.find(e => e.name === part && e.type === 'directory');
        if (!dirEntry || !dirEntry.children) return null;
        currentLevel = dirEntry.children;
      }

      // Find the target entry in the current level
      const targetName = parts[parts.length - 1];
      const entryIndex = currentLevel.findIndex(e => e.name === targetName);
      if (entryIndex === -1) return null;

      const entry = currentLevel[entryIndex];
      return { entry, parent: currentLevel, index: entryIndex };
    };

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

    // Move file content in localStorage if it's a file or folder
    function moveFileContent(entry: FileSystemEntry, oldPath: string, newPath: string) {
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
    }

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

  const handleDeleteEntry = (path: string) => {
    console.log(`Deleting ${path}`);
    
    // Deep copy the file system to avoid mutation issues
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    // Helper to find an entry and its parent recursively
    const findEntryAndParent = (fs: FileSystemEntry[], path: string): { entry: FileSystemEntry, parent: FileSystemEntry[], index: number } | null => {
      const parts = path.split('/').filter(part => part !== '');
      if (parts.length === 0) return null;
      
      let currentLevel = fs;

      // Navigate to the correct level
      for (let i = 0; i < parts.length - 1; i++) {
        const part = parts[i];
        const dirEntry = currentLevel.find(e => e.name === part && e.type === 'directory');
        if (!dirEntry || !dirEntry.children) return null;
        currentLevel = dirEntry.children;
      }

      // Find the target entry in the current level
      const targetName = parts[parts.length - 1];
      const entryIndex = currentLevel.findIndex(e => e.name === targetName);
      if (entryIndex === -1) return null;

      const entry = currentLevel[entryIndex];
      return { entry, parent: currentLevel, index: entryIndex };
    };

    // Find and remove the entry
    const entryInfo = findEntryAndParent(newFileSystem, path);
    if (!entryInfo) {
      console.error("Entry not found for deletion:", path);
      return;
    }

    // Remove from file system
    entryInfo.parent.splice(entryInfo.index, 1);

    // Remove file content from localStorage
    function removeFileContent(entry: FileSystemEntry, path: string) {
      if (entry.type === 'file') {
        const key = STORAGE_KEYS.FILE_CONTENT_PREFIX + path;
        localStorage.removeItem(key);
      } else if (entry.type === 'directory' && entry.children) {
        entry.children.forEach(child => {
          const childPath = path + '/' + child.name;
          removeFileContent(child, childPath);
        });
      }
    }

    removeFileContent(entryInfo.entry, path);

    // Update the file system
    setMockFileSystem(newFileSystem);
    saveFileSystem(newFileSystem);

    // Remove deleted files from open files and update active file
    const deletedEntryId = entryInfo.entry.id;
    const newOpenFiles = openFiles().filter(f => f.id !== deletedEntryId);
    setOpenFiles(newOpenFiles);

    // If the deleted file was active, switch to another file or null
    if (activeFile() && activeFile()!.id === deletedEntryId) {
      const newActiveFile = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null;
      setActiveFile(newActiveFile);
      
      if (newActiveFile) {
        // Load the new active file's content
        const newFilePath = getPathFromId(newFileSystem, newActiveFile.id) || newActiveFile.name;
        const content = loadFileContent(newFilePath);
        const update = editor.editorView().state.update({
          changes: {
            from: 0, 
            to: editor.editorView().state.doc.length,
            insert: content
          }
        });
        editor.editorView().update([update]);
        
        // Update language
        setTimeout(() => {
          editor.updateLanguage(newActiveFile.name);
        }, 10);
      }
    }

    // Clear selection if deleted items were selected
    const newSelection = selectedFiles().filter(selectedPath => selectedPath !== path);
    setSelectedFiles(newSelection);
  };

  const showDeleteConfirmDialog = (paths: string[]) => {
    setDeleteConfirmation({
      isOpen: true,
      paths
    });
  };

  const handleConfirmedDelete = () => {
    const confirmation = deleteConfirmation();
    confirmation.paths.forEach(path => {
      handleDeleteEntry(path);
    });
    setDeleteConfirmation({ isOpen: false, paths: [] });
  };

  const handleCanceledDelete = () => {
    setDeleteConfirmation({ isOpen: false, paths: [] });
  };

  // Save file system whenever it changes
  createEffect(() => {
    saveFileSystem(mockFileSystem());
  });

  let [activeTab, setActiveTab] = createSignal("console");
  const handleExecutionTabClick = (tab: string) => {
    setActiveTab(tab);
  };

  let [nodes, setNodes] = createSignal([]);
  let [edges, setEdges] = createSignal([]);
  let [isRun, setIsRun] = createSignal(true);

  let [stdout, setStdout] = createSignal("The console output will appear here.");
  let [out, setOut] = createSignal("The execution output will appear here.");
  let [commgraphout, setCommGraphOut] = createSignal([]); //messageflow graph
  let [vm_states, setVmStates] = createSignal<any[]>([]); //to display vm states information
  let [activeAction, setActiveAction] = createSignal<string | null>(null);
  const [loadingAction, setLoadingAction] = createSignal<string | null>(null);
  const [creationError, setCreationError] = createSignal<string | null>(null);


  const renderExecContent = () => {
    if (isRun()) {
      if (activeTab() === "console") {
        return (
          <div class="console">
            <pre>{stdout()}</pre>
          </div>
        );
      } else if (activeTab() === "execution") {
        return (
          <div class="console">
            <pre>{out()}</pre>
          </div>
        );
      } else if (activeTab() === "msg_flow") {
        return (
          <div class="console">
            {renderMessageFlowGraph(commgraphout(), vm_states())}
          </div>
        );
      } else if (activeTab() === "vm_states") {
        return (
          <div class="console">
            {rendervmStates(vm_states())}
          </div>
        );
      }
    } else {
      setActiveTab("vm_states");
      return (
        <div class="console">
          <Graph nodes={nodes()} edges={edges()} theme="dark" />
        </div>
      );
    }
  };

  return (
    <>
      <div id="header">
          <div class="brand">
            <Logo />
            <h3>Althread</h3>
          </div>
          <div class="actions">
            <button
              class={`vscode-button${loadingAction() === "load" ? " active" : ""}`}
              onClick={async () => {
                setLoadingAction("load");
                try {
                let up = editor.editorView().state.update({
                  changes: {
                    from: 0, 
                    to: editor.editorView().state.doc.length,
                    insert: Example1
                  }
                })
                editor.editorView().update([up]);
              } catch (error) {
                console.error("Error loading example:", error);
              } finally {
                setTimeout(() => {
                    setLoadingAction(null);
                    setActiveAction(null);
                }, animationTimeOut);
              }
              }}>
              <i class={loadingAction() === "load" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-file"}></i>
              Load Example
            </button>

            <button
              class={`vscode-button${loadingAction() === "run" ? " active" : ""}`}
              disabled={loadingAction() === "run"}
              onClick={async () => {
                setLoadingAction("run");
                try {
                  setIsRun(true);
                  let res = run(editor.editorView().state.doc.toString());
                  setOut(res.debug);
                  setCommGraphOut(res.message_flow_graph);
                  setVmStates(res.vm_states);
                  setStdout(res.stdout.join('\n'));
                  setActiveTab("console");
                } catch(e: any) {
                  setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
                } finally {
                  setTimeout(() => {
                    setLoadingAction(null);
                    setActiveAction(null);
                  }, animationTimeOut);
                }
              }}>
              <i class={loadingAction() === "run" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-play"}></i>
              Run
            </button>

            <button
              class={`vscode-button${activeAction() === "check" ? " active" : ""}`}
              onClick={() => {
                setActiveAction(activeAction() === "check" ? null : "check");
                try {
                  let res = check(editor.editorView().state.doc.toString())
                  setOut(res);
                  
                  console.log(res);
                  let colored_path: string[] = [];
                  if(res[0].length > 0) { // a violation occurred
                    res[0].forEach((path) => {
                      colored_path.push(nodeToString(path.to));
                    });
                  }

                  let nodes = {};
                  setNodes(res[1].nodes.map((n, i) => {
                    let label = nodeToString(n[0]);
                    const {level, predecessor, successors} = n[1];
                    nodes[label] = i;
                    const isViolationNode = colored_path.includes(label) || (colored_path.length > 0 && level == 0);
                    const background = isViolationNode ? "#4d3131" : "#314d31";
                    const border = isViolationNode ? "#ec9999" : "#a6dfa6";
                    return {
                      id: i,
                      level,
                      label,
                      color: {
                        border,
                        background,
                        highlight: {
                          border: "hsla(29.329, 66.552%, 52.544%)", // theme primary
                          background // keep original background
                        },
                        hover: {
                          border: "hsla(29.329, 66.552%, 52.544%)",
                          background
                        }
                      }
                    }
                  }));

                  let edges: any = [];
                  res[1].nodes.forEach((n, i) => {
                    let label = nodeToString(n[0]);
                    const {level, predecessor, successors} = n[1];
                    successors.forEach(({lines, pid, name, to}) => {
                      to = nodeToString(to);
                      edges.push({
                        from: i,
                        to: nodes[to],
                        label: name+'#'+pid+': '+lines.join(',')
                      });
                    })
                    // console.log(node_entirely(n[0]));
                  });
                  setEdges(edges);
                  setIsRun(false);

                } catch(e: any) {
                  setOut("ERROR: "+(e.pos && ('line '+e.pos.line))+"\n"+e.message);
                }
              }}>
              <i class="codicon codicon-check"></i>
              Check
            </button>

            <button
              class={`vscode-button${loadingAction() === "reset" ? " active" : ""}`}
              onClick={async () => {
                setLoadingAction("reset");
                try {
                  setIsRun(true);
                  setOut("The execution output will appear here.");
                  setStdout("The console output will appear here.");
                  setCommGraphOut([]);
                  setNodes([]);
                  setEdges([]);
                  setVmStates([]);
                } finally {
                  setTimeout(() => {
                    setLoadingAction(null);
                  }, 100);
                }
              }}>
              <i class={loadingAction() === "reset" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-clear-all"}></i>
              Reset
            </button>

            <button
              class={`vscode-button${loadingAction() === "tutorial" ? " active" : ""}`}
              onClick={() => {
                setLoadingAction("tutorial");
                navigate('/tutorials');
              }}>
              <i class="codicon codicon-book"></i>
              Tutorials
            </button>
            <button
              class="vscode-button"
              onClick={() => {
                window.location.href = "https://althread.github.io/en/docs/guide/intro/";
              }}>
              <i class="codicon codicon-repo"></i>
              Documentation
            </button>
          </div>
      </div>
      <Resizable id="content">
        <Resizable.Panel initialSize={0.15} minSize={0.1}>
            <FileExplorer 
                files={mockFileSystem()} 
                onFileSelect={handleFileSelect}
                onNewFile={handleNewFile}
                onNewFolder={handleNewFolder}
                onMoveEntry={handleMoveEntry}
                onRenameEntry={handleRenameEntry}
                onDeleteEntry={handleDeleteEntry}
                onFileUpload={handleFileUpload}
                activeFile={activeFile()}
                getFilePath={(entry) => getPathFromId(mockFileSystem(), entry.id) || entry.name}
                selectedFiles={selectedFiles()}
                onSelectionChange={setSelectedFiles}
                creationError={creationError()}
                setCreationError={setCreationError}
                checkNameConflict={checkNameConflict}
                showConfirmDialog={showMoveConfirmDialog}
                showDeleteConfirmDialog={showDeleteConfirmDialog}
            />
        </Resizable.Panel>
        <Resizable.Handle class="Resizable-handle"/>
          <Resizable.Panel class="editor-panel"
          initialSize={0.55}
          minSize={0.2}>
          <FileTabs 
            openFiles={openFiles()}
            activeFile={activeFile()}
            getFilePath={(entry) => getPathFromId(mockFileSystem(), entry.id) || entry.name}
            onTabClick={handleFileTabClick}
            onTabClose={handleTabClose}
          />
          <div class="editor-instance-wrapper" ref={editor.ref} />
          </Resizable.Panel>
        <Resizable.Handle class="Resizable-handle"/>
        <Resizable.Panel class="right-panel"
initialSize={0.30}
minSize={0.2}>
    <div class="execution-content">
    <div class="tab">
        <button class={`tab_button ${activeTab() === "console" ? "active" : ""}`}
                onclick={() => handleExecutionTabClick("console")}
                disabled={!isRun()}
        >
        <h3>Console</h3>
        </button>
        <button class={`tab_button ${activeTab() === "execution" ? "active" : ""}`}
                onclick={() => handleExecutionTabClick("execution")}
                disabled={!isRun()}
        >
        <h3>Execution</h3>
        </button>
        <button class={`tab_button ${activeTab() === "msg_flow" ? "active" : ""}`}
                onclick={() => handleExecutionTabClick("msg_flow")}
                disabled={!isRun()}
        >
        <h3>Message flow</h3>
        </button>
        <button class={`tab_button ${activeTab() === "vm_states" ? "active" : ""}`}
                onclick={() => handleExecutionTabClick("vm_states")}
        >
        <h3>VM states</h3>
        </button>
    </div>

    <div class="tab-content">
        {renderExecContent()}
    </div>
    </div>

</Resizable.Panel>
      </Resizable>

      {/* Move Confirmation Dialog */}
      <MoveConfirmationDialog
        isOpen={moveConfirmation().isOpen}
        title="Replace Existing Item"
        message="An item with this name already exists in the destination folder. Do you want to replace it?"
        fileName={moveConfirmation().conflictingName}
        onConfirm={handleConfirmedMove}
        onCancel={handleCanceledMove}
      />

      {/* Delete Confirmation Dialog */}
      <DeleteConfirmationDialog
        isOpen={deleteConfirmation().isOpen}
        paths={deleteConfirmation().paths}
        onConfirm={handleConfirmedDelete}
        onCancel={handleCanceledDelete}
      />
    </>
  );
}

// Confirmation Dialog Component for App
const MoveConfirmationDialog = (props: {
  isOpen: boolean;
  title: string;
  message: string;
  fileName: string;
  onConfirm: () => void;
  onCancel: () => void;
}) => {
  return (
    <Show when={props.isOpen}>
      <div class="confirmation-dialog-overlay" onClick={props.onCancel}>
        <div class="confirmation-dialog" onClick={(e) => e.stopPropagation()}>
          <div class="confirmation-dialog-header">
            <i class="codicon codicon-warning"></i>
            {props.title}
          </div>
          <div class="confirmation-dialog-body">
            {props.message}
            <br />
            <span class="confirmation-dialog-file-name">{props.fileName}</span>
          </div>
          <div class="confirmation-dialog-actions">
            <button class="button-secondary" onClick={props.onCancel}>
              Cancel
            </button>
            <button class="button-primary" onClick={props.onConfirm}>
              Replace
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};

// Delete Confirmation Dialog Component
const DeleteConfirmationDialog = (props: {
  isOpen: boolean;
  paths: string[];
  onConfirm: () => void;
  onCancel: () => void;
}) => {
  const getDeleteMessage = () => {
    const count = props.paths.length;
    if (count === 1) {
      return (
        <>
          Are you sure you want to delete <span class="confirmation-dialog-file-name">{props.paths[0]}</span>?
        </>
      );
    } else {
      return `Are you sure you want to delete ${count} items?`;
    }
  };

  return (
    <Show when={props.isOpen}>
      <div class="confirmation-dialog-overlay" onClick={props.onCancel}>
        <div class="confirmation-dialog" onClick={(e) => e.stopPropagation()}>
          <div class="confirmation-dialog-header">
            <i class="codicon codicon-trash"></i>
            Delete {props.paths.length === 1 ? 'Item' : 'Items'}
          </div>
          <div class="confirmation-dialog-body">
            {getDeleteMessage()}
            <Show when={props.paths.length > 1}>
              <br />
              <br />
              <div style="max-height: 120px; overflow-y: auto; font-size: 12px; color: #999;">
                <For each={props.paths}>
                  {(path) => <div>• <span class="confirmation-dialog-file-name">{path}</span></div>}
                </For>
              </div>
            </Show>
          </div>
          <div class="confirmation-dialog-actions">
            <button class="button-secondary" onClick={props.onCancel}>
              Cancel
            </button>
            <button class="button-primary" onClick={props.onConfirm} style="background-color: #c74e39;">
              Delete
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};

