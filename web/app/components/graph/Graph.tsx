import { useGraphMaximizeHotkeys } from "@hooks/useGraphMaximizeHotkeys";
import { MAX_VISIBLE_GRAPH_NODES } from "@utils/graphBuilders";
import {
	createEffect,
	createSignal,
	on,
	onCleanup,
	Show,
	untrack,
} from "solid-js";
import vis from "vis-network/dist/vis-network.esm";
import { exportStatesToCSV } from "./exportToCSV";
import GraphToolbar from "./GraphToolbar";
import MetadataDisplay from "./MetadataDisplay";
import { nodeToString } from "./Node";
import {
	createGraphToolbarHandlers,
	formatState,
	revealNode,
} from "./visHelpers";
import { edgeStyle, networkOptions, nodeStyle } from "./visOptions";

interface GraphProps {
	nodes: any[];
	edges: any[];
	vm_states?: any[];
	setLoadingAction?: (action: string | null) => void;
	theme?: "light" | "dark";
	onEdgeClick?: (edgeId: string, edgeData: any) => void;
	onNodeSelect?: (node: any | null) => void;
	tooLargeStatusMessage?: string;
	/** Node to select (and focus) in the graph, e.g. the current step of a trace. */
	selectedNode?: number | string | null;
}

export { MAX_VISIBLE_GRAPH_NODES };

/** Below this zoom level nodes are too small to be clicked comfortably. */
const MIN_INITIAL_SCALE = 0.9;
/** Distance between two dots of the background grid, in graph units. */
const GRID_SPACING = 20;
/** Smallest on-screen distance between two dots, in pixels. */
const MIN_GRID_SPACING = 10;

