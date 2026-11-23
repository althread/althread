/**
 * Execution controller.
 *
 * Owns every piece of state produced by "Run", "Check" and the step-by-step
 * (interactive) mode, so that the UI components only have to render it.
 */

import { type FormattedError, formatAlthreadError } from "@utils/error";
import { getFileContentFromVirtualFS } from "@utils/fileSystemUtils";
import {
	buildGraphFromNodes,
	MAX_VISIBLE_GRAPH_NODES,
} from "@utils/graphBuilders";
import { workerClient } from "@utils/workerClient";
import { batch, createSignal } from "solid-js";
import {
	execute_interactive_step,
	start_interactive_session,
} from "../../pkg/althread_web";
import type {
	CheckResult,
	GraphBuildResult,
	GraphNode,
	MessageFlowEvent,
	NextStateOption,
	RunResult,
	VMState,
	VMStateSelection,
} from "../types/vm-state";

export const WEB_CHECK_MAX_STATES = 10_000;
export const WEB_RUN_MAX_STEPS = 1_000;

export type ExecutionMode = "none" | "run" | "check" | "interactive";
export type ExecutionAction = "run" | "check" | "interactive";
export type ResultTab =
	| "step"
	| "console"
	| "execution"
	| "msg_flow"
	| "vm_states";

export type StatusTone = "neutral" | "busy" | "success" | "warning" | "error";

export interface ExecutionStatus {
	tone: StatusTone;
	icon: string;
	title: string;
	detail?: string;
}

export interface ExecutionContext {
	source: string;
	filePath: string;
	virtualFS: Record<string, string>;
}

interface ExecutionDeps {
	/** Returns what is needed to execute the active file, or null if nothing can be executed. */
	getContext: () => ExecutionContext | null;
}

const EMPTY_GRAPH: GraphBuildResult = { nodes: [], edges: [] };

const isDeadlockError = (error: any) =>
	typeof error?.message === "string" &&
	error.message.toLowerCase().includes("deadlock");

const appendNotice = (text: string, notice: string) => {
	const trimmed = text.trimEnd();
	return trimmed.length > 0 ? `${trimmed}\n\n${notice}` : notice;
};

const plural = (n: number, word: string) =>
	`${n.toLocaleString()} ${word}${n === 1 ? "" : "s"}`;

/** The last line of an Althread error is "Message: ...", which is the most useful summary. */
const summarizeError = (error: any): string | undefined => {
	const message = typeof error?.message === "string" ? error.message : "";
	const location =
		error?.pos?.line !== undefined
			? ` (${error.pos.file_path ? `${error.pos.file_path}:` : "line "}${error.pos.line})`
			: "";
	return message ? `${message}${location}` : undefined;
};

