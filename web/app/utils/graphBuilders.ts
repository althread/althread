/**
 * Graph Builder Utilities
 * 
 * Unified graph building for both run and check modes using GraphNode structure.
 */

import type { GraphNode, VisGraphNode, VisGraphEdge, GraphBuildResult, VMState } from '../types/vm-state';
import { nodeToString } from '@components/graph/Node';

// Helper to create a compatible object for nodeToString
function vmStateToNode(vm: VMState): any {
    // nodeToString expects the old Map-based format, but we can pass our typed VMState
    // The function will work with the new structure
    return vm;
}

interface GraphBuildOptions {
    mode?: 'run' | 'check';
    stepLines?: number[][];
    violationPath?: string[];
}

/**
 * Build vis.js graph from GraphNode array (works for both run and check modes)
 */
export function buildGraphFromNodes(nodes: GraphNode[], options: GraphBuildOptions = {}): GraphBuildResult {
    const { mode = 'check', stepLines = [], violationPath = [] } = options;
    const visNodes: VisGraphNode[] = [];
    const visEdges: VisGraphEdge[] = [];

    for (let i = 0; i < nodes.length; i++) {
        const node = nodes[i];
        const metadata = node.metadata;

        // Create a summary label for the node
        const processCount = node.vm.locals.length;
        const label = mode === 'run' 
            ? `${i}` // Run mode: show sequential index
            : `${i}`; // Check mode: show node index

        // Create detailed state for tooltip
        const globalCount = Object.keys(node.vm.globals).length;
        const channelCount = node.vm.channels.length;
        const pendingCount = node.vm.pending_deliveries.length;
        
        const title = `Level: ${metadata.level}\nProcesses: ${processCount}\nGlobals: ${globalCount}\nChannels: ${channelCount}\nPending: ${pendingCount}`;

        // Generate full state string for display
        const fullLabel = nodeToString(vmStateToNode(node.vm));

        // Determine node color based on mode and violation path
        const nodeLabel = nodeToString(vmStateToNode(node.vm));
        const isViolationNode = violationPath.includes(nodeLabel) || (violationPath.length > 0 && metadata.level === 0);
        
        let backgroundColor, borderColor;
        if (mode === 'run') {
            backgroundColor = "#314d31";
            borderColor = "#a6dfa6";
        } else {
            backgroundColor = isViolationNode ? "#4d3131" : "#314d31";
            borderColor = isViolationNode ? "#ec9999" : "#a6dfa6";
        }

        visNodes.push({
            id: i,
            label,
            level: metadata.level,
            color: { 
                background: backgroundColor, 
                border: borderColor,
                highlight: {
                    border: "hsla(29.329, 66.552%, 52.544%)",
                    background: backgroundColor
                },
                hover: {
                    border: "hsla(29.329, 66.552%, 52.544%)",
                    background: backgroundColor
                }
            },
            font: { size: 10, color: '#ffffff' } as any,
            borderWidth: 1,
            title,
            fullLabel,
            // Unified format for both run and check modes
            rawState: { 
                vm: node.vm, 
                stepIndex: mode === 'run' ? i : undefined,
                level: metadata.level 
            },
            isViolationNode,
        } as any);

        // Build edges
        if (mode === 'run') {
            // Run mode: sequential edges with step lines
            if (i < nodes.length - 1) {
                const lines = stepLines[i] || [];
                visEdges.push({
                    id: i,
                    from: i,
                    to: i + 1,
                    label: `step ${i + 1}`,
                    lines,
                    font: { size: 0 }, // Hidden by default
                });
            }
        } else {
            // Check mode: explicit successors with indices
            if (metadata.successors) {
                for (const succ of metadata.successors) {
                    const edgeLabel = succ.name + '#' + succ.pid + ': ' + succ.lines.join(',');
                    
                    visEdges.push({
                        id: `${i}-${succ.to_index}`,
                        from: i,
                        to: succ.to_index,
                        label: edgeLabel,
                        lines: succ.lines.map((l: number) => Number(l)),
                        font: { size: 0 }, // Hidden by default
                    });
                }
            }
        }
    }

    return { nodes: visNodes, edges: visEdges };
}

/**
 * Find node index by VM state (for matching states across graphs)
 */
export function findNodeByState(nodes: GraphNode[], targetState: any): number {
    // This is a simple implementation - could be improved with better state comparison
    for (let i = 0; i < nodes.length; i++) {
        if (JSON.stringify(nodes[i].vm) === JSON.stringify(targetState)) {
            return i;
        }
    }
    return -1;
}

/**
 * Extract path from graph nodes (for error traces in check mode)
 */
export function extractPath(nodes: GraphNode[], startIndex: number, endIndex: number): GraphNode[] {
    // For now, return a simple slice - could implement BFS for complex graphs
    if (startIndex <= endIndex) {
        return nodes.slice(startIndex, endIndex + 1);
    }
    return [];
}
