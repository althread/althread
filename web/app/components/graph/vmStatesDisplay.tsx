/** @jsxImportSource solid-js */
import vis from "vis-network/dist/vis-network.esm";
import { createSignal, onCleanup, onMount } from "solid-js";
import {nodeToString} from "./Node";
import GraphToolbar from "./GraphToolbar";
import { themes } from "./visOptions";
import { setupNodeClickZoom, createGraphToolbarHandlers } from "./visHelpers";
import { useGraphMaximizeHotkeys } from "@hooks/useGraphMaximizeHotkeys";
import MetadataDisplay from "./MetadataDisplay";
import { exportStatesToCSV } from "./exportToCSV";

export const rendervmStates = (vm_states) => {
    console.log(vm_states);
    let container: HTMLDivElement | undefined;
    let network: vis.Network | null = null;
    const [maximized, setMaximized] = createSignal(false);
    const [showDetails, setDetails] = createSignal(false);
    let data = {
        nodes: [],
        edges: []
    };

    if (!vm_states || vm_states.length === 0) {
        return <pre>The VM states will appear here.</pre>;
    }

    onMount(() => {
        if (!container) {
            console.error("Graph container element not found.");
            return;
        }


        const nodes: any = [];
        const edges: any = [];

        vm_states.forEach((vm, i) =>{
            //one node for each vm state
            let vm_node = {id: `${i}`, label: nodeToString(vm), shape: "box", font: { align: "left" }};
            nodes.push(vm_node);
        })
        vm_states.forEach((vm,i) =>{
            //arrow between parent node and its child
            if (i < vm_states.length - 1){
                edges.push({
                    from: i,
                    to: i+1,
                })
            }
        })
        data = { nodes, edges };
        console.log("Network created with nodes:", data.nodes, "and edges:", data.edges);
        const options = themes.dark;
        network = new vis.Network(container, data, options);
        setupNodeClickZoom(network);
        network.once('stabilized', function() {
            if (network) network.fit();
        });


        onCleanup(() => { if (network) network.destroy(); });
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
        {showDetails() ? <MetadataDisplay nodes={data.nodes} /> : null}
        <GraphToolbar
          onFullscreen={handleMaximize}
          onRecenter={handleRecenter}
          onDownload={handleDownload}
          onDownloadCSV={() => exportStatesToCSV(data.nodes, data.edges)}
          isFullscreen={maximized()}
          onDetails={handleDetails}
        />
      </div>
    );
}
