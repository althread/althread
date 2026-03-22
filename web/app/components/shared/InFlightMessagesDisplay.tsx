import { For, Show } from "solid-js";
import type { PendingDelivery, WaitingSend } from "../../types/vm-state";
import "./InFlightMessagesDisplay.css";
import { LiteralDisplay } from "./Literal";

interface InFlightMessagesDisplayProps {
	pendingDeliveries: PendingDelivery[];
	waitingSend: WaitingSend[];
	collapsed: boolean;
	canCollapse: boolean;
	onToggle: () => void;
	hasChanged?: boolean;
}

export default function InFlightMessagesDisplay(
	props: InFlightMessagesDisplayProps,
) {
	const hasMessages = () =>
		props.pendingDeliveries.length > 0 || props.waitingSend.length > 0;
	const totalCount = () =>
		props.pendingDeliveries.length + props.waitingSend.length;

	const isLocked = () => !props.collapsed && !props.canCollapse;

	return (
		<div class={`section${props.collapsed ? " section-collapsed" : ""}`}>
			<div
				class={`collapsible-header${isLocked() ? " header-locked" : ""}`}
				onClick={props.onToggle}
				title={
					isLocked()
						? "Cannot collapse — only section open"
						: props.collapsed
							? "Expand Messages"
							: "Collapse Messages"
				}
			>
				<i
					class={`codicon ${props.collapsed ? "codicon-chevron-left" : "codicon-chevron-right"} header-toggle-icon`}
				></i>
				<i class="codicon codicon-mail header-section-icon"></i>
				<span class="section-label">In-flight messages</span>
				<div class={`header-count${props.hasChanged ? " changed" : ""}`}>
					<Show when={props.hasChanged}>
						<span class="change-indicator">!</span>
					</Show>
					{totalCount()}
				</div>
			</div>
			<div
				class={`section-body${props.collapsed ? " section-body-hidden" : ""}`}
			>
				<Show
					when={hasMessages()}
					fallback={<div class="empty-state">No pending messages</div>}
				>
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
												<i class="codicon codicon-arrow-right route-arrow"></i>
												<span class="message-endpoint to">
													{item.to_pid}:{item.to_channel}
												</span>
											</div>
											<div class="message-payload">
												<For each={item.values}>
													{(val) => (
														<div class="payload-item">
															<LiteralDisplay value={val} />
														</div>
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
												<i class="codicon codicon-question route-arrow"></i>
												<span class="message-status">No connection</span>
											</div>
											<div class="message-payload">
												<For each={item.values}>
													{(val) => (
														<div class="payload-item">
															<LiteralDisplay value={val} />
														</div>
													)}
												</For>
											</div>
										</div>
									)}
								</For>
							</div>
						</div>
					</Show>
				</Show>
			</div>
		</div>
	);
}
