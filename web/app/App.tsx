// @refresh granular
/** @jsxImportSource solid-js */
import { createSignal, createEffect } from "solid-js";
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
  if (stored) {
    return JSON.parse(stored);
  }
  // Default file system if nothing stored
  return [
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
  ];
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
    return Example1;
  }
  if (fileName === 'README.md') {
    return '# Project README\n\nThis is your project documentation.';
  }
  if (fileName === 'helpers.alt' || fileName === 'math.alt') {
    return '// Helper functions\n';
  }
  
  return '// New file\n';
};

const getFilePathFromEntry = (entry: FileSystemEntry, fileSystem: FileSystemEntry[], currentPath: string = ''): string => {
  for (const item of fileSystem) {
    const itemPath = currentPath ? `${currentPath}/${item.name}` : item.name;
    
    console.log("Checking item:", itemPath, "against entry:", entry.name);

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
        const filePath = getFilePathFromEntry(activeFile()!, mockFileSystem());
        saveFileContent(filePath, value);
      }
    }
  });

  // Add state for open files and active file
  const [openFiles, setOpenFiles] = createSignal<FileSystemEntry[]>([]);
  const [activeFile, setActiveFile] = createSignal<FileSystemEntry | null>(null);
  const [selectedFiles, setSelectedFiles] = createSignal<string[]>([]);

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
      // Add to open files if not already open
      const isAlreadyOpen = openFiles().some(f => getFilePathFromEntry(f, mockFileSystem()) === path);
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
    const filePath = getFilePathFromEntry(file, mockFileSystem());
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
    const filePath = getFilePathFromEntry(file, mockFileSystem());
    const newOpenFiles = openFiles().filter(f => getFilePathFromEntry(f, mockFileSystem()) !== filePath);
    setOpenFiles(newOpenFiles);
    
    // If we closed the active file, switch to another open file or null
    if (activeFile() && getFilePathFromEntry(activeFile()!, mockFileSystem()) === filePath) {
      const newActiveFile = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null;
      setActiveFile(newActiveFile);
      
      if (newActiveFile) {
        // Load the new active file's content
        const newFilePath = getFilePathFromEntry(newActiveFile, mockFileSystem());
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

  const handleNewFile = (name: string) => {
    const newFile: FileSystemEntry = { name, type: 'file' };

    // Check if file already exists in the same path
    const existingFile = mockFileSystem().find(f => getFilePathFromEntry(f, mockFileSystem()) === name);

    if (existingFile) {
      setCreationError("A file or folder with this name already exists.");
      return;
    }
    setCreationError(null);

    const updatedFileSystem = [...mockFileSystem(), newFile];
    setMockFileSystem(updatedFileSystem);
    saveFileSystem(updatedFileSystem);
    
    // Save empty content for new file
    const defaultContent = getDefaultContentForFile(name);
    saveFileContent(name, defaultContent);
    
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

  const handleNewFolder = (name: string) => {
    const exists = mockFileSystem().some(f => getFilePathFromEntry(f, mockFileSystem()) === name);
    if (exists) {
      setCreationError("A file or folder with this name already exists.");
      return;
    }
    setCreationError(null);

    const newFolder: FileSystemEntry = { name, type: 'directory', children: [] };
    const updatedFileSystem = [...mockFileSystem(), newFolder];
    setMockFileSystem(updatedFileSystem);
    saveFileSystem(updatedFileSystem);
  };

  const handleMoveEntry = (sourcePath: string, destPath: string) => {
    console.log(`Moving ${sourcePath} to ${destPath}`);
    
    // Prevent moving into itself or into a child directory
    if (destPath.startsWith(sourcePath + '/') || sourcePath === destPath) {
      console.warn('Cannot move into itself or child directory');
      return;
    }

    // If sourcePath and destPath are the same, do nothing
    if (sourcePath.startsWith(destPath + '/') || sourcePath === destPath) {
      console.warn('Source and destination paths are the same');
      return;
    }
    
    // Deep copy the file system to avoid mutation issues
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    // Helper to find an entry and its parent recursively
    const findEntryAndParent = (fs: FileSystemEntry[], path: string, parentArray: FileSystemEntry[] | null = null): { entry: FileSystemEntry, parent: FileSystemEntry[] | null, index: number } | null => {
      if (path === '') return null;
      
      const parts = path.split('/');
      let currentLevel = fs;
      let currentParent: FileSystemEntry[] | null = parentArray;

      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const entryIndex = currentLevel.findIndex(e => e.name === part);
        if (entryIndex === -1) return null;

        const entry = currentLevel[entryIndex];
        
        if (i === parts.length - 1) {
          // Found the target entry
          return { entry, parent: currentParent || fs, index: entryIndex };
        }

        if (entry.type === 'directory' && entry.children) {
          currentParent = currentLevel;
          currentLevel = entry.children;
        } else {
          return null; // Path goes through a file
        }
      }
      return null;
    };

    // Find and remove the source entry
    const sourceInfo = findEntryAndParent(newFileSystem, sourcePath);
    if (!sourceInfo) {
      console.error("Source not found:", sourcePath);
      return;
    }

    const [movedEntry] = sourceInfo.parent!.splice(sourceInfo.index, 1);

    // Find destination and add the entry
    if (destPath === '') { 
      // Moving to root
      newFileSystem.push(movedEntry);
    } else {
      const destInfo = findEntryAndParent(newFileSystem, destPath);
      if (!destInfo || destInfo.entry.type !== 'directory') {
        console.error("Destination not found or is not a directory:", destPath);
        // Re-add the entry to its original position if dest is invalid
        sourceInfo.parent!.splice(sourceInfo.index, 0, movedEntry);
        return;
      }
      
      if (!destInfo.entry.children) {
        destInfo.entry.children = [];
      }
      destInfo.entry.children.push(movedEntry);
    }

    setMockFileSystem(newFileSystem);

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
    if (sourcePath !== (destPath === '' ? movedEntry.name : `${destPath}/${movedEntry.name}`)) {
      moveFileContent(
        movedEntry,
        sourcePath,
        destPath === '' ? movedEntry.name : `${destPath}/${movedEntry.name}`
      );
    }

    console.log("File system after move:", newFileSystem);
    saveFileSystem(newFileSystem);
    
    // Update openFiles to reference the moved entry at its new location
    const oldPath = sourcePath;
    const newPath = destPath === '' ? movedEntry.name : `${destPath}/${movedEntry.name}`;
    setOpenFiles(openFiles().map(f => {
      const filePath = getFilePathFromEntry(f, mockFileSystem());
      // If this open file matches the old path, find the new entry in the updated file system
      if (filePath === oldPath) {
        // Find the new entry by path in the updated file system
        const updatedEntry = findFileByPath(newFileSystem, newPath);
        return updatedEntry ? updatedEntry : f;
      }
      return f;
    }));

    // Update activeFile if it was the moved file
    if (activeFile() && getFilePathFromEntry(activeFile()!, mockFileSystem()) === oldPath) {
      const updatedActiveFile = findFileByPath(newFileSystem, newPath);
      setActiveFile(updatedActiveFile ? updatedActiveFile : null);
    }
  };

  const handleFileUpload = async (files: File[], destPath: string) => {
    let newFileSystem = JSON.parse(JSON.stringify(mockFileSystem()));

    for (const file of files) {
      const reader = new FileReader();
      reader.onload = (e) => {
        const content = e.target?.result as string;
        const newFile: FileSystemEntry = { name: file.name, type: 'file' };
        
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
                onFileUpload={handleFileUpload}
                activeFile={activeFile()}
                getFilePath={(entry) => getFilePathFromEntry(entry, mockFileSystem())}
                selectedFiles={selectedFiles()}
                onSelectionChange={setSelectedFiles}
                creationError={creationError()}
                setCreationError={setCreationError}
            />
        </Resizable.Panel>
        <Resizable.Handle class="Resizable-handle"/>
          <Resizable.Panel class="editor-panel"
          initialSize={0.55}
          minSize={0.2}>
          <FileTabs 
            openFiles={openFiles()}
            activeFile={activeFile()}
            getFilePath={(entry) => getFilePathFromEntry(entry, mockFileSystem())}
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
    </>
  );
}

