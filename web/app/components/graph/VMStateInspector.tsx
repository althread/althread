import { For, Show, createMemo } from "solid-js";
import { Node, literal } from "./Node";
import "./VMStateInspector.css";

interface VMStateInspectorProps {
    node: Node | null;
    onClose: () => void;
}

export default function VMStateInspector(props: VMStateInspectorProps) {
    const getField = (obj: any, field: string) => {
        if (!obj) return undefined;
        if (typeof obj.get === 'function') {
            return obj.get(field);
        }
        return obj[field];
    };

    const asEntries = (value: any): [any, any][] => {
        if (!value) return [];
        if (Array.isArray(value)) {
            if (value.length > 0 && Array.isArray(value[0]) && value[0].length === 2) {
                return value as [any, any][];
            }
            return value.map((v, i) => [i, v]);
        }
        if (value instanceof Map) return Array.from(value.entries());
        if (typeof value.entries === 'function') return Array.from(value.entries());
        if (typeof value === 'object') return Object.entries(value);
        return [];
    };

    const asArray = (value: any): any[] => {
        if (!value) return [];
        if (Array.isArray(value)) return value;
        if (value instanceof Map) return Array.from(value.values());
        if (typeof value.values === 'function') return Array.from(value.values());
        return [];
    };

    const normalizePrograms = (node: any) => {
        const locals = getField(node, 'locals') ?? getField(node, 'programs');
        if (!locals) return [];
        if (Array.isArray(locals)) return locals;
        return asEntries(locals).map(([, value]) => value);
    };

    const getChannelsForProcess = (node: any, pid: number) => {
        if (pid === undefined || pid === null) return [];
        const channels: {name: string, values: any[]}[] = [];
        const nodeChannels = getField(node, 'channels');
        if (!nodeChannels) return [];

        if (Array.isArray(nodeChannels)) {
            for (const ch of nodeChannels) {
                const chPid = getField(ch, 'pid');
                if (chPid === pid) {
                    channels.push({
                        name: String(getField(ch, 'name')),
                        values: asArray(getField(ch, 'values'))
                    });
                }
            }
            return channels;
        }

        for (const [key, value] of asEntries(nodeChannels)) {
            if (value && typeof value === 'object' && !Array.isArray(value)) {
                const vPid = getField(value, 'pid');
                const vName = getField(value, 'name');
                if (vPid === pid && vName !== undefined) {
                    channels.push({
                        name: String(vName),
                        values: asArray(getField(value, 'values'))
                    });
                    continue;
                }
            }

            let k: any = key;
            if (typeof key === 'string' && (key.startsWith('[') || key.startsWith('('))) {
                try {
                    k = JSON.parse(key.replace(/\(/g, '[').replace(/\)/g, ']'));
                } catch (e) {}
            }

            if (Array.isArray(k) && k.length === 2 && k[0] === pid) {
                channels.push({
                    name: String(k[1]),
                    values: Array.isArray(value) ? value : [value]
                });
            }
        }
        return channels;
    };

    const flattenMessages = (value: any) => {
        if (!value) return [];
        if (Array.isArray(value)) return value;
        const entries = asEntries(value);
        const flattened: any[] = [];
        for (const [, messages] of entries) {
            if (Array.isArray(messages)) {
                flattened.push(...messages);
            } else if (messages) {
                flattened.push(messages);
            }
        }
        return flattened;
    };

    const parseTupleKey = (key: any): any[] | null => {
        if (Array.isArray(key)) return key;
        if (typeof key === 'string' && (key.startsWith('[') || key.startsWith('('))) {
            try {
                return JSON.parse(key.replace(/\(/g, '[').replace(/\)/g, ']'));
            } catch (e) {
                return null;
            }
        }
        return null;
    };

    const normalizePendingMessages = (value: any) => {
        if (!value) return [];
        if (Array.isArray(value)) return value;
        const entries = asEntries(value);
        const normalized: any[] = [];
        for (const [key, messages] of entries) {
            const tuple = parseTupleKey(key);
            if (tuple && tuple.length === 4) {
                const [from_pid, from_channel, to_pid, to_channel] = tuple;
                const vals = Array.isArray(messages) ? messages : [messages];
                for (const msg of vals) {
                    normalized.push({
                        from_pid,
                        from_channel,
                        to_pid,
                        to_channel,
                        values: Array.isArray(msg) ? msg : [msg]
                    });
                }
            } else if (Array.isArray(messages)) {
                normalized.push(...messages);
            } else if (messages) {
                normalized.push(messages);
            }
        }
        return normalized;
    };

    const normalizeWaitingMessages = (value: any) => {
        if (!value) return [];
        if (Array.isArray(value)) return value;
        const entries = asEntries(value);
        const normalized: any[] = [];
        for (const [key, messages] of entries) {
            const tuple = parseTupleKey(key);
            if (tuple && tuple.length === 2) {
                const [pid, name] = tuple;
                const vals = Array.isArray(messages) ? messages : [messages];
                for (const msg of vals) {
                    normalized.push({
                        pid,
                        name,
                        values: Array.isArray(msg) ? msg : [msg]
                    });
                }
            } else if (Array.isArray(messages)) {
                normalized.push(...messages);
            } else if (messages) {
                normalized.push(messages);
            }
        }
        return normalized;
    };

    const hasVmFields = (obj: any) => {
        if (!obj) return false;
        return (
            getField(obj, 'globals') !== undefined ||
            getField(obj, 'channels') !== undefined ||
            getField(obj, 'locals') !== undefined ||
            getField(obj, 'programs') !== undefined
        );
    };

    const state = createMemo(() => {
        const node = props.node as any;
        if (!node) return null;
        if (hasVmFields(node)) return node;
        const nested =
            getField(node, 'rawState') ??
            getField(node, 'vm_state') ??
            getField(node, 'current_state') ??
            getField(node, 'state') ??
            null;
        if (nested && hasVmFields(nested)) return nested;
        return nested ?? node;
    });

    const globalsEntries = createMemo(() => {
        const s = state();
        if (!s) return [];
        return asEntries(getField(s, 'globals'));
    });

    const programs = createMemo(() => {
        const s = state();
        if (!s) return [];
        return normalizePrograms(s);
    });

    const pendingMessages = createMemo(() => {
        const s = state();
        if (!s) return [];
        return normalizePendingMessages(getField(s, 'pending_deliveries'));
    });

    const waitingMessages = createMemo(() => {
        const s = state();
        if (!s) return [];
        return normalizeWaitingMessages(getField(s, 'waiting_send'));
    });

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
                {(node) => (
                    <div class="inspector-sections">
                        <div class="inspector-left-column">
                            {/* Globals Section */}
                            <div class="section globals-section">
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
                                                        <span class="var-name">{String(key)}</span>{ " = " }
                                                        <span class="var-value">{literal(value)}</span>
                                                    </div>
                                                )}
                                            </For>
                                        </div>
                                    </Show>
                                </div>
                            </div>

                            {/* Pending Messages Section */}
                            <Show when={pendingMessages().length > 0 || waitingMessages().length > 0}>
                                <div class="section pending-section">
                                    <div class="section-header">In-flight Messages</div>
                                    <div class="section-body">
                                        <Show when={pendingMessages().length > 0}>
                                            <div class="subsection">
                                                <div class="subsection-title">Pending Delivery</div>
                                                <div class="message-list">
                                                    <For each={pendingMessages()}>
                                                        {(item) => {
                                                            const from_pid = getField(item, 'from_pid');
                                                            const from_chan = getField(item, 'from_channel');
                                                            const to_pid = getField(item, 'to_pid');
                                                            const to_chan = getField(item, 'to_channel');
                                                            const values = asArray(getField(item, 'values'));
                                                            return (
                                                                <div class="message-card">
                                                                    <div class="message-route">
                                                                        PID {from_pid}.{from_chan} → PID {to_pid}.{to_chan}
                                                                    </div>
                                                                    <div class="message-contents">
                                                                        <For each={values}>
                                                                            {(val) => <div class="msg-val">{literal(val)}</div>}
                                                                        </For>
                                                                    </div>
                                                                </div>
                                                            );
                                                        }}
                                                    </For>
                                                </div>
                                            </div>
                                        </Show>
                                        <Show when={waitingMessages().length > 0}>
                                            <div class="subsection">
                                                <div class="subsection-title">Waiting (Unconnected)</div>
                                                <div class="message-list">
                                                    <For each={waitingMessages()}>
                                                        {(item) => {
                                                            const pid = getField(item, 'pid');
                                                            const name = getField(item, 'name');
                                                            const values = asArray(getField(item, 'values'));
                                                            return (
                                                                <div class="message-card waiting">
                                                                    <div class="message-route">
                                                                        PID {pid}.{name} → ?
                                                                    </div>
                                                                    <div class="message-contents">
                                                                        <For each={values}>
                                                                            {(val) => <div class="msg-val">{literal(val)}</div>}
                                                                        </For>
                                                                    </div>
                                                                </div>
                                                            );
                                                        }}
                                                    </For>
                                                </div>
                                            </div>
                                        </Show>
                                    </div>
                                </div>
                            </Show>
                        </div>

                        {/* Programs Section */}
                        <div class="section programs-section">
                            <div class="section-header">Processes</div>
                            <div class="section-body">
                                <Show
                                    when={programs().length > 0}
                                    fallback={<div class="empty-state">No processes</div>}
                                >
                                    <For each={programs()}>
                                        {(prog: any) => {
                                            const pid = getField(prog, 'pid') ?? getField(prog, 'id');
                                            const name = getField(prog, 'name') ?? `PID ${pid ?? '?'}`;
                                            const pc = getField(prog, 'instruction_pointer') ?? getField(prog, 'pc') ?? getField(prog, 'instructionPointer');
                                            const memory = getField(prog, 'memory') ?? getField(prog, 'stack');
                                            const memoryValues = Array.isArray(memory) ? memory : asArray(memory);
                                            const channels = getChannelsForProcess(node(), pid);

                                            return (
                                                <div class="process-card">
                                                    <div class="process-card-header">
                                                        <span class="process-name">{name}</span>
                                                        <span class="process-id">PID {pid ?? '?'}</span>
                                                        <span class="process-pc">PC {pc ?? '-'}</span>
                                                    </div>
                                                    <div class="process-card-body">
                                                        <div class="stack-row">
                                                            <span class="row-label">Stack</span>
                                                            <div class="stack-container">
                                                                <For each={memoryValues}>
                                                                    {(val) => <span class="stack-val">{literal(val)}</span>}
                                                                </For>
                                                                {memoryValues.length === 0 && <span class="empty-val">empty</span>}
                                                            </div>
                                                        </div>

                                                        <Show when={channels.length > 0}>
                                                            <div class="channels-row">
                                                                <span class="row-label">Channels</span>
                                                                <div class="channels-container">
                                                                    <For each={channels}>
                                                                        {(ch) => (
                                                                            <div class="channel-item">
                                                                                <span class="ch-name">{ch.name}</span>
                                                                                <span class="ch-arrow">←</span>
                                                                                <span class="ch-vals">{ch.values.map(v => literal(v)).join(', ')}</span>
                                                                            </div>
                                                                        )}
                                                                    </For>
                                                                </div>
                                                            </div>
                                                        </Show>
                                                    </div>
                                                </div>
                                            );
                                        }}
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
