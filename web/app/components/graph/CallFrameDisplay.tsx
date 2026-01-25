import { For, Show } from "solid-js";
import VariableDisplay from "./VariableDisplay";
import type { CallFrame } from "../../types/vm-state";
import "./CallFrameDisplay.css";

interface CallFrameDisplayProps {
    frames: CallFrame[];
    fallbackMemory?: string[];
}

export default function CallFrameDisplay(props: CallFrameDisplayProps) {
    return (
        <div class="call-frame-display">
            <Show 
                when={props.frames && props.frames.length > 0}
                fallback={
                    <Show when={props.fallbackMemory}>
                        <div class="frame-card">
                            <div class="frame-header">
                                <span class="frame-title">Stack Memory</span>
                            </div>
                            <div class="frame-body">
                                <VariableDisplay 
                                    variables={{}} 
                                    fallbackMemory={props.fallbackMemory} 
                                />
                            </div>
                        </div>
                    </Show>
                }
            >
                <For each={props.frames}>
                    {(frame, index) => (
                        <div class="frame-card" classList={{ "top-frame": index() === 0 }}>
                            <div class="frame-header">
                                <span class="frame-title">{frame.function}</span>
                                <div class="frame-meta">
                                    <Show when={frame.line}>
                                        <span class="frame-location" title="Source line">
                                            <i class="codicon codicon-file-code"></i>
                                            Line {frame.line}
                                        </span>
                                    </Show>
                                    <span class="frame-ip" title="Instruction pointer">
                                        IP: {frame.instruction_pointer}
                                    </span>
                                </div>
                            </div>
                            <div class="frame-body">
                                <Show 
                                    when={frame.variables && Object.keys(frame.variables).length > 0}
                                    fallback={
                                        <div class="no-variables">No local variables in scope</div>
                                    }
                                >
                                    <VariableDisplay 
                                        variables={frame.variables || {}} 
                                        fallbackMemory={index() === 0 ? props.fallbackMemory : undefined}
                                    />
                                </Show>
                            </div>
                        </div>
                    )}
                </For>
            </Show>
        </div>
    );
}
