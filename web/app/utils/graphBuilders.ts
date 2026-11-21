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

/** Above this number of nodes the state graph is not rendered. */
export const MAX_VISIBLE_GRAPH_NODES = 200;

interface GraphBuildOptions {
	mode?: "run" | "check";
	stepLines?: number[][];
	violationPath?: number[];
}

/** Role of a node in the state graph, used to pick its color. */
export type GraphNodeKind = "initial" | "violation" | "final" | "normal";

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
	const { mode = "check", stepLines = [], violationPath = [] } = options;
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
		const isViolationNode = mode === "check" && violationNodeSet.has(i);
		const isFinal =
			mode === "run"
				? i === nodes.length - 1
				: (metadata.successors?.length ?? 0) === 0;
		const kind: GraphNodeKind =
			i === 0
				? "initial"
				: isViolationNode
					? "violation"
					: isFinal
						? "final"
						: "normal";

		const name = mode === "run" ? `Step ${i}` : `State ${i}`;
		const details = [
			mode === "check" ? `depth ${metadata.level}` : null,
			kind === "initial" ? "initial state" : null,
			kind === "final"
				? mode === "run"
					? "last state"
					: "no successor"
				: null,
			isViolationNode ? "on the counterexample" : null,
		].filter(Boolean);

		visNodes.push({
			id: i,
			label: "",
			level: metadata.level,
			kind,
			title: details.length ? `${name} · ${details.join(" · ")}` : name,
			// Unified format for both run and check modes
			rawState: {
				vm: node.vm,
				stepIndex: mode === "run" ? i : undefined,
				nodeIndex: i,
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
						isViolationEdge,
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
