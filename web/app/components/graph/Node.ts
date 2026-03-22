import type { Color } from "vis-network";
import type { ProgramStateJS } from "./State";

export type Node = {
	// This represents the VM state in JS
	channels: any[]; // Array of ChannelState
	globals: Record<string, any>; // Record of globals
	locals: ProgramStateJS[]; // Array of program states
};

export type VisNode = {
	id: number;
	label: string;
	fullLabel?: string;
	level?: number;
	color: Color;
	isViolationNode: boolean;
};

//////////////////////////////////////
export const node_entirely = (n: any) => {
	return JSON.stringify(n, null, 2);
};
//////////////////////////////////////

export const literal = (value: any): string => {
	if (!value || typeof value !== "object") {
		return String(value);
	}

	if (value.type === "Null") return "null";
	if (value.type === "Int" || value.type === "Float" || value.type === "Bool") {
		return String(value.value);
	}
	if (value.type === "String") {
		return `"${value.value}"`;
	}

	if (value.type === "Tuple") {
		const tupleValues = value.value;
		if (Array.isArray(tupleValues)) {
			return "(" + tupleValues.map(literal).join(",") + ")";
		}
		return "()";
	}

	if (value.type === "List") {
		const listValues = value.value;
		if (Array.isArray(listValues)) {
			return "[" + listValues.map(literal).join(",") + "]";
		}
		return "[]";
	}

	if (value.type === "Process") {
		return `Proc(${value.value[0]}#${value.value[1]})`;
	}

	// Fallback for more complex objects on stack, or adjust as needed
	return JSON.stringify(value);
};

export const nodeToString = (n: any): string => {
	if (!n) {
		return "VM state is not available.";
	}

	// Global section for globals
	const globalEntries = Object.entries(n.globals || {});
	const globals_label =
		"*Globals:*\n" +
		(globalEntries.length > 0
			? globalEntries.map(([k, v]) => "  " + k + " = " + literal(v)).join("\n")
			: "  _No global variables_");

	const locals_and_channels_label =
		"\n\n*Program States & Channels:*\n" +
		(n.locals && n.locals.length > 0
			? n.locals
					.map((prog_state: ProgramStateJS) => {
						const program_details =
							`  *Program ${prog_state.name}* (pid ${prog_state.pid}, clock ${prog_state.clock}):\n` +
							`    pc: ${prog_state.instruction_pointer}\n` +
							`    stack: [${(prog_state.memory || []).map((v) => literal(v)).join(", ")}]`;

						const progChannels = (n.channels || []).filter(
							(c: any) => c.pid === prog_state.pid,
						);
						let program_channels_output = "";
						if (progChannels.length > 0) {
							program_channels_output += "\n    *Channels:*\n";
							program_channels_output += progChannels
								.map(
									(c: any) =>
										`      ${c.name} <- [${(c.values || []).map((v) => literal(v)).join(", ")}]`,
								)
								.join("\n");
						} else {
							program_channels_output =
								"\n    *Channels:*\n      _No active input channels._";
						}

						return program_details + program_channels_output;
					})
					.sort()
					.join("\n\n")
			: "  _No running programs_");

	const result = globals_label + locals_and_channels_label;
	return result;
};
