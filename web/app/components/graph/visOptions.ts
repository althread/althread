// Use the theme's primary color for consistency with CSS
const THEME_PRIMARY = "hsla(29.329, 66.552%, 52.544%)";

const baseOptions = {
	edges: {
		arrows: "to",
		font: {
			size: 0,
			strokeWidth: 0,
		},
		color: {
			color: "#666",
			highlight: THEME_PRIMARY,
			hover: THEME_PRIMARY,
		},
	},
	interaction: {
		hover: true,
		multiselect: false, // Disable multiselect to make edge selection clearer
		selectConnectedEdges: false,
	},
	physics: {
		enabled: true,
		hierarchicalRepulsion: {
			avoidOverlap: 1,
		},
	},
};

const lightTheme = {
	layout: {
		hierarchical: {
			direction: "LR",
			sortMethod: "directed",
			levelSeparation: 100,
			nodeSpacing: 100,
		},
	},
	nodes: {
		shape: "box",
		shapeProperties: {
			borderRadius: 4,
		},
		margin: 5,
		color: {
			background: "#fffaf2",
			border: "#d9c8b3",
			highlight: {
				background: "#fff2dc",
				border: THEME_PRIMARY, // was "#6FA6F9"
			},
			hover: {
				background: "#fff2dc",
				border: THEME_PRIMARY, // was "#228be6"
			},
		},
		borderWidth: 2,
		borderWidthSelected: 4,
		shadow: {
			enabled: true,
			color: "rgba(121, 89, 46, 0.14)",
			size: 12,
			x: 0,
			y: 4,
		},
		font: {
			color: "#3d3228",
			face: 'Menlo, Monaco, "Courier New", monospace',
			size: 12,
			align: "left",
			multi: "markdown",
			bold: {
				color: THEME_PRIMARY,
			},
			ital: {
				color: "#8b7f72",
				size: 11,
			},
		},
	},
};

const darkTheme = {
	layout: {
		hierarchical: {
			direction: "LR",
			sortMethod: "directed",
			levelSeparation: 100,
			nodeSpacing: 100,
		},
	},
	nodes: {
		shape: "box",
		shapeProperties: {
			borderRadius: 4,
		},
		margin: 5,
		color: {
			background: "#2a2a2e",
			border: "#444",
			highlight: {
				background: "#3c3c42",
				border: THEME_PRIMARY, // was '#9cdcfe'
			},
			hover: {
				background: "#3c3c42",
				border: THEME_PRIMARY, // was '#6FA6F9'
			},
		},
		font: {
			color: "#cccccc",
			face: 'Menlo, Monaco, "Courier New", monospace',
			size: 12,
			align: "left",
			multi: "markdown",
			bold: { color: THEME_PRIMARY },
			ital: { color: "#a0a0a0", size: 11 },
		},
	},
};

// Merge base options with themes
export const themes = {
	light: { ...baseOptions, ...lightTheme },
	dark: { ...baseOptions, ...darkTheme },
};

// Default export for existing components that don't specify a theme
export default themes.light;
