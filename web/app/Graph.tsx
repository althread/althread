/** @jsxImportSource solid-js */
import vis from "vis-network/dist/vis-network.esm";
import { createEffect, onCleanup, onMount, createSignal } from "solid-js";
import GraphToolbar from "./GraphToolbar";
import visOptions from "./visOptions";
import { setupNodeClickZoom, createGraphToolbarHandlers } from "./visHelpers";

export default (props /*: GraphProps*/) => {
    let container: HTMLDivElement | undefined; // Renamed for clarity
    let network: vis.Network | null = null;
    const [maximized, setMaximized] = createSignal(false);

    const nodes = new vis.DataSet(props.nodes || []);
    const edges = new vis.DataSet(props.edges || []);

    createEffect(() => {
        nodes.clear();
        nodes.add(props.nodes || []);
        edges.clear();
        edges.add(props.edges || []);
    });

    onMount(() => {
        if (!container) {
            console.error("Graph container element not found.");
            return;
        }

        const data = {
            nodes: nodes.get(),
            edges: edges.get()
        };

        network = new vis.Network(container, data, visOptions);
        setupNodeClickZoom(network);

        network.once('stabilized', function() {
          if (network) network.fit();
        });

        onCleanup(() => {
            if (network) {
                network.destroy();
                network = null;
            }
        });
    });

    const { handleMaximize, handleRecenter, handleDownload } = createGraphToolbarHandlers(
        () => network,
        () => container,
        () => setMaximized((v: boolean) => !v)
    );

    return (
      <div
        class={`state-graph${maximized() ? " maximized" : ""}`}
      >
        <div
          ref={container}
          style="width: 100%; height: 100%;"
        />
        <GraphToolbar
          onFullscreen={handleMaximize}
          onRecenter={handleRecenter}
          onDownload={handleDownload}
          isFullscreen={maximized()}
        />
      </div>
    );
};