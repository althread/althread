import { For, Show } from "solid-js";
import VariableDisplay from "./VariableDisplay";
import "./CallFrameDisplay.css";

interface Frame {
    function: string;
    frame_pointer: number;
    instruction_pointer: number;
    line?: number;
    variables?: Record<string, { value: string; type: string }>;
}

interface CallFrameDisplayProps {
    frames: Frame[];
    fallbackMemory?: any[];
}

export default function CallFrameDisplay(props: CallFrameDisplayProps) {
    console.log("CallFrameDisplay received frames:", props.frames);
    console.log("CallFrameDisplay received fallbackMemory:", props.fallbackMemory);
    
    // Convert Maps to plain objects (serde_wasm_bindgen serializes HashMaps as JS Maps)
    const normalizeFrame = (frame: any): Frame => {
        const getField = (obj: any, key: string) => {
            if (obj instanceof Map) return obj.get(key);
            return obj?.[key];
        };
        
        const normalizeVariables = (vars: any): Record<string, { value: string; type: string }> => {
            if (!vars) return {};
            const result: Record<string, { value: string; type: string }> = {};
            
            if (vars instanceof Map) {
                vars.forEach((varData: any, varName: string) => {
                    if (varData instanceof Map) {
                        result[varName] = {
                            value: varData.get('value') ?? '',
                            type: varData.get('type') ?? ''
                        };
                    } else {
                        result[varName] = {
                            value: varData?.value ?? '',
                            type: varData?.type ?? ''
                        };
                    }
                });
            } else if (typeof vars === 'object') {
                Object.entries(vars).forEach(([varName, varData]: [string, any]) => {
                    result[varName] = {
                        value: varData?.value ?? '',
                        type: varData?.type ?? ''
                    };
                });
            }
            
            return result;
        };
        
        return {
            function: getField(frame, 'function') ?? 'unknown',
            frame_pointer: getField(frame, 'frame_pointer') ?? 0,
            instruction_pointer: getField(frame, 'instruction_pointer') ?? 0,
            line: getField(frame, 'line'),
            variables: normalizeVariables(getField(frame, 'variables'))
        };
    };
    
    const normalizedFrames = Array.isArray(props.frames) 
        ? props.frames.map(normalizeFrame)
        : [];
    
    return (
        <div class="call-frame-display">
            <Show 
                when={normalizedFrames && normalizedFrames.length > 0}
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
                <For each={normalizedFrames}>
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
