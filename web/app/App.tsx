// @refresh granular
/** @jsxImportSource solid-js */
import { createSignal, createEffect } from "solid-js";
import Resizable from '@corvu/resizable'
import { Example1 } from "@examples/example1";
import { useNavigate } from "@solidjs/router";

import init, { compile, run, check } from '../pkg/althread_web';
import createEditor from '@components/editor/Editor';
import Graph from "@components/graph/Graph";
import { Logo } from "@assets/images/Logo";
import { renderMessageFlowGraph } from "@components/graph/CommGraph";
import { rendervmStates } from "@components/graph/vmStatesDisplay";
import { nodeToString } from "@components/graph/Node";
import FileExplorer from '@components/fileexplorer/FileExplorer';
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import '@components/fileexplorer/FileExplorer.css';
import FileTabs from "@components/fileexplorer/FileTabs";

// Import our new modules
import { STORAGE_KEYS, loadFileSystem, saveFileSystem, loadFileContent, saveFileContent } from '@utils/storage';
import { getPathFromId, buildVirtualFileSystem } from '@utils/fileSystemUtils';  // Add buildVirtualFileSystem here
import { createFileOperationsHandlers } from '@hooks/useFileOperations';
import { createEditorManager } from '@hooks/useEditorManager';
import { MoveConfirmationDialog, DeleteConfirmationDialog } from '@components/dialogs/ConfirmationDialogs';
import { LoadExampleDialog } from '@components/dialogs/LoadExampleDialog';
import { EmptyEditor } from '@components/editor/EmptyEditor';

init().then(() => {
  console.log('loaded');
});

