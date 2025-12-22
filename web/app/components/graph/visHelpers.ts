import type { Network } from "vis-network";

export function setupNodeClickZoom(network: Network, onNodeClick?: (nodeId: string | number) => void) {
  network.on("click", function (params) {
    if (params.nodes && params.nodes.length > 0) {
      const nodeId = params.nodes[0];
      if (onNodeClick) {
        onNodeClick(nodeId);
      }
      if (network) {
        network.selectNodes([nodeId]);
        network.focus(nodeId, {
          scale: 0.75,
          animation: {
            duration: 250,
            easingFunction: "easeInOutQuad"
          }
        });
      }
    } else {
      // Deselect if clicking on empty space
      if (onNodeClick) {
        onNodeClick(null as any);
      }
    }
  });
}

export function createGraphToolbarHandlers(
    getNetwork: () => Network | null,
    getContainer: () => HTMLDivElement | undefined,
    toggleMaximized: () => void,
    toggleDetails: () => void
) {
    const handleMaximize = () => {
        toggleMaximized();
    };

    const handleDetails = () => {
        toggleDetails();
    }

    const handleRecenter = () => {
        const network = getNetwork();
        if (network) {
            network.fit();
        }
    };

    const handleDownload = () => {
      const network = getNetwork();
      const container = getContainer();
      if (!network || !container) return;

      const canvas = container.querySelector("canvas");
      if (canvas instanceof HTMLCanvasElement) {
        const dataURL = canvas.toDataURL("image/png");
        const link = document.createElement("a");
        link.href = dataURL;
        link.download = "graph.png";
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
      }
    };

    return {
        handleMaximize,
        handleRecenter,
        handleDownload,
        handleDetails
    };
}

export function formatState(text: string): string {
    // Escape HTML
    let html = text
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");

    // Bold: *text*
    html = html.replace(/\*([^*]+)\*/g, "<b>$1</b>");

    // Italics: _text_
    html = html.replace(/_([^_]+)_/g, "<i>$1</i>");

    return html;
}
