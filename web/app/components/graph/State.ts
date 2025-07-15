export type ProgramStateJS = {
  pid: number;
  name: string;
  memory: any[]; // This will be the array of serialized Literal objects (the stack)
  instruction_pointer: number;
  clock: number; // Assuming 'clock' is also part of the serialized program state
};