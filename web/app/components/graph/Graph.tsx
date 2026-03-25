import vis from "vis-network/dist/vis-network.esm";
import { createEffect, onCleanup, createSignal } from "solid-js";
import GraphToolbar from "./GraphToolbar";
import { nodeToString } from "./Node";
import { themes } from "./visOptions";
import { setupNodeClickZoom, createGraphToolbarHandlers, formatState } from "./visHelpers";
import { useGraphMaximizeHotkeys } from "@hooks/useGraphMaximizeHotkeys";
import MetadataDisplay from "./MetadataDisplay";
import { exportStatesToCSV } from "./exportToCSV";

interface GraphProps {
    nodes: any[];
    edges: any[];
    vm_states?: any[];
    setLoadingAction: (action: string | null) => void;
    theme?: 'light' | 'dark';
    onEdgeClick?: (edgeId: string, edgeData: any) => void;
    onNodeSelect?: (node: any | null) => void;
    tooLargeStatusMessage?: string;
    ref?: (instance: { selectNode: (nodeId: string | number) => void }) => void;
}

export const MAX_VISIBLE_GRAPH_NODES = 200;

export default (props: GraphProps) => {
    let container: HTMLDivElement | undefined;
    let network: vis.Network | null = null;
    const [maximized, setMaximized] = createSignal(false);
    const [showDetails, setDetails] = createSignal(false);
    const [selectedNodeContent, setSelectedNodeContent] = createSignal<string | null>(null);
    const isGraphTooLarge = () => (props.nodes?.length || 0) > MAX_VISIBLE_GRAPH_NODES;
    const resolveVMState = (rawState: any) => {
        if (!rawState) return null;
        return rawState.vm ?? rawState.state ?? rawState;
    };
    const resolveNodeState = (node: any, originalNode: any) => {
        return (
            node?.rawState ??
            originalNode?.rawState ??
            node?.state ??
            node?.vm_state ??
            originalNode?.state ??
            originalNode?.vm_state ??
            null
        );
    };
    const isDarkTheme = () => props.theme !== 'light';
    const graphOverlayStyles = () => ({
        background: isDarkTheme() ? '#1e1e1e' : '#fffaf2',
        color: isDarkTheme() ? '#ffffff' : '#332c26',
        border: isDarkTheme() ? '#454545' : '#d8c9b3',
        shadow: isDarkTheme() ? 'rgba(0,0,0,0.3)' : 'rgba(121, 89, 46, 0.18)',
        buttonColor: isDarkTheme() ? '#cccccc' : '#6f5b46',
    });

    createEffect(() => {
        if (isGraphTooLarge()) {
            setSelectedNodeContent(null);
            if (props.onNodeSelect) {
                props.onNodeSelect(null);
            }
            props.setLoadingAction(null);
            return;
        }

        if (!container) {
            return;
        }

        let nodes = new vis.DataSet(props.nodes || []);
        let edges = new vis.DataSet(props.edges || []);

        const data = {
            nodes: nodes.get(),
            edges: edges.get()
        };

        const isLargeGraph = (props.nodes?.length || 0) > 1000 || (props.edges?.length || 0) > 3000;
        const baseOptions: any = props.theme === 'dark' ? themes.dark : themes.light;
        const options: any = isLargeGraph
            ? {
                ...baseOptions,
                physics: { ...(baseOptions.physics || {}), enabled: false },
                layout: { ...(baseOptions.layout || {}) },
                interaction: { ...(baseOptions.interaction || {}) },
            }
            : baseOptions;
        network = new vis.Network(container, data, options);
        
        // Setup node click handler
        setupNodeClickZoom(network, (nodeId) => {
            if (nodeId === null) {
                setSelectedNodeContent(null);
                if (props.onNodeSelect) props.onNodeSelect(null);
                return;
            }
            const node = nodes.get(nodeId);
            const originalNode = (props.nodes || []).find((n: any) => String(n.id) === String(nodeId));
            if (!node && !originalNode) {
                setSelectedNodeContent(null);
                if (props.onNodeSelect) props.onNodeSelect(null);
                return;
            }

            const fullLabel = node?.fullLabel ?? originalNode?.fullLabel;
            const rawState = resolveNodeState(node, originalNode);
            const vmState = resolveVMState(rawState);

            if (fullLabel) {
                setSelectedNodeContent(fullLabel);
            } else if (vmState) {
                setSelectedNodeContent(nodeToString(vmState));
            }
            if (props.onNodeSelect) {
                props.onNodeSelect(rawState || null);
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
                            color: isDarkTheme() ? '#cccccc' : '#544639',
                            background: isDarkTheme() ? 'rgba(30, 30, 30, 0.8)' : 'rgba(255, 248, 239, 0.92)',
                            strokeWidth: 2,
                            strokeColor: isDarkTheme() ? '#000' : '#e7d8c3'
                        } 
                    });
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
            }
        });

                if (isLargeGraph) {
                        if (network) {
                                network.fit({ animation: false });
                        }
                        props.setLoadingAction(null);
                } else {
                        network.once('stabilized', function() {
                            if (network) {
                                network.fit();
                                props.setLoadingAction(null);
                            }
                        });
                }

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

    // Expose methods via ref
    if (props.ref) {
        props.ref({
            selectNode: (nodeId: string | number) => {
                if (network) {
                    network.selectNodes([nodeId]);
                    network.focus(nodeId, { animation: true, scale: 1.5 });
                }
            }
        });
    }

    return (
      <div
        class={`state-graph${maximized() ? " maximized" : ""}`}
      >
                {isGraphTooLarge() ? (
                    <div style={`display: flex; height: 100%; width: 100%; align-items: center; justify-content: center; padding: 24px; color: ${isDarkTheme() ? '#cccccc' : '#5c4d3d'}; text-align: center;`}>
                        <div>
                                        <div style="font-weight: 600; margin-bottom: 8px;">Graph not displayed</div>
                            {props.tooLargeStatusMessage ? (
                                <div style="margin-bottom: 8px; opacity: 0.9;">
                                    {props.tooLargeStatusMessage}
                                </div>
                            ) : null}
                                        <div>
                                                This graph has {props.nodes.length} nodes, which is above the display limit of {MAX_VISIBLE_GRAPH_NODES}.
                                        </div>
                                </div>
                        </div>
                ) : (
                        <div
                            ref={container}
                            style="width: 100%; height: 100%;"
                        />
                )}

        {!props.onNodeSelect && selectedNodeContent() && (
            <div class="node-details-overlay" style={`position: absolute; top: 10px; right: 10px; background: ${graphOverlayStyles().background}; color: ${graphOverlayStyles().color}; padding: 10px; border: 1px solid ${graphOverlayStyles().border}; border-radius: 5px; max-width: 400px; max-height: 80%; overflow: auto; white-space: pre-wrap; z-index: 1000; box-shadow: 0 4px 6px ${graphOverlayStyles().shadow};`}>
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                    <h3 style="margin: 0;">Node State</h3>
                    <button style={`background: none; border: none; color: ${graphOverlayStyles().buttonColor}; cursor: pointer;`} onClick={() => setSelectedNodeContent(null)}>
                        <i class="codicon codicon-close"></i>
                    </button>
                </div>
                <div style="font-family: monospace;" innerHTML={formatState(selectedNodeContent()!)}></div>
            </div>
        )}

                {!isGraphTooLarge() && showDetails() ? <MetadataDisplay nodes={props.nodes} /> : null}
                {!isGraphTooLarge() ? (
                        <GraphToolbar
                            onFullscreen={handleMaximize}
                            onRecenter={handleRecenter}
                            onDownload={handleDownload}
                            onDownloadCSV={() => exportStatesToCSV(props.nodes, props.edges)}
                            onDetails={handleDetails}
                            isFullscreen={maximized()}
                        />
                ) : null}
      </div>
    );
};