export default (props: GraphProps) => {
	let container: HTMLDivElement | undefined;
	let network: vis.Network | null = null;
	let lastClickedNode: string | number | null = null;
	/** The transition picked in the graph, described in a chip over the graph. */
	const [selectedEdge, setSelectedEdge] = createSignal<{
		label: string;
		lines: number[];
	} | null>(null);
	const [maximized, setMaximized] = createSignal(false);
	const [showDetails, setDetails] = createSignal(false);
	const [selectedNodeContent, setSelectedNodeContent] = createSignal<
		string | null
	>(null);
	const isGraphTooLarge = () =>
		(props.nodes?.length || 0) > MAX_VISIBLE_GRAPH_NODES;
	const resolveVMState = (rawState: any) => {
		if (!rawState) return null;
		return rawState.vm ?? rawState.state ?? rawState;
	};
	const resolveNodeState = (node: any) =>
		node?.rawState ?? node?.state ?? node?.vm_state ?? null;
	const isDarkTheme = () => props.theme !== "light";
	const graphOverlayStyles = () => ({
		background: "var(--app-surface)",
		color: "var(--app-text)",
		border: "var(--app-border)",
		shadow: isDarkTheme() ? "var(--app-shadow-md)" : "var(--app-shadow-sm)",
		buttonColor: "var(--app-text-muted)",
	});

	createEffect(() => {
		if (isGraphTooLarge()) {
			setSelectedNodeContent(null);
			if (props.onNodeSelect) {
				props.onNodeSelect(null);
			}
			props.setLoadingAction?.(null);
			return;
		}

		if (!container) {
			return;
		}

		const theme = props.theme === "light" ? "light" : "dark";
		const nodes = new vis.DataSet(
			(props.nodes || []).map((node: any) => ({
				...node,
				...nodeStyle(node.kind, theme),
			})),
		);
		// Edges going back to an earlier (or the same) level are drawn as arcs,
		// so that cycles do not overlap the forward edges.
		const levels = new Map<unknown, number>(
			(props.nodes || []).map((node: any) => [node.id, node.level ?? 0]),
		);
		const edges = new vis.DataSet(
			(props.edges || []).map((edge: any) => {
				const isBackEdge =
					(levels.get(edge.to) ?? 0) <= (levels.get(edge.from) ?? 0);
				return {
					...edge,
					...edgeStyle(!!edge.isViolationEdge, theme),
					...(isBackEdge
						? { smooth: { enabled: true, type: "curvedCW", roundness: 0.35 } }
						: {}),
				};
			}),
		);

		network = new vis.Network(
			container,
			{ nodes, edges } as any,
			networkOptions as any,
		);
		const net = network;

		const selectNodeById = (nodeId: string | number | null) => {
			if (nodeId === null) {
				setSelectedNodeContent(null);
				props.onNodeSelect?.(null);
				return;
			}
			const node: any = nodes.get(nodeId as any);
			if (!node) {
				setSelectedNodeContent(null);
				props.onNodeSelect?.(null);
				return;
			}
			const rawState = resolveNodeState(node);
			const vmState = resolveVMState(rawState);
			if (node.fullLabel) {
				setSelectedNodeContent(node.fullLabel);
			} else if (vmState) {
				setSelectedNodeContent(nodeToString(vmState));
			}
			lastClickedNode = nodeId;
			props.onNodeSelect?.(rawState || null);
		};

		// vis-network runs this handler while it processes the pointer-up event.
		// An exception here would leave its gesture recognizer believing the
		// button is still pressed: the graph would then follow the mouse.
		// Clicking selects (no zoom). Clicking an edge keeps the node selection.
		net.on("click", (params) => {
			try {
				if (params.nodes?.length > 0) {
					setSelectedEdge(null);
					selectNodeById(params.nodes[0]);
				} else if (params.edges?.length > 0) {
					const edgeId = params.edges[0];
					const edge: any = edges.get(edgeId);
					if (!edge) return;
					setSelectedEdge({
						label: String(edge.label ?? ""),
						lines: [...new Set<number>(edge.lines ?? [])],
					});
					props.onEdgeClick?.(edgeId, edge);
				} else {
					net.unselectAll();
					setSelectedEdge(null);
					selectNodeById(null);
				}
			} catch (error) {
				console.error("Graph click handler failed:", error);
			}
		});

		// Keep the background dot grid attached to the graph while panning/zooming.
		const syncGrid = () => {
			if (!container) return;
			let spacing = GRID_SPACING * net.getScale();
			// Too dense once zoomed out: skip dots (multiples keep the alignment).
			while (spacing < MIN_GRID_SPACING) spacing *= 2;
			const origin = net.canvasToDOM({ x: 0, y: 0 });
			container.style.backgroundSize = `${spacing}px ${spacing}px`;
			container.style.backgroundPosition = `${origin.x}px ${origin.y}px`;
		};
		net.on("afterDrawing", syncGrid);

		// The layout is static: fit once, then only the user moves the view.
		// Long graphs would become unreadable once fitted: keep a minimum zoom
		// and start on the selected node (or the initial state).
		net.fit({ animation: false });
		const selected = untrack(() => props.selectedNode);
		const hasSelection = selected !== null && selected !== undefined;
		if (hasSelection) net.selectNodes([selected]);
		if (net.getScale() < MIN_INITIAL_SCALE) {
			const anchor = hasSelection ? selected : props.nodes[0]?.id;
			const position = anchor !== undefined ? net.getPosition(anchor) : null;
			if (position) {
				// Leave some room on the left of the initial state.
				const offset = hasSelection
					? 0
					: (container.clientWidth / 2 - 40) / MIN_INITIAL_SCALE;
				net.moveTo({
					position: { x: position.x + offset, y: position.y },
					scale: MIN_INITIAL_SCALE,
				});
			}
		}
		props.setLoadingAction?.(null);

		onCleanup(() => {
			if (network) {
				network.destroy();
				network = null;
			}
			setSelectedEdge(null);
		});
	});

	createEffect(
		on(
			() => props.selectedNode,
			(selected) => {
				if (!network || selected === null || selected === undefined) return;
				setSelectedEdge(null);
				network.selectNodes([selected]);
				// The user just clicked it: it is already visible.
				if (selected === lastClickedNode) return;
				revealNode(network, container, selected);
			},
			{ defer: true },
		),
	);

	useGraphMaximizeHotkeys(setMaximized);

	const { handleMaximize, handleRecenter, handleDownload, handleDetails } =
		createGraphToolbarHandlers(
			() => network,
			() => container,
			() => setMaximized((v: boolean) => !v),
			() => setDetails((v: boolean) => !v),
		);

	return (
		<div class={`state-graph${maximized() ? " maximized" : ""}`}>
			{isGraphTooLarge() ? (
				<div
					style={`display: flex; height: 100%; align-items: center; justify-content: center; padding: 24px; color: var(--app-text-muted); text-align: center;`}
				>
					<div>
						<div style="font-weight: 600; margin-bottom: 8px;">
							Graph not displayed
						</div>
						{props.tooLargeStatusMessage ? (
							<div style="margin-bottom: 8px; opacity: 0.9;">
								{props.tooLargeStatusMessage}
							</div>
						) : null}
						<div>
							This graph has {props.nodes.length} nodes, which is above the
							display limit of {MAX_VISIBLE_GRAPH_NODES}.
						</div>
					</div>
				</div>
			) : (
				<div ref={container} class="graph-canvas" />
			)}

			{!props.onNodeSelect && selectedNodeContent() && (
				<div
					class="node-details-overlay"
					style={`position: absolute; top: 10px; right: 10px; background: ${graphOverlayStyles().background}; color: ${graphOverlayStyles().color}; padding: 10px; border: 1px solid ${graphOverlayStyles().border}; border-radius: 5px; max-width: 400px; max-height: 80%; overflow: auto; white-space: pre-wrap; z-index: 1000; box-shadow: 0 4px 6px ${graphOverlayStyles().shadow};`}
				>
					<div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
						<h3 style="margin: 0;">Node State</h3>
						<button
							type="button"
							style={`background: none; border: none; color: ${graphOverlayStyles().buttonColor}; cursor: pointer;`}
							onClick={() => setSelectedNodeContent(null)}
						>
							<i class="codicon codicon-close"></i>
						</button>
					</div>
					<div
						style="font-family: monospace;"
						innerHTML={formatState(selectedNodeContent()!)}
					></div>
				</div>
			)}

			<Show when={!isGraphTooLarge() && selectedEdge()}>
				{(edge) => (
					<div class="graph-edge-chip" role="status">
						<i class="codicon codicon-arrow-right" />
						<span class="graph-edge-chip-label">{edge().label}</span>
						<Show when={edge().lines.length > 0}>
							<span class="graph-edge-chip-lines">
								{edge().lines.length === 1 ? "line" : "lines"}{" "}
								{edge().lines.join(", ")}
							</span>
						</Show>
					</div>
				)}
			</Show>

			{!isGraphTooLarge() && showDetails() ? (
				<MetadataDisplay nodes={props.nodes} />
			) : null}
			{!isGraphTooLarge() ? (
				<GraphToolbar
					onFullscreen={handleMaximize}
					onRecenter={handleRecenter}
					onDownload={handleDownload}
					onDownloadCSV={() => exportStatesToCSV(props.nodes, props.edges)}
					onDetails={handleDetails}
					isFullscreen={maximized()}
				/>
			) : null}
		</div>
	);
};