export function createExecutionController(deps: ExecutionDeps) {
	// ---- General state -------------------------------------------------------
	const [mode, setMode] = createSignal<ExecutionMode>("none");
	const [busy, setBusy] = createSignal<ExecutionAction | null>(null);
	const [status, setStatus] = createSignal<ExecutionStatus | null>(null);
	const [activeTab, setActiveTab] = createSignal<ResultTab>("console");

	// ---- Outputs shared by run and interactive modes ------------------------
	const [stdout, setStdout] = createSignal("");
	const [stdoutWarning, setStdoutWarning] = createSignal<string | null>(null);
	const [executionOutput, setExecutionOutput] = createSignal("");
	const [executionError, setExecutionError] = createSignal(false);
	const [structuredError, setStructuredError] =
		createSignal<FormattedError | null>(null);
	const [messageFlow, setMessageFlow] = createSignal<MessageFlowEvent[]>([]);

	// ---- Linear trace (run, counterexample replay, interactive) ------------
	const [traceNodes, setTraceNodes] = createSignal<GraphNode[]>([]);
	const [traceGraph, setTraceGraph] =
		createSignal<GraphBuildResult>(EMPTY_GRAPH);
	const [stepLines, setStepLines] = createSignal<number[][]>([]);

	// ---- Model checking ------------------------------------------------------
	const [checkGraph, setCheckGraph] =
		createSignal<GraphBuildResult>(EMPTY_GRAPH);
	const [checkStateCount, setCheckStateCount] = createSignal(0);
	const [counterexample, setCounterexample] = createSignal<GraphNode[]>([]);
	const [counterexampleStdout, setCounterexampleStdout] = createSignal<
		string[]
	>([]);
	const [counterexampleDebug, setCounterexampleDebug] = createSignal("");
	const [counterexampleMessageFlow, setCounterexampleMessageFlow] =
		createSignal<MessageFlowEvent[]>([]);

	// ---- Interactive mode ----------------------------------------------------
	const [choices, setChoices] = createSignal<NextStateOption[]>([]);
	const [history, setHistory] = createSignal<number[]>([]);
	const [interactiveFinished, setInteractiveFinished] = createSignal(false);
	const [interactiveDeadlock, setInteractiveDeadlock] = createSignal(false);
	/** Source snapshot the interactive session was started with. */
	let interactiveContext: ExecutionContext | null = null;

	// ---- Selection in the state graph ---------------------------------------
	const [selectedVM, setSelectedVM] = createSignal<VMStateSelection | null>(
		null,
	);

	const formatError = (error: any, fallbackPath = ""): FormattedError => {
		const ctx = deps.getContext();
		const path = error?.pos?.file_path || fallbackPath;
		return formatAlthreadError(
			error,
			ctx ? getFileContentFromVirtualFS(ctx.virtualFS, path) : undefined,
		);
	};

	const loadTrace = (nodes: GraphNode[], lines: number[][]) => {
		setTraceNodes(nodes);
		setStepLines(lines);
		setTraceGraph(
			buildGraphFromNodes(nodes, {
				mode: "run",
				stepLines: lines,
			}),
		);
	};

	const clearOutputs = () => {
		setStdout("");
		setStdoutWarning(null);
		setExecutionOutput("");
		setExecutionError(false);
		setStructuredError(null);
		setMessageFlow([]);
		setSelectedVM(null);
		loadTrace([], []);
	};

	const clearInteractive = () => {
		interactiveContext = null;
		setChoices([]);
		setHistory([]);
		setInteractiveFinished(false);
		setInteractiveDeadlock(false);
	};

	const showError = (error: any, fallbackPath = "") => {
		const info = formatError(error, fallbackPath);
		batch(() => {
			setStructuredError(info);
			setExecutionOutput(info.message);
			setExecutionError(true);
			setStatus({
				tone: "error",
				icon: "error",
				title: "Error",
				detail: summarizeError(error),
			});
			setActiveTab("execution");
		});
	};

	// ---- Run -----------------------------------------------------------------

	const run = async () => {
		const ctx = deps.getContext();
		if (!ctx || busy()) return;

		batch(() => {
			clearInteractive();
			clearOutputs();
			setMode("run");
			setBusy("run");
			setStatus({ tone: "busy", icon: "loading", title: "Running…" });
		});

		try {
			const res: RunResult = await workerClient.run(
				ctx.source,
				ctx.filePath,
				ctx.virtualFS,
			);
			const steps = Math.max(0, res.nodes.length - 1);
			const plainStdout = res.stdout.join("\n");

			batch(() => {
				setExecutionOutput(res.debug);
				setMessageFlow(res.message_flow_events);
				loadTrace(res.nodes, res.step_lines || []);
				setStdout(plainStdout);
				setStdoutWarning(
					res.max_steps_reached
						? `Maximum number of steps reached (${WEB_RUN_MAX_STEPS}). Execution was stopped.`
						: null,
				);

				const runtimeError = res.runtime_error;
				if (runtimeError && isDeadlockError(runtimeError)) {
					const info = formatError(runtimeError, ctx.filePath);
					setExecutionOutput(appendNotice(res.debug, info.message));
					setStdout(appendNotice(plainStdout, "Deadlock detected."));
					setStatus({
						tone: "warning",
						icon: "debug-pause",
						title: "Deadlock",
						detail: `All processes are blocked after ${plural(steps, "step")}.`,
					});
					setActiveTab("console");
				} else if (runtimeError) {
					const info = formatError(runtimeError, ctx.filePath);
					setStructuredError(info);
					setExecutionOutput(res.debug);
					setExecutionError(true);
					setStatus({
						tone: "error",
						icon: "error",
						title:
							runtimeError.error_type === "InvariantError"
								? "Invariant violated"
								: "Runtime error",
						detail: summarizeError(runtimeError),
					});
					setActiveTab("execution");
				} else if (res.max_steps_reached) {
					setStatus({
						tone: "warning",
						icon: "watch",
						title: "Step limit reached",
						detail: `Stopped after ${plural(WEB_RUN_MAX_STEPS, "step")}. The program may not terminate.`,
					});
					setActiveTab("console");
				} else {
					setStatus({
						tone: "success",
						icon: "pass",
						title: "Program finished",
						detail: `${plural(steps, "step")} executed (one random interleaving).`,
					});
					setActiveTab("console");
				}
			});
		} catch (e) {
			console.error("Execution error:", e);
			batch(() => {
				clearOutputs();
				showError(e);
			});
		} finally {
			setBusy(null);
		}
	};

	// ---- Check ---------------------------------------------------------------

	const check = async () => {
		const ctx = deps.getContext();
		if (!ctx || busy()) return;

		batch(() => {
			clearInteractive();
			clearOutputs();
			setCheckGraph(EMPTY_GRAPH);
			setCheckStateCount(0);
			setCounterexample([]);
			setCounterexampleStdout([]);
			setCounterexampleDebug("");
			setCounterexampleMessageFlow([]);
			// Switch mode right away so the previous run graph is not displayed while checking.
			setMode("check");
			setBusy("check");
			setStatus({
				tone: "busy",
				icon: "loading",
				title: "Exploring the state space…",
				detail: `Up to ${plural(WEB_CHECK_MAX_STATES, "state")}.`,
			});
			setActiveTab("vm_states");
		});

		try {
			const res: CheckResult = await workerClient.check(
				ctx.source,
				ctx.filePath,
				ctx.virtualFS,
				WEB_CHECK_MAX_STATES,
			);
			const states = res.nodes.length;

			batch(() => {
				setCheckStateCount(states);
				setCounterexample(res.path);
				setCounterexampleStdout(res.path_stdout ?? []);
				setCounterexampleDebug(res.path_debug ?? "");
				setCounterexampleMessageFlow(res.path_message_flow_events ?? []);
				setCheckGraph(
					states > MAX_VISIBLE_GRAPH_NODES
						? EMPTY_GRAPH
						: buildGraphFromNodes(res.nodes, {
								mode: "check",
								violationPath: res.violation_path,
							}),
				);

				if (res.verdict === "violated") {
					const pathSteps = Math.max(0, res.path.length - 1);
					setStatus({
						tone: "error",
						icon: "error",
						title: "Property violated",
						detail: res.exhaustive
							? `Counterexample of ${plural(pathSteps, "step")} found (${plural(states, "state")} explored).`
							: `Counterexample of ${plural(pathSteps, "step")} found before the exploration completed (${plural(states, "state")} explored).`,
					});
					setExecutionOutput(
						res.exhaustive
							? "Violation found! See the highlighted path in the state graph."
							: "Violation found before exploration completed. The counterexample is valid, but the explored graph is partial.",
					);
				} else if (res.verdict === "verified") {
					setStatus({
						tone: "success",
						icon: "pass",
						title: "All properties hold",
						detail: `Every reachable state was explored (${plural(states, "state")}).`,
					});
					setExecutionOutput(
						"Verification complete: No property violations found.",
					);
				} else {
					setStatus({
						tone: "warning",
						icon: "warning",
						title: "Inconclusive",
						detail: `The exploration stopped at ${plural(states, "state")} before it could complete.`,
					});
					setExecutionOutput(
						"Verification inconclusive: The state limit was reached.",
					);
				}
			});
		} catch (e) {
			console.error("Check error:", e);
			showError(e);
		} finally {
			setBusy(null);
		}
	};

	/** Load the checker's counterexample as a linear trace that can be walked step by step. */
	const replayCounterexample = () => {
		const path = counterexample();
		if (path.length === 0) return;
		const replayedStdout = counterexampleStdout();
		const replayedDebug = counterexampleDebug();
		const replayedMessageFlow = counterexampleMessageFlow();

		batch(() => {
			clearOutputs();
			setMode("run");
			loadTrace(
				path,
				path.slice(1).map((node) => node.metadata.lines ?? []),
			);
			setStdout(replayedStdout.join("\n"));
			setMessageFlow(replayedMessageFlow);
			setStatus({
				tone: "error",
				icon: "debug-alt",
				title: "Counterexample replay",
				detail: `${plural(path.length - 1, "step")} leading to the violation. Use ← / → to walk through it.`,
			});
			setExecutionOutput(
				replayedDebug || "Counter-example replay loaded from the checker path.",
			);
			setActiveTab("vm_states");
		});
		selectTraceStep(0);
	};

	// ---- Interactive ---------------------------------------------------------

	/** Records the outcome of get_next_interactive_states (or a deadlock) in the UI state. */
	const applyNextStates = (
		next: NextStateOption[] | undefined,
		deadlockError?: any,
	) => {
		const steps = history().length;
		if (deadlockError) {
			const info = formatError(deadlockError, interactiveContext?.filePath);
			setStdout((s) => appendNotice(s, "Deadlock detected."));
			setExecutionOutput((s) => appendNotice(s, info.message));
			setChoices([]);
			setInteractiveFinished(true);
			setInteractiveDeadlock(true);
			setStatus({
				tone: "warning",
				icon: "debug-pause",
				title: "Deadlock",
				detail: `All processes are blocked after ${plural(steps, "step")}.`,
			});
			return;
		}

		const options = next ?? [];
		setChoices(options);
		setInteractiveFinished(options.length === 0);
		setInteractiveDeadlock(false);
		if (options.length === 0) {
			setExecutionOutput((s) =>
				appendNotice(s, "Program execution completed."),
			);
			setStatus({
				tone: "success",
				icon: "pass",
				title: "Program finished",
				detail: `${plural(steps, "step")} executed.`,
			});
		} else {
			setStatus({
				tone: "neutral",
				icon: "debug-step-over",
				title: "Step-by-step execution",
				detail: `${plural(steps, "step")} so far · ${plural(options.length, "possible next instruction")}.`,
			});
		}
	};

	const startInteractive = () => {
		const ctx = deps.getContext();
		if (!ctx || busy()) return;

		batch(() => {
			clearInteractive();
			clearOutputs();
			setMode("interactive");
			setActiveTab("step");
			interactiveContext = ctx;

			try {
				const res = start_interactive_session(
					ctx.source,
					ctx.filePath,
					ctx.virtualFS,
				);
				loadTrace([{ vm: res.current_state, metadata: { level: 0 } }], []);
				setStdout(res.output?.length ? `${res.output.join("\n")}\n` : "");
				setExecutionOutput(
					"Interactive session started. Choose the next instruction to execute.",
				);
				applyNextStates(res.next_states);
				selectTraceStep(0);
			} catch (e: any) {
				console.error("Interactive mode error:", e);
				if (isDeadlockError(e)) {
					applyNextStates(undefined, e);
				} else {
					clearInteractive();
					setMode("none");
					showError(e, ctx.filePath);
				}
			}
		});
	};

	const executeStep = (choiceIndex: number) => {
		const ctx = interactiveContext;
		if (!ctx || mode() !== "interactive") return;

		batch(() => {
			try {
				const stepResult = execute_interactive_step(
					ctx.source,
					ctx.filePath,
					ctx.virtualFS,
					history(),
					choiceIndex,
				);
				const newHistory = [...history(), choiceIndex];
				setHistory(newHistory);

				if (stepResult.output?.length) {
					setStdout((s) => `${s}${stepResult.output.join("\n")}\n`);
				}
				setMessageFlow((events) => [
					...events,
					...(stepResult.message_flow_events || []),
				]);

				const executed = stepResult.executed_step;
				const stepInfo = executed
					? `Executed: ${executed.prog_name}#${executed.prog_id}\n${stepResult.debug || ""}`
					: stepResult.debug || "";
				setExecutionOutput((s) => (s ? `${s}\n\n${stepInfo}` : stepInfo));

				const nodes = traceNodes();
				loadTrace(
					[
						...nodes,
						{
							vm: stepResult.new_state,
							metadata: { level: nodes.length, lines: stepResult.lines },
						},
					],
					[...stepLines(), stepResult.lines || []],
				);

				if (stepResult.deadlock) {
					applyNextStates(undefined, stepResult.deadlock);
				} else {
					applyNextStates(stepResult.next_states);
				}
				selectTraceStep(traceNodes().length - 1);
			} catch (e: any) {
				console.error("Interactive step error:", e);
				if (isDeadlockError(e)) {
					applyNextStates(undefined, e);
				} else {
					showError(e, ctx.filePath);
				}
			}
		});
	};

	/** Undo the last interactive step by replaying the history without it. */
	const undoStep = () => {
		const ctx = interactiveContext;
		const previous = history();
		if (!ctx || previous.length === 0) return;
		const replay = previous.slice(0, -1);

		batch(() => {
			clearInteractive();
			clearOutputs();
			setMode("interactive");
			interactiveContext = ctx;
			try {
				const res = start_interactive_session(
					ctx.source,
					ctx.filePath,
					ctx.virtualFS,
				);
				loadTrace([{ vm: res.current_state, metadata: { level: 0 } }], []);
				setStdout(res.output?.length ? `${res.output.join("\n")}\n` : "");
				applyNextStates(res.next_states);
			} catch (e) {
				showError(e, ctx.filePath);
				return;
			}
			for (const choice of replay) executeStep(choice);
			setExecutionOutput((s) => appendNotice(s, "Last step undone."));
			setActiveTab("step");
		});
	};

	const stopInteractive = () => {
		batch(() => {
			clearInteractive();
			setMode(traceNodes().length > 0 ? "run" : "none");
			setStatus(null);
			if (activeTab() === "step") setActiveTab("console");
		});
	};

	// ---- Trace navigation ----------------------------------------------------

	const selectTraceStep = (index: number) => {
		const nodes = traceNodes();
		if (nodes.length === 0) return;
		const clamped = Math.max(0, Math.min(index, nodes.length - 1));
		setSelectedVM({
			vm: nodes[clamped].vm as VMState,
			stepIndex: clamped,
			nodeIndex: clamped,
		});
	};

	const reset = () => {
		batch(() => {
			clearInteractive();
			clearOutputs();
			setCheckGraph(EMPTY_GRAPH);
			setCheckStateCount(0);
			setCounterexample([]);
			setCounterexampleStdout([]);
			setCounterexampleDebug("");
			setCounterexampleMessageFlow([]);
			setMode("none");
			setStatus(null);
			setActiveTab("console");
		});
	};

	return {
		// state
		mode,
		busy,
		status,
		activeTab,
		setActiveTab,
		stdout,
		stdoutWarning,
		executionOutput,
		executionError,
		structuredError,
		messageFlow,
		traceNodes,
		traceGraph,
		stepLines,
		checkGraph,
		checkStateCount,
		counterexample,
		choices,
		history,
		interactiveFinished,
		interactiveDeadlock,
		selectedVM,
		setSelectedVM,
		// actions
		run,
		check,
		replayCounterexample,
		startInteractive,
		executeStep,
		undoStep,
		stopInteractive,
		selectTraceStep,
		reset,
	};
}

export type ExecutionController = ReturnType<typeof createExecutionController>;
