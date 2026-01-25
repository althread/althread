// @refresh granular
import { createSignal, createEffect, Show, Switch, Match, onCleanup } from "solid-js";
import Resizable from '@corvu/resizable'

import init, { initialize, start_interactive_session, get_next_interactive_states, execute_interactive_step } from '../pkg/althread_web';
import createEditor from '@components/editor/Editor';
import Graph from "@components/graph/Graph";
import { Logo } from "@assets/images/Logo";
import { renderMessageFlowGraph } from "@components/graph/CommGraph";
import { nodeToString } from "@components/graph/Node";
import type { FileSystemEntry } from '@components/fileexplorer/FileExplorer';
import '@components/fileexplorer/FileExplorer.css';
import FileTabs from "@components/fileexplorer/FileTabs";
import Sidebar, { type SidebarView } from '@components/sidebar/Sidebar';
import InteractivePanel from '@components/interactive/InteractivePanel';
import VMStateInspector from "@components/graph/VMStateInspector";

// Import our new modules
import { loadFileSystem, saveFileSystem, loadFileContent, saveFileContent } from '@utils/storage';
import { getPathFromId, buildVirtualFileSystem, getFileContentFromVirtualFS, findFileByPath } from '@utils/fileSystemUtils';  // Add buildVirtualFileSystem here
import { createFileOperationsHandlers } from '@hooks/useFileOperations';
import { createEditorManager } from '@hooks/useEditorManager';
import { MoveConfirmationDialog, DeleteConfirmationDialog } from '@components/dialogs/ConfirmationDialogs';
import { LoadExampleDialog } from '@components/dialogs/LoadExampleDialog';
import { EmptyEditor } from '@components/editor/EmptyEditor';
import { formatAlthreadError } from '@utils/error';
import ErrorDisplay from '@components/error/ErrorDisplay';
import { workerClient } from '@utils/workerClient';
import { buildGraphFromNodes } from '@utils/graphBuilders';
import type { GraphNode, RunResult, CheckResult, VMStateSelection } from './types/vm-state';

init().then(() => {
  console.log('loaded');
  initialize(); // Initialize the panic hook
});

const animationTimeOut = 100; //ms

