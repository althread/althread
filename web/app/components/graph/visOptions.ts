import type { GraphNodeKind } from "@utils/graphBuilders";

/** Same orange as `--ifm-color-primary`. */
export const THEME_PRIMARY = "hsl(29.329, 66.552%, 52.544%)";

export type GraphTheme = "light" | "dark";

interface Swatch {
	background: string;
	border: string;
}

interface GraphPalette {
	nodes: Record<GraphNodeKind, Swatch>;
	edge: string;
	violationEdge: string;
	label: { color: string; background: string };
}

export const palettes: Record<GraphTheme, GraphPalette> = {
	dark: {
		nodes: {
			normal: { background: "#4a4d52", border: "#7a7f86" },
			initial: { background: "#2f5f8a", border: "#75beff" },
			final: { background: "#2e2e32", border: "#a0a0a0" },
			violation: { background: "#7a2f2f", border: "#f48771" },
		},
		edge: "#6b6b6b",
		violationEdge: "#f48771",
		label: { color: "#e0e0e0", background: "rgba(30, 30, 30, 0.9)" },
	},
	light: {
		nodes: {
			normal: { background: "#e6dccf", border: "#a8998a" },
			initial: { background: "#cfe3f7", border: "#2a6fb0" },
			final: { background: "#fffaf2", border: "#6f6256" },
			violation: { background: "#f9d9d4", border: "#c4314b" },
		},
		edge: "#b5a796",
		violationEdge: "#c4314b",
		label: { color: "#3d3228", background: "rgba(255, 248, 239, 0.95)" },
	},
};

/** vis-network style of a node, according to its role and the theme. */
export function nodeStyle(kind: GraphNodeKind | undefined, theme: GraphTheme) {
	const swatch = palettes[theme].nodes[kind ?? "normal"];
	return {
		color: { ...swatch, highlight: swatch, hover: swatch },
		size: kind === "initial" || kind === "violation" ? 9 : 7,
	};
}

/** vis-network style of an edge. Labels stay hidden until the edge is selected. */
export function edgeStyle(isViolationEdge: boolean, theme: GraphTheme) {
	const color = isViolationEdge
		? palettes[theme].violationEdge
		: palettes[theme].edge;
	return {
		color: { color, highlight: THEME_PRIMARY, hover: THEME_PRIMARY },
		width: isViolationEdge ? 2 : 1,
		font: { size: 0 },
	};
}

/** Font used to show the label of the selected edge. */
export function edgeLabelFont(theme: GraphTheme) {
	const label = palettes[theme].label;
	return {
		size: 12,
		color: label.color,
		background: label.background,
		strokeWidth: 0,
		face: 'Menlo, Monaco, "Courier New", monospace',
	};
}

/**
 * The layout is computed once (hierarchical, left to right) and physics is
 * disabled, so the graph never moves unless the user pans or zooms it.
 */
export const networkOptions = {
	autoResize: true,
	layout: {
		hierarchical: {
			enabled: true,
			direction: "LR",
			sortMethod: "directed",
			shakeTowards: "roots",
			levelSeparation: 48,
			nodeSpacing: 30,
			treeSpacing: 60,
		},
	},
	physics: { enabled: false },
	nodes: {
		shape: "dot",
		borderWidth: 2,
		borderWidthSelected: 3,
		chosen: {
			// A custom function replaces vis' default highlight: set it here.
			node: (
				values: any,
				_id: unknown,
				selected: boolean,
				hovering: boolean,
			) => {
				if (selected || hovering) values.borderColor = THEME_PRIMARY;
				if (selected) {
					values.size += 3;
					values.borderWidth = 3;
				}
			},
			label: false,
		},
	},
	edges: {
		arrows: { to: { enabled: true, scaleFactor: 0.5 } },
		smooth: {
			enabled: true,
			type: "cubicBezier",
			forceDirection: "horizontal",
		},
		selectionWidth: 1.5,
		hoverWidth: 0.5,
	},
	interaction: {
		hover: true,
		tooltipDelay: 150,
		dragNodes: false,
		multiselect: false,
		selectConnectedEdges: false,
		zoomSpeed: 0.6,
	},
};
