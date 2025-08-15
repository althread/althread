/** @jsxImportSource solid-js */
import vis from "vis-network/dist/vis-network.esm";
import { createEffect, onCleanup, createSignal } from "solid-js";
import GraphToolbar from "./GraphToolbar";
import { themes } from "./visOptions";
import { setupNodeClickZoom, createGraphToolbarHandlers } from "./visHelpers";
import { useGraphMaximizeHotkeys } from "@hooks/useGraphMaximizeHotkeys";
import MetadataDisplay from "./MetadataDisplay";
import { exportStatesToCSV } from "./exportToCSV";

export default (props /*: GraphProps & { theme?: 'light' | 'dark' } */) => {
    let container: HTMLDivElement | undefined;
    let network: vis.Network | null = null;
    const [maximized, setMaximized] = createSignal(false);
    const [showDetails, setDetails] = createSignal(false);

    createEffect(() => {
        if (!container) {
            console.error("Graph container element not found.");
            return;
        }

        let nodes = new vis.DataSet(props.nodes || []);
        let edges = new vis.DataSet(props.edges || []);

        const data = {
            nodes: nodes.get(),
            edges: edges.get()
        };

        const options = props.theme === 'dark' ? themes.dark : themes.light;
        network = new vis.Network(container, data, options);
        setupNodeClickZoom(network);

        network.once('stabilized', function() {
          if (network) {
            network.fit();
            props.setLoadingAction(null);
          }
        });

        onCleanup(() => {
            if (network) {
                network.destroy();
                network = null;
            }
        });
    });

    useGraphMaximizeHotkeys(setMaximized);

    const { handleMaximize, handleRecenter, handleDownload, handleDetails } = createGraphToolbarHandlers(
        () => network,
        () => container,
        () => setMaximized((v: boolean) => !v),
        () => setDetails((v: boolean) => !v)
    );

    return (
      <div
        class={`state-graph${maximized() ? " maximized" : ""}`}
      >
        <div
          ref={container}
          style="width: 100%; height: 100%;"
        />

        {showDetails() ? <MetadataDisplay nodes={props.nodes} /> : null}
        <GraphToolbar
          onFullscreen={handleMaximize}
          onRecenter={handleRecenter}
          onDownload={handleDownload}
          onDownloadCSV={() => exportStatesToCSV(props.nodes, props.edges)}
          onDetails={handleDetails}
          isFullscreen={maximized()}
        />
      </div>
    );
};