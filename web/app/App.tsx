// @refresh granular

import { Logo } from "@assets/images/Logo";
import {
	DeleteConfirmationDialog,
	MoveConfirmationDialog,
} from "@components/dialogs/ConfirmationDialogs";
import { LoadExampleDialog } from "@components/dialogs/LoadExampleDialog";
import createEditor from "@components/editor/Editor";
import { EmptyEditor } from "@components/editor/EmptyEditor";
import type { FileSystemEntry } from "@components/fileexplorer/FileExplorer";
import FileTabs from "@components/fileexplorer/FileTabs";
import ResultsPanel from "@components/results/ResultsPanel";
import Sidebar, { type SidebarView } from "@components/sidebar/Sidebar";
import Resizable from "@corvu/resizable";
import { createEditorManager } from "@hooks/useEditorManager";
import {
	createExecutionController,
	type ExecutionAction,
	type ExecutionContext,
} from "@hooks/useExecution";
import { createFileOperationsHandlers } from "@hooks/useFileOperations";
import {
	buildVirtualFileSystem,
	findFileByPath,
	getPathFromId,
} from "@utils/fileSystemUtils";
import { isMac, modKey } from "@utils/platform";
import {
	clearSharedEditorFileFromUrl,
	loadSharedEditorFileFromUrl,
} from "@utils/share";
import {
	type EditorTheme,
	loadEditorTheme,
	loadFileContent,
	loadFileSystem,
	saveEditorTheme,
	saveFileContent,
	saveFileSystem,
} from "@utils/storage";
import type { JSX } from "solid-js";
import {
	createEffect,
	createSignal,
	on,
	onCleanup,
	onMount,
	Show,
} from "solid-js";
import "@components/fileexplorer/FileExplorer.css";
import init, { initialize } from "../pkg/althread_web";
import "./theme-overrides.css";

init().then(() => {
	initialize(); // Install the panic hook
});

/** Returns `name.ext`, or `name-2.ext`, `name-3.ext`… if the name is already taken at the root. */
const uniqueFileName = (fileSystem: FileSystemEntry[], fileName: string) => {
	if (!findFileByPath(fileSystem, fileName)) return fileName;
	const dot = fileName.lastIndexOf(".");
	const base = dot > 0 ? fileName.slice(0, dot) : fileName;
	const ext = dot > 0 ? fileName.slice(dot) : "";
	let i = 2;
	while (findFileByPath(fileSystem, `${base}-${i}${ext}`)) i++;
	return `${base}-${i}${ext}`;
};

/** Does `destPath` (a directory path, "" for the root) already contain an entry named `name`? */
const directoryContains = (
	fileSystem: FileSystemEntry[],
	destPath: string,
	name: string,
) => {
	let level: FileSystemEntry[] | undefined = fileSystem;
	for (const part of destPath.split("/").filter(Boolean)) {
		level = level?.find(
			(e) => e.name === part && e.type === "directory",
		)?.children;
		if (!level) return false;
	}
	return level.some((entry) => entry.name === name);
};

interface FileDialogState {
	isOpen: boolean;
	content: string;
	fileName: string;
}
const CLOSED_FILE_DIALOG: FileDialogState = {
	isOpen: false,
	content: "",
	fileName: "",
};

