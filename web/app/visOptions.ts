const visOptions = {
  layout: {
    hierarchical: {
      direction: "UD",
      sortMethod: "directed",
    },
  },
  edges: {
    arrows: "to",
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

export default visOptions;