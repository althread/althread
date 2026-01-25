import { createMemo, For, Show } from "solid-js";
import "./GlobalsDisplay.css";
import { LiteralDisplay } from "./Literal";
import type { Literal } from "../../types/vm-state";

interface GlobalsDisplayProps {
    globals: Record<string, Literal>;
}

export default function GlobalsDisplay(props: GlobalsDisplayProps) {
    const globalsEntries = createMemo(() => {
        return Object.entries(props.globals).sort((a, b) => a[0].localeCompare(b[0]));
    });
    
    return (
        <div class="globals-section">
            <div class="section-header">Globals</div>
            <div class="section-body">
                <Show
                    when={globalsEntries().length > 0}
                    fallback={<div class="empty-state">No globals</div>}
                >
                    <div class="variables-grid">
                        <For each={globalsEntries()}>
                            {([key, value]) => (
                                <div class="variable-item">
                                    <span class="var-name">{key}</span>{ " = " }
                                    <span class="var-value"><LiteralDisplay value={value} /></span>
                                </div>
                            )}
                        </For>
                    </div>
                </Show>
            </div>
        </div>
    );
}
