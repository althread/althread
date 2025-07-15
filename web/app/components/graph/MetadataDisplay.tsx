/** @jsxImportSource solid-js */
import { VisNode } from "./Node";
import "./MetadataDisplay.css";

export default function MetadataDisplay( { nodes }: { nodes: VisNode[] } ) {
    const calculateMetadata = () => {
        if (!nodes || nodes.length === 0) {
            return null;
        }

        const statesStored = nodes.length;
        const depthReached = nodes.reduce((maxDepth, node) => Math.max(maxDepth, node.level ?? 0), 0);
        const violations = nodes.filter(node => node.isViolationNode).length;

        return {
            statesStored,
            depthReached,
            violations
        };
    };

    const metadata = calculateMetadata();
    if (!metadata) {
        return <div class="metadata-display"><h3>No details available.</h3></div>;
    }

    return (
        <div class="metadata-display">
            <h3>Details:</h3>
            <div><span>States explored:</span><pre> {JSON.stringify(metadata.statesStored, null, 2)}</pre></div>
            <div><span>Max depth reached:</span><pre> {JSON.stringify(metadata.depthReached, null, 2)}</pre></div>
            <div><span>Violations:</span><pre> {JSON.stringify(metadata.violations, null, 2)}</pre></div>
        </div>
    );
}