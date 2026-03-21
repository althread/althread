import { createMemo, For, Show } from "solid-js";
import "./GlobalsDisplay.css";
import "./InFlightMessagesDisplay.css";
import { LiteralDisplay } from "./Literal";
import type { Literal } from "../../types/vm-state";

interface GlobalsDisplayProps {
    globals: Record<string, Literal>;
    collapsed: boolean;
    canCollapse: boolean;
    onToggle: () => void;
}

export default function GlobalsDisplay(props: GlobalsDisplayProps) {
    const globalsEntries = createMemo(() => {
        return Object.entries(props.globals).sort((a, b) => a[0].localeCompare(b[0]));
    });

    const isLocked = () => !props.collapsed && !props.canCollapse;

    return (
        <div class={`section${props.collapsed ? " section-collapsed" : ""}`}>
            <div
                class={`collapsible-header${isLocked() ? " header-locked" : ""}`}
                onClick={props.onToggle}
                title={
                    isLocked()
                        ? "Cannot collapse — only section open"
                        : props.collapsed
                            ? "Expand Globals"
                            : "Collapse Globals"
                }
            >
                <i class={`codicon ${props.collapsed ? "codicon-chevron-left" : "codicon-chevron-right"} header-toggle-icon`}></i>
                <i class="codicon codicon-symbol-variable header-section-icon"></i>
                <span class="section-label">Globals</span>
                <span class="header-count">{globalsEntries().length}</span>
            </div>
            <div class={`section-body${props.collapsed ? " section-body-hidden" : ""}`}>
                <Show
                    when={globalsEntries().length > 0}
                    fallback={<div class="empty-state">No globals</div>}
                >
                    <div class="variables-grid">
                        <For each={globalsEntries()}>
                            {([key, value]) => (
                                <div class="variable-item">
                                    <span class="var-name">{key}</span>{" = "}
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
