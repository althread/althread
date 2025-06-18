const baseOptions = {
  edges: {
    arrows: "to",
  },
  interaction: {
    hover: true,
    multiselect: true
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
      direction: "UD",
      sortMethod: "directed",
    },
  },
  nodes: {
    color: {
      highlight: {
        border: "#6FA6F9"
      },
      hover: {
        border: "#228be6"
      }
    },
    borderWidth: 2,
    borderWidthSelected: 4,
    shadow: true,
    font: {
      color: "#222",
      size: 16,
      face: "sans-serif",
      bold: {
        color: "#222",
        size: 18,
        mod: "bold"
      }
    }
  }
};

const darkTheme = {
  layout: {
    hierarchical: {
      direction: "LR",
      sortMethod: "directed",
      levelSeparation: 150,
      nodeSpacing: 500,
    },
  },
  nodes: {
    shape: "box",
    shapeProperties: {
        borderRadius: 4,
    },
    margin: 12,
    color: {
        background: '#2a2a2e',
        border: '#444',
        highlight: {
            background: '#3c3c42',
            border: '#9cdcfe'
        },
        hover: {
            background: '#3c3c42',
            border: '#6FA6F9'
        }
    },
    font: {
        color: '#cccccc',
        face: 'Menlo, Monaco, "Courier New", monospace',
        size: 12,
        align: 'left',
        multi: 'markdown',
        bold: { color: '#9cdcfe' },
        ital: { color: '#a0a0a0', size: 11 },
    },
    widthConstraint: { minimum: 50 },
    heightConstraint: { minimum: 50, valign: 'top'}
  }
};

// Merge base options with themes
export const themes = {
  light: { ...baseOptions, ...lightTheme },
  dark: { ...baseOptions, ...darkTheme }
};

// Default export for existing components that don't specify a theme
export default themes.light;