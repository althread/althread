import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import type {
	ChannelState,
	VMState,
	VMStateSelection,
} from "../../types/vm-state";
import { stableStringify } from "../../utils/graphBuilders";
import GlobalsDisplay from "../shared/GlobalsDisplay";
import InFlightMessagesDisplay from "../shared/InFlightMessagesDisplay";
import ProcessStateCard from "../shared/ProcessStateCard";
import "./VMStateInspector.css";

interface VMStateInspectorProps {
	node: VMStateSelection | null;
	onClose: () => void;
}

export default function VMStateInspector(props: VMStateInspectorProps) {
	// All collapse state lives here so we can enforce "last open" guard
	const [globalsCollapsed, setGlobalsCollapsed] = createSignal(false);
	const [messagesCollapsed, setMessagesCollapsed] = createSignal(false);
	const [processesCollapsed, setProcessesCollapsed] = createSignal(false);

	const [globalsChanged, setGlobalsChanged] = createSignal(false);
	const [messagesChanged, setMessagesChanged] = createSignal(false);

	const state = createMemo((): VMState | null => {
		return props.node?.vm ?? null;
	});

	const getChannelsForProcess = (pid: number): ChannelState[] => {
		const s = state();
		if (!s) return [];
		return s.channels.filter((ch: ChannelState) => ch.pid === pid);
	};

	// Auto-collapse sections if they are empty, but allow manual expansion.
	// We also track changes for those that are collapsed.
	let lastSeenGlobalsRaw = "";
	let lastSeenMessagesRaw = "";
	let lastNode: VMStateSelection | null = null;
	let wasGlobalsEmpty = false;
	let wasMessagesEmpty = false;
	let wasGlobalsCollapsed = false;
	let wasMessagesCollapsed = false;

	createEffect(() => {
		const s = state();
		const node = props.node;
		const currentGlobalsCollapsed = globalsCollapsed();
		const currentMessagesCollapsed = messagesCollapsed();

		if (!s || !node) return;

		const isGlobalsEmpty = Object.keys(s.globals).length === 0;
		const isMessagesEmpty =
			s.pending_deliveries.length === 0 && s.waiting_send.length === 0;

		// Default collapse purely on NEW selection OR when content BECOMES empty
		if (node !== lastNode) {
			lastNode = node;
			if (isGlobalsEmpty) setGlobalsCollapsed(true);
			if (isMessagesEmpty) setMessagesCollapsed(true);
		} else {
			if (isGlobalsEmpty && !wasGlobalsEmpty) setGlobalsCollapsed(true);
			if (isMessagesEmpty && !wasMessagesEmpty) setMessagesCollapsed(true);
		}

		wasGlobalsEmpty = isGlobalsEmpty;
		wasMessagesEmpty = isMessagesEmpty;

		// Globals change tracking
		if (!currentGlobalsCollapsed) {
			// Expanded: clear change indicator
			setGlobalsChanged(false);
		} else {
			// Collapsed: If just collapsed, snapshot state. Otherwise check if it changed.
			if (!wasGlobalsCollapsed) {
				lastSeenGlobalsRaw = stableStringify(s.globals);
			} else if (!isGlobalsEmpty) {
				const currentGlobalsRaw = stableStringify(s.globals);
				if (currentGlobalsRaw !== lastSeenGlobalsRaw) {
					setGlobalsChanged(true);
				}
			}
		}

		// Messages change tracking
		if (!currentMessagesCollapsed) {
			// Expanded: clear indicator
			setMessagesChanged(false);
		} else {
			// Collapsed: If just collapsed, snapshot state. Otherwise check if it changed.
			if (!wasMessagesCollapsed) {
				lastSeenMessagesRaw = stableStringify({
					p: s.pending_deliveries,
					w: s.waiting_send,
				});
			} else if (!isMessagesEmpty) {
				const currentMessagesRaw = stableStringify({
					p: s.pending_deliveries,
					w: s.waiting_send,
				});
				if (currentMessagesRaw !== lastSeenMessagesRaw) {
					setMessagesChanged(true);
				}
			}
		}

		wasGlobalsCollapsed = currentGlobalsCollapsed;
		wasMessagesCollapsed = currentMessagesCollapsed;
	});

	// Count how many sections are currently visible AND expanded
	const expandedCount = createMemo(() => {
		let count = 0;
		if (!globalsCollapsed()) count++;
		if (!messagesCollapsed()) count++;
		if (!processesCollapsed()) count++;
		return count;
	});

	// A section can collapse only if it is already collapsed (i.e. re-expanding)
	// OR if there is more than one section open right now.
	const canCollapse = () => expandedCount() > 1;

	const toggleGlobals = () => {
		if (!globalsCollapsed() && !canCollapse()) return;
		setGlobalsCollapsed((c) => !c);
	};

	const toggleMessages = () => {
		if (!messagesCollapsed() && !canCollapse()) return;
		setMessagesCollapsed((c) => !c);
	};

	const toggleProcesses = () => {
		if (!processesCollapsed() && !canCollapse()) return;
		setProcessesCollapsed((c) => !c);
	};

	return (
		<div class="vm-state-inspector">
			<div class="inspector-header">
				<div class="header-title">
					<i class="codicon codicon-debug-console"></i>
					<span>State Details</span>
				</div>
				<button
					type="button"
					class="close-btn"
					onClick={props.onClose}
					title="Close Inspector"
				>
					<i class="codicon codicon-close"></i>
				</button>
			</div>

			<Show
				when={state()}
				fallback={
					<div class="inspector-empty-state">
						Select a node in the graph to inspect its state
					</div>
				}
			>
				{(vmState) => (
					<div class="inspector-sections">
						<GlobalsDisplay
							globals={vmState().globals}
							collapsed={globalsCollapsed()}
							canCollapse={canCollapse()}
							onToggle={toggleGlobals}
							hasChanged={globalsChanged()}
						/>
						<InFlightMessagesDisplay
							pendingDeliveries={vmState().pending_deliveries}
							waitingSend={vmState().waiting_send}
							collapsed={messagesCollapsed()}
							canCollapse={canCollapse()}
							onToggle={toggleMessages}
							hasChanged={messagesChanged()}
						/>

						<div
							class={`section programs-section${processesCollapsed() ? " section-collapsed" : ""}`}
						>
							<div
								class={`collapsible-header${!processesCollapsed() && !canCollapse() ? " header-locked" : ""}`}
								onClick={toggleProcesses}
								onKeyDown={(e) => {
									if (e.key === "Enter" || e.key === " ") {
										e.preventDefault();
										toggleProcesses();
									}
								}}
								role="button"
								tabIndex={!processesCollapsed() && !canCollapse() ? -1 : 0}
								aria-expanded={!processesCollapsed()}
								aria-controls="processes-section-body"
								title={
									!processesCollapsed() && !canCollapse()
										? "Cannot collapse — only section open"
										: processesCollapsed()
											? "Expand Processes"
											: "Collapse Processes"
								}
							>
								<i
									class={`codicon ${processesCollapsed() ? "codicon-chevron-left" : "codicon-chevron-right"} header-toggle-icon`}
								></i>
								<i class="codicon codicon-debug-all header-section-icon"></i>
								<span class="section-label">Processes</span>
								<span class="header-count">{vmState().locals.length}</span>
							</div>
							<div
								id="processes-section-body"
								class={`section-body${processesCollapsed() ? " section-body-hidden" : ""}`}
							>
								<Show
									when={vmState().locals.length > 0}
									fallback={<div class="empty-state">No processes</div>}
								>
									<For each={vmState().locals}>
										{(prog) => (
											<ProcessStateCard
												process={prog}
												channels={getChannelsForProcess(prog.pid)}
											/>
										)}
									</For>
								</Show>
							</div>
						</div>
					</div>
				)}
			</Show>
		</div>
	);
}
