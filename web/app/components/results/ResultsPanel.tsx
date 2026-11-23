import ErrorDisplay from "@components/error/ErrorDisplay";
import VMStateInspector from "@components/graph/VMStateInspector";
import InteractiveChoices from "@components/interactive/InteractiveChoices";
import Resizable from "@corvu/resizable";
import type { ExecutionController, ResultTab } from "@hooks/useExecution";
import { MAX_VISIBLE_GRAPH_NODES } from "@utils/graphBuilders";
import {
	createEffect,
	For,
	type JSX,
	lazy,
	Match,
	on,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import "./ResultsPanel.css";

// vis-network is heavy: only load the graphs when they are displayed.
const Graph = lazy(() => import("@components/graph/Graph"));
const MessageFlowGraph = lazy(() => import("@components/graph/CommGraph"));

interface ResultsPanelProps {
	exec: ExecutionController;
	editor: any;
	theme: "dark" | "light";
	runShortcut: string;
	onErrorFileClick: (filePath: string, line?: number, col?: number) => void;
	onPreviewLines: (lines: number[] | null) => void;
	/** Highlights the lines of the transition clicked in a state graph. */
	onEdgeLines: (lines: number[] | null) => void;
}

interface TabSpec {
	id: ResultTab;
	label: string;
	icon: string;
}

const isEditableTarget = (target: EventTarget | null) => {
	const el = target as HTMLElement | null;
	return (
		!!el &&
		(el.tagName === "INPUT" ||
			el.tagName === "TEXTAREA" ||
			el.tagName === "SELECT" ||
			el.isContentEditable ||
			!!el.closest?.(".cm-editor"))
	);
};

function EmptyState(props: {
	icon: string;
	title: string;
	children?: JSX.Element;
}) {
	return (
		<div class="results-empty">
			<i class={`codicon codicon-${props.icon}`} />
			<h4>{props.title}</h4>
			<Show when={props.children}>
				<p>{props.children}</p>
			</Show>
		</div>
	);
}

/** State inspector on top, state graph below. */
function StatesLayout(props: {
	id: string;
	exec: ExecutionController;
	graph: JSX.Element;
}) {
	return (
		<Resizable id={props.id} orientation="vertical" class="states-layout">
			<Resizable.Panel initialSize={0.4} minSize={0.15} class="states-pane">
				<VMStateInspector
					node={props.exec.selectedVM()}
					onClose={() => props.exec.setSelectedVM(null)}
				/>
			</Resizable.Panel>
			<Resizable.Handle class="Resizable-handle" />
			<Resizable.Panel initialSize={0.6} minSize={0.15} class="states-pane">
				{props.graph}
			</Resizable.Panel>
		</Resizable>
	);
}

export default function ResultsPanel(props: ResultsPanelProps) {
	const exec = props.exec;
	let consoleRef: HTMLPreElement | undefined;
	let executionRef: HTMLDivElement | undefined;

	const tabs = (): TabSpec[] => [
		...(exec.mode() === "interactive"
			? [{ id: "step" as const, label: "Step", icon: "debug-step-over" }]
			: []),
		{ id: "console", label: "Console", icon: "terminal" },
		{ id: "execution", label: "Execution", icon: "output" },
		{ id: "msg_flow", label: "Messages", icon: "send" },
		{ id: "vm_states", label: "States", icon: "type-hierarchy-sub" },
	];

	const highlightEdge = (_edgeId: string, edgeData: any) => {
		props.onEdgeLines(edgeData?.lines?.length ? edgeData.lines : null);
	};

	// Auto-scroll outputs to the latest line.
	createEffect(
		on([exec.stdout, exec.activeTab], () => {
			if (consoleRef) consoleRef.scrollTop = consoleRef.scrollHeight;
		}),
	);
	createEffect(
		on([exec.executionOutput, exec.activeTab], () => {
			if (executionRef) executionRef.scrollTop = executionRef.scrollHeight;
		}),
	);

	// ← / → walk through a linear trace while the States tab is open,
	// unless the user is typing somewhere (editor, inputs).
	createEffect(() => {
		if (exec.activeTab() !== "vm_states" || exec.mode() === "check") return;
		const handler = (event: KeyboardEvent) => {
			if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
			if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)
				return;
			if (isEditableTarget(event.target)) return;
			if (exec.traceNodes().length === 0) return;
			const current = exec.selectedVM()?.stepIndex ?? -1;
			exec.selectTraceStep(
				event.key === "ArrowRight" ? current + 1 : current - 1,
			);
			event.preventDefault();
		};
		window.addEventListener("keydown", handler);
		onCleanup(() => window.removeEventListener("keydown", handler));
	});

	const getSourceLines = (lines: number[]): string[] => {
		const view = props.editor.safeEditorView?.();
		if (!view || !lines?.length) return [];
		const doc = view.state.doc;
		const text = lines
			.filter((n) => n > 0 && n <= doc.lines)
			.map((n) => doc.line(n).text.trim())
			.filter((line) => line !== "");
		return [...new Set(text)];
	};

	const statusActions = () => (
		<Show when={exec.mode() === "check" && exec.counterexample().length > 0}>
			<button
				type="button"
				class="status-action"
				onClick={() => exec.replayCounterexample()}
				title="Load the violating execution as a trace you can walk step by step"
			>
				<i class="codicon codicon-debug-alt" />
				Replay counterexample
			</button>
		</Show>
	);

	const renderStates = () => (
		<Switch>
			<Match when={exec.mode() === "check" && exec.busy() === "check"}>
				<EmptyState
					icon="loading codicon-modifier-spin"
					title="Model checking…"
				>
					Exploring every reachable state of the program.
				</EmptyState>
			</Match>
			<Match when={exec.mode() === "check" && exec.checkStateCount() === 0}>
				<EmptyState icon="type-hierarchy-sub" title="No state graph">
					The check did not produce any state.
				</EmptyState>
			</Match>
			<Match when={exec.mode() === "check"}>
				<StatesLayout
					id="checker-states-layout"
					exec={exec}
					graph={
						<Show
							when={exec.checkStateCount() <= MAX_VISIBLE_GRAPH_NODES}
							fallback={
								<EmptyState icon="graph" title="Graph too large to display">
									{exec.checkStateCount().toLocaleString()} states were
									explored, above the display limit of {MAX_VISIBLE_GRAPH_NODES}
									.
									<Show when={exec.counterexample().length > 0}>
										{" "}
										Use “Replay counterexample” to inspect the violating trace.
									</Show>
								</EmptyState>
							}
						>
							<Graph
								nodes={exec.checkGraph().nodes}
								edges={exec.checkGraph().edges}
								theme={props.theme}
								onEdgeClick={highlightEdge}
								onNodeSelect={exec.setSelectedVM}
							/>
						</Show>
					}
				/>
			</Match>
			<Match when={exec.traceNodes().length > 0}>
				<StatesLayout
					id="vm-states-layout"
					exec={exec}
					graph={
						<Graph
							nodes={exec.traceGraph().nodes}
							edges={exec.traceGraph().edges}
							theme={props.theme}
							onEdgeClick={highlightEdge}
							onNodeSelect={exec.setSelectedVM}
							selectedNode={exec.selectedVM()?.stepIndex ?? null}
						/>
					}
				/>
			</Match>
			<Match when={true}>
				<EmptyState icon="type-hierarchy-sub" title="No states yet">
					<b>Run</b> shows the states of one execution; <b>Check</b> explores
					all of them and verifies your <code>always</code> /{" "}
					<code>eventually</code> properties.
				</EmptyState>
			</Match>
		</Switch>
	);

	const renderContent = () => (
		<Switch>
			<Match
				when={exec.activeTab() === "step" && exec.mode() === "interactive"}
			>
				<InteractiveChoices
					choices={exec.choices()}
					finished={exec.interactiveFinished()}
					deadlock={exec.interactiveDeadlock()}
					stepCount={exec.history().length}
					getSourceLines={getSourceLines}
					onExecute={exec.executeStep}
					onPreviewLines={props.onPreviewLines}
					onUndo={exec.undoStep}
					onRestart={exec.startInteractive}
					onStop={exec.stopInteractive}
				/>
			</Match>

			<Match when={exec.activeTab() === "execution"}>
				<div class="console" ref={executionRef}>
					<Show
						when={exec.executionOutput() || exec.structuredError()}
						fallback={
							<EmptyState icon="output" title="No execution trace">
								The instructions executed by each process will be listed here.
							</EmptyState>
						}
					>
						<Show
							when={exec.executionError()}
							fallback={<pre>{exec.executionOutput()}</pre>}
						>
							<div class="execution-error-box">
								<Show
									when={exec.structuredError()}
									fallback={<pre>{exec.executionOutput()}</pre>}
								>
									{(error) => (
										<>
											<ErrorDisplay
												error={error()}
												onFileClick={props.onErrorFileClick}
											/>
											<Show
												when={
													exec.executionOutput() &&
													exec.executionOutput() !== error().message
												}
											>
												<pre>{exec.executionOutput()}</pre>
											</Show>
										</>
									)}
								</Show>
							</div>
						</Show>
					</Show>
				</div>
			</Match>

			<Match when={exec.activeTab() === "msg_flow"}>
				<div class="console">
					<Show
						when={exec.messageFlow().length > 0}
						fallback={
							<EmptyState icon="send" title="No messages">
								{exec.mode() === "check"
									? "Message flow is shown for a single execution: use Run, Step or Replay counterexample."
									: "Messages exchanged through channels will be drawn here."}
							</EmptyState>
						}
					>
						<MessageFlowGraph
							events={exec.messageFlow()}
							editor={props.editor}
							theme={props.theme}
						/>
					</Show>
				</div>
			</Match>

			<Match when={exec.activeTab() === "vm_states"}>
				<div class="console">{renderStates()}</div>
			</Match>

			<Match when={true}>
				<div class="console">
					<Show
						when={exec.stdout() || exec.stdoutWarning()}
						fallback={
							<EmptyState icon="terminal" title="No output">
								{exec.mode() === "check"
									? "Model checking does not print. Use Run or Step to see the program output."
									: exec.mode() === "none"
										? `Press Run (${props.runShortcut}) to execute the program.`
										: "The program did not print anything."}
							</EmptyState>
						}
					>
						<pre ref={consoleRef}>{exec.stdout()}</pre>
						<Show when={exec.stdoutWarning()}>
							{(warning) => <pre class="console-warning">{warning()}</pre>}
						</Show>
					</Show>
				</div>
			</Match>
		</Switch>
	);

	return (
		<div class="execution-content">
			<div class="tab" role="tablist">
				<For each={tabs()}>
					{(tab) => (
						<button
							type="button"
							role="tab"
							aria-selected={exec.activeTab() === tab.id}
							class="tab_button"
							classList={{
								active: exec.activeTab() === tab.id,
								"execution-error":
									tab.id === "execution" && exec.executionError(),
								highlighted: tab.id === "step" && exec.activeTab() !== "step",
							}}
							onClick={() => exec.setActiveTab(tab.id)}
							title={tab.label}
						>
							<i class={`codicon codicon-${tab.icon}`} />
							<span>{tab.label}</span>
						</button>
					)}
				</For>
				<button
					type="button"
					class="tab-clear-button"
					onClick={() => exec.reset()}
					disabled={exec.mode() === "none" && !exec.status()}
					title="Clear results"
				>
					<i class="codicon codicon-clear-all" />
				</button>
			</div>

			<Show when={exec.status()}>
				{(status) => (
					<div class={`status-banner tone-${status().tone}`} role="status">
						<i
							class={`codicon codicon-${status().icon}${status().tone === "busy" ? " codicon-modifier-spin" : ""}`}
						/>
						<div class="status-text">
							<div class="status-title">{status().title}</div>
							<Show when={status().detail}>
								<div class="status-detail">{status().detail}</div>
							</Show>
						</div>
						{statusActions()}
					</div>
				)}
			</Show>

			<div class="tab-content">{renderContent()}</div>
		</div>
	);
}
