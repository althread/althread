/** @jsxImportSource solid-js */
import vis from "vis-network/dist/vis-network.esm";
import { createEffect, onCleanup, createSignal } from "solid-js";
import GraphToolbar from "./GraphToolbar";
import { themes } from "./visOptions";
import { setupNodeClickZoom, createGraphToolbarHandlers, formatState } from "./visHelpers";
import { useGraphMaximizeHotkeys } from "@hooks/useGraphMaximizeHotkeys";
import MetadataDisplay from "./MetadataDisplay";
import { exportStatesToCSV } from "./exportToCSV";

export default (props /*: GraphProps & { theme?: 'light' | 'dark' } */) => {
    let container: HTMLDivElement | undefined;
    let network: vis.Network | null = null;
    const [maximized, setMaximized] = createSignal(false);
    const [showDetails, setDetails] = createSignal(false);
    const [selectedNodeContent, setSelectedNodeContent] = createSignal<string | null>(null);
    const [selectedEdge, setSelectedEdge] = createSignal<string | null>(null);

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
        
        // Setup node click handler
        setupNodeClickZoom(network, (nodeId) => {
            if (nodeId === null) {
                setSelectedNodeContent(null);
                return;
            }
            const node = nodes.get(nodeId);
            if (node && node.fullLabel) {
                setSelectedNodeContent(node.fullLabel);
            } else {
                setSelectedNodeContent(null);
            }
        });
        
        // Setup edge click handler
        network.on('selectEdge', (params) => {
            if (params.edges.length > 0) {
                const edgeId = params.edges[0];
                const edge = edges.get(edgeId);
                
                if (edge) {
                    // Update edge to show label with proper styling
                    edges.update({ 
                        id: edgeId, 
                        font: { 
                            size: 12,
                            color: '#cccccc',
                            background: 'rgba(30, 30, 30, 0.8)',
                            strokeWidth: 2,
                            strokeColor: '#000'
                        } 
                    });
                    setSelectedEdge(edgeId);
                    
                    // Call the onEdgeClick callback if provided
                    if (props.onEdgeClick) {
                        props.onEdgeClick(edgeId, edge);
                    }
                }
            }
        });
        
        // Hide label when edge is deselected
        network.on('deselectEdge', (params) => {
            if (params.previousSelection.edges.length > 0) {
                const edgeId = params.previousSelection.edges[0];
                edges.update({ 
                    id: edgeId, 
                    font: { size: 0 } 
                });
                setSelectedEdge(null);
            }
        });

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

        {selectedNodeContent() && (
            <div class="node-details-overlay" style="position: absolute; top: 10px; right: 10px; background: #1e1e1e; color: #ffffff; padding: 10px; border: 1px solid #454545; border-radius: 5px; max-width: 400px; max-height: 80%; overflow: auto; white-space: pre-wrap; z-index: 1000; box-shadow: 0 4px 6px rgba(0,0,0,0.3);">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                    <h3 style="margin: 0;">Node State</h3>
                    <button style="background: none; border: none; color: #cccccc; cursor: pointer;" onClick={() => setSelectedNodeContent(null)}>
                        <i class="codicon codicon-close"></i>
                    </button>
                </div>
                <div style="font-family: monospace;" innerHTML={formatState(selectedNodeContent()!)}></div>
            </div>
        )}

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