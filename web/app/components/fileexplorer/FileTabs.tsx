import { For } from "solid-js";
import type { FileSystemEntry } from "./FileExplorer";
import "./FileTabs.css";

type FileTabsProps = {
	openFiles: FileSystemEntry[];
	activeFile: FileSystemEntry | null;
	getFilePath: (entry: FileSystemEntry) => string;
	onTabClick: (file: FileSystemEntry) => void;
	onTabClose: (file: FileSystemEntry) => void;
};

const FileTabs = (props: FileTabsProps) => {
	// Helper function to check if a file is in deps directory (read-only)
	const isInDepsDirectory = (file: FileSystemEntry) => {
		const path = props.getFilePath(file);
		return path === "deps" || path.startsWith("deps/");
	};

	return (
		<div class="file-tabs-container">
			<For each={props.openFiles}>
				{(file) => {
					return (
						/* biome-ignore lint/a11y/useSemanticElements: Complex tab container needs to be a div */
						<div
							class="file-tab"
							role="button"
							tabIndex={0}
							classList={{
								active:
									props.activeFile !== null &&
									props.getFilePath(props.activeFile) ===
										props.getFilePath(file),
								"read-only": isInDepsDirectory(file),
							}}
							onClick={() => props.onTabClick(file)}
							onKeyDown={(e) => {
								if (e.key === "Enter" || e.key === " ") {
									props.onTabClick(file);
								}
							}}
						>
							<i class="codicon codicon-file"></i>
							<span class="tab-label">{file.name}</span>
							{isInDepsDirectory(file) && (
								<i
									class="codicon codicon-lock read-only-icon"
									title="Read-only file"
								></i>
							)}
							<button
								type="button"
								class="tab-close-button"
								title="Close"
								onClick={(e) => {
									e.stopPropagation();
									props.onTabClose(file);
								}}
							>
								<i class="codicon codicon-close"></i>
							</button>
						</div>
					);
				}}
			</For>
		</div>
	);
};

export default FileTabs;
