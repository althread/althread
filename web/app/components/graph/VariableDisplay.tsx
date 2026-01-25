import { For, Show } from "solid-js";
import { literal } from "@utils/vmStateUtils";
import type { VariableInfo } from "../../types/vm-state";
import "./VariableDisplay.css";

interface VariableDisplayProps {
    variables: Record<string, VariableInfo>;
    fallbackMemory?: string[];
}

export default function VariableDisplay(props: VariableDisplayProps) {
    const hasVariables = () => Object.keys(props.variables || {}).length > 0;
    
    return (
        <div class="variable-display">
            <Show when={hasVariables()} fallback={
                <Show when={props.fallbackMemory && props.fallbackMemory.length > 0}>
                    <div class="fallback-memory">
                        <div class="memory-label">Raw Memory:</div>
                        <div class="stack-container">
                            <For each={props.fallbackMemory}>
                                {(val, idx) => (
                                    <div class="memory-item">
                                        <span class="memory-index">[{idx()}]</span>
                                        <span class="memory-value">{literal(val)}</span>
                                    </div>
                                )}
                            </For>
                        </div>
                    </div>
                </Show>
            }>
                <div class="variables-list">
                    <For each={Object.entries(props.variables)}>
                        {([name, variable]) => (
                            <div class="variable-row">
                                <span class="var-name" title={variable.type}>{name}</span>
                                <span class="var-separator">:</span>
                                <span class="var-type">{variable.type}</span>
                                <span class="var-equals">=</span>
                                <span class="var-value">{variable.value}</span>
                            </div>
                        )}
                    </For>
                </div>
            </Show>
        </div>
    );
}
