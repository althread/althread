import { createEffect, onCleanup, Show } from "solid-js";

export interface LoadExampleDialogProps {
	isOpen: boolean;
	onLoadInCurrent: () => void;
	onLoadInNew: () => void;
	onCancel: () => void;
	title?: string;
	message?: string;
	detail?: string;
	currentLabel?: string;
	newLabel?: string;
}

export const LoadExampleDialog = (props: LoadExampleDialogProps) => {
	createEffect(() => {
		if (props.isOpen) {
			const handleKeyDown = (e: KeyboardEvent) => {
				if (e.key === "Escape") props.onCancel();
			};
			window.addEventListener("keydown", handleKeyDown);
			onCleanup(() => window.removeEventListener("keydown", handleKeyDown));
		}
	});

	return (
		<Show when={props.isOpen}>
			{/* biome-ignore lint/a11y/useSemanticElements: Backdrop needs to be a div */}
			<div
				class="confirmation-dialog-overlay"
				onClick={(e) => {
					if (e.target === e.currentTarget) props.onCancel();
				}}
				onKeyDown={(e) => {
					if (e.key === "Escape") props.onCancel();
				}}
				role="button"
				tabIndex={-1}
			>
				<div class="confirmation-dialog" role="dialog" aria-modal="true">
					<div class="confirmation-dialog-header">
						<i class="codicon codicon-file-code"></i>
						{props.title || "Load Example"}
					</div>
					<div class="confirmation-dialog-body">
						{props.message || "Where would you like to load the example code?"}
						<br />
						<br />
						<div style="font-size: 12px; color: #858585;">
							{props.detail ||
								"Choose to replace the current file content or create a new file tab."}
						</div>
					</div>
					<div class="confirmation-dialog-actions">
						<button
							type="button"
							class="button-secondary"
							onClick={props.onCancel}
						>
							<i class="codicon codicon-close"></i>
							Cancel
						</button>
						<button
							type="button"
							class="button-secondary"
							onClick={props.onLoadInCurrent}
						>
							<i class="codicon codicon-file"></i>
							{props.currentLabel || "Current File"}
						</button>
						<button
							type="button"
							class="button-primary"
							onClick={props.onLoadInNew}
						>
							<i class="codicon codicon-new-file"></i>
							{props.newLabel || "New File"}
						</button>
					</div>
				</div>
			</div>
		</Show>
	);
};
