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

export const rendervmStates = (vm_states, editor?: any) => {
    console.log(vm_states);
    let container: HTMLDivElement | undefined;
    let network: vis.Network | null = null;
    const [maximized, setMaximized] = createSignal(false);
    const [showDetails, setDetails] = createSignal(false);
    const [selectedEdge, setSelectedEdge] = createSignal<string | null>(null);
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
                const edgeId = `edge_${i}_to_${i+1}`;
                const label = `Step ${i + 1}`;
                // Check if vm has transition info (instruction, lines, etc.)
                const lines = vm.last_instruction?.lines || vm.lines;
                
                edges.push({
                    id: edgeId,
                    from: i,
                    to: i+1,
                    label: label,
                    lines: lines, // Store line numbers if available
                    font: { 
                        size: 0, // Hide labels by default
                        color: '#cccccc',
                        background: 'rgba(30, 30, 30, 0.8)',
                        strokeWidth: 2,
                        strokeColor: '#000'
                    }
                })
            }
        })
        data = { nodes, edges };
        console.log("Network created with nodes:", data.nodes, "and edges:", data.edges);
        const options = themes.dark;
        network = new vis.Network(container, data, options);
        setupNodeClickZoom(network);
        
        // Setup edge click handler
        network.on('selectEdge', (params) => {
            if (params.edges.length > 0) {
                const edgeId = params.edges[0];
                const edgeIndex = edges.findIndex((e: any) => e.id === edgeId);
                
                if (edgeIndex !== -1) {
                    const edge = edges[edgeIndex];
                    // Update edge to show label with proper styling
                    edges[edgeIndex] = {
                        ...edge,
                        font: { 
                            size: 12,
                            color: '#cccccc',
                            background: 'rgba(30, 30, 30, 0.8)',
                            strokeWidth: 2,
                            strokeColor: '#000'
                        }
                    };
                    // Update the network
                    if (network) {
                        network.setData({ nodes, edges });
                    }
                    setSelectedEdge(edgeId);
                    
                    // Highlight lines in editor if available
                    if (edge.lines && editor && editor.highlightLines) {
                        editor.highlightLines(edge.lines);
                    }
                }
            }
        });
        
        // Hide label when edge is deselected
        network.on('deselectEdge', (params) => {
            if (params.previousSelection.edges.length > 0) {
                const edgeId = params.previousSelection.edges[0];
                const edgeIndex = edges.findIndex((e: any) => e.id === edgeId);
                
                if (edgeIndex !== -1) {
                    const edge = edges[edgeIndex];
                    edges[edgeIndex] = {
                        ...edge,
                        font: { size: 0 }
                    };
                    // Update the network
                    if (network) {
                        network.setData({ nodes, edges });
                    }
                    setSelectedEdge(null);
                    
                    // Clear highlights in editor
                    if (editor && editor.clearHighlights) {
                        editor.clearHighlights();
                    }
                }
            }
        });
        
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