const animationTimeOut = 100; //ms

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
  const [selectedFiles, setSelectedFiles] = createSignal<string[]>([]);
  const [creationError, setCreationError] = createSignal<string | null>(null);
  
  // Global file creation state - shared between FileExplorer and EmptyEditor
  const [globalFileCreation, setGlobalFileCreation] = createSignal<{ type: 'file' | 'folder', parentPath: string } | null>(null);

  // Initialize editor (no default file content)
  let editor = createEditor({
    compile: (source: string) => {
      const virtualFS = buildVirtualFileSystem(mockFileSystem());
      return compile(source, virtualFS); // Now compile expects virtual_fs parameter
    }, 
    defaultValue: '// Welcome to Althread\n',
    fileName: 'untitled.alt',
    onValueChange: (value) => {
      // Save current file content when editor changes
      // Use a delayed check since editorManager might not be initialized yet
      setTimeout(() => {
        if (editorManager && editorManager.activeFile && editorManager.activeFile()) {
          const filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id) || editorManager.activeFile()!.name;
          saveFileContent(filePath, value);
        }
      }, 0);
    }
  });

  // Initialize editor manager
  const editorManager = createEditorManager(editor);

  // Initialize file operations handlers
  const fileOperations = createFileOperationsHandlers(
    mockFileSystem,
    setMockFileSystem,
    setCreationError,
    editorManager.openFiles,
    editorManager.setOpenFiles,
    editorManager.activeFile,
    editorManager.setActiveFile,
    selectedFiles,
    setSelectedFiles,
    editor,
    loadFileContent
  );

  // No automatic file opening - let user choose what to open

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

  // Load example dialog state
  const [loadExampleDialog, setLoadExampleDialog] = createSignal<{
    isOpen: boolean;
  }>({
    isOpen: false
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
    
    // Execute the move with replacement for each source path
    confirmation.sourcePaths.forEach(sourcePath => {
      fileOperations.handleMoveWithReplacement(sourcePath, confirmation.destPath, confirmation.conflictingName);
    });
    
    setMoveConfirmation({ isOpen: false, sourcePaths: [], destPath: '', conflictingName: '' });
  };

  const handleCanceledMove = () => {
    setMoveConfirmation({ isOpen: false, sourcePaths: [], destPath: '', conflictingName: '' });
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
      fileOperations.handleDeleteEntry(path);
    });
    setDeleteConfirmation({ isOpen: false, paths: [] });
  };

  const handleCanceledDelete = () => {
    setDeleteConfirmation({ isOpen: false, paths: [] });
  };

  // Load example dialog handlers
  const handleLoadExampleClick = () => {
    // If no file is active, directly create a new file with the example
    if (!editorManager.activeFile()) {
      const fileName = `example-${Date.now()}.alt`;
      editorManager.createNewFileWithContent(fileName, Example1, fileOperations, mockFileSystem);
      return;
    }
    
    // Otherwise show the dialog to choose
    setLoadExampleDialog({ isOpen: true });
  };

  const handleLoadInCurrentFile = () => {
    setLoadExampleDialog({ isOpen: false });
    
    // If no file is active, create a new one
    if (!editorManager.activeFile()) {
      const fileName = `example-${Date.now()}.alt`;
      editorManager.createNewFileWithContent(fileName, Example1, fileOperations, mockFileSystem);
      return;
    }
    
    // Load into current file - update both editor and saved content
    if (editor && editor.safeUpdateContent) {
      editor.safeUpdateContent(Example1);
    } else {
      // Fallback for older editor instances
      const up = editor.editorView().state.update({
        changes: {
          from: 0, 
          to: editor.editorView().state.doc.length,
          insert: Example1
        }
      });
      editor.editorView().update([up]);
    }
    
    // Also update the saved content in localStorage
    const activeFile = editorManager.activeFile();
    if (activeFile) {
      const filePath = getPathFromId(mockFileSystem(), activeFile.id) || activeFile.name;
      saveFileContent(filePath, Example1);
    }
  };

  const handleLoadInNewFile = () => {
    setLoadExampleDialog({ isOpen: false });
    const fileName = `example-${Date.now()}.alt`;
    editorManager.createNewFileWithContent(fileName, Example1, fileOperations, mockFileSystem);
  };

  const handleCancelLoadExample = () => {
    setLoadExampleDialog({ isOpen: false });
  };

  // New file prompt handlers
  const handleNewFileClick = () => {
    // Trigger global file creation state to show the inline input in FileExplorer
    setGlobalFileCreation({ type: 'file', parentPath: '' });
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
    return null; // fallback return
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
                  handleLoadExampleClick();
              } catch (error) {
                console.error("Error showing load example dialog:", error);
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
              disabled={loadingAction() === "run" || !editorManager.activeFile()}
              onClick={async () => {
                if (!editorManager.activeFile()) return;
                
                setLoadingAction("run");
                try {
                  setIsRun(true);
                  
                  const virtualFS = buildVirtualFileSystem(mockFileSystem());
                  let res = run(editor.editorView().state.doc.toString(), virtualFS); // Pass virtualFS
                  
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
              disabled={!editorManager.activeFile()}
              onClick={() => {
                if (!editorManager.activeFile()) return;
                
                setActiveAction(activeAction() === "check" ? null : "check");
                try {
                  const virtualFS = buildVirtualFileSystem(mockFileSystem());
                  let res = check(editor.editorView().state.doc.toString(), virtualFS) // Add virtualFS parameter to check() call too
                  setOut(res);
                  
                  console.log(res);
                  let colored_path: string[] = [];
                  if(res[0].length > 0) { // a violation occurred
                    res[0].forEach((path: any) => {
                      colored_path.push(nodeToString(path.to));
                    });
                  }

                  let nodes: Record<string, number> = {};
                  setNodes(res[1].nodes.map((n: any, i: number) => {
                    let label = nodeToString(n[0]);
                    const {level} = n[1];
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
                  res[1].nodes.forEach((n: any, i: number) => {
                    const {successors} = n[1];
                    successors.forEach(({lines, pid, name, to}: any) => {
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
                onFileSelect={(path) => editorManager.handleFileSelect(path, mockFileSystem())}
                onNewFile={fileOperations.handleNewFile}
                onNewFolder={fileOperations.handleNewFolder}
                onMoveEntry={fileOperations.handleMoveEntry}
                onRenameEntry={fileOperations.handleRenameEntry}
                onDeleteEntry={fileOperations.handleDeleteEntry}
                onFileUpload={fileOperations.handleFileUpload}
                activeFile={editorManager.activeFile()}
                getFilePath={(entry) => getPathFromId(mockFileSystem(), entry.id) || entry.name}
                selectedFiles={selectedFiles()}
                onSelectionChange={setSelectedFiles}
                creationError={creationError()}
                setCreationError={setCreationError}
                checkNameConflict={checkNameConflict}
                showConfirmDialog={showMoveConfirmDialog}
                showDeleteConfirmDialog={showDeleteConfirmDialog}
                globalFileCreation={globalFileCreation()}
                setGlobalFileCreation={setGlobalFileCreation}
            />
        </Resizable.Panel>
        <Resizable.Handle class="Resizable-handle"/>
          <Resizable.Panel class="editor-panel"
          initialSize={0.55}
          minSize={0.2}>
          <FileTabs 
            openFiles={editorManager.openFiles()}
            activeFile={editorManager.activeFile()}
            getFilePath={(entry) => getPathFromId(mockFileSystem(), entry.id) || entry.name}
            onTabClick={(file) => editorManager.handleFileTabClick(file, mockFileSystem())}
            onTabClose={(file) => editorManager.handleTabClose(file, mockFileSystem())}
          />
          {editorManager.activeFile() ? (
            <div class="editor-instance-wrapper" ref={editor.ref} />
          ) : (
            <EmptyEditor onNewFile={handleNewFileClick} />
          )}
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

      {/* Load Example Dialog */}
      <LoadExampleDialog
        isOpen={loadExampleDialog().isOpen}
        onLoadInCurrent={handleLoadInCurrentFile}
        onLoadInNew={handleLoadInNewFile}
        onCancel={handleCancelLoadExample}
      />
    </>
  );
}
