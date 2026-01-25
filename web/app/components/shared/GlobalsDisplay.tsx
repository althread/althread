import { For, Show } from "solid-js";
import { literal } from "@utils/vmStateUtils";
import "./GlobalsDisplay.css";

interface GlobalsDisplayProps {
    globals: Record<string, string>;
}

export default function GlobalsDisplay(props: GlobalsDisplayProps) {
    const globalsEntries = () => Object.entries(props.globals);

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
                                    <span class="var-value">{literal(value)}</span>
                                </div>
                            )}
                        </For>
                    </div>
                </Show>
            </div>
        </div>
    );
}
