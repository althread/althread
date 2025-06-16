import type { Network } from "vis-network";

export function setupNodeClickZoom(network: Network) {
  network.on("click", function (params) {
    if (params.nodes && params.nodes.length > 0) {
      const nodeId = params.nodes[0];
      if (network) {
        network.selectNodes([nodeId]);
        network.focus(nodeId, {
          scale: 1.5,
          animation: {
            duration: 250,
            easingFunction: "easeInOutQuad"
          }
        });
      }
    }
  });
}