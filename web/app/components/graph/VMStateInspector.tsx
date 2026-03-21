import { createMemo, createSignal, For, Show } from "solid-js";
import GlobalsDisplay from "../shared/GlobalsDisplay";
import InFlightMessagesDisplay from "../shared/InFlightMessagesDisplay";
import ProcessStateCard from "../shared/ProcessStateCard";
import type { VMState, ChannelState, VMStateSelection } from "../../types/vm-state";
import "./VMStateInspector.css";

interface VMStateInspectorProps {
    node: VMStateSelection | null;
    onClose: () => void;
}

export default function VMStateInspector(props: VMStateInspectorProps) {
    // All collapse state lives here so we can enforce "last open" guard
    const [globalsCollapsed, setGlobalsCollapsed] = createSignal(false);
    const [messagesCollapsed, setMessagesCollapsed] = createSignal(false);
    const [processesCollapsed, setProcessesCollapsed] = createSignal(false);

    const state = createMemo((): VMState | null => {
        return props.node?.vm ?? null;
    });

    const getChannelsForProcess = (pid: number): ChannelState[] => {
        const s = state();
        if (!s) return [];
        return s.channels.filter((ch: ChannelState) => ch.pid === pid);
    };

    // True only when there are messages to show
    const hasMessages = createMemo(() => {
        const s = state();
        return !!s && (s.pending_deliveries.length > 0 || s.waiting_send.length > 0);
    });

    // Count how many sections are currently visible AND expanded
    const expandedCount = createMemo(() => {
        let count = 0;
        if (!globalsCollapsed()) count++;
        if (hasMessages() && !messagesCollapsed()) count++;
        if (!processesCollapsed()) count++;
        return count;
    });

    // A section can collapse only if it is already collapsed (i.e. re-expanding)
    // OR if there is more than one section open right now.
    const canCollapse = () => expandedCount() > 1;

    const toggleGlobals = () => {
        if (!globalsCollapsed() && !canCollapse()) return;
        setGlobalsCollapsed(c => !c);
    };

    const toggleMessages = () => {
        if (!messagesCollapsed() && !canCollapse()) return;
        setMessagesCollapsed(c => !c);
    };

    const toggleProcesses = () => {
        if (!processesCollapsed() && !canCollapse()) return;
        setProcessesCollapsed(c => !c);
    };

    return (
        <div class="vm-state-inspector">
            <div class="inspector-header">
                <div class="header-title">
                    <i class="codicon codicon-debug-console"></i>
                    <span>State Details</span>
                </div>
                <button class="close-btn" onClick={props.onClose} title="Close Inspector">
                    <i class="codicon codicon-close"></i>
                </button>
            </div>

            <Show when={state()} fallback={
                <div class="inspector-empty-state">
                    Select a node in the graph to inspect its state
                </div>
            }>
                {(vmState) => (
                    <div class="inspector-sections">
                        <GlobalsDisplay
                            globals={vmState().globals}
                            collapsed={globalsCollapsed()}
                            canCollapse={canCollapse()}
                            onToggle={toggleGlobals}
                        />
                        <InFlightMessagesDisplay
                            pendingDeliveries={vmState().pending_deliveries}
                            waitingSend={vmState().waiting_send}
                            collapsed={messagesCollapsed()}
                            canCollapse={canCollapse()}
                            onToggle={toggleMessages}
                        />

                        <div class={`section programs-section${processesCollapsed() ? " section-collapsed" : ""}`}>
                            <div
                                class={`collapsible-header${!processesCollapsed() && !canCollapse() ? " header-locked" : ""}`}
                                onClick={toggleProcesses}
                                title={
                                    !processesCollapsed() && !canCollapse()
                                        ? "Cannot collapse — only section open"
                                        : processesCollapsed()
                                            ? "Expand Processes"
                                            : "Collapse Processes"
                                }
                            >
                                <i class={`codicon ${processesCollapsed() ? "codicon-chevron-left" : "codicon-chevron-right"} header-toggle-icon`}></i>
                                <i class="codicon codicon-debug-all header-section-icon"></i>
                                <span class="section-label">Processes</span>
                                <span class="header-count">{vmState().locals.length}</span>
                            </div>
                            <div class={`section-body${processesCollapsed() ? " section-body-hidden" : ""}`}>
                                <Show
                                    when={vmState().locals.length > 0}
                                    fallback={<div class="empty-state">No processes</div>}
                                >
                                    <For each={vmState().locals}>
                                        {(prog) => (
                                            <ProcessStateCard
                                                process={prog}
                                                channels={getChannelsForProcess(prog.pid)}
                                            />
                                        )}
                                    </For>
                                </Show>
                            </div>
                        </div>
                    </div>
                )}
            </Show>
        </div>
    );
}
