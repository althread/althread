import { For, Show } from "solid-js";
import { literal } from "@utils/vmStateUtils";
import type { PendingDelivery, WaitingSend } from "../../types/vm-state";
import "./InFlightMessagesDisplay.css";

interface InFlightMessagesDisplayProps {
    pendingDeliveries: PendingDelivery[];
    waitingSend: WaitingSend[];
}

export default function InFlightMessagesDisplay(props: InFlightMessagesDisplayProps) {
    return (
        <Show when={props.pendingDeliveries.length > 0 || props.waitingSend.length > 0}>
            <div class="inflight-section">
                <div class="section-header">In-flight Messages</div>
                <div class="section-body">
                    <Show when={props.pendingDeliveries.length > 0}>
                        <div class="subsection">
                            <div class="subsection-title">Pending Delivery</div>
                            <div class="message-list">
                                <For each={props.pendingDeliveries}>
                                    {(item) => (
                                        <div class="message-card">
                                            <div class="message-route">
                                                <span class="message-endpoint from">
                                                    {item.from_pid}:{item.from_channel}
                                                </span>
                                                <i class="codicon codicon-arrow-right"></i>
                                                <span class="message-endpoint to">
                                                    {item.to_pid}:{item.to_channel}
                                                </span>
                                            </div>
                                            <div class="message-payload">
                                                <For each={item.values}>
                                                    {(val) => (
                                                        <div class="payload-item">{literal(val)}</div>
                                                    )}
                                                </For>
                                            </div>
                                        </div>
                                    )}
                                </For>
                            </div>
                        </div>
                    </Show>
                    <Show when={props.waitingSend.length > 0}>
                        <div class="subsection">
                            <div class="subsection-title">Waiting (Unconnected)</div>
                            <div class="message-list">
                                <For each={props.waitingSend}>
                                    {(item) => (
                                        <div class="message-card waiting">
                                            <div class="message-route">
                                                <span class="message-endpoint from">
                                                    {item.pid}:{item.name}
                                                </span>
                                                <i class="codicon codicon-question"></i>
                                                <span class="message-status">No connection</span>
                                            </div>
                                            <div class="message-payload">
                                                <For each={item.values}>
                                                    {(val) => (
                                                        <div class="payload-item">{literal(val)}</div>
                                                    )}
                                                </For>
                                            </div>
                                        </div>
                                    )}
                                </For>
                            </div>
                        </div>
                    </Show>
                </div>
            </div>
        </Show>
    );
}
