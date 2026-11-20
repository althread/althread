import type { Color } from "vis-network";
import type {
	ChannelState,
	Literal,
	ProgramState,
	VMState,
} from "../../types/vm-state";

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

export const literal = (value: Literal | unknown): string => {
	if (!value || typeof value !== "object") {
		return String(value);
	}
	const lit = value as Literal;

	if (lit.type === "Null") return "null";
	if (lit.type === "Int" || lit.type === "Float" || lit.type === "Bool") {
		return String(lit.value);
	}
	if (lit.type === "String") {
		return `"${lit.value}"`;
	}

	if (lit.type === "Tuple") {
		const tupleValues = lit.value;
		if (Array.isArray(tupleValues)) {
			return "(" + tupleValues.map(literal).join(",") + ")";
		}
		return "()";
	}

	if (lit.type === "List") {
		const listValues = lit.value;
		if (Array.isArray(listValues)) {
			return "[" + listValues.map(literal).join(",") + "]";
		}
		return "[]";
	}

	if (lit.type === "Process") {
		return `Proc(${lit.value[0]}#${lit.value[1]})`;
	}

	// Fallback for more complex objects on stack, or adjust as needed
	return JSON.stringify(value);
};

export const nodeToString = (n: VMState | null | undefined): string => {
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
					.map((prog_state: ProgramState) => {
						const program_details =
							`  *Program ${prog_state.name}* (pid ${prog_state.pid}, clock ${prog_state.clock}):\n` +
							`    pc: ${prog_state.instruction_pointer}\n` +
							`    stack: [${(prog_state.memory || []).map(literal).join(", ")}]`;

						const progChannels = (n.channels || []).filter(
							(c: ChannelState) => c.pid === prog_state.pid,
						);
						let program_channels_output = "";
						if (progChannels.length > 0) {
							program_channels_output += "\n    *Channels:*\n";
							program_channels_output += progChannels
								.map(
									(c: ChannelState) =>
										`      ${c.name} <- [${(c.values || []).map(literal).join(", ")}]`,
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
