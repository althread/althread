import { LiteralDisplay } from "@components/shared/Literal";
import { useGraphMaximizeHotkeys } from "@hooks/useGraphMaximizeHotkeys";
import {
	createEffect,
	createSignal,
	For,
	type JSX,
	onCleanup,
	Show,
} from "solid-js";
import type { DataSet } from "vis-data";
import vis from "vis-network/dist/vis-network.esm";
import type {
	Literal,
	MessageFlowEvent,
	ProgramState,
} from "../../types/vm-state";
import { exportCommGraphToCSV } from "./exportToCSV";
import GraphToolbar from "./GraphToolbar";
import { createGraphToolbarHandlers } from "./visHelpers";

/** Node of the message flow graph: process lines, labels and message events. */
interface FlowNode {
	id: string;
	x: number;
	y: number;
	shape: string;
	size: number;
	color: string;
	label?: string;
	font?: { size: number; color: string };
	/** Only set on message event nodes. */
	event?: MessageFlowEvent;
	broadcast?: boolean;
}

interface FlowEdge {
	id?: string;
	from: string;
	to: string;
	color: string;
	width?: number;
	label?: string;
	lines?: number[];
	arrows?: string;
	font?: Record<string, unknown>;
}

const SEND_EVENT = "s".charCodeAt(0);
const RECEIVE_EVENT = "r".charCodeAt(0);

const buildMessageNodeId = (event: MessageFlowEvent, index: number) => {
	if (event.evt_type === RECEIVE_EVENT) {
		return `p${event.receiver}_recv_from${event.sender}_${event.number}_${index}`;
	}

	const receiverId = event.receiver === undefined ? "B" : event.receiver;
	return `p${event.sender}_send_to${receiverId}_${event.number}_${index}`;
};

const DEFAULT_MESSAGE_EDGE_COLOR = "hsla(29, 67%, 53%, 1)";
const MAIN_MESSAGE_EDGE_COLOR = "hsla(205, 18%, 58%, 0.75)";

const getMessageEdgeColor = (event: MessageFlowEvent) =>
	event.sender === 0 || event.receiver === 0
		? MAIN_MESSAGE_EDGE_COLOR
		: DEFAULT_MESSAGE_EDGE_COLOR;

/** Renders literals separated by ", " (DOM nodes cannot be joined as strings). */
const LiteralList = (props: { values: Literal[] }) => (
	<For each={props.values}>
		{(value, i) => (
			<>
				{i() > 0 && ", "}
				<LiteralDisplay value={value} />
			</>
		)}
	</For>
);

const EmptyNote = (props: { children: JSX.Element }) => (
	<p style={{ "padding-left": "1rem", "font-style": "italic" }}>
		{props.children}
	</p>
);

const VmStatePopup = (props: { event: MessageFlowEvent }) => {
	const [activeTab, setActiveTab] = createSignal<"details" | "vmState">(
		"details",
	);
	const vmState = () => props.event.vm_state;
	const globals = () => Object.entries(vmState()?.globals ?? {});
	const channelsOf = (pid: number) =>
		(vmState()?.channels ?? []).filter((c) => c.pid === pid);

	return (
		<Show when={vmState()} fallback={<div>VM state is not available.</div>}>
			<div class="vm-state-popup-content">
				<div class="popup-tabs">
					<button
						type="button"
						class={`popup-tab-button ${activeTab() === "details" ? "active" : ""}`}
						onClick={() => setActiveTab("details")}
					>
						Event Details
					</button>
					<button
						type="button"
						class={`popup-tab-button ${activeTab() === "vmState" ? "active" : ""}`}
						onClick={() => setActiveTab("vmState")}
					>
						VM State
					</button>
				</div>
				<div class="popup-tab-content">
					<Show when={activeTab() === "details"}>
						<p>
							<strong>Type:</strong>{" "}
							{props.event.evt_type === SEND_EVENT ? "Send" : "Receive"}
						</p>
						<p>
							<strong>Sender:</strong> {props.event.sender}
						</p>
						<p>
							<strong>Receiver:</strong> {props.event.receiver ?? "Broadcast"}
						</p>
						<p>
							<strong>Message:</strong> {props.event.message}
						</p>
					</Show>
					<Show when={activeTab() === "vmState"}>
						<div>
							<strong>Globals:</strong>
							<Show
								when={globals().length > 0}
								fallback={<EmptyNote>No global variables.</EmptyNote>}
							>
								<ul>
									<For each={globals()}>
										{([key, value]) => (
											<li>
												{key} = <LiteralDisplay value={value} />
											</li>
										)}
									</For>
								</ul>
							</Show>
						</div>
						<div>
							<strong>Program States & Channels:</strong>
							<Show
								when={(vmState()?.locals.length ?? 0) > 0}
								fallback={<EmptyNote>No running programs</EmptyNote>}
							>
								<For each={vmState()?.locals}>
									{(prog: ProgramState) => (
										<div class="program-state">
											<p>
												<strong>Program {prog.name}</strong> (pid {prog.pid},
												clock {prog.clock})
											</p>
											<ul>
												<li>
													<strong>pc:</strong> {prog.instruction_pointer}
												</li>
												<li>
													<strong>stack:</strong> [
													<LiteralList values={prog.memory} />]
												</li>
												<li>
													<strong>Channels:</strong>
													<Show
														when={channelsOf(prog.pid).length > 0}
														fallback={
															<EmptyNote>No active input channels.</EmptyNote>
														}
													>
														<ul>
															<For each={channelsOf(prog.pid)}>
																{(channel) => (
																	<li>
																		{channel.name} &lt;- [
																		<LiteralList values={channel.values} />]
																	</li>
																)}
															</For>
														</ul>
													</Show>
												</li>
											</ul>
										</div>
									)}
								</For>
							</Show>
						</div>
					</Show>
				</div>
			</div>
		</Show>
	);
};