export default function App() {
  // Load file system from localStorage
  let initialFileSystem = loadFileSystem();

  const [mockFileSystem, setMockFileSystem] = createSignal<FileSystemEntry[]>(initialFileSystem);
  const [selectedFiles, setSelectedFiles] = createSignal<string[]>([]);
  const [creationError, setCreationError] = createSignal<string | null>(null);
  const [didAutoOpenDefault, setDidAutoOpenDefault] = createSignal(false);
  
  // Global file creation state - shared between FileExplorer and EmptyEditor
  const [globalFileCreation, setGlobalFileCreation] = createSignal<{ type: 'file' | 'folder', parentPath: string } | null>(null);

  // Sidebar view state
  const [sidebarView, setSidebarView] = createSignal<SidebarView>('help');
  const [sidebarCollapsed, setSidebarCollapsed] = createSignal(false);

  const toggleSidebarCollapse = () => {
    setSidebarCollapsed(prev => !prev);
  };

  // Initialize editor (no default file content)
  let editor = createEditor({
    compile: async (source: string) => {
      const activeFile = editorManager.activeFile();
      if (!activeFile) return null;
      const filePath = getPathFromId(mockFileSystem(), activeFile.id) || activeFile.name;
      const virtualFS = buildVirtualFileSystem(mockFileSystem());
      return await workerClient.compile(source, filePath, virtualFS); 
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

  // Auto-open main.alt by default (once), so the editor isn't empty on load.
  createEffect(() => {
    if (didAutoOpenDefault()) return;
    if (editorManager.activeFile()) {
      setDidAutoOpenDefault(true);
      return;
    }

    const fs = mockFileSystem();
    const main = findFileByPath(fs, 'main.alt');

    if (main && main.type === 'file') {
      editorManager.handleFileSelect('main.alt', fs);
      setDidAutoOpenDefault(true);
      return;
    }

    // Fallback: if there's exactly one file in root, open it.
    const singleRootFile = fs.length === 1 && fs[0].type === 'file' ? fs[0] : null;
    if (singleRootFile) {
      const filePath = getPathFromId(fs, singleRootFile.id) || singleRootFile.name;
      editorManager.handleFileSelect(filePath, fs);
    }
    setDidAutoOpenDefault(true);
  });

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

  let [nodes, setNodes] = createSignal<any[]>([]);
  let [edges, setEdges] = createSignal<any[]>([]);
  let [isRun, setIsRun] = createSignal(true);

  let [stdout, setStdout] = createSignal("The console output will appear here.");
  let [out, setOut] = createSignal("The execution output will appear here.");
  let [commgraphout, setCommGraphOut] = createSignal<any[]>([]); //messageflow graph
  let [runGraphNodes, setRunGraphNodes] = createSignal<GraphNode[]>([]); // For run mode - stores graph nodes
  let [runBuiltGraph, setRunBuiltGraph] = createSignal<{ nodes: any[], edges: any[] }>({ nodes: [], edges: [] }); // Built graph for vis.js
  let [stepLines, setStepLines] = createSignal<number[][]>([]); //to store lines for each step
  let [activeAction, setActiveAction] = createSignal<string | null>(null);
  const [loadingAction, setLoadingAction] = createSignal<string | null>(null);
  const [executionError, setExecutionError] = createSignal(false);
  const [structuredError, setStructuredError] = createSignal<any>(null);
  const [selectedVM, setSelectedVM] = createSignal<VMStateSelection | null>(null);

  const selectRunVmByIndex = (index: number) => {
    const nodes = runGraphNodes();
    if (!nodes || nodes.length === 0) return;
    const clamped = Math.max(0, Math.min(index, nodes.length - 1));
    setSelectedVM({ vm: nodes[clamped].vm, stepIndex: clamped });
  };

  // Highlight lines in editor when a VM state is selected
  createEffect(() => {
    const selection = selectedVM();
    if (!selection || !editor || !editor.highlightLines) {
      if (editor) editor.clearHighlights?.();
      return;
    }

    const vmState = selection.vm;
    
    if (vmState && vmState.locals && vmState.locals.length > 0) {
      // Collect lines/labels from all programs in the current VM state
      const specs = vmState.locals
        .map((p: any) => ({ line: p.line, label: `${p.name}#${p.pid}` }))
        .filter((s: any) => typeof s.line === 'number' && s.line > 0);
        
      if (specs.length > 0) {
        editor.highlightLines(specs);
        return;
      }
    }

    // If we have stepIndex (from Run mode), use stepLines as fallback
    if (selection.stepIndex !== undefined) {
      const lines = stepLines()[selection.stepIndex];
      if (lines && lines.length > 0) {
        editor.highlightLines(lines);
        return;
      }
    }
    
    editor.clearHighlights?.();
  });

  createEffect(() => {
    if (!isRun() || activeTab() !== "vm_states") return;

    const handler = (event: KeyboardEvent) => {
      if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;

      const nodes = runGraphNodes();
      if (!nodes || nodes.length === 0) return;

      const vm = selectedVM();
      let currentIndex = vm?.stepIndex ?? 0;
      if (currentIndex < 0) currentIndex = 0;

      const nextIndex = event.key === "ArrowRight" ? currentIndex + 1 : currentIndex - 1;
      selectRunVmByIndex(nextIndex);
      event.preventDefault();
    };

    window.addEventListener("keydown", handler);
    onCleanup(() => window.removeEventListener("keydown", handler));
  });

  // Interactive mode state
  const [isInteractiveMode, setIsInteractiveMode] = createSignal(false);
  const [interactiveStates, setInteractiveStates] = createSignal<any[]>([]);
  const [executionHistory, setExecutionHistory] = createSignal<number[]>([]);
  const [currentVMState, setCurrentVMState] = createSignal<any>(null);
  const [interactiveFinished, setInteractiveFinished] = createSignal(false);
  const [interactiveStepLines, setInteractiveStepLines] = createSignal<number[][]>([]);
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

      // Update interactive step lines
      const currentInteractiveStepLines = interactiveStepLines();
      const newStepLines = stepResult.lines || [];
      setInteractiveStepLines([...currentInteractiveStepLines, newStepLines]);

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
      const errorInfo = formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos?.file_path || ""));
      setStructuredError(errorInfo);
      setOut(errorInfo.message);
      setExecutionError(true);
      // Switch to execution tab to show the error
      setActiveTab("execution");
    }
  }

  // Function to handle file opening from error display
  const handleErrorFileClick = (filePath: string) => {
    // Find the file in the file system and open it
    const file = findFileByPath(mockFileSystem(), filePath);
    if (file) {
      editorManager.handleFileSelect(filePath, mockFileSystem());
      
      // If sidebar is collapsed, expand it to show the file
      if (sidebarCollapsed()) {
        setSidebarCollapsed(false);
      }
      setSidebarView('explorer');
    } else {
      console.warn(`File not found: ${filePath}`);
    }
  };

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
      setInteractiveStepLines([]);
      
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
      const errorInfo = formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos?.file_path || ""));
      setStructuredError(errorInfo);
      setOut(errorInfo.message);
      setExecutionError(true);
    }
  }

  const resetAllState = async () => {
    setExecutionError(false);
    setStructuredError(null);
    setIsRun(true);
    setIsInteractiveMode(false);
    resetSetOut();
    setStdout("The console output will appear here.");
    setCommGraphOut([]);
    setNodes([]);
    setEdges([]);
    setRunGraphNodes([]);
    setRunBuiltGraph({ nodes: [], edges: [] });
    setActiveAction(null);
    setSelectedVM(null);
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

  const renderExecContent = () => (
    <Switch>
      <Match when={activeTab() === "execution"}>
        <div class="console">
          {executionError() && structuredError() ? (
            <div class="execution-error-box">
              <ErrorDisplay 
                error={structuredError()} 
                onFileClick={handleErrorFileClick} 
              />
            </div>
          ) : executionError() ? (
            <div class="execution-error-box">
              <pre>{out()}</pre>
            </div>
          ) : (
            <pre>{out()}</pre>
          )}
        </div>
      </Match>

      <Match when={isRun() && activeTab() === "console"}>
        <div class="console">
          <pre>{stdout()}</pre>
        </div>
      </Match>

      <Match when={isRun() && activeTab() === "msg_flow"}>
        <div class="console">
          {renderMessageFlowGraph(commgraphout(), [], editor)}
        </div>
      </Match>

      <Match when={isRun() && activeTab() === "vm_states"}>
        <div class="console">
          <Resizable id="vm-states-layout" orientation="vertical" style={{ height: "100%", width: "100%", display: "flex", "flex-direction": "column" }}>
            <Resizable.Panel initialSize={0.4} minSize={0.2} style={{ display: "flex", "flex-direction": "column", overflow: "hidden" }}>
              <VMStateInspector node={selectedVM()} onClose={() => setSelectedVM(null)} />
            </Resizable.Panel>
            <Resizable.Handle class="Resizable-handle" />
            <Resizable.Panel initialSize={0.6} minSize={0.2} style={{ display: "flex", "flex-direction": "column", overflow: "hidden" }}>
              <Graph
                nodes={runBuiltGraph().nodes}
                edges={runBuiltGraph().edges}
                setLoadingAction={setLoadingAction}
                theme="dark"
                onEdgeClick={(_edgeId: string, edgeData: any) => {
                  if (edgeData && edgeData.lines && editor.highlightLines) {
                    editor.highlightLines(edgeData.lines);
                  }
                }}
                onNodeSelect={setSelectedVM}
              />
            </Resizable.Panel>
          </Resizable>
        </div>
      </Match>

      <Match when={!isRun()}>
        <div class="console">
          <Resizable id="checker-states-layout" orientation="vertical" style={{ height: "100%", width: "100%", display: "flex", "flex-direction": "column" }}>
            <Resizable.Panel initialSize={0.4} minSize={0.2} style={{ display: "flex", "flex-direction": "column", overflow: "hidden" }}>
              <VMStateInspector node={selectedVM()} onClose={() => setSelectedVM(null)} />
            </Resizable.Panel>
            <Resizable.Handle class="Resizable-handle" />
            <Resizable.Panel initialSize={0.6} minSize={0.2} style={{ display: "flex", "flex-direction": "column", overflow: "hidden" }}>
              <Graph 
                nodes={nodes()} 
                edges={edges()} 
                setLoadingAction={setLoadingAction} 
                theme="dark" 
                onEdgeClick={(_edgeId: string, edgeData: any) => {
                    if (edgeData && edgeData.lines && editor.highlightLines) {
                        editor.highlightLines(edgeData.lines);
                    }
                }}
                onNodeSelect={setSelectedVM}
              />
            </Resizable.Panel>
          </Resizable>
        </div>
      </Match>
    </Switch>
  );

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
                  const errorInfo = formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos?.file_path || ""));
                  setStructuredError(errorInfo);
                  setOut(errorInfo.message);
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
                setSelectedVM(null);
                try {
                  
                  const virtualFS = buildVirtualFileSystem(mockFileSystem());
                  let filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id, '');
                  if (!filePath) {
                    filePath = editorManager.activeFile()!.name; // Fallback to name if ID not found
                  }
                  let res: RunResult = await workerClient.run(editor.editorView().state.doc.toString(), filePath, virtualFS); 
                  if (res.debug.length === 0) {
                    resetSetOut();
                  } else {
                    setOut(res.debug);
                  }
                  setCommGraphOut(res.message_flow_events);
                  setRunGraphNodes(res.nodes);
                  setStepLines(res.step_lines || []);
                  
                  // Build the graph for visualization
                  const builtGraph = buildGraphFromNodes(res.nodes, { mode: 'run', stepLines: res.step_lines || [] });
                  setRunBuiltGraph(builtGraph);
                  
                  setStdout(res.stdout.join('\n'));
                  setActiveTab("console");
                } catch(e: any) {
                  console.error("Execution error:", e);
                  // show error in execution tab
                  const errorInfo = formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos?.file_path || ""));
                  setStructuredError(errorInfo);
                  setOut(errorInfo.message);
                  setActiveTab("execution");
                  // reset other tabs to initial state
                  setStdout("The console output will appear here.");
                  setCommGraphOut([]);
                  setRunGraphNodes([]);
                  setRunBuiltGraph({ nodes: [], edges: [] });
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
              onClick={async () => {
                if (loadingAction() !== "check") setLoadingAction("check");
                if (activeAction() !== "check") setActiveAction("check");
                setSelectedVM(null);
                setActiveTab("vm_states");
                if (executionError()) setExecutionError(false);
                if (!editorManager.activeFile()) return;
                
                try {
                  const virtualFS = buildVirtualFileSystem(mockFileSystem());

                  let filePath = getPathFromId(mockFileSystem(), editorManager.activeFile()!.id, '');
                  if (!filePath) {
                    filePath = editorManager.activeFile()!.name; // Fallback to name if ID not found
                  }

                  let res: CheckResult = await workerClient.check(editor.editorView().state.doc.toString(), filePath, virtualFS);
                  
                  console.log(res);
                  
                  if (res.path.length > 0) {
                      setOut("Violation found! See the highlighted path in the VM states graph.");
                  } else if (res.exhaustive) {
                    setOut("Verification complete: No execution errors found.");
                  } else {
                    setOut("Warning: Exploration limit reached. The state space was not fully explored. No violation found in the explored part.");
                  }
                  
                  // Extract violation path node labels
                  let violationPath: string[] = [];
                  if(res.path.length > 0) {
                    res.path.forEach((pathItem: any) => {
                      violationPath.push(nodeToString(pathItem.to));
                    });
                  }

                  // Build the graph with violation highlighting
                  const builtGraph = buildGraphFromNodes(res.nodes, { 
                    mode: 'check', 
                    violationPath 
                  });
                  
                  setNodes(builtGraph.nodes);
                  setEdges(builtGraph.edges);
                  setIsRun(false);

                } catch(e: any) {
                  // show error in execution tab
                  const errorInfo = formatAlthreadError(e, getFileContentFromVirtualFS(buildVirtualFileSystem(mockFileSystem()), e.pos?.file_path || ""));
                  setStructuredError(errorInfo);
                  setOut(errorInfo.message);
                  setActiveTab("execution");
                  setLoadingAction(null);
                  // reset other tabs to initial state
                  setStdout("The console output will appear here.");
                  setCommGraphOut([]);
                  setRunGraphNodes([]);
                  setRunBuiltGraph({ nodes: [], edges: [] });
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
              minSize={0.1}
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
              minSize={0.1}
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
        vmStates={[]}
        isRun={isRun()}
        interactiveMessageFlow={interactiveMessageFlow()}
        interactiveVmStates={interactiveVmStates()}
        interactiveStepLines={interactiveStepLines()}
        editor={editor}
      />
    </>
  );
}
