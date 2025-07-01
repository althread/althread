import type { Network } from "vis-network";

export function setupNodeClickZoom(network: Network) {
  network.on("click", function (params) {
    if (params.nodes && params.nodes.length > 0) {
      const nodeId = params.nodes[0];
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
    }
  });
}

export function createGraphToolbarHandlers(
    getNetwork: () => Network | null,
    getContainer: () => HTMLDivElement | undefined,
    toggleMaximized: () => void,
) {
    const handleMaximize = () => {
        toggleMaximized();
    };

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
        handleDownload
    };

};
