import { createMemo, For, Show } from "solid-js";
import "./GlobalsDisplay.css";
import "./InFlightMessagesDisplay.css";
import type { Literal } from "../../types/vm-state";
import { LiteralDisplay } from "./Literal";

interface GlobalsDisplayProps {
	globals: Record<string, Literal>;
	collapsed: boolean;
	canCollapse: boolean;
	onToggle: () => void;
	hasChanged?: boolean;
}

export default function GlobalsDisplay(props: GlobalsDisplayProps) {
	const globalsEntries = createMemo(() => {
		return Object.entries(props.globals).sort((a, b) =>
			a[0].localeCompare(b[0]),
		);
	});

	const isLocked = () => !props.collapsed && !props.canCollapse;

	return (
		<div class={`section${props.collapsed ? " section-collapsed" : ""}`}>
			<div
				class={`collapsible-header${isLocked() ? " header-locked" : ""}`}
				onClick={props.onToggle}
				onKeyDown={(e) => {
					if (e.key === "Enter" || e.key === " ") {
						e.preventDefault();
						props.onToggle();
					}
				}}
				role="button"
				tabIndex={isLocked() ? -1 : 0}
				aria-expanded={!props.collapsed}
				aria-controls="globals-section-body"
				title={
					isLocked()
						? "Cannot collapse — only section open"
						: props.collapsed
							? "Expand Globals"
							: "Collapse Globals"
				}
			>
				<i
					class={`codicon ${props.collapsed ? "codicon-chevron-left" : "codicon-chevron-right"} header-toggle-icon`}
				></i>
				<i class="codicon codicon-symbol-variable header-section-icon"></i>
				<span class="section-label">Globals</span>
				<div class={`header-count${props.hasChanged ? " changed" : ""}`}>
					<Show when={props.hasChanged}>
						<span class="change-indicator">!</span>
					</Show>
					{globalsEntries().length}
				</div>
			</div>
			<div
				id="globals-section-body"
				class={`section-body${props.collapsed ? " section-body-hidden" : ""}`}
			>
				<Show
					when={globalsEntries().length > 0}
					fallback={<div class="empty-state">No globals</div>}
				>
					<div class="variables-grid">
						<For each={globalsEntries()}>
							{([key, value]) => (
								<div class="variable-item">
									<span class="var-name">{key}</span>
									{" = "}
									<span class="var-value">
										<LiteralDisplay value={value} />
									</span>
								</div>
							)}
						</For>
					</div>
				</Show>
			</div>
		</div>
	);
}
