// @refresh granular
/** @jsxImportSource solid-js */
import { createSignal, createEffect, Show } from "solid-js";
import Resizable from '@corvu/resizable'

import init, { initialize, compile, run, check, start_interactive_session, get_next_interactive_states, execute_interactive_step } from '../pkg/althread_web';
import createEditor from '@components/editor/Editor';
import Graph from "@components/graph/Graph";
import { Logo } from "@assets/images/Logo";
import { renderMessageFlowGraph } from "@components/graph/CommGraph";
import { rendervmStates } from "@components/graph/vmStatesDisplay";
import { nodeToString } from "@components/graph/Node";
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import '@components/fileexplorer/FileExplorer.css';
import FileTabs from "@components/fileexplorer/FileTabs";
import Sidebar, { type SidebarView } from '@components/sidebar/Sidebar';
import InteractivePanel from '@components/interactive/InteractivePanel';

// Import our new modules
import { STORAGE_KEYS, loadFileSystem, saveFileSystem, loadFileContent, saveFileContent } from '@utils/storage';
import { getPathFromId, buildVirtualFileSystem, getFileContentFromVirtualFS } from '@utils/fileSystemUtils';  // Add buildVirtualFileSystem here
import { createFileOperationsHandlers } from '@hooks/useFileOperations';
import { createEditorManager } from '@hooks/useEditorManager';
import { MoveConfirmationDialog, DeleteConfirmationDialog } from '@components/dialogs/ConfirmationDialogs';
import { LoadExampleDialog } from '@components/dialogs/LoadExampleDialog';
import { EmptyEditor } from '@components/editor/EmptyEditor';
import { formatAlthreadError } from '@utils/error';

init().then(() => {
  console.log('loaded');
  initialize(); // Initialize the panic hook
});

const animationTimeOut = 100; //ms

