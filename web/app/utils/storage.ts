import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import { Example1 } from '@examples/example1';

export const STORAGE_KEYS = {
  FILE_SYSTEM: 'althread-file-system',
  FILE_CONTENT_PREFIX: 'althread-file-content-'
};

export const saveFileSystem = (fileSystem: FileSystemEntry[]) => {
  localStorage.setItem(STORAGE_KEYS.FILE_SYSTEM, JSON.stringify(fileSystem));
};

export const loadFileSystem = (): FileSystemEntry[] => {
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

export const saveFileContent = (fileName: string, content: string) => {
  localStorage.setItem(STORAGE_KEYS.FILE_CONTENT_PREFIX + fileName, content);
};

export const loadFileContent = (fileName: string): string => {
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

export const getDefaultContentForFile = (fileName: string): string => {
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
