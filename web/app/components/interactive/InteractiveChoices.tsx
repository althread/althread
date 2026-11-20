import { For, type JSX, onMount, Show } from "solid-js";
import type { NextStateOption } from "../../types/vm-state";
import "./InteractiveChoices.css";

interface InteractiveChoicesProps {
	choices: NextStateOption[];
	finished: boolean;
	deadlock: boolean;
	stepCount: number;
	/** Returns the source text of the given 1-based lines of the active file. */
	getSourceLines: (lines: number[]) => string[];
	onExecute: (index: number) => void;
	onPreviewLines: (lines: number[] | null) => void;
	onUndo: () => void;
	onRestart: () => void;
	onStop: () => void;
}

const MESSAGE_OPS = ["SEND", "RECEIVE", "DELIVER"];

export default function InteractiveChoices(props: InteractiveChoicesProps) {
	const describe = (choice: NextStateOption): JSX.Element => {
		const lines = props.getSourceLines(choice.lines);
		const messageOps = choice.instructions.filter((inst) =>
			MESSAGE_OPS.some((op) => inst.includes(op)),
		);

		if (lines.length === 0) {
			return <code>{choice.instructions.join("; ")}</code>;
		}
		return (
			<>
				<For each={lines}>{(line) => <code>{line}</code>}</For>
				<Show when={messageOps.length > 0}>
					<span class="choice-message-op">
						<i class="codicon codicon-arrow-right" /> {messageOps.join(", ")}
					</span>
				</Show>
			</>
		);
	};

	// Keyboard shortcuts: 1-9 pick a choice, Backspace / u undoes the last step.
	const onKeyDown = (e: KeyboardEvent) => {
		if (e.ctrlKey || e.metaKey || e.altKey) return;
		const n = Number.parseInt(e.key, 10);
		if (n >= 1 && n <= 9 && n <= props.choices.length) {
			e.preventDefault();
			props.onExecute(n - 1);
		} else if ((e.key === "Backspace" || e.key === "u") && props.stepCount > 0) {
			e.preventDefault();
			props.onUndo();
		}
	};

	let containerRef: HTMLDivElement | undefined;
	// Focus the list so that the number keys work right away.
	onMount(() => containerRef?.focus({ preventScroll: true }));

	return (
		<section
			class="interactive-choices"
			aria-label="Next instruction"
			ref={containerRef}
			tabIndex={-1}
			onKeyDown={onKeyDown}
		>
			<div class="interactive-toolbar">
				<span class="interactive-step-count">
					Step <strong>{props.stepCount}</strong>
				</span>
				<div class="interactive-toolbar-actions">
					<button
						type="button"
						class="icon-button"
						onClick={() => props.onUndo()}
						disabled={props.stepCount === 0}
						title="Undo last step (Backspace)"
					>
						<i class="codicon codicon-discard" />
						<span>Undo</span>
					</button>
					<button
						type="button"
						class="icon-button"
						onClick={() => props.onRestart()}
						title="Restart from the initial state"
					>
						<i class="codicon codicon-debug-restart" />
						<span>Restart</span>
					</button>
					<button
						type="button"
						class="icon-button"
						onClick={() => props.onStop()}
						title="Leave step-by-step mode"
					>
						<i class="codicon codicon-debug-stop" />
						<span>Stop</span>
					</button>
				</div>
			</div>

			<Show
				when={props.choices.length > 0}
				fallback={
					<div class="results-empty">
						<i
							class={`codicon ${props.deadlock ? "codicon-debug-pause" : "codicon-pass"}`}
						/>
						<h4>{props.deadlock ? "Deadlock" : "Execution completed"}</h4>
						<p>
							{props.deadlock
								? "Every process is blocked. Inspect the States tab to see what each one is waiting for."
								: "No process can execute any further instruction."}
						</p>
					</div>
				}
			>
				<p class="interactive-hint">
					Pick the process that executes next. Hover a choice to see its code
					highlighted in the editor.
				</p>
				<ol class="choice-list">
					<For each={props.choices}>
						{(choice, index) => (
							<li>
								<button
									type="button"
									class="choice-item"
									onClick={() => props.onExecute(index())}
									onMouseEnter={() => props.onPreviewLines(choice.lines)}
									onMouseLeave={() => props.onPreviewLines(null)}
									onFocus={() => props.onPreviewLines(choice.lines)}
									onBlur={() => props.onPreviewLines(null)}
									title={
										index() < 9
											? `Execute (press ${index() + 1})`
											: "Execute this instruction"
									}
								>
									<span class="choice-key">{index() < 9 ? index() + 1 : ""}</span>
									<span class="choice-process">
										{choice.prog_name}
										<span class="choice-pid">#{choice.prog_id}</span>
									</span>
									<span class="choice-code">{describe(choice)}</span>
									<i class="codicon codicon-debug-step-over choice-go" />
								</button>
							</li>
						)}
					</For>
				</ol>
			</Show>
		</section>
	);
}
