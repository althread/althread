import { ProgramStateJS } from "./State";
import { Color } from "vis-network";

export type Node = { // This represents the VM state in JS
  channels: Map<any, any[]>; // Or however channels are structured from Rust
  globals: Map<any, any>;    // Or however globals are structured from Rust
  locals: ProgramStateJS[];  // This is the key change: an array of program states
};

export type VisNode = {
  id: number;
  label: string;
  level?: number;
  color: Color;
  isViolationNode: boolean;
};

//////////////////////////////////////
export const node_entirely = (n: Node) => {
  return JSON.stringify(n, null, 2);
};
//////////////////////////////////////

export const literal = (value: any): string => {
  if (!value || typeof value !== 'object') {
    return String(value); // Handle primitive types or null/undefined gracefully
  }
  const keys = Object.keys(value);
  if (keys.length === 0) {
    return '{}'; // Handle empty objects
  }
  const firstKey = keys[0];

  if (firstKey === "tuple") {
    const tupleValues = value[firstKey];
    if (Array.isArray(tupleValues)) {
      return '(' + tupleValues.map(literal).join(',') + ')';
    }
    return '()'; // Empty tuple
  }
  // For other literal types like {int: 5}, {bool: true}, {string: "hello"}
  // or your {program: 'B', pid: 1} which might appear on a stack
  if (keys.length === 1 && typeof value[firstKey] !== 'object') {
     if (firstKey === "program" && value.pid !== undefined) { // Special handling for process literals
        return `Process(${value[firstKey]}, pid ${value.pid})`;
     }
    return String(value[firstKey]);
  }
  // Fallback for more complex objects on stack, or adjust as needed
  return JSON.stringify(value);
};

export const nodeToString = (n: Node): string => {
  if (!n) {
    return "VM state is not available.";
  }

  // Global section for globals
  let globals_label = '*Globals:*\n' + (
    n.globals && n.globals.size > 0
      ? [...Array.from(n.globals.entries()).map(
          ([k, v]) => '  ' + String(k) + ' = ' + literal(v)
        )].join('\n')
      : '  _No global variables_'
  );

  let locals_and_channels_label = '\n\n*Program States & Channels:*\n' + (
    n.locals && n.locals.length > 0
      ? n.locals.map(
          (prog_state: ProgramStateJS) => {
            let program_details =
              `  *Program ${prog_state.name}* (pid ${prog_state.pid}, clock ${prog_state.clock}):\n` +
              `    pc: ${prog_state.instruction_pointer}\n` +
              `    stack: [${prog_state.memory.map(v => literal(v)).join(', ')}]`;

            let program_channels_output = '';
            let has_prog_channels = false;
            if (n.channels && n.channels.size > 0) {
              for (const [key, value] of n.channels.entries()) {
                if (Array.isArray(key) && key.length === 2 && key[0] === prog_state.pid) {
                  if (!has_prog_channels) {
                    program_channels_output += '\n    *Channels:*\n';
                    has_prog_channels = true;
                  }
                  const channelName = key[1];
                  program_channels_output += `      ${channelName} <- ${
                    Array.isArray(value) ? value.map(l => literal(l)).join(',') : String(value)
                  }`;
                }
              }
            }
            if (!has_prog_channels) {
              program_channels_output = '\n    *Channels:*\n      _No active input channels._';
            }
            
            return program_details + program_channels_output;
          }
        ).join('\n\n')
      : '  _No running programs_'
  );
  
  const result = globals_label + locals_and_channels_label;
  return result;
};