export default function App() {
	const sharedEditorFile = loadSharedEditorFileFromUrl();

	const [mockFileSystem, setMockFileSystem] = createSignal<FileSystemEntry[]>(
		loadFileSystem(),
	);
	const [selectedFiles, setSelectedFiles] = createSignal<string[]>([]);
	const [creationError, setCreationError] = createSignal<string | null>(null);
	const [didAutoOpenDefault, setDidAutoOpenDefault] = createSignal(
		Boolean(sharedEditorFile),
	);
	const [theme, setTheme] = createSignal<EditorTheme>(loadEditorTheme());

	// Global file creation state - shared between FileExplorer and EmptyEditor
	const [globalFileCreation, setGlobalFileCreation] = createSignal<{
		type: "file" | "folder";
		parentPath: string;
	} | null>(null);

	// ---- Sidebar ---------------------------------------------------------------

	const [sidebarView, setSidebarView] = createSignal<SidebarView>("help");
	const [sidebarCollapsed, setSidebarCollapsed] = createSignal(false);

	// corvu resizable size accessors — set by the Resizable `as` wrapper below
	let getResizableSizes: (() => number[]) | null = null;
	let setResizableSizes: ((sizes: number[]) => void) | null = null;
	const [sidebarPrevSize, setSidebarPrevSize] = createSignal(0.2);
	/** Width of the icon strip shown when the sidebar is collapsed (see `.sidebar-collapsed`). */
	const COLLAPSED_STRIP_WIDTH = 48;

	const toggleSidebarCollapse = () => {
		const next = !sidebarCollapsed();
		const setSizes = setResizableSizes;
		const rightPanelEl = document.querySelector<HTMLElement>(".right-panel");
		const containerEl = document.getElementById("content");

		if (getResizableSizes && setSizes && rightPanelEl && containerEl) {
			const containerW = containerEl.getBoundingClientRect().width;
			const rightW = rightPanelEl.getBoundingClientRect().width;

			// Keep the right panel at its current pixel width while the container
			// grows/shrinks by the width of the collapsed icon strip.
			if (next) {
				const targetW = containerW - COLLAPSED_STRIP_WIDTH;
				const sidebarW =
					document
						.querySelector<HTMLElement>(
							"#content > [data-corvu-resizable-panel]:first-child",
						)
						?.getBoundingClientRect().width || containerW * 0.2;
				setSidebarPrevSize(sidebarW);
				const right = rightW / targetW;
				setSizes([0, 1 - right, right]);
			} else {
				const targetW = containerW + COLLAPSED_STRIP_WIDTH;
				const left = sidebarPrevSize() / targetW;
				const right = rightW / targetW;
				setSizes([left, 1 - left - right, right]);
			}
		}

		setSidebarCollapsed(next);
	};

	const showSidebarView = (view: SidebarView) => {
		if (sidebarCollapsed()) toggleSidebarCollapse();
		setSidebarView(view);
	};

	// ---- Editor ------------------------------------------------------------------

	const pathOf = (entry: FileSystemEntry) =>
		getPathFromId(mockFileSystem(), entry.id) || entry.name;

	const editor = createEditor({
		getWorkspace: () => buildVirtualFileSystem(mockFileSystem()),
		onWorkspaceChange: (files) => {
			for (const [path, content] of Object.entries(files))
				saveFileContent(path, content);
		},
		onNavigate: (path, position) => {
			if (!findFileByPath(mockFileSystem(), path)) return;
			editorManager.handleFileSelect(path, mockFileSystem());
			const view = editor.safeEditorView();
			if (!view) return;
			const line = view.state.doc.line(
				Math.min(position.line + 1, view.state.doc.lines),
			);
			const anchor = Math.min(line.from + position.character, line.to);
			view.dispatch({ selection: { anchor }, scrollIntoView: true });
			view.focus();
		},
		defaultValue: "// Welcome to Althread\n",
		filePath: "untitled.alt",
		theme: theme(),
		onValueChange: (value) => {
			const active = editorManager.activeFile();
			if (active) saveFileContent(pathOf(active), value);
		},
	});

	const editorManager = createEditorManager(editor);

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

	const openNewFile = (fileName: string, content: string) => {
		editorManager.createNewFileWithContent(
			uniqueFileName(mockFileSystem(), fileName),
			content,
			fileOperations,
			mockFileSystem,
		);
	};

	const isAltFile = () =>
		editorManager.activeFile()?.name.endsWith(".alt") ?? false;

	// ---- Execution -------------------------------------------------------------

	const executionContext = (): ExecutionContext | null => {
		const active = editorManager.activeFile();
		const view = editor.safeEditorView();
		if (!active || !isAltFile() || !view) return null;
		const fs = mockFileSystem();
		return {
			source: view.state.doc.toString(),
			filePath: getPathFromId(fs, active.id, "") || active.name,
			virtualFS: buildVirtualFileSystem(fs),
		};
	};

	const exec = createExecutionController({
		getContext: executionContext,
		theme,
	});

	/** Lines temporarily highlighted while hovering an interactive choice. */
	const [previewLines, setPreviewLines] = createSignal<number[] | null>(null);

	// Highlight in the editor the lines of the selected state (or hovered choice).
	createEffect(() => {
		const preview = previewLines();
		if (preview && preview.length > 0) {
			editor.highlightLines?.(preview);
			return;
		}

		const selection = exec.selectedVM();
		const specs = (selection?.vm?.locals ?? [])
			.map((p: any) => ({ line: p.line, label: `${p.name}#${p.pid}` }))
			.filter((s: any) => typeof s.line === "number" && s.line > 0);
		if (specs.length > 0) {
			editor.highlightLines?.(specs);
			return;
		}
		if (selection?.stepIndex !== undefined) {
			const lines = exec.stepLines()[selection.stepIndex];
			if (lines?.length) {
				editor.highlightLines?.(lines);
				return;
			}
		}
		editor.clearHighlights?.();
	});

	// Results describe the file they were computed for: drop them when switching files.
	createEffect(
		on(
			() => editorManager.activeFile()?.id,
			() => {
				setPreviewLines(null);
				if (exec.mode() === "interactive") exec.stopInteractive();
			},
			{ defer: true },
		),
	);

	const canExecute = () => !!editorManager.activeFile() && isAltFile();

	const actionTitle = (label: string, shortcut?: string) => {
		if (!editorManager.activeFile()) return `${label} — open a file first`;
		if (!isAltFile()) return `${label} — only .alt files can be executed`;
		return shortcut ? `${label} (${shortcut})` : label;
	};

	const runShortcut = `${modKey}+Enter`;
	const checkShortcut = `${modKey}+Shift+Enter`;

	// Global shortcuts. Capture phase so that they win over the editor keymap.
	onMount(() => {
		const handler = (e: KeyboardEvent) => {
			const mod = isMac ? e.metaKey : e.ctrlKey;
			if (!mod || e.key !== "Enter" || e.altKey) return;
			e.preventDefault();
			e.stopPropagation();
			if (e.shiftKey) exec.check();
			else exec.run();
		};
		window.addEventListener("keydown", handler, true);
		onCleanup(() => window.removeEventListener("keydown", handler, true));
	});

	// ---- Startup -----------------------------------------------------------------

	onMount(() => {
		if (!sharedEditorFile) return;
		clearSharedEditorFileFromUrl();

		const existingFile = findFileByPath(
			mockFileSystem(),
			sharedEditorFile.fileName,
		);
		if (existingFile?.type === "file") {
			setSharedImportDialog({
				isOpen: true,
				content: sharedEditorFile.content,
				fileName: sharedEditorFile.fileName,
			});
			return;
		}
		openNewFile(sharedEditorFile.fileName, sharedEditorFile.content);
	});

	// Auto-open main.alt (or the only root file) once, so the editor isn't empty on load.
	createEffect(() => {
		if (didAutoOpenDefault()) return;
		setDidAutoOpenDefault(true);
		if (editorManager.activeFile()) return;

		const fs = mockFileSystem();
		const main = findFileByPath(fs, "main.alt");
		if (main?.type === "file") {
			editorManager.handleFileSelect("main.alt", fs);
		} else if (fs.length === 1 && fs[0].type === "file") {
			editorManager.handleFileSelect(
				getPathFromId(fs, fs[0].id) || fs[0].name,
				fs,
			);
		}
	});

	createEffect(() => saveFileSystem(mockFileSystem()));

	createEffect(() => {
		const currentTheme = theme();
		document.documentElement.dataset.theme = currentTheme;
		document.body.dataset.theme = currentTheme;
		editor.updateTheme?.(currentTheme);
	});

	// Reload active file content after the sidebar layout changes to preserve editor content
	createEffect(
		on(sidebarCollapsed, () => {
			setTimeout(() => {
				const activeFile = editorManager.activeFile();
				const view = editor.safeEditorView();
				if (!activeFile || !view) return;
				const content = loadFileContent(pathOf(activeFile));
				view.dispatch({
					changes: { from: 0, to: view.state.doc.length, insert: content },
				});
			}, 50);
		}),
	);

	// ---- Dialogs -----------------------------------------------------------------

	const [moveConfirmation, setMoveConfirmation] = createSignal<{
		isOpen: boolean;
		sourcePaths: string[];
		destPath: string;
		conflictingName: string;
	}>({ isOpen: false, sourcePaths: [], destPath: "", conflictingName: "" });
	const closeMoveDialog = () =>
		setMoveConfirmation({
			isOpen: false,
			sourcePaths: [],
			destPath: "",
			conflictingName: "",
		});

	const [deleteConfirmation, setDeleteConfirmation] = createSignal<{
		isOpen: boolean;
		paths: string[];
	}>({ isOpen: false, paths: [] });

	const [loadExampleDialog, setLoadExampleDialog] =
		createSignal<FileDialogState>(CLOSED_FILE_DIALOG);
	const [sharedImportDialog, setSharedImportDialog] =
		createSignal<FileDialogState>(CLOSED_FILE_DIALOG);

	const handleConfirmedMove = () => {
		const { sourcePaths, destPath, conflictingName } = moveConfirmation();
		for (const sourcePath of sourcePaths) {
			fileOperations.handleMoveWithReplacement(
				sourcePath,
				destPath,
				conflictingName,
			);
		}
		closeMoveDialog();
	};

	const handleConfirmedDelete = () => {
		for (const path of deleteConfirmation().paths) {
			fileOperations.handleDeleteEntry(path);
		}
		setDeleteConfirmation({ isOpen: false, paths: [] });
	};

	const handleLoadInCurrentFile = () => {
		const dialog = loadExampleDialog();
		setLoadExampleDialog(CLOSED_FILE_DIALOG);

		const activeFile = editorManager.activeFile();
		if (!activeFile) {
			openNewFile(dialog.fileName, dialog.content);
			return;
		}
		editor.safeUpdateContent(dialog.content);
		saveFileContent(pathOf(activeFile), dialog.content);
	};

	const handleLoadInNewFile = () => {
		const dialog = loadExampleDialog();
		setLoadExampleDialog(CLOSED_FILE_DIALOG);
		openNewFile(dialog.fileName, dialog.content);
	};

	const handleReplaceSharedImport = () => {
		const dialog = sharedImportDialog();
		setSharedImportDialog(CLOSED_FILE_DIALOG);
		saveFileContent(dialog.fileName, dialog.content);
		editorManager.handleFileSelect(dialog.fileName, mockFileSystem());
	};

	const handleSharedImportInNewFile = () => {
		const dialog = sharedImportDialog();
		setSharedImportDialog(CLOSED_FILE_DIALOG);
		openNewFile(dialog.fileName, dialog.content);
	};

	const handleCancelSharedImport = () => {
		setSharedImportDialog(CLOSED_FILE_DIALOG);
		if (!editorManager.activeFile()) setDidAutoOpenDefault(false);
	};

	const handleNewFileClick = () => {
		showSidebarView("explorer");
		setGlobalFileCreation({ type: "file", parentPath: "" });
	};

	const handleErrorFileClick = (filePath: string) => {
		if (!findFileByPath(mockFileSystem(), filePath)) {
			console.warn(`File not found: ${filePath}`);
			return;
		}
		editorManager.handleFileSelect(filePath, mockFileSystem());
		showSidebarView("explorer");
	};

	// ---- Rendering ---------------------------------------------------------------

	const AppSidebar = () => (
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
			getFilePath={pathOf}
			selectedFiles={selectedFiles()}
			onSelectionChange={setSelectedFiles}
			creationError={creationError()}
			setCreationError={setCreationError}
			checkNameConflict={(destPath, name) =>
				directoryContains(mockFileSystem(), destPath, name)
			}
			showConfirmDialog={(sourcePaths, destPath, conflictingName) =>
				setMoveConfirmation({
					isOpen: true,
					sourcePaths,
					destPath,
					conflictingName,
				})
			}
			showDeleteConfirmDialog={(paths) =>
				setDeleteConfirmation({ isOpen: true, paths })
			}
			globalFileCreation={globalFileCreation()}
			setGlobalFileCreation={setGlobalFileCreation}
			setFileSystem={setMockFileSystem}
			onLoadExample={(content, fileName) =>
				setLoadExampleDialog({ isOpen: true, content, fileName })
			}
			activeView={sidebarView()}
			onViewChange={setSidebarView}
			isCollapsed={sidebarCollapsed()}
			onToggleCollapse={toggleSidebarCollapse}
		/>
	);

	const ActionButton = (props: {
		action: ExecutionAction;
		label: string;
		icon: string;
		shortcut?: string;
		primary?: boolean;
		onClick: () => void;
	}) => (
		<button
			type="button"
			class="vscode-button"
			classList={{
				primary: props.primary,
				active: props.action === "interactive" && exec.mode() === "interactive",
			}}
			disabled={!canExecute() || exec.busy() !== null}
			title={actionTitle(props.label, props.shortcut)}
			onClick={() => props.onClick()}
		>
			<i
				class={
					exec.busy() === props.action
						? "codicon codicon-loading codicon-modifier-spin"
						: `codicon codicon-${props.icon}`
				}
			/>
			<span>{props.label}</span>
		</button>
	);

	return (
		<div class="app-shell" data-theme={theme()}>
			<header id="header">
				<div class="brand">
					<Logo />
					<h3>Althread</h3>
				</div>
				<div class="actions">
					<div class="action-group">
						<ActionButton
							action="run"
							label="Run"
							icon="play"
							shortcut={runShortcut}
							primary
							onClick={exec.run}
						/>
						<ActionButton
							action="check"
							label="Check"
							icon="workspace-trusted"
							shortcut={checkShortcut}
							onClick={exec.check}
						/>
						<ActionButton
							action="interactive"
							label="Step"
							icon="debug-step-over"
							onClick={exec.startInteractive}
						/>
					</div>
					<span class="actions-separator" />
					<button
						type="button"
						class="vscode-button icon-only theme-toggle-button"
						title={`Switch to ${theme() === "dark" ? "light" : "dark"} theme`}
						aria-label="Toggle theme"
						onClick={() => {
							const next = theme() === "dark" ? "light" : "dark";
							saveEditorTheme(next);
							setTheme(next);
						}}
					>
						<i class="codicon codicon-color-mode" />
					</button>
				</div>
			</header>

			{/* Collapsed sidebar icon strip — overlays the layout while the sidebar panel is at 0 */}
			<Show when={sidebarCollapsed()}>
				<div class="collapsed-sidebar-container">
					<AppSidebar />
				</div>
			</Show>

			<div
				class="content-wrapper"
				classList={{ "sidebar-collapsed": sidebarCollapsed() }}
			>
				<Resizable
					id="content"
					as={(props: JSX.HTMLAttributes<HTMLDivElement>) => {
						const ctx = Resizable.useContext();
						getResizableSizes = ctx.sizes;
						setResizableSizes = ctx.setSizes;
						return <div {...props} />;
					}}
				>
					<Resizable.Panel
						initialSize={0.2}
						minSize={0.15}
						collapsible
						collapsedSize={0}
						onCollapse={() => setSidebarCollapsed(true)}
						onExpand={() => setSidebarCollapsed(false)}
						style={{ overflow: "hidden" }}
					>
						<AppSidebar />
					</Resizable.Panel>

					<Resizable.Handle class="Resizable-handle" />

					<Resizable.Panel class="editor-panel" initialSize={0.5} minSize={0.2}>
						<FileTabs
							openFiles={editorManager.openFiles()}
							activeFile={editorManager.activeFile()}
							getFilePath={pathOf}
							onTabClick={(file) =>
								editorManager.handleFileTabClick(file, mockFileSystem())
							}
							onTabClose={(file) =>
								editorManager.handleTabClose(file, mockFileSystem())
							}
						/>
						<Show
							when={editorManager.activeFile()}
							fallback={<EmptyEditor onNewFile={handleNewFileClick} />}
						>
							<div class="editor-instance-wrapper" ref={editor.ref} />
						</Show>
					</Resizable.Panel>

					<Resizable.Handle class="Resizable-handle" />

					<Resizable.Panel class="right-panel" initialSize={0.3} minSize={0.15}>
						<ResultsPanel
							exec={exec}
							editor={editor}
							theme={theme()}
							runShortcut={runShortcut}
							onErrorFileClick={handleErrorFileClick}
							onPreviewLines={setPreviewLines}
						/>
					</Resizable.Panel>
				</Resizable>
			</div>

			<MoveConfirmationDialog
				isOpen={moveConfirmation().isOpen}
				title="Replace Existing Item"
				message="An item with this name already exists in the destination folder. Do you want to replace it?"
				fileName={moveConfirmation().conflictingName}
				onConfirm={handleConfirmedMove}
				onCancel={closeMoveDialog}
			/>

			<DeleteConfirmationDialog
				isOpen={deleteConfirmation().isOpen}
				paths={deleteConfirmation().paths}
				onConfirm={handleConfirmedDelete}
				onCancel={() => setDeleteConfirmation({ isOpen: false, paths: [] })}
			/>

			<LoadExampleDialog
				isOpen={loadExampleDialog().isOpen}
				detail={`Replace the content of the current file, or open “${loadExampleDialog().fileName}” in a new file.`}
				onLoadInCurrent={handleLoadInCurrentFile}
				onLoadInNew={handleLoadInNewFile}
				onCancel={() => setLoadExampleDialog(CLOSED_FILE_DIALOG)}
			/>

			<LoadExampleDialog
				isOpen={sharedImportDialog().isOpen}
				onLoadInCurrent={handleReplaceSharedImport}
				onLoadInNew={handleSharedImportInNewFile}
				onCancel={handleCancelSharedImport}
				title="Shared Link Import"
				message="A file with the same name already exists. What should the editor do with the shared content?"
				detail={`Replace ${sharedImportDialog().fileName} or create a new file with a unique name.`}
				currentLabel="Replace File"
				newLabel="Create Copy"
			/>
		</div>
	);
}