export default function App() {
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

  // Sidebar view state
  const [sidebarView, setSidebarView] = createSignal<SidebarView>('explorer');
  const [sidebarCollapsed, setSidebarCollapsed] = createSignal(false);

  const toggleSidebarCollapse = () => {
    setSidebarCollapsed(prev => !prev);
  };

  // Initialize editor (no default file content)
  let editor = createEditor({
    compile: (source: string) => {
      const filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id) || editorManager.activeFile()!.name;
      const virtualFS = buildVirtualFileSystem(mockFileSystem());
      return compile(source, filePath, virtualFS); // Now compile expects virtual_fs parameter
    }, 
    defaultValue: '// Welcome to Althread\n',
    filePath: 'untitled.alt',
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
    content: string;
    fileName: string;
  }>({
    isOpen: false,
    content: '',
    fileName: ''
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

  // Load example handlers
  const handleLoadExample = (content: string, fileName: string) => {
    // Show the dialog with the loaded content
    setLoadExampleDialog({
      isOpen: true,
      content,
      fileName
    });
  };

  const handleLoadInCurrentFile = () => {
    const dialog = loadExampleDialog();
    setLoadExampleDialog({ isOpen: false, content: '', fileName: '' });
    
    // If no file is active, create a new one
    if (!editorManager.activeFile()) {
      const fileName = `${dialog.fileName.replace('.alt', '')}-${Date.now()}.alt`;
      editorManager.createNewFileWithContent(fileName, dialog.content, fileOperations, mockFileSystem);
      return;
    }
    
    // Load into current file - update both editor and saved content
    if (editor && editor.safeUpdateContent) {
      editor.safeUpdateContent(dialog.content);
    } else {
      // Fallback for older editor instances
      const up = editor.editorView().state.update({
        changes: {
          from: 0, 
          to: editor.editorView().state.doc.length,
          insert: dialog.content
        }
      });
      editor.editorView().update([up]);
    }
    
    // Also update the saved content in localStorage
    const activeFile = editorManager.activeFile();
    if (activeFile) {
      const filePath = getPathFromId(mockFileSystem(), activeFile.id) || activeFile.name;
      saveFileContent(filePath, dialog.content);
    }
  };

  const handleLoadInNewFile = () => {
    const dialog = loadExampleDialog();
    setLoadExampleDialog({ isOpen: false, content: '', fileName: '' });
    const fileName = `${dialog.fileName.replace('.alt', '')}-${Date.now()}.alt`;
    editorManager.createNewFileWithContent(fileName, dialog.content, fileOperations, mockFileSystem);
  };

  const handleCancelLoadExample = () => {
    setLoadExampleDialog({ isOpen: false, content: '', fileName: '' });
  };

  // New file prompt handlers
  const handleNewFileClick = () => {
    // If sidebar is collapsed, expand it first
    if (sidebarCollapsed()) {
      setSidebarCollapsed(false);
    }
    // Switch to explorer view and trigger global file creation state
    setSidebarView('explorer');
    setGlobalFileCreation({ type: 'file', parentPath: '' });
  };

  // Helper function to check if active file has .alt extension
  const isAltFile = () => {
    const activeFile = editorManager.activeFile();
    if (!activeFile) return false;
    return activeFile.name.endsWith('.alt');
  };

  // Save file system whenever it changes
  createEffect(() => {
    saveFileSystem(mockFileSystem());
  });

  // Reload active file content when sidebar state changes to preserve editor content
  createEffect(() => {
    // This effect runs whenever sidebarCollapsed changes
    sidebarCollapsed(); // Read the signal to create dependency
    
    // Reload the active file content after the layout change
    setTimeout(() => {
      const activeFile = editorManager.activeFile();
      if (activeFile && editor && editor.editorView) {
        const filePath = getPathFromId(mockFileSystem(), activeFile.id) || activeFile.name;
        const content = loadFileContent(filePath);
        
        // Update the editor content
        const editorView = editor.editorView();
        const transaction = editorView.state.update({
          changes: {
            from: 0,
            to: editorView.state.doc.length,
            insert: content
          }
        });
        editorView.update([transaction]);
      }
    }, 50); // Small delay to ensure DOM is updated
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
  const [executionError, setExecutionError] = createSignal(false);
  const [graphKey, setGraphKey] = createSignal(0);

  // Interactive mode state
  const [isInteractiveMode, setIsInteractiveMode] = createSignal(false);
  const [interactiveStates, setInteractiveStates] = createSignal<any[]>([]);
  const [executionHistory, setExecutionHistory] = createSignal<number[]>([]);
  const [currentVMState, setCurrentVMState] = createSignal<any>(null);
  const [interactiveFinished, setInteractiveFinished] = createSignal(false);
  const [accumulatedOutput, setAccumulatedOutput] = createSignal<string>("");
  const [accumulatedExecutionOutput, setAccumulatedExecutionOutput] = createSignal<string>("");
  const [interactiveMessageFlow, setInteractiveMessageFlow] = createSignal<any[]>([]);
  const [interactiveVmStates, setInteractiveVmStates] = createSignal<any[]>([]);

  const resetSetOut = () => {
    setOut("The execution output will appear here.");
  }

  const executeInteractiveStep = async (selectedIndex: number) => {
    try {
      if (!editorManager.activeFile()) return;
      
      const virtualFS = buildVirtualFileSystem(mockFileSystem());
      let filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id, '');
      if (!filePath) {
        filePath = editorManager.activeFile()!.name;
      }
      let stepResult;
      try {
      // Execute the step and get the output
        stepResult = execute_interactive_step(
        editor.editorView().state.doc.toString(),
        filePath,
        virtualFS, 
        executionHistory(),
        selectedIndex
      );} catch (e: any) {
        console.error("Error executing interactive step:", e);
      }

      // Add the selected step to history
      const newHistory = [...executionHistory(), selectedIndex];
      setExecutionHistory(newHistory);
      
      // Update the accumulated console output (print statements)
      const currentConsoleOutput = accumulatedOutput();
      const newStepOutput = stepResult.output || [];
      
      let updatedConsoleOutput = currentConsoleOutput;
      if (newStepOutput.length > 0) {
        updatedConsoleOutput += newStepOutput.join('\n') + '\n';
      }
      setAccumulatedOutput(updatedConsoleOutput);
      
      // Update accumulated message flow events
      const currentMessageFlow = interactiveMessageFlow();
      const newMessageFlowEvents = stepResult.message_flow_events || [];
      setInteractiveMessageFlow([...currentMessageFlow, ...newMessageFlowEvents]);
      
      // Update accumulated VM states
      const currentVmStates = interactiveVmStates();
      const newVmState = stepResult.current_state;
      if (newVmState) {
        setInteractiveVmStates([...currentVmStates, newVmState]);
      }

      // Update execution output to show step details (debug info) - accumulate all steps
      const executedStep = stepResult.executed_step;
      const debugOutput = stepResult.debug || '';
      
      if (executedStep) {
        const stepInfo = `Executed: ${executedStep.prog_name}#${executedStep.prog_id}\n${debugOutput}`;
        const currentExecutionOutput = accumulatedExecutionOutput();
        const updatedExecutionOutput = currentExecutionOutput + (currentExecutionOutput ? '\n\n' : '') + stepInfo;
        setAccumulatedExecutionOutput(updatedExecutionOutput);
        setOut(updatedExecutionOutput);
      } else {
        const currentExecutionOutput = accumulatedExecutionOutput();
        const updatedExecutionOutput = currentExecutionOutput + (currentExecutionOutput ? '\n\n' : '') + debugOutput;
        setAccumulatedExecutionOutput(updatedExecutionOutput);
        setOut(updatedExecutionOutput);
      }
      
      // Get next states with updated history
      let res = get_next_interactive_states(
        editor.editorView().state.doc.toString(), 
        filePath, 
        virtualFS, 
        newHistory
      );

      setInteractiveStates(res.get('states') || []);
      setCurrentVMState(stepResult.new_state || res.get('current_state'));
      setInteractiveFinished(res.get('is_finished'));
      
      if (res.get('is_finished')) {
        const currentExecutionOutput = accumulatedExecutionOutput();
        const finalOutput = currentExecutionOutput + (currentExecutionOutput ? '\n\n' : '') + "Program execution completed.";
        setAccumulatedExecutionOutput(finalOutput);
        setOut(finalOutput);
      } else if (!res.get('states') || res.get('states').length === 0) {
        const currentExecutionOutput = accumulatedExecutionOutput();
        const finalOutput = currentExecutionOutput + (currentExecutionOutput ? '\n\n' : '') + "No more states available.";
        setAccumulatedExecutionOutput(finalOutput);
        setOut(finalOutput);
        setInteractiveFinished(true);
      }
    } catch(e: any) {
      console.error("Interactive step error:", e);
      setOut(formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos?.file_path || "")));
      setExecutionError(true);
      // Switch to execution tab to show the error
      setActiveTab("execution");
    }
  }

  const closeInteractiveMode = () => {
    setIsInteractiveMode(false);
    setInteractiveStates([]);
    setExecutionHistory([]);
    setCurrentVMState(null);
    setInteractiveFinished(false);
    setAccumulatedOutput("");
    setAccumulatedExecutionOutput("");
    setInteractiveMessageFlow([]);
    setInteractiveVmStates([]);
    setActiveAction(null);
  }

  const resetInteractiveMode = async () => {
    if (!editorManager.activeFile()) return;
    
    try {
      setExecutionError(false);
      resetSetOut();
      setAccumulatedOutput("");
      setAccumulatedExecutionOutput("");
      setInteractiveMessageFlow([]);
      setInteractiveVmStates([]);
      
      const virtualFS = buildVirtualFileSystem(mockFileSystem());
      let filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id, '');
      if (!filePath) {
        filePath = editorManager.activeFile()!.name;
      }
      
      let res = start_interactive_session(editor.editorView().state.doc.toString(), filePath, virtualFS);
      
      setInteractiveStates(res.get('states') || []);
      setCurrentVMState(res.get('current_state'));
      setInteractiveFinished(res.get('is_finished'));
      setExecutionHistory([]);
      
      if (res.get('is_finished')) {
        setOut("Program execution completed immediately.");
      } else if (!res.get('states') || res.get('states').length === 0) {
        setOut("No more states available.");
        setInteractiveFinished(true);
      } else {
        setOut("Interactive session restarted. Choose the next instruction to execute.");
      }
    } catch(e: any) {
      console.error("Interactive reset error:", e);
      setOut(formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos?.file_path || "")));
      setExecutionError(true);
    }
  }

  const resetAllState = async () => {
    setExecutionError(false);
    setIsRun(true);
    setIsInteractiveMode(false);
    resetSetOut();
    setStdout("The console output will appear here.");
    setCommGraphOut([]);
    setNodes([]);
    setEdges([]);
    setVmStates([]);
    setActiveAction(null);
    // Reset interactive mode state
    setInteractiveStates([]);
    setExecutionHistory([]);
    setCurrentVMState(null);
    setInteractiveFinished(false);
    setAccumulatedOutput("");
    setAccumulatedExecutionOutput("");
    setInteractiveMessageFlow([]);
    setInteractiveVmStates([]);
  }

  const renderExecContent = () => {
    // Don't render inline interactive content anymore - it's handled by the popup panel
    if (activeTab() === "execution") {
        return (
          <div class="console">
            {executionError() ? (
              <div class="execution-error-box">
                <pre>{out()}</pre>
              </div>
            ) : (
              <pre>{out()}</pre>
            )}
          </div>
        );
      } else
    if (isRun()) {
      if (activeTab() === "console") {
        return (
          <div class="console">
            <pre>{stdout()}</pre>
          </div>
        );
      } if (activeTab() === "msg_flow") {
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
      return (
        <div class="console">
          <Graph key={graphKey()} nodes={nodes()} edges={edges()} vm_states={vm_states()} setLoadingAction={setLoadingAction} theme="dark" />
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
              class={`vscode-button${activeAction() === "interactive" ? " active" : ""}`}
              disabled={loadingAction() === "interactive" || !editorManager.activeFile() || !isAltFile()}
              onClick={async () => {
                if (!editorManager.activeFile()) return;
                if (activeAction() !== "interactive") setActiveAction("interactive");
                if (loadingAction() !== "interactive") setLoadingAction("interactive");
                
                try {
                  setIsInteractiveMode(true);
                  setExecutionError(false);
                  resetSetOut();
                  // Reset accumulated output for new session
                  setAccumulatedOutput("");
                  // Go to console by default, execution only if there are errors
                  setActiveTab("console");
                  
                  const virtualFS = buildVirtualFileSystem(mockFileSystem());
                  let filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id, '');
                  if (!filePath) {
                    filePath = editorManager.activeFile()!.name;
                  }
                  
                  let res = start_interactive_session(editor.editorView().state.doc.toString(), filePath, virtualFS);
                  setInteractiveStates(res.get('states') || []);
                  setCurrentVMState(res.get('current_state'));
                  setInteractiveFinished(res.get('is_finished'));
                  setExecutionHistory([]);
                  
                  if (res.get('is_finished')) {
                    setOut("Program execution completed.");
                  } else if (!res.get('states') || res.get('states').length === 0) {
                    setOut("No interactive choices available.");
                  } else {
                    setOut("Interactive mode started. Select the next instruction to execute.");
                  }
                } catch(e: any) {
                  console.error("Interactive mode error:", e);
                  setOut(formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos?.file_path || "")));
                  setExecutionError(true);
                  setIsInteractiveMode(false);
                  // Switch to execution tab to show the error
                  setActiveTab("execution");
                } finally {
                  setTimeout(() => {
                    setLoadingAction(null);
                  }, animationTimeOut);
                }
              }}>
              <i class={loadingAction() === "interactive" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-debug-step-over"}></i>
              Interactive
            </button>

            <button
              class={`vscode-button${activeAction() === "run" ? " active" : ""}`}
              disabled={loadingAction() === "run" || !editorManager.activeFile() || !isAltFile()}
              onClick={async () => {
                if (!editorManager.activeFile()) return;
                if (!isRun()) setIsRun(true);
                setExecutionError(false);
                resetSetOut();
                if (activeAction() !== "run") setActiveAction("run");
                if (loadingAction() !== "run") setLoadingAction("run");
                try {
                  
                  const virtualFS = buildVirtualFileSystem(mockFileSystem());
                  let filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id, '');
                  if (!filePath) {
                    filePath = editorManager.activeFile()!.name; // Fallback to name if ID not found
                  }
                  let res = run(editor.editorView().state.doc.toString(), filePath, virtualFS); 
                  console.log(res);
                  if (res.debug.length === 0) {
                    resetSetOut();
                  } else {
                    setOut(res.debug);
                  }
                  console.log(res.message_flow_graph);
                  setCommGraphOut(res.message_flow_graph);
                  setVmStates(res.vm_states);
                  setStdout(res.stdout.join('\n'));
                  setActiveTab("console");
                } catch(e: any) {
                  console.error("Execution error:", e);
                  // show error in execution tab
                  setOut(formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos.file_path)));
                  setActiveTab("execution");
                  // reset other tabs to initial state
                  setStdout("The console output will appear here.");
                  setCommGraphOut([]);
                  setVmStates([]);
                  setExecutionError(true);
                } finally {
                  setTimeout(() => {
                    setLoadingAction(null);
                  }, animationTimeOut);
                }
              }}>
              <i class={loadingAction() === "run" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-play"}></i>
              Run
            </button>

            <button
              class={`vscode-button${activeAction() === "check" ? " active" : ""}`}
              disabled={!editorManager.activeFile() || !isAltFile()}
              onClick={() => {
                if (loadingAction() !== "check") setLoadingAction("check");
                if (activeAction() !== "check") setActiveAction("check");
                setActiveTab("vm_states");
                if (executionError()) setExecutionError(false);
                if (!editorManager.activeFile()) return;

                // Increment key to force re-render of Graph component
                setGraphKey(k => k + 1);
                
                try {
                  const virtualFS = buildVirtualFileSystem(mockFileSystem());

                  let filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id, '');
                  if (!filePath) {
                    filePath = editorManager.activeFile()!.name; // Fallback to name if ID not found
                  }

                  let res = check(editor.editorView().state.doc.toString(), filePath, virtualFS);
                  setOut("No execution errors found.");
                  
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
                      isViolationNode,
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
                  // show error in execution tab
                  setOut(formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos.file_path)));
                  setActiveTab("execution");
                  setLoadingAction(null);
                  // reset other tabs to initial state
                  setStdout("The console output will appear here.");
                  setCommGraphOut([]);
                  setVmStates([]);
                  setExecutionError(true);
                }
              }}>
              <i class={loadingAction() === "check" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-check"}></i>
              Check
            </button>

            <button
              class={`vscode-button${loadingAction() === "reset" ? " active" : ""}`}
              onClick={async () => {
                setLoadingAction("reset");
                try {
                  await resetAllState();
                } finally {
                  setTimeout(() => {
                    setLoadingAction(null);
                  }, 100);
                }
              }}>
              <i class={loadingAction() === "reset" ? "codicon codicon-loading codicon-modifier-spin" : "codicon codicon-clear-all"}></i>
              Reset
            </button>
          </div>
      </div>
      
      {/* Collapsed sidebar positioned absolutely */}
      {sidebarCollapsed() && (
        <div class="collapsed-sidebar-container">
          <Sidebar
            files={mockFileSystem()} 
            onFileSelect={(path) => editorManager.handleFileSelect(path, mockFileSystem())}
            onNewFile={fileOperations.handleNewFile}
            onNewFolder={fileOperations.handleNewFolder}
            onMoveEntry={fileOperations.handleMoveEntry}
            onRenameEntry={fileOperations.handleRenameEntry}
            onDeleteEntry={fileOperations.handleDeleteEntry}
            onCopyEntry={fileOperations.handleCopyEntry}
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
            setFileSystem={setMockFileSystem}
            onLoadExample={handleLoadExample}
            activeView={sidebarView()}
            onViewChange={setSidebarView}
            isCollapsed={sidebarCollapsed()}
            onToggleCollapse={toggleSidebarCollapse}
          />
        </div>
      )}

      <div class={`content-wrapper ${sidebarCollapsed() ? 'sidebar-collapsed' : ''}`}>
        <Show when={sidebarCollapsed()} fallback={
          // Expanded layout: 3 panels (sidebar + editor + right)
          <Resizable id="content">
            <Resizable.Panel initialSize={0.20} minSize={0.20}>
                <Sidebar
                    files={mockFileSystem()} 
                    onFileSelect={(path) => editorManager.handleFileSelect(path, mockFileSystem())}
                    onNewFile={fileOperations.handleNewFile}
                    onNewFolder={fileOperations.handleNewFolder}
                    onMoveEntry={fileOperations.handleMoveEntry}
                    onRenameEntry={fileOperations.handleRenameEntry}
                    onDeleteEntry={fileOperations.handleDeleteEntry}
                    onCopyEntry={fileOperations.handleCopyEntry}
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
                    setFileSystem={setMockFileSystem}
                    onLoadExample={handleLoadExample}
                    activeView={sidebarView()}
                    onViewChange={setSidebarView}
                    isCollapsed={sidebarCollapsed()}
                    onToggleCollapse={toggleSidebarCollapse}
                />
            </Resizable.Panel>
            <Resizable.Handle class="Resizable-handle"/>
            
            <Resizable.Panel 
              class="editor-panel"
              initialSize={0.50}
              minSize={0.2}
            >
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
            
            <Resizable.Panel 
              class="right-panel"
              initialSize={0.30}
              minSize={0.2}
            >
              <div class="execution-content">
                <div class="tab">
                    <button class={`tab_button ${activeTab() === "console" ? "active" : ""}`}
                            onclick={() => handleExecutionTabClick("console")}
                            disabled={!isRun()}
                    >
                    <i class="codicon codicon-terminal"></i> Console
                    </button>
                    <button
                      class={`tab_button 
                               ${activeTab()   === "execution" ? "active"          : ""} 
                               ${executionError()              ? "execution-error" : ""}`}
                      onclick={() => handleExecutionTabClick("execution")}
                    >
                    <i class="codicon codicon-play"></i> Execution
                    </button>
                    <button class={`tab_button ${activeTab() === "msg_flow" ? "active" : ""}`}
                            onclick={() => handleExecutionTabClick("msg_flow")}
                            disabled={!isRun()}
                    >
                    <i class="codicon codicon-send"></i> Message flow
                    </button>
                    <button class={`tab_button ${activeTab() === "vm_states" ? "active" : ""}`}
                            onclick={() => handleExecutionTabClick("vm_states")}
                    >
                    <i class="codicon codicon-type-hierarchy-sub"></i> VM states
                    </button>
                </div>

                <div class="tab-content">
                    {renderExecContent()}
                </div>
              </div>
            </Resizable.Panel>
          </Resizable>
        }>
          {/* Collapsed layout: 2 panels (editor + right) */}
          <Resizable id="content-collapsed">
            <Resizable.Panel 
              class="editor-panel"
              initialSize={0.70}
              minSize={0.2}
            >
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
            
            <Resizable.Panel 
              class="right-panel"
              initialSize={0.30}
              minSize={0.2}
            >
              <div class="execution-content">
                <div class="tab">
                    <button class={`tab_button ${activeTab() === "console" ? "active" : ""}`}
                            onclick={() => handleExecutionTabClick("console")}
                            disabled={!isRun()}
                    >
                    <i class="codicon codicon-terminal"></i> Console
                    </button>
                    <button
                      class={`tab_button 
                               ${activeTab()   === "execution" ? "active"          : ""} 
                               ${executionError()              ? "execution-error" : ""}`}
                      onclick={() => handleExecutionTabClick("execution")}
                      // disabled={!isRun()}
                    >
                    <i class="codicon codicon-play"></i> Execution
                    </button>
                    <button class={`tab_button ${activeTab() === "msg_flow" ? "active" : ""}`}
                            onclick={() => handleExecutionTabClick("msg_flow")}
                            disabled={!isRun()}
                    >
                    <i class="codicon codicon-send"></i> Message flow
                    </button>
                    <button class={`tab_button ${activeTab() === "vm_states" ? "active" : ""}`}
                            onclick={() => handleExecutionTabClick("vm_states")}
                    >
                    <i class="codicon codicon-type-hierarchy-sub"></i> VM states
                    </button>
                </div>

                <div class="tab-content">
                    {renderExecContent()}
                </div>
              </div>
            </Resizable.Panel>
          </Resizable>
        </Show>
      </div>

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

      {/* Interactive Panel */}
      <InteractivePanel
        isVisible={isInteractiveMode()}
        interactiveStates={interactiveStates()}
        currentVMState={currentVMState()}
        isFinished={interactiveFinished()}
        executionOutput={out()}
        executionError={executionError()}
        onExecuteStep={executeInteractiveStep}
        onClose={closeInteractiveMode}
        onReset={resetInteractiveMode}
        stdout={accumulatedOutput()}
        commGraphOut={commgraphout()}
        vmStates={vm_states()}
        isRun={isRun()}
        interactiveMessageFlow={interactiveMessageFlow()}
        interactiveVmStates={interactiveVmStates()}
      />
    </>
  );
}
