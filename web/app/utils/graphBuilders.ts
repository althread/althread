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
    violationPathStates?: VMState[];
}

function stableStringify(value: unknown): string {
    if (value === null || value === undefined) return JSON.stringify(value);

    if (Array.isArray(value)) {
        return `[${value.map((item) => stableStringify(item)).join(',')}]`;
    }

    if (typeof value === 'object') {
        const entries = Object.entries(value as Record<string, unknown>)
            .sort(([a], [b]) => a.localeCompare(b))
            .map(([key, val]) => `${JSON.stringify(key)}:${stableStringify(val)}`);
        return `{${entries.join(',')}}`;
    }

    return JSON.stringify(value);
}

export function vmStateSignature(vm: VMState): string {
    const normalized = {
        globals: Object.entries(vm.globals || {})
            .sort(([a], [b]) => a.localeCompare(b))
            .map(([key, value]) => ({ key, value })),
        channels: [...(vm.channels || [])]
            .map((channel) => ({ ...channel }))
            .sort((a, b) => (a.pid - b.pid) || a.name.localeCompare(b.name)),
        pending_deliveries: [...(vm.pending_deliveries || [])]
            .map((delivery) => ({ ...delivery }))
            .sort(
                (a, b) =>
                    (a.from_pid - b.from_pid)
                    || a.from_channel.localeCompare(b.from_channel)
                    || (a.to_pid - b.to_pid)
                    || a.to_channel.localeCompare(b.to_channel)
            ),
        waiting_send: [...(vm.waiting_send || [])]
            .map((waiting) => ({ ...waiting }))
            .sort((a, b) => (a.pid - b.pid) || a.name.localeCompare(b.name)),
        channel_connections: [...(vm.channel_connections || [])]
            .map((connection) => ({ ...connection }))
            .sort(
                (a, b) =>
                    (a.from.pid - b.from.pid)
                    || a.from.channel.localeCompare(b.from.channel)
                    || (a.to.pid - b.to.pid)
                    || a.to.channel.localeCompare(b.to.channel)
            ),
        locals: [...(vm.locals || [])]
            .map((local) => ({ ...local }))
            .sort((a, b) => (a.pid - b.pid) || a.name.localeCompare(b.name)),
    };

    return stableStringify(normalized);
}

/**
 * Build vis.js graph from GraphNode array (works for both run and check modes)
 */
export function buildGraphFromNodes(nodes: GraphNode[], options: GraphBuildOptions = {}): GraphBuildResult {
    const { mode = 'check', stepLines = [], violationPath = [], violationPathStates = [] } = options;
    const visNodes: VisGraphNode[] = [];
    const visEdges: VisGraphEdge[] = [];
    const hasViolationPathStates = violationPathStates.length > 0;
    const needsViolationSignatures = mode === 'check' && hasViolationPathStates;
    const nodeSignatures = needsViolationSignatures
        ? nodes.map((node) => vmStateSignature(node.vm))
        : [];
    const violationPathSignatures = needsViolationSignatures
        ? violationPathStates.map((state) => vmStateSignature(state))
        : [];
    const violationNodeSet = new Set(violationPathSignatures);
    const violationEdgeSet = new Set<string>();
    const fallbackViolationLabelSet = new Set(violationPath);
    const needsFallbackLabels = mode === 'check' && !needsViolationSignatures && fallbackViolationLabelSet.size > 0;

    for (let i = 0; i < violationPathSignatures.length - 1; i++) {
        const from = violationPathSignatures[i];
        const to = violationPathSignatures[i + 1];
        violationEdgeSet.add(`${from}->${to}`);
    }

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

        // Determine node color based on mode and violation path
        const nodeSignature = needsViolationSignatures ? nodeSignatures[i] : undefined;
        const nodeLabel = needsFallbackLabels ? nodeToString(vmStateToNode(node.vm)) : undefined;
        const isViolationNode = mode === 'check' && (
            needsViolationSignatures
                ? (typeof nodeSignature === 'string' && violationNodeSet.has(nodeSignature))
                : fallbackViolationLabelSet.has(nodeLabel)
        );
        
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
                    id: `e${i}`,
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
                let succIndex = 0;
                for (const succ of metadata.successors) {
                    const edgeLabel = succ.name + '#' + succ.pid + ': ' + succ.lines.join(',');
                    const toSignature = needsViolationSignatures ? nodeSignatures[succ.to_index] : undefined;
                    const isViolationEdge = needsViolationSignatures
                        && typeof toSignature === 'string'
                        && typeof nodeSignature === 'string'
                        && violationEdgeSet.has(`${nodeSignature}->${toSignature}`);
                    
                    visEdges.push({
                        id: `e${i}-${succ.to_index}-${succIndex++}`,
                        from: i,
                        to: succ.to_index,
                        label: edgeLabel,
                        lines: succ.lines.map((l: number) => Number(l)),
                        font: { size: 0 }, // Hidden by default
                        color: isViolationEdge
                            ? { color: '#ec9999', highlight: '#ec9999', hover: '#ec9999' }
                            : undefined,
                        width: isViolationEdge ? 2 : undefined,
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
