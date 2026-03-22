// @refresh granular

import { Logo } from "@assets/images/Logo";
import createEditor from "@components/editor/Editor";
import type { FileSystemEntry } from "@components/fileexplorer/FileExplorer";
import { renderMessageFlowGraph } from "@components/graph/CommGraph";
import Graph, { MAX_VISIBLE_GRAPH_NODES } from "@components/graph/Graph";
import Resizable from "@corvu/resizable";
import type { JSX } from "solid-js";
import {
	createEffect,
	createSignal,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import init, {
	execute_interactive_step,
	get_next_interactive_states,
	initialize,
	start_interactive_session,
} from "../pkg/althread_web";
import "@components/fileexplorer/FileExplorer.css";
import {
	DeleteConfirmationDialog,
	MoveConfirmationDialog,
} from "@components/dialogs/ConfirmationDialogs";
import { LoadExampleDialog } from "@components/dialogs/LoadExampleDialog";
import { EmptyEditor } from "@components/editor/EmptyEditor";
import ErrorDisplay from "@components/error/ErrorDisplay";
import FileTabs from "@components/fileexplorer/FileTabs";
import VMStateInspector from "@components/graph/VMStateInspector";
import InteractivePanel from "@components/interactive/InteractivePanel";
import Sidebar, { type SidebarView } from "@components/sidebar/Sidebar";
import { createEditorManager } from "@hooks/useEditorManager";
import { createFileOperationsHandlers } from "@hooks/useFileOperations";
import { formatAlthreadError } from "@utils/error";
import {
	buildVirtualFileSystem,
	findFileByPath,
	getFileContentFromVirtualFS,
	getPathFromId,
} from "@utils/fileSystemUtils"; // Add buildVirtualFileSystem here
import { buildGraphFromNodes, vmStateSignature } from "@utils/graphBuilders";
// Import our new modules
import {
	loadFileContent,
	loadFileSystem,
	saveFileContent,
	saveFileSystem,
} from "@utils/storage";
import { workerClient } from "@utils/workerClient";
import type {
	CheckResult,
	GraphNode,
	RunResult,
	VMStateSelection,
} from "./types/vm-state";

init().then(() => {
	console.log("loaded");
	initialize(); // Initialize the panic hook
});

const animationTimeOut = 100; //ms
const DEFAULT_WEB_CHECK_MAX_STATES = 10_000;

export default function App() {
	// Load file system from localStorage
	const initialFileSystem = loadFileSystem();

	const [mockFileSystem, setMockFileSystem] =
		createSignal<FileSystemEntry[]>(initialFileSystem);
	const [selectedFiles, setSelectedFiles] = createSignal<string[]>([]);
	const [creationError, setCreationError] = createSignal<string | null>(null);
	const [didAutoOpenDefault, setDidAutoOpenDefault] = createSignal(false);

	// Global file creation state - shared between FileExplorer and EmptyEditor
	const [globalFileCreation, setGlobalFileCreation] = createSignal<{
		type: "file" | "folder";
		parentPath: string;
	} | null>(null);

	// Sidebar view state
	const [sidebarView, setSidebarView] = createSignal<SidebarView>("help");
	const [sidebarCollapsed, setSidebarCollapsed] = createSignal(false);

	// Ref to corvu resizable collapse/expand functions — set by MainLayoutWrapper
	let getResizableSizes: (() => number[]) | null = null;
	let setResizableSizes: ((sizes: number[]) => void) | null = null;
	const [sidebarPrevSize, setSidebarPrevSize] = createSignal(0.2);

	const toggleSidebarCollapse = () => {
		const next = !sidebarCollapsed();

		// Capture to locally scoped consts to satisfy TS non-null checks without !
		const getSizesFn = getResizableSizes;
		const setSizesFn = setResizableSizes;

		if (getSizesFn && setSizesFn) {
			const rightPanelEl = document.querySelector(
				".right-panel",
			) as HTMLElement;
			const containerEl = document.getElementById("content") as HTMLElement;

			if (rightPanelEl && containerEl) {
				const containerW = containerEl.getBoundingClientRect().width;
				const rightW = rightPanelEl.getBoundingClientRect().width;

				// Provide exact pixel math. The layout has 0px handles now.
				// However, the container's physical width changes by exactly 48px
				// because of the `.sidebar-collapsed` class adding `margin-left: 48px`.
				if (next) {
					// Collapsing: container will shrink by 48px
					const targetContainerW = containerW - 48;

					// Save the sidebar's current pixel width so we can restore it exactly
					const currentSidebarW =
						(
							document.querySelector(
								"#content > [data-corvu-resizable-panel]:first-child",
							) as HTMLElement
						)?.getBoundingClientRect()?.width || containerW * 0.2;
					setSidebarPrevSize(currentSidebarW);

					// Right panel should maintain its exact current pixel width
					const newRightPercent = rightW / targetContainerW;

					setSizesFn([0, 1 - newRightPercent, newRightPercent]);
				} else {
					// Expanding: container will grow by 48px
					const targetContainerW = containerW + 48;

					// Restore sidebar to its exact previous pixel width
					const restoreSidebarW = sidebarPrevSize();
					const newLeftPercent = restoreSidebarW / targetContainerW;

					// Right panel should maintain its exact current pixel width
					const newRightPercent = rightW / targetContainerW;

					setSizesFn([
						newLeftPercent,
						1 - newLeftPercent - newRightPercent,
						newRightPercent,
					]);
				}
			}
		}

		setSidebarCollapsed(next);
	};

	// Initialize editor (no default file content)
	const editor = createEditor({
		compile: async (source: string) => {
			const activeFile = editorManager.activeFile();
			if (!activeFile) return null;
			const filePath =
				getPathFromId(mockFileSystem(), activeFile.id) || activeFile.name;
			const virtualFS = buildVirtualFileSystem(mockFileSystem());
			return await workerClient.compile(source, filePath, virtualFS);
		},
		defaultValue: "// Welcome to Althread\n",
		filePath: "untitled.alt",
		onValueChange: (value) => {
			// Save current file content when editor changes
			// Use a delayed check since editorManager might not be initialized yet
			setTimeout(() => {
				if (
					editorManager &&
					editorManager.activeFile &&
					editorManager.activeFile()
				) {
					const filePath =
						getPathFromId(mockFileSystem(), editorManager.activeFile()!.id) ||
						editorManager.activeFile()!.name;
					saveFileContent(filePath, value);
				}
			}, 0);
		},
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
		loadFileContent,
	);

	// Auto-open main.alt by default (once), so the editor isn't empty on load.
	createEffect(() => {
		if (didAutoOpenDefault()) return;
		if (editorManager.activeFile()) {
			setDidAutoOpenDefault(true);
			return;
		}

		const fs = mockFileSystem();
		const main = findFileByPath(fs, "main.alt");

		if (main && main.type === "file") {
			editorManager.handleFileSelect("main.alt", fs);
			setDidAutoOpenDefault(true);
			return;
		}

		// Fallback: if there's exactly one file in root, open it.
		const singleRootFile =
			fs.length === 1 && fs[0].type === "file" ? fs[0] : null;
		if (singleRootFile) {
			const filePath =
				getPathFromId(fs, singleRootFile.id) || singleRootFile.name;
			editorManager.handleFileSelect(filePath, fs);
		}
		setDidAutoOpenDefault(true);
	});

	// Conflict checking functions for file operations
	const checkNameConflict = (destPath: string, movingName: string): boolean => {
		if (destPath === "") {
			// Moving to root
			return mockFileSystem().some((entry) => entry.name === movingName);
		}

		// Find the destination directory
		const findDirectory = (
			files: FileSystemEntry[],
			targetPath: string,
		): FileSystemEntry | null => {
			const parts = targetPath.split("/").filter((part) => part !== "");
			let currentLevel = files;

			for (const part of parts) {
				const dir = currentLevel.find(
					(e) => e.name === part && e.type === "directory",
				);
				if (!dir || !dir.children) return null;
				currentLevel = dir.children;
			}

			// Return a synthetic entry representing the directory
			return {
				id: "temp",
				name: "",
				type: "directory",
				children: currentLevel,
			};
		};

		const destDir = findDirectory(mockFileSystem(), destPath);
		return (
			destDir?.children?.some((entry) => entry.name === movingName) || false
		);
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
		destPath: "",
		conflictingName: "",
	});

	// Delete confirmation dialog state
	const [deleteConfirmation, setDeleteConfirmation] = createSignal<{
		isOpen: boolean;
		paths: string[];
	}>({
		isOpen: false,
		paths: [],
	});

	// Load example dialog state
	const [loadExampleDialog, setLoadExampleDialog] = createSignal<{
		isOpen: boolean;
		content: string;
		fileName: string;
	}>({
		isOpen: false,
		content: "",
		fileName: "",
	});

	const showMoveConfirmDialog = (
		sourcePaths: string[],
		destPath: string,
		conflictingName: string,
	) => {
		setMoveConfirmation({
			isOpen: true,
			sourcePaths,
			destPath,
			conflictingName,
		});
	};

	const handleConfirmedMove = () => {
		const confirmation = moveConfirmation();

		// Execute the move with replacement for each source path
		confirmation.sourcePaths.forEach((sourcePath) => {
			fileOperations.handleMoveWithReplacement(
				sourcePath,
				confirmation.destPath,
				confirmation.conflictingName,
			);
		});

		setMoveConfirmation({
			isOpen: false,
			sourcePaths: [],
			destPath: "",
			conflictingName: "",
		});
	};

	const handleCanceledMove = () => {
		setMoveConfirmation({
			isOpen: false,
			sourcePaths: [],
			destPath: "",
			conflictingName: "",
		});
	};

	const showDeleteConfirmDialog = (paths: string[]) => {
		setDeleteConfirmation({
			isOpen: true,
			paths,
		});
	};

	const handleConfirmedDelete = () => {
		const confirmation = deleteConfirmation();
		confirmation.paths.forEach((path) => {
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
			fileName,
		});
	};

	const handleLoadInCurrentFile = () => {
		const dialog = loadExampleDialog();
		setLoadExampleDialog({ isOpen: false, content: "", fileName: "" });

		// If no file is active, create a new one
		if (!editorManager.activeFile()) {
			const fileName = `${dialog.fileName.replace(".alt", "")}-${Date.now()}.alt`;
			editorManager.createNewFileWithContent(
				fileName,
				dialog.content,
				fileOperations,
				mockFileSystem,
			);
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
					insert: dialog.content,
				},
			});
			editor.editorView().update([up]);
		}

		// Also update the saved content in localStorage
		const activeFile = editorManager.activeFile();
		if (activeFile) {
			const filePath =
				getPathFromId(mockFileSystem(), activeFile.id) || activeFile.name;
			saveFileContent(filePath, dialog.content);
		}
	};

	const handleLoadInNewFile = () => {
		const dialog = loadExampleDialog();
		setLoadExampleDialog({ isOpen: false, content: "", fileName: "" });
		const fileName = `${dialog.fileName.replace(".alt", "")}-${Date.now()}.alt`;
		editorManager.createNewFileWithContent(
			fileName,
			dialog.content,
			fileOperations,
			mockFileSystem,
		);
	};

	const handleCancelLoadExample = () => {
		setLoadExampleDialog({ isOpen: false, content: "", fileName: "" });
	};

	// New file prompt handlers
	const handleNewFileClick = () => {
		// If sidebar is collapsed, expand it first
		if (sidebarCollapsed()) {
			setSidebarCollapsed(false);
		}
		// Switch to explorer view and trigger global file creation state
		setSidebarView("explorer");
		setGlobalFileCreation({ type: "file", parentPath: "" });
	};

	// Helper function to check if active file has .alt extension
	const isAltFile = () => {
		const activeFile = editorManager.activeFile();
		if (!activeFile) return false;
		return activeFile.name.endsWith(".alt");
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
				const filePath =
					getPathFromId(mockFileSystem(), activeFile.id) || activeFile.name;
				const content = loadFileContent(filePath);

				// Update the editor content
				const editorView = editor.editorView();
				const transaction = editorView.state.update({
					changes: {
						from: 0,
						to: editorView.state.doc.length,
						insert: content,
					},
				});
				editorView.update([transaction]);
			}
		}, 50); // Small delay to ensure DOM is updated
	});

	const [activeTab, setActiveTab] = createSignal("console");
	const handleExecutionTabClick = (tab: string) => {
		setActiveTab(tab);
	};

	const [nodes, setNodes] = createSignal<any[]>([]);
	const [edges, setEdges] = createSignal<any[]>([]);
	const [isRun, setIsRun] = createSignal(true);

	const [stdout, setStdout] = createSignal(
		"The console output will appear here.",
	);
	const [out, setOut] = createSignal("The execution output will appear here.");
	const [commgraphout, setCommGraphOut] = createSignal<any[]>([]); //messageflow graph
	const [runGraphNodes, setRunGraphNodes] = createSignal<GraphNode[]>([]); // For run mode - stores graph nodes
	const [runBuiltGraph, setRunBuiltGraph] = createSignal<{
		nodes: any[];
		edges: any[];
	}>({ nodes: [], edges: [] }); // Built graph for vis.js
	const [counterexampleReplayNodes, setCounterexampleReplayNodes] =
		createSignal<GraphNode[]>([]); // Replayable counter-example path
	const [counterexampleReplayStepLines, setCounterexampleReplayStepLines] =
		createSignal<number[][]>([]); // Step lines for counter-example replay
	const [checkTooLargeStatus, setCheckTooLargeStatus] = createSignal<
		string | null
	>(null);
	const [stepLines, setStepLines] = createSignal<number[][]>([]); //to store lines for each step
	const [activeAction, setActiveAction] = createSignal<string | null>(null);
	const [loadingAction, setLoadingAction] = createSignal<string | null>(null);
	const [executionError, setExecutionError] = createSignal(false);
	const [structuredError, setStructuredError] = createSignal<any>(null);
	const [selectedVM, setSelectedVM] = createSignal<VMStateSelection | null>(
		null,
	);

	// Graph ref for programmatic control
	let graphRef: { selectNode: (nodeId: string | number) => void } | null = null;

	const selectRunVmByIndex = (index: number) => {
		const nodes = runGraphNodes();
		if (!nodes || nodes.length === 0) return;
		const clamped = Math.max(0, Math.min(index, nodes.length - 1));
		setSelectedVM({ vm: nodes[clamped].vm, stepIndex: clamped });

		// Select the corresponding node in the graph
		if (graphRef) {
			graphRef.selectNode(clamped);
		}
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
				.filter((s: any) => typeof s.line === "number" && s.line > 0);

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

			const nextIndex =
				event.key === "ArrowRight" ? currentIndex + 1 : currentIndex - 1;
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
	const [interactiveDeadlock, setInteractiveDeadlock] = createSignal(false);
	const [interactiveStepLines, setInteractiveStepLines] = createSignal<
		number[][]
	>([]);
	const [accumulatedOutput, setAccumulatedOutput] = createSignal<string>("");
	const [accumulatedExecutionOutput, setAccumulatedExecutionOutput] =
		createSignal<string>("");
	const [interactiveMessageFlow, setInteractiveMessageFlow] = createSignal<
		any[]
	>([]);
	const [interactiveVmStates, setInteractiveVmStates] = createSignal<any[]>([]);

	const resetSetOut = () => {
		setOut("The execution output will appear here.");
	};

	const isDeadlockError = (error: any) =>
		typeof error?.message === "string" &&
		error.message.toLowerCase().includes("deadlock");

	const appendStatusNotice = (text: string, notice: string) => {
		const trimmed = text.trimEnd();
		return trimmed.length > 0 ? `${trimmed}\n\n${notice}` : notice;
	};

	const getCheckTooLargeStatus = (result: CheckResult) => {
		if (result.path.length > 0) {
			if (result.exhaustive) {
				return "Verification result: a violation was found.";
			}
			return "Verification result: a violation was found before exploration completed. The counterexample is valid, but the explored graph is partial.";
		}
		if (result.exhaustive) {
			return "Verification result: the check succeeded with no violation.";
		}
		return "Verification result: exploration stopped before the check could complete.";
	};

	const buildHiddenGraphPlaceholders = (count: number) =>
		Array.from({ length: count }, (_, id) => ({ id }));

	const getDeadlockExecutionOutput = (error: any, fallbackOutput = "") => {
		const errorInfo = formatAlthreadError(
			error,
			getFileContentFromVirtualFS(
				buildVirtualFileSystem(mockFileSystem()),
				error?.pos?.file_path || "",
			),
		);
		return appendStatusNotice(fallbackOutput, errorInfo.message);
	};

	const executeInteractiveStep = async (selectedIndex: number) => {
		try {
			if (!editorManager.activeFile()) return;
			setInteractiveDeadlock(false);

			const virtualFS = buildVirtualFileSystem(mockFileSystem());
			let filePath = getPathFromId(
				mockFileSystem(),
				editorManager.activeFile()!.id,
				"",
			);
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
					selectedIndex,
				);
			} catch (e: any) {
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
				updatedConsoleOutput += newStepOutput.join("\n") + "\n";
			}
			setAccumulatedOutput(updatedConsoleOutput);

			// Update accumulated message flow events
			const currentMessageFlow = interactiveMessageFlow();
			const newMessageFlowEvents = stepResult.message_flow_events || [];
			setInteractiveMessageFlow([
				...currentMessageFlow,
				...newMessageFlowEvents,
			]);

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
			const debugOutput = stepResult.debug || "";

			if (executedStep) {
				const stepInfo = `Executed: ${executedStep.prog_name}#${executedStep.prog_id}\n${debugOutput}`;
				const currentExecutionOutput = accumulatedExecutionOutput();
				const updatedExecutionOutput =
					currentExecutionOutput +
					(currentExecutionOutput ? "\n\n" : "") +
					stepInfo;
				setAccumulatedExecutionOutput(updatedExecutionOutput);
				setOut(updatedExecutionOutput);
			} else {
				const currentExecutionOutput = accumulatedExecutionOutput();
				const updatedExecutionOutput =
					currentExecutionOutput +
					(currentExecutionOutput ? "\n\n" : "") +
					debugOutput;
				setAccumulatedExecutionOutput(updatedExecutionOutput);
				setOut(updatedExecutionOutput);
			}

			let res;
			try {
				// Get next states with updated history
				res = get_next_interactive_states(
					editor.editorView().state.doc.toString(),
					filePath,
					virtualFS,
					newHistory,
				);
			} catch (e: any) {
				if (!isDeadlockError(e)) {
					throw e;
				}

				const deadlockConsoleOutput = appendStatusNotice(
					updatedConsoleOutput,
					"Deadlock detected.",
				);
				const deadlockExecutionOutput = getDeadlockExecutionOutput(
					e,
					accumulatedExecutionOutput(),
				);

				setAccumulatedOutput(deadlockConsoleOutput);
				setAccumulatedExecutionOutput(deadlockExecutionOutput);
				setOut(deadlockExecutionOutput);
				setInteractiveStates([]);
				setInteractiveFinished(true);
				setInteractiveDeadlock(true);
				setExecutionError(false);
				return;
			}

			setInteractiveStates(res.next_states || []);
			setCurrentVMState(stepResult.new_state || res.current_state);
			setInteractiveFinished(!res.next_states || res.next_states.length === 0);
			setInteractiveDeadlock(false);

			if (!res.next_states || res.next_states.length === 0) {
				const currentExecutionOutput = accumulatedExecutionOutput();
				const finalOutput =
					currentExecutionOutput +
					(currentExecutionOutput ? "\n\n" : "") +
					"Program execution completed.";
				setAccumulatedExecutionOutput(finalOutput);
				setOut(finalOutput);
			} else if (res.next_states.length === 0) {
				const currentExecutionOutput = accumulatedExecutionOutput();
				const finalOutput =
					currentExecutionOutput +
					(currentExecutionOutput ? "\n\n" : "") +
					"No more states available.";
				setAccumulatedExecutionOutput(finalOutput);
				setOut(finalOutput);
				setInteractiveFinished(true);
			}
		} catch (e: any) {
			console.error("Interactive step error:", e);
			if (isDeadlockError(e)) {
				const deadlockConsoleOutput = appendStatusNotice(
					accumulatedOutput(),
					"Deadlock detected.",
				);
				const deadlockExecutionOutput = getDeadlockExecutionOutput(
					e,
					accumulatedExecutionOutput(),
				);

				setAccumulatedOutput(deadlockConsoleOutput);
				setAccumulatedExecutionOutput(deadlockExecutionOutput);
				setOut(deadlockExecutionOutput);
				setInteractiveStates([]);
				setInteractiveFinished(true);
				setInteractiveDeadlock(true);
				setExecutionError(false);
				return;
			}

			const errorInfo = formatAlthreadError(
				e,
				getFileContentFromVirtualFS(
					buildVirtualFileSystem(mockFileSystem()),
					e.pos?.file_path || "",
				),
			);
			setStructuredError(errorInfo);
			setOut(errorInfo.message);
			setExecutionError(true);
			// Switch to execution tab to show the error
			setActiveTab("execution");
		}
	};

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
			setSidebarView("explorer");
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
		setInteractiveDeadlock(false);
		setAccumulatedOutput("");
		setAccumulatedExecutionOutput("");
		setInteractiveMessageFlow([]);
		setInteractiveVmStates([]);
		setActiveAction(null);
	};

	const buildCounterexampleReplay = (
		pathNodes: GraphNode[],
		graphNodes: GraphNode[],
	) => {
		if (!pathNodes || pathNodes.length === 0) {
			return {
				replayNodes: [] as GraphNode[],
				replayStepLines: [] as number[][],
			};
		}

		const initialGraphNode = graphNodes.find(
			(node) => node.metadata.level === 0,
		);
		let replayNodes = [...pathNodes];

		if (initialGraphNode) {
			const initialKey = vmStateSignature(initialGraphNode.vm);
			const firstPathKey = vmStateSignature(pathNodes[0].vm);
			if (initialKey !== firstPathKey) {
				replayNodes = [initialGraphNode, ...pathNodes];
			}
		}

		const replayStepLines: number[][] = [];
		for (let i = 0; i < replayNodes.length - 1; i++) {
			const rawLines = replayNodes[i + 1]?.metadata?.lines || [];
			const normalizedLines = rawLines
				.map((line: number) => Number(line))
				.filter((line: number) => Number.isFinite(line) && line > 0);
			replayStepLines.push(normalizedLines);
		}

		return { replayNodes, replayStepLines };
	};

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
			setInteractiveDeadlock(false);

			const virtualFS = buildVirtualFileSystem(mockFileSystem());
			let filePath = getPathFromId(
				mockFileSystem(),
				editorManager.activeFile()!.id,
				"",
			);
			if (!filePath) {
				filePath = editorManager.activeFile()!.name;
			}

			const res = start_interactive_session(
				editor.editorView().state.doc.toString(),
				filePath,
				virtualFS,
			);

			setInteractiveStates(res.next_states || []);
			setCurrentVMState(res.current_state);
			setInteractiveFinished(!res.next_states || res.next_states.length === 0);
			setExecutionHistory([]);

			if (!res.next_states || res.next_states.length === 0) {
				setOut("Program execution completed immediately.");
			} else if (res.next_states.length === 0) {
				setOut("No more states available.");
				setInteractiveFinished(true);
			} else {
				setOut(
					"Interactive session restarted. Choose the next instruction to execute.",
				);
			}
		} catch (e: any) {
			console.error("Interactive reset error:", e);
			if (isDeadlockError(e)) {
				setInteractiveStates([]);
				setCurrentVMState(null);
				setInteractiveFinished(true);
				setInteractiveDeadlock(true);
				setAccumulatedOutput("Deadlock detected.");
				setAccumulatedExecutionOutput(getDeadlockExecutionOutput(e));
				setOut(getDeadlockExecutionOutput(e));
				setExecutionError(false);
				return;
			}

			const errorInfo = formatAlthreadError(
				e,
				getFileContentFromVirtualFS(
					buildVirtualFileSystem(mockFileSystem()),
					e.pos?.file_path || "",
				),
			);
			setStructuredError(errorInfo);
			setOut(errorInfo.message);
			setExecutionError(true);
		}
	};

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
		setCounterexampleReplayNodes([]);
		setCounterexampleReplayStepLines([]);
		setActiveAction(null);
		setSelectedVM(null);
		// Reset interactive mode state
		setInteractiveStates([]);
		setExecutionHistory([]);
		setCurrentVMState(null);
		setInteractiveFinished(false);
		setInteractiveDeadlock(false);
		setAccumulatedOutput("");
		setAccumulatedExecutionOutput("");
		setInteractiveMessageFlow([]);
		setInteractiveVmStates([]);
	};

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
					<Resizable
						id="vm-states-layout"
						orientation="vertical"
						style={{
							height: "100%",
							width: "100%",
							display: "flex",
							"flex-direction": "column",
						}}
					>
						<Resizable.Panel
							initialSize={0.4}
							minSize={0.2}
							style={{
								display: "flex",
								"flex-direction": "column",
								overflow: "hidden",
							}}
						>
							<VMStateInspector
								node={selectedVM()}
								onClose={() => setSelectedVM(null)}
							/>
						</Resizable.Panel>
						<Resizable.Handle class="Resizable-handle" />
						<Resizable.Panel
							initialSize={0.6}
							minSize={0.2}
							style={{
								display: "flex",
								"flex-direction": "column",
								overflow: "hidden",
							}}
						>
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
								ref={(instance) => (graphRef = instance)}
							/>
						</Resizable.Panel>
					</Resizable>
				</div>
			</Match>

			<Match when={!isRun()}>
				<div class="console">
					<Resizable
						id="checker-states-layout"
						orientation="vertical"
						style={{
							height: "100%",
							width: "100%",
							display: "flex",
							"flex-direction": "column",
						}}
					>
						<Resizable.Panel
							initialSize={0.4}
							minSize={0.2}
							style={{
								display: "flex",
								"flex-direction": "column",
								overflow: "hidden",
							}}
						>
							<VMStateInspector
								node={selectedVM()}
								onClose={() => setSelectedVM(null)}
							/>
						</Resizable.Panel>
						<Resizable.Handle class="Resizable-handle" />
						<Resizable.Panel
							initialSize={0.6}
							minSize={0.2}
							style={{
								display: "flex",
								"flex-direction": "column",
								overflow: "hidden",
							}}
						>
							<Graph
								nodes={nodes()}
								edges={edges()}
								setLoadingAction={setLoadingAction}
								theme="dark"
								tooLargeStatusMessage={checkTooLargeStatus() ?? undefined}
								onEdgeClick={(_edgeId: string, edgeData: any) => {
									if (edgeData && edgeData.lines && editor.highlightLines) {
										editor.highlightLines(edgeData.lines);
									}
								}}
								onNodeSelect={setSelectedVM}
								ref={(instance) => (graphRef = instance)}
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
						disabled={
							loadingAction() === "interactive" ||
							!editorManager.activeFile() ||
							!isAltFile()
						}
						onClick={async () => {
							if (!editorManager.activeFile()) return;
							if (activeAction() !== "interactive")
								setActiveAction("interactive");
							if (loadingAction() !== "interactive")
								setLoadingAction("interactive");

							try {
								setIsInteractiveMode(true);
								setExecutionError(false);
								resetSetOut();
								setInteractiveDeadlock(false);
								// Reset accumulated output for new session
								setAccumulatedOutput("");
								// Go to console by default, execution only if there are errors
								setActiveTab("console");

								const virtualFS = buildVirtualFileSystem(mockFileSystem());
								let filePath = getPathFromId(
									mockFileSystem(),
									editorManager.activeFile()!.id,
									"",
								);
								if (!filePath) {
									filePath = editorManager.activeFile()!.name;
								}

								const res = start_interactive_session(
									editor.editorView().state.doc.toString(),
									filePath,
									virtualFS,
								);
								setInteractiveStates(res.next_states || []);
								setCurrentVMState(res.current_state);
								setInteractiveFinished(
									!res.next_states || res.next_states.length === 0,
								);
								setExecutionHistory([]);

								if (!res.next_states || res.next_states.length === 0) {
									setOut("Program execution completed.");
								} else if (res.next_states.length === 0) {
									setOut("No interactive choices available.");
								} else {
									setOut(
										"Interactive mode started. Select the next instruction to execute.",
									);
								}
							} catch (e: any) {
								console.error("Interactive mode error:", e);
								if (isDeadlockError(e)) {
									setInteractiveStates([]);
									setCurrentVMState(null);
									setInteractiveFinished(true);
									setInteractiveDeadlock(true);
									setAccumulatedOutput("Deadlock detected.");
									setAccumulatedExecutionOutput(getDeadlockExecutionOutput(e));
									setOut(getDeadlockExecutionOutput(e));
									setExecutionError(false);
									return;
								}

								const errorInfo = formatAlthreadError(
									e,
									getFileContentFromVirtualFS(
										buildVirtualFileSystem(mockFileSystem()),
										e.pos?.file_path || "",
									),
								);
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
						}}
					>
						<i
							class={
								loadingAction() === "interactive"
									? "codicon codicon-loading codicon-modifier-spin"
									: "codicon codicon-debug-step-over"
							}
						></i>
						Interactive
					</button>

					<button
						class={`vscode-button${activeAction() === "run" ? " active" : ""}`}
						disabled={
							loadingAction() === "run" ||
							!editorManager.activeFile() ||
							!isAltFile()
						}
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
								let filePath = getPathFromId(
									mockFileSystem(),
									editorManager.activeFile()!.id,
									"",
								);
								if (!filePath) {
									filePath = editorManager.activeFile()!.name; // Fallback to name if ID not found
								}
								const res: RunResult = await workerClient.run(
									editor.editorView().state.doc.toString(),
									filePath,
									virtualFS,
								);
								const runtimeError = (res as any).runtime_error;

								if (res.debug.length === 0) {
									resetSetOut();
								} else {
									setOut(res.debug);
								}
								setCommGraphOut(res.message_flow_events);
								setRunGraphNodes(res.nodes);
								setStepLines(res.step_lines || []);

								// Build the graph for visualization
								const builtGraph = buildGraphFromNodes(res.nodes, {
									mode: "run",
									stepLines: res.step_lines || [],
								});
								setRunBuiltGraph(builtGraph);

								const plainStdout = res.stdout.join("\n");
								setStdout(plainStdout);

								if (runtimeError) {
									const runtimeInfo = formatAlthreadError(
										runtimeError,
										getFileContentFromVirtualFS(
											virtualFS,
											runtimeError.pos?.file_path || filePath,
										),
									);
									const executionOutput =
										res.debug.length > 0
											? `${res.debug}\n${runtimeInfo.message}`
											: runtimeInfo.message;

									setStructuredError(null);
									setOut(executionOutput);
									if (isDeadlockError(runtimeError)) {
										setStdout(
											appendStatusNotice(plainStdout, "Deadlock detected."),
										);
										setExecutionError(false);
										setActiveTab("console");
									} else {
										setExecutionError(true);
										setActiveTab("execution");
									}
								} else {
									setStructuredError(null);
									setExecutionError(false);
									setActiveTab("console");
								}
							} catch (e: any) {
								console.error("Execution error:", e);
								// show error in execution tab
								const errorInfo = formatAlthreadError(
									e,
									getFileContentFromVirtualFS(
										buildVirtualFileSystem(mockFileSystem()),
										e.pos?.file_path || "",
									),
								);
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
						}}
					>
						<i
							class={
								loadingAction() === "run"
									? "codicon codicon-loading codicon-modifier-spin"
									: "codicon codicon-play"
							}
						></i>
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
							setCheckTooLargeStatus(null);
							if (!editorManager.activeFile()) return;

							try {
								const virtualFS = buildVirtualFileSystem(mockFileSystem());

								let filePath = getPathFromId(
									mockFileSystem(),
									editorManager.activeFile()!.id,
									"",
								);
								if (!filePath) {
									filePath = editorManager.activeFile()!.name; // Fallback to name if ID not found
								}

								const res: CheckResult = await workerClient.check(
									editor.editorView().state.doc.toString(),
									filePath,
									virtualFS,
									DEFAULT_WEB_CHECK_MAX_STATES,
								);

								if (res.path.length > 0) {
									if (res.exhaustive) {
										setOut(
											"Violation found! See the highlighted path in the VM states graph.",
										);
									} else {
										setOut(
											"Violation found before exploration completed. The counterexample is valid, but the explored graph is partial. See the highlighted path in the VM states graph.",
										);
									}
								} else if (res.exhaustive) {
									setOut("Verification complete: No execution errors found.");
								} else {
									setOut(
										"Warning: Exploration limit reached. The state space was not fully explored. No violation found in the explored part.",
									);
								}
								setCheckTooLargeStatus(getCheckTooLargeStatus(res));

								if (res.path.length > 0) {
									const replay = buildCounterexampleReplay(res.path, res.nodes);
									setCounterexampleReplayNodes(replay.replayNodes);
									setCounterexampleReplayStepLines(replay.replayStepLines);
								} else {
									setCounterexampleReplayNodes([]);
									setCounterexampleReplayStepLines([]);
								}

								const violationPathStates = res.path.map(
									(pathItem) => pathItem.vm,
								);

								if (res.nodes.length > MAX_VISIBLE_GRAPH_NODES) {
									setNodes(buildHiddenGraphPlaceholders(res.nodes.length));
									setEdges([]);
								} else {
									const builtGraph = buildGraphFromNodes(res.nodes, {
										mode: "check",
										violationPathStates,
									});

									setNodes(builtGraph.nodes);
									setEdges(builtGraph.edges);
								}
								setIsRun(false);
							} catch (e: any) {
								// show error in execution tab
								const errorInfo = formatAlthreadError(
									e,
									getFileContentFromVirtualFS(
										buildVirtualFileSystem(mockFileSystem()),
										e.pos?.file_path || "",
									),
								);
								setStructuredError(errorInfo);
								setOut(errorInfo.message);
								setActiveTab("execution");
								setLoadingAction(null);
								setCheckTooLargeStatus(null);
								setCounterexampleReplayNodes([]);
								setCounterexampleReplayStepLines([]);
								// reset other tabs to initial state
								setStdout("The console output will appear here.");
								setCommGraphOut([]);
								setRunGraphNodes([]);
								setRunBuiltGraph({ nodes: [], edges: [] });
								setExecutionError(true);
							}
						}}
					>
						<i
							class={
								loadingAction() === "check"
									? "codicon codicon-loading codicon-modifier-spin"
									: "codicon codicon-check"
							}
						></i>
						Check
					</button>

					<button
						class={`vscode-button${activeAction() === "run-counterexample" ? " active" : ""}`}
						disabled={
							loadingAction() === "run-counterexample" ||
							counterexampleReplayNodes().length === 0
						}
						onClick={async () => {
							const replayNodes = counterexampleReplayNodes();
							if (replayNodes.length === 0) return;

							if (!isRun()) setIsRun(true);
							setExecutionError(false);
							setStructuredError(null);
							if (activeAction() !== "run-counterexample")
								setActiveAction("run-counterexample");
							if (loadingAction() !== "run-counterexample")
								setLoadingAction("run-counterexample");
							setSelectedVM(null);

							try {
								const replayStepLines = counterexampleReplayStepLines();
								setRunGraphNodes(replayNodes);
								setStepLines(replayStepLines);

								const builtGraph = buildGraphFromNodes(replayNodes, {
									mode: "run",
									stepLines: replayStepLines,
								});

								setRunBuiltGraph(builtGraph);
								setCommGraphOut([]);
								setStdout("Counter-example replay loaded from checker path.");
								setOut(
									"Counter-example replay loaded. Open VM states to walk through the violating trace.",
								);
								setActiveTab("vm_states");
							} finally {
								setTimeout(() => {
									setLoadingAction(null);
								}, animationTimeOut);
							}
						}}
					>
						<i
							class={
								loadingAction() === "run-counterexample"
									? "codicon codicon-loading codicon-modifier-spin"
									: "codicon codicon-debug-start"
							}
						></i>
						Run CE
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
						}}
					>
						<i
							class={
								loadingAction() === "reset"
									? "codicon codicon-loading codicon-modifier-spin"
									: "codicon codicon-clear-all"
							}
						></i>
						Reset
					</button>
				</div>
			</div>

			{/* Collapsed sidebar icon strip — absolutely positioned so it overlays when sidebar panel is at 0 */}
			<Show when={sidebarCollapsed()}>
				<div class="collapsed-sidebar-container">
					<Sidebar
						files={mockFileSystem()}
						onFileSelect={(path) =>
							editorManager.handleFileSelect(path, mockFileSystem())
						}
						onNewFile={fileOperations.handleNewFile}
						onNewFolder={fileOperations.handleNewFolder}
						onMoveEntry={fileOperations.handleMoveEntry}
						onRenameEntry={fileOperations.handleRenameEntry}
						onDeleteEntry={fileOperations.handleDeleteEntry}
						onCopyEntry={fileOperations.handleCopyEntry}
						onFileUpload={fileOperations.handleFileUpload}
						activeFile={editorManager.activeFile()}
						getFilePath={(entry) =>
							getPathFromId(mockFileSystem(), entry.id) || entry.name
						}
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
			</Show>

			<div
				class={`content-wrapper ${sidebarCollapsed() ? "sidebar-collapsed" : ""}`}
			>
				<Resizable
					id="content"
					as={(props: JSX.HTMLAttributes<HTMLDivElement>) => {
						// Access corvu context here so we can wire exact size logic
						const ctx = Resizable.useContext();
						getResizableSizes = ctx.sizes;
						setResizableSizes = ctx.setSizes;
						return <div {...props} />;
					}}
				>
					{/* Sidebar panel — collapsible, hidden when sidebarCollapsed */}
					<Resizable.Panel
						initialSize={0.2}
						minSize={0.15}
						collapsible
						collapsedSize={0}
						onCollapse={() => {
							if (!sidebarCollapsed()) setSidebarCollapsed(true);
						}}
						onExpand={() => {
							if (sidebarCollapsed()) setSidebarCollapsed(false);
						}}
						style={{ overflow: "hidden" }}
					>
						<Sidebar
							files={mockFileSystem()}
							onFileSelect={(path) =>
								editorManager.handleFileSelect(path, mockFileSystem())
							}
							onNewFile={fileOperations.handleNewFile}
							onNewFolder={fileOperations.handleNewFolder}
							onMoveEntry={fileOperations.handleMoveEntry}
							onRenameEntry={fileOperations.handleRenameEntry}
							onDeleteEntry={fileOperations.handleDeleteEntry}
							onCopyEntry={fileOperations.handleCopyEntry}
							onFileUpload={fileOperations.handleFileUpload}
							activeFile={editorManager.activeFile()}
							getFilePath={(entry) =>
								getPathFromId(mockFileSystem(), entry.id) || entry.name
							}
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

					<Resizable.Handle class="Resizable-handle" />

					<Resizable.Panel class="editor-panel" initialSize={0.5} minSize={0.2}>
						<FileTabs
							openFiles={editorManager.openFiles()}
							activeFile={editorManager.activeFile()}
							getFilePath={(entry) =>
								getPathFromId(mockFileSystem(), entry.id) || entry.name
							}
							onTabClick={(file) =>
								editorManager.handleFileTabClick(file, mockFileSystem())
							}
							onTabClose={(file) =>
								editorManager.handleTabClose(file, mockFileSystem())
							}
						/>
						{editorManager.activeFile() ? (
							<div class="editor-instance-wrapper" ref={editor.ref} />
						) : (
							<EmptyEditor onNewFile={handleNewFileClick} />
						)}
					</Resizable.Panel>

					<Resizable.Handle class="Resizable-handle" />

					<Resizable.Panel class="right-panel" initialSize={0.3} minSize={0.1}>
						<div class="execution-content">
							<div class="tab">
								<button
									class={`tab_button ${activeTab() === "console" ? "active" : ""}`}
									onclick={() => handleExecutionTabClick("console")}
									disabled={!isRun()}
									title="Console"
								>
									<i class="codicon codicon-terminal"></i> <span>Console</span>
								</button>
								<button
									class={`tab_button 
                           ${activeTab() === "execution" ? "active" : ""} 
                           ${executionError() ? "execution-error" : ""}`}
									onclick={() => handleExecutionTabClick("execution")}
									title="Execution"
								>
									<i class="codicon codicon-play"></i> <span>Execution</span>
								</button>
								<button
									class={`tab_button ${activeTab() === "msg_flow" ? "active" : ""}`}
									onclick={() => handleExecutionTabClick("msg_flow")}
									disabled={!isRun()}
									title="Message flow"
								>
									<i class="codicon codicon-send"></i> <span>Message flow</span>
								</button>
								<button
									class={`tab_button ${activeTab() === "vm_states" ? "active" : ""}`}
									onclick={() => handleExecutionTabClick("vm_states")}
									title="VM states"
								>
									<i class="codicon codicon-type-hierarchy-sub"></i>{" "}
									<span>VM states</span>
								</button>
							</div>
							<div class="tab-content">{renderExecContent()}</div>
						</div>
					</Resizable.Panel>
				</Resizable>
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
				deadlockDetected={interactiveDeadlock()}
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
