/**
 * Literal Display Component
 * 
 * Renders typed literal values with proper styling and recursive display for lists/tuples.
 */

import { Switch, Match, For } from "solid-js";
import type { Literal } from "../../types/vm-state";
import "./Literal.css";

export function LiteralDisplay(props: { value: Literal }) {
    return (
        <Switch>
            <Match when={props.value.type === "Null"}>
                <span class="literal-null">null</span>
            </Match>
            <Match when={props.value.type === "Int"}>
                <span class="literal-int">{(props.value as any).value}</span>
            </Match>
            <Match when={props.value.type === "Float"}>
                <span class="literal-float">{(props.value as any).value}</span>
            </Match>
            <Match when={props.value.type === "String"}>
                <span class="literal-string">"{(props.value as any).value}"</span>
            </Match>
            <Match when={props.value.type === "Bool"}>
                <span class="literal-bool">{(props.value as any).value ? "true" : "false"}</span>
            </Match>
            <Match when={props.value.type === "List"}>
                <span class="literal-list">
                    [
                    <For each={(props.value as any).value}>
                        {(item: Literal, index) => (
                            <>
                                {index() > 0 && ", "}
                                <LiteralDisplay value={item} />
                            </>
                        )}
                    </For>
                    ]
                </span>
            </Match>
            <Match when={props.value.type === "Tuple"}>
                <span class="literal-tuple">
                    (
                    <For each={(props.value as any).value}>
                        {(item: Literal, index) => (
                            <>
                                {index() > 0 && ", "}
                                <LiteralDisplay value={item} />
                            </>
                        )}
                    </For>
                    )
                </span>
            </Match>
            <Match when={props.value.type === "Process"}>
                <span class="literal-process">
                    Proc({(props.value as any).value[0]}#{(props.value as any).value[1]})
                </span>
            </Match>
        </Switch>
    );
}