/** Finds the not-yet-matched send node corresponding to a receive event. */
const searchSenderNode = (
	graphNodes: DataSet<FlowNode>,
	receivingEvent: MessageFlowEvent,
	matchedIds: Set<string>,
): FlowNode | undefined => {
	let senderNode: FlowNode | undefined;
	graphNodes.forEach((node) => {
		if (
			node.event &&
			node.event.evt_type === SEND_EVENT &&
			node.event.sender === receivingEvent.sender &&
			node.event.receiver === receivingEvent.receiver &&
			node.event.number === receivingEvent.number &&
			!matchedIds.has(node.id)
		) {
			senderNode = node;
		}
	});

	if (senderNode) matchedIds.add(senderNode.id);
	return senderNode;
};

/**
 * Message sequence diagram: one horizontal line per process, one dot per
 * send/receive event and an arrow from each send to its receive. Clicking an
 * event shows its details and the VM state at that time.
 */
export const renderMessageFlowGraph = (
	commGraphData: MessageFlowEvent[],
	editor?: any,
) => {
	let container!: HTMLDivElement;
	let network: vis.Network | null = null;
	let popupRef: HTMLDivElement | undefined;
	const [maximized, setMaximized] = createSignal(false);
	const [popupVisible, setPopupVisible] = createSignal(false);
	const [popupContent, setPopupContent] = createSignal<MessageFlowEvent | null>(
		null,
	);
	const [popupPosition, setPopupPosition] = createSignal({ x: 0, y: 0 });
	const [finalPopupPosition, setFinalPopupPosition] = createSignal({
		x: 0,
		y: 0,
	});
	const [isPopupReady, setIsPopupReady] = createSignal(false);
	const currentTheme = () =>
		document.documentElement.dataset.theme === "light" ? "light" : "dark";
	const palette = () =>
		currentTheme() === "light"
			? {
					line: "#7f6e5b",
					node: "#6c5949",
					active: "#0f6bbd",
					popupBg: "#fffaf2",
					popupText: "#332c26",
					popupBorder: "#d8c9b3",
				}
			: {
					line: "white",
					node: "#cccccc",
					active: "#0080ff",
					popupBg: "#232323",
					popupText: "#cccccc",
					popupBorder: "#444",
				};

	// Add this state to remember alignment
	const [popupAlignment, setPopupAlignment] = createSignal<{
		vertical: "top" | "bottom";
		horizontal: "left" | "right";
	}>({ vertical: "bottom", horizontal: "right" });

	if (!commGraphData || commGraphData.length === 0) {
		return (
			<pre>
				The communication graph will appear here (if any communication events
				are recorded).
			</pre>
		);
	}

	createEffect(() => {
		// Clear matchedSendNodeIds for each new graph
		// matchedSendNodeIds.clear();
		const matchedSendNodeIds = new Set<string>();

		const nodes = new vis.DataSet<FlowNode>();
		const edges = new vis.DataSet<FlowEdge>();
		const processes: Map<number, string> = new Map();

		// extract processes name to make one line per process
		commGraphData.forEach((event) => {
			const getProcessName = (pid: number) => {
				const prog = event.vm_state?.locals?.find((p) => p.pid === pid);
				return prog ? prog.name : `P${pid}`;
			};

			if (event.evt_type === SEND_EVENT) {
				processes.set(event.sender, event.actor_prog_name);
				// Ensure receiver is also created if it's a unicast send
				if (event.receiver !== undefined && !processes.has(event.receiver)) {
					processes.set(event.receiver, getProcessName(event.receiver));
				}
			} else {
				processes.set(event.receiver, event.actor_prog_name);
				// Ensure sender is also created
				if (event.sender !== undefined && !processes.has(event.sender)) {
					processes.set(event.sender, getProcessName(event.sender));
				}
			}
		});

		const ySpacing = 150; // space between processes vertically
		const xStart = -50;
		const xEnd = 450;

		// lines generation
		processes.forEach((processName, index) => {
			// define two nodes for each process (start and end of the line)
			const startNode: FlowNode = {
				id: `p${index}_start`,
				y: index * ySpacing,
				x: xStart,
				shape: "dot",
				size: 1,
				color: palette().line,
			};
			const endNode: FlowNode = {
				id: `p${index}_end`,
				y: index * ySpacing,
				x: xEnd,
				shape: "dot",
				size: 1,
				color: palette().line,
			};
			nodes.add([startNode, endNode]);
			edges.add({
				from: startNode.id,
				to: endNode.id,
				color: palette().line,
				width: 2,
			}); // line for each process

			const processNumberNode: FlowNode = {
				id: `p${index}_number`,
				y: index * ySpacing,
				x: xStart - 40,
				label: `P${index} \n(${processName})`,
				shape: "text",
				size: 0,
				color: palette().line,
				font: {
					size: 18,
					color: palette().line,
				},
			};

			nodes.add(processNumberNode);
		});

		// message arrows
		let i: number = 0;
		commGraphData.forEach((event, eventIndex) => {
			const nodeId = buildMessageNodeId(event, eventIndex);
			const isBroadcast = event.receiver === undefined;

			//lengthen the process lines for each new event so it doesnt go overboard
			processes.forEach((_, processNumber) => {
				nodes.update({
					id: `p${processNumber}_end`,
					x: 500 + commGraphData.length * 1.5 * i,
				} as FlowNode);
			});

			// a reception is drawn on the receiver line, a send on the sender line
			const yposLine =
				event.evt_type === RECEIVE_EVENT ? event.receiver : event.sender;

			const msgNode: FlowNode = {
				id: nodeId,
				y: yposLine * ySpacing,
				x: xStart + 20 + i * 50,
				shape: "dot",
				size: 5,
				color: palette().node,
				event: event,
				broadcast: isBroadcast,
			};
			nodes.add(msgNode);
			i++;
		});

		nodes.forEach((node) => {
			if (node.event?.evt_type === RECEIVE_EVENT) {
				const sender = searchSenderNode(nodes, node.event, matchedSendNodeIds);
				if (sender) {
					edges.add({
						from: sender.id,
						to: node.id,
						label: node.event.message,
						lines: node.event.lines,
						font: {
							size: 20,
							color: palette().line,
							align: "middle",
							background: "none",
							strokeWidth: 0,
						},
						arrows: "to",
						color: getMessageEdgeColor(node.event),
					});
				}
			}
		});

		const data = { nodes: nodes.get(), edges: edges.get() };

		const options = {
			layout: {
				hierarchical: false,
			},
			edges: {
				smooth: false,
			},
			physics: false,
			nodes: {
				fixed: true,
			},
		};

		network = new vis.Network(container, data, options);
		network.once("stabilized", () => {
			if (network) network.fit();
		});

		/* to display the associated vm state when clicking on an event node */
		let previous_node_id: string | null = null;
		let previous_node_colour: string | null = null;
		network.on("click", (event) => {
			if (previous_node_id) {
				//change previous clicked node back to its original colour
				nodes.update({
					id: previous_node_id,
					color: previous_node_colour,
				} as FlowNode);
				setPopupVisible(false);
			}
			if (event.nodes.length > 0) {
				const node_id = String(event.nodes[0]);
				//popup creation & change node colour
				if (
					!(
						node_id.includes("_start") ||
						node_id.includes("_end") ||
						node_id.includes("_number")
					)
				) {
					//clicked node is only one of the communication event
					const node = nodes.get(node_id);
					if (!node?.event) return;
					const pos = event.pointer.DOM;
					setIsPopupReady(false);
					setPopupContent(node.event);
					setPopupPosition({ x: pos.x, y: pos.y });
					setPopupAlignment({ vertical: "bottom", horizontal: "right" }); // <--- Reset alignment on new popup
					setPopupVisible(true);
					previous_node_id = node_id;
					previous_node_colour = node.color;
					nodes.update({ id: node_id, color: palette().active } as FlowNode);

					// Also highlight lines if available
					if (node.event.lines?.length) {
						editor?.highlightLines?.(node.event.lines);
					}
				}
			} else if (event.edges.length > 0) {
				const edgeId = event.edges[0];
				const edge = edges.get(edgeId as string);
				if (edge?.lines?.length) {
					editor?.highlightLines?.(edge.lines);
				}
			} else {
				setPopupVisible(false);
			}
		});

		onCleanup(() => {
			if (network) network.destroy();
		});
	});

	createEffect(() => {
		const popupEl = popupRef;
		if (popupVisible() && popupEl) {
			const reposition = () => {
				if (!container) return;
				const graphContainer = container;

				const graphWidth = graphContainer.offsetWidth;
				const graphHeight = graphContainer.offsetHeight;
				const popupWidth = popupEl.offsetWidth;
				const popupHeight = popupEl.offsetHeight;

				const initialPos = popupPosition();
				let finalX = initialPos.x + 15;
				let finalY = initialPos.y + 15;

				// Use current alignment as starting point
				let { vertical, horizontal } = popupAlignment();

				// Only change alignment if the popup would overflow
				let changed = false;

				// Horizontal
				if (horizontal === "right" && finalX + popupWidth > graphWidth) {
					horizontal = "left";
					changed = true;
				} else if (horizontal === "left" && finalX < 0) {
					horizontal = "right";
					changed = true;
				}

				// Vertical
				if (vertical === "bottom" && finalY + popupHeight > graphHeight) {
					vertical = "top";
					changed = true;
				} else if (vertical === "top" && finalY < 0) {
					vertical = "bottom";
					changed = true;
				}

				if (changed) setPopupAlignment({ vertical, horizontal });

				// Recalculate position based on alignment
				finalX =
					horizontal === "right"
						? initialPos.x + 15
						: initialPos.x - popupWidth - 15;
				finalY =
					vertical === "bottom"
						? initialPos.y + 15
						: initialPos.y - popupHeight - 15;

				// Clamp as fallback
				if (finalX < 5) finalX = 5;
				if (finalY < 5) finalY = 5;
				if (finalX + popupWidth > graphWidth - 5)
					finalX = graphWidth - popupWidth - 5;
				if (finalY + popupHeight > graphHeight - 5)
					finalY = graphHeight - popupHeight - 5;

				setFinalPopupPosition({ x: finalX, y: finalY });
				setIsPopupReady(true);
			};

			// Use a ResizeObserver to automatically reposition the popup when its size changes (e.g., switching tabs).
			const observer = new ResizeObserver(reposition);
			observer.observe(popupEl);

			onCleanup(() => {
				observer.disconnect();
			});
		} else {
			setIsPopupReady(false);
		}
	});

	useGraphMaximizeHotkeys(setMaximized);

	const { handleMaximize, handleRecenter, handleDownload } =
		createGraphToolbarHandlers(
			() => network,
			() => container,
			() => setMaximized((v: boolean) => !v),
		);

	return (
		<>
			<div class={`state-graph${maximized() ? " maximized" : ""}`}>
				<div ref={container} style="height: 100%;" />
				<GraphToolbar
					onFullscreen={handleMaximize}
					onRecenter={handleRecenter}
					onDownloadCSV={() => exportCommGraphToCSV(commGraphData)}
					onDownload={handleDownload}
					isFullscreen={maximized()}
				/>
				{popupVisible() && popupContent() && (
					<div
						ref={popupRef}
						class="vm-state-popup"
						style={{
							position: "absolute",
							top: `${finalPopupPosition().y}px`,
							left: `${finalPopupPosition().x}px`,
							visibility: isPopupReady() ? "visible" : "hidden",
							background: palette().popupBg,
							color: palette().popupText,
							border: `1px solid ${palette().popupBorder}`,
						}}
					>
						<VmStatePopup event={popupContent() as MessageFlowEvent} />
					</div>
				)}
			</div>
		</>
	);
};

/** Component wrapper so that the graph (and vis-network) can be lazy-loaded. */
export default function MessageFlowGraph(props: {
	events: MessageFlowEvent[];
	editor?: any;
}) {
	return <>{renderMessageFlowGraph(props.events, props.editor)}</>;
}
