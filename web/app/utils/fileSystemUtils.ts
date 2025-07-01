import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';

export const findEntryById = (fs: FileSystemEntry[], id: string): FileSystemEntry | null => {
  for (const entry of fs) {
    if (entry.id === id) return entry;
    if (entry.children) {
      const found = findEntryById(entry.children, id);
      if (found) return found;
    }
  }
  return null;
};

export const getPathFromId = (fs: FileSystemEntry[], id: string, currentPath: string = ''): string | null => {
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

export const findFileByPath = (files: FileSystemEntry[], targetPath: string): FileSystemEntry | null => {
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

export const getFilePathFromEntry = (entry: FileSystemEntry, fileSystem: FileSystemEntry[], currentPath: string = ''): string => {
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
};

export const findTargetDirectory = (fs: FileSystemEntry[], path: string): FileSystemEntry[] | null => {
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

export const findEntryAndParent = (fs: FileSystemEntry[], path: string): { entry: FileSystemEntry, parent: FileSystemEntry[], index: number } | null => {
  const parts = path.split('/').filter(part => part !== ''); // Remove empty parts
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

export const collectDeletedFileIds = (entry: FileSystemEntry): string[] => {
  const ids: string[] = [];
  if (entry.type === 'file') {
    ids.push(entry.id!);
  } else if (entry.type === 'directory' && entry.children) {
    entry.children.forEach(child => {
      ids.push(...collectDeletedFileIds(child));
    });
  }
  return ids;
};
