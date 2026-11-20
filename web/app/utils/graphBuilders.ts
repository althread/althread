/**
 * Graph Builder Utilities
 *
 * Unified graph building for both run and check modes using GraphNode structure.
 */

import type {
	GraphBuildResult,
	GraphNode,
	VisGraphEdge,
	VisGraphNode,
} from "../types/vm-state";

interface GraphBuildOptions {
	mode?: "run" | "check";
	stepLines?: number[][];
	violationPath?: number[];
	theme?: "dark" | "light";
}

export function stableStringify(value: unknown): string {
	if (value === null || value === undefined) return JSON.stringify(value);

	if (Array.isArray(value)) {
		return `[${value.map((item) => stableStringify(item)).join(",")}]`;
	}

	if (typeof value === "object") {
		const entries = Object.entries(value as Record<string, unknown>)
			.sort(([a], [b]) => a.localeCompare(b))
			.map(([key, val]) => `${JSON.stringify(key)}:${stableStringify(val)}`);
		return `{${entries.join(",")}}`;
	}

	return JSON.stringify(value);
}

/**
 * Build vis.js graph from GraphNode array (works for both run and check modes)
 */
export function buildGraphFromNodes(
	nodes: GraphNode[],
	options: GraphBuildOptions = {},
): GraphBuildResult {
	const {
		mode = "check",
		stepLines = [],
		violationPath = [],
		theme = "dark",
	} = options;
	const visNodes: VisGraphNode[] = [];
	const visEdges: VisGraphEdge[] = [];
	const violationNodeSet = new Set(violationPath);
	const violationEdgeSet = new Set<string>();
	for (let i = 0; i < violationPath.length - 1; i++) {
		violationEdgeSet.add(`${violationPath[i]}->${violationPath[i + 1]}`);
	}

	for (let i = 0; i < nodes.length; i++) {
		const node = nodes[i];
		const metadata = node.metadata;

		// Create a summary label for the node
		const processCount = node.vm.locals.length;
		const label =
			mode === "run"
				? `${i}` // Run mode: show sequential index
				: `${i}`; // Check mode: show node index

		// Create detailed state for tooltip
		const globalCount = Object.keys(node.vm.globals).length;
		const channelCount = node.vm.channels.length;
		const pendingCount = node.vm.pending_deliveries.length;

		const title = `Level: ${metadata.level}\nProcesses: ${processCount}\nGlobals: ${globalCount}\nChannels: ${channelCount}\nPending: ${pendingCount}`;

		// Determine node color based on mode and violation path
		const isViolationNode = mode === "check" && violationNodeSet.has(i);

		let backgroundColor, borderColor, fontColor;
		if (mode === "run") {
			backgroundColor = "#314d31";
			borderColor = "#a6dfa6";
			fontColor = theme === "dark" ? "#ffffff" : "#1a3a1a";
		} else {
			backgroundColor = isViolationNode ? "#4d3131" : "#314d31";
			borderColor = isViolationNode ? "#ec9999" : "#a6dfa6";
			fontColor =
				theme === "dark" ? "#ffffff" : isViolationNode ? "#8b1a1a" : "#1a3a1a";
		}

		visNodes.push({
			id: i,
			label,
			level: metadata.level,
			color: {
				background: backgroundColor,
				border: borderColor,
				highlight: {
					border: "hsla(29.329, 66.552%, 52.544%)",
					background: backgroundColor,
				},
				hover: {
					border: "hsla(29.329, 66.552%, 52.544%)",
					background: backgroundColor,
				},
			},
			font: { size: 10, color: fontColor } as any,
			borderWidth: 1,
			title,
			// Unified format for both run and check modes
			rawState: {
				vm: node.vm,
				stepIndex: mode === "run" ? i : undefined,
				level: metadata.level,
			},
			isViolationNode,
		} as any);

		// Build edges
		if (mode === "run") {
			// Run mode: sequential edges with step lines
			if (i < nodes.length - 1) {
				const lines = stepLines[i] || [];
				visEdges.push({
					id: `e${i}`,
					from: i,
					to: i + 1,
					label: `step ${i + 1}`,
					lines,
					font: { size: 0 }, // Hidden by default
				});
			}
		} else {
			// Check mode: explicit successors with indices
			if (metadata.successors) {
				let succIndex = 0;
				for (const succ of metadata.successors) {
					const edgeLabel =
						succ.name + "#" + succ.pid + ": " + succ.lines.join(",");
					const isViolationEdge = violationEdgeSet.has(
						`${i}->${succ.to_index}`,
					);

					visEdges.push({
						id: `e${i}-${succ.to_index}-${succIndex++}`,
						from: i,
						to: succ.to_index,
						label: edgeLabel,
						lines: succ.lines.map((l: number) => Number(l)),
						font: { size: 0 }, // Hidden by default
						color: isViolationEdge
							? { color: "#ec9999", highlight: "#ec9999", hover: "#ec9999" }
							: undefined,
						width: isViolationEdge ? 2 : undefined,
					});
				}
			}
		}
	}

	return { nodes: visNodes, edges: visEdges };
}

/**
 * Find node index by VM state (for matching states across graphs)
 */
export function findNodeByState(nodes: GraphNode[], targetState: any): number {
	// This is a simple implementation - could be improved with better state comparison
	for (let i = 0; i < nodes.length; i++) {
		if (JSON.stringify(nodes[i].vm) === JSON.stringify(targetState)) {
			return i;
		}
	}
	return -1;
}

/**
 * Extract path from graph nodes (for error traces in check mode)
 */
export function extractPath(
	nodes: GraphNode[],
	startIndex: number,
	endIndex: number,
): GraphNode[] {
	// For now, return a simple slice - could implement BFS for complex graphs
	if (startIndex <= endIndex) {
		return nodes.slice(startIndex, endIndex + 1);
	}
	return [];
}
