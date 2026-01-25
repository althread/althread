import { createMemo, For, Show } from "solid-js";
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
    // Extract VM state - now it's properly typed!
    const state = createMemo((): VMState | null => {
        return props.node?.vm ?? null;
    });

    // Helper to get channels for a specific process
    const getChannelsForProcess = (pid: number): ChannelState[] => {
        const s = state();
        if (!s) return [];
        return s.channels.filter((ch: ChannelState) => ch.pid === pid);
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
                        <div class="inspector-left-column">
                            <GlobalsDisplay globals={vmState().globals} />
                            <InFlightMessagesDisplay 
                                pendingDeliveries={vmState().pending_deliveries}
                                waitingSend={vmState().waiting_send}
                            />
                        </div>

                        <div class="section programs-section">
                            <div class="section-header">Processes</div>
                            <div class="section-body">
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
