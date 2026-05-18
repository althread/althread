/**
 * VM State Types
 *
 * This file re-exports types generated from Rust via tsify and adds
 * additional frontend-only types for UI state management.
 */

// Re-export all generated types from WASM package
export type {
	CallFrame,
	ChannelConnection,
	ChannelEndpoint,
	ChannelState,
	CheckResult,
	ExecutedStepInfo,
	GraphNode,
	InteractiveSessionState,
	InteractiveStepResult,
	Literal,
	MessageFlowEvent,
	NextStateOption,
	NodeMetadata,
	PendingDelivery,
	ProgramState,
	RunResult,
	Successor,
	VariableInfo,
	VMState,
	WaitingSend,
} from "../../pkg/althread_web";

// Re-import types for use in this file
import type { VMState } from "../../pkg/althread_web";

/**
 * Frontend-only types for UI state management
 */

/**
 * Node data for vis.js graph visualization
 */
export interface VisGraphNode {
	id: number | string;
	label: string;
	level: number;
	color?: {
		background?: string;
		border?: string;
	};
	font?: {
		color?: string;
	};
	title?: string;
	fullLabel?: string;
	rawState?: VMState | { state: VMState; stepIndex: number };
}

/**
 * Edge data for vis.js graph visualization
 */
export interface VisGraphEdge {
	id?: number | string;
	from: number | string;
	to: number | string;
	label?: string;
	lines?: number[];
	color?:
		| string
		| {
				color?: string;
				highlight?: string;
				hover?: string;
		  };
	width?: number;
	font?: {
		size?: number;
		color?: string;
		background?: string;
		strokeWidth?: number;
		strokeColor?: string;
	};
	arrows?: string | { to?: boolean | { enabled?: boolean } };
}

/**
 * Result from graph building utilities
 */
export interface GraphBuildResult {
	nodes: VisGraphNode[];
	edges: VisGraphEdge[];
}

/**
 * VM state selection for UI highlighting
 */
/**
 * Selection state for a VM node (used for inspector display and highlighting)
 * This is a unified format used across run and check modes
 */
export interface VMStateSelection {
	vm: VMState;
	stepIndex?: number; // Optional: only present in run mode for step navigation
	level?: number; // Optional: only present in check mode for level info
}

/**
 * Execution mode for the editor
 */
export type ExecutionMode = "run" | "check" | "interactive" | null;

/**
 * Active execution tab
 */
export type ExecutionTab = "console" | "execution" | "commgraph" | "vm_states";

/**
 * Sidebar view type
 */
export type SidebarView = "help" | "explorer" | "search" | "tutorials";

/**
 * File system entry
 */
export interface FileSystemEntry {
	id: string;
	name: string;
	type: "file" | "directory";
	content?: string;
	children?: FileSystemEntry[];
}

/**
 * Open file in editor
 */
export interface OpenFile {
	id: string;
	name: string;
	content: string;
}

/**
 * Helper type for literal formatting
 */
export type LiteralValue =
	| string
	| number
	| boolean
	| null
	| LiteralValue[]
	| { [key: string]: LiteralValue };
