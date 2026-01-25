import { For, Show } from "solid-js";
import CallFrameDisplay from "../graph/CallFrameDisplay";
import type { ProgramState, ChannelState } from "../../types/vm-state";
import "./ProcessStateCard.css";
import { LiteralDisplay } from "./Literal";

interface ProcessStateCardProps {
    process: ProgramState;
    channels: ChannelState[];
}

export default function ProcessStateCard(props: ProcessStateCardProps) {
    return (
        <div class="process-card">
            <div class="process-card-header">
                <span class="process-name">{props.process.name}</span>
                <span class="process-id">PID {props.process.pid}</span>
                <span class="process-pc">PC {props.process.instruction_pointer}</span>
                <Show when={props.process.line}>
                    <span class="process-line">Line {props.process.line}</span>
                </Show>
            </div>
            <div class="process-card-body">
                <CallFrameDisplay 
                    frames={props.process.frames}
                    fallbackMemory={props.process.memory}
                />

                <Show when={props.channels.length > 0}>
                    <div class="channels-row">
                        <span class="row-label">Channels</span>
                        <div class="channels-container">
                            <For each={props.channels}>
                                {(ch) => (
                                    <div class="channel-pill">
                                        <span class="channel-name">{ch.name}</span>
                                        <Show when={ch.values.length > 0}>
                                            <span class="channel-count">[{ch.values.length}]</span>
                                            <div class="channel-values">
                                                <For each={ch.values}>
                                                    {(val) => (
                                                        <div class="channel-value-item"><LiteralDisplay value={val} /></div>
                                                    )}
                                                </For>
                                            </div>
                                        </Show>
                                    </div>
                                )}
                            </For>
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    );
}
