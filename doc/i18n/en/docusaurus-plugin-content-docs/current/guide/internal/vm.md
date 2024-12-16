---
sidebar_position: 2
---

# Althread Virtual Machine

## Virtual Machine State

The state of the virtual machine is represented by:
- The values of global variables
- The state of communication channels (messages in transit)
- A structure for each process in execution. The state of an executing process includes its execution stack, i.e., the process's local variables, and the value of the instruction pointer, which is the index of the instruction currently being executed.

The execution stack of a process contains no debugging information; it only includes the values of local variables and intermediate expression results in the form of a `Literal` array. Expressions using local variables refer to the index of the variable in the process's execution stack. The index of a local variable is determined during compilation, enabling fast access during execution.

To simplify, global variables are stored in a HashMap (dictionary), and their values are directly accessible by name.

## Instructions

The virtual machine's instructions are represented by an enum `InstructionType`. Each instruction contains fields with the information required for its execution:

export const InstructionType = ({children, color, code, args}) => (
    <div id={code}>
        <div
            style={{
                fontFamily: "var(--ifm-font-family-monospace)",
                fontSize: "var(--ifm-code-font-size)",
                color: "#008cdf",
            }}><span
                style={{
                    color: "#008cdf",
                }}>
                    {code}
                </span>{' '}
                <span
                style={{
                    color: "rgb(174, 0, 223)",
                }}>
                    {args}
                </span>
            </div>
        <div
        style={{
            borderRadius: '2px',
            padding: "0 15px",
        }}>
        {children}
        </div>
    </div>
);

### Instruction Types

<table>
<tr><td>
- [Empty](#Empty)
- [Expression](#Expression)
- [Push](#Push)
- [Unstack](#Unstack)
- [Destruct](#Destruct)
- [GlobalReads](#GlobalReads)
- [GlobalAssignment](#GlobalAssignment)
- [LocalAssignment](#LocalAssignment)
</td><td>
- [Declaration](#Declaration)
- [RunCall](#RunCall)
- [FnCall](#FnCall)
- [JumpIf](#JumpIf)
- [Jump](#Jump)
- [Break](#Break)
- [ChannelPeek](#ChannelPeek)
- [ChannelPop](#ChannelPop)
</td><td>
- [WaitStart](#WaitStart)
- [Wait](#Wait)
- [Send](#Send)
- [Connect](#Connect)
- [AtomicStart](#AtomicStart)
- [AtomicEnd](#AtomicEnd)
- [EndProgram](#EndProgram)
- [Exit](#Exit)
</td></tr>
</table>

<InstructionType code="Empty">
    An empty instruction, does nothing.
</InstructionType>

<InstructionType code="Expression" args="(LocalExpressionNode)">
    Evaluates an expression and pushes the result onto the stack. `LocalExpressionNode` is the root of a tree representing the expression.
</InstructionType>

<InstructionType code="Push" args="(Literal)">
Pushes the given literal onto the stack.
</InstructionType>

<InstructionType code="Unstack" args="{unstack_len: usize}">
Removes `unstack_len` values from the stack.
</InstructionType>

<InstructionType code="Destruct" args="">
Replaces the tuple at the top of the stack with its elements. A tuple with 3 elements will be replaced by 3 values on the stack.
</InstructionType>

<InstructionType code="GlobalReads" args="{variables: Vec<String>, only_const: bool}">
Pushes the values of the global variables onto the stack. The `only_const` field indicates whether all variables are constants (if so, the instruction can be optimized since it’s not global).
</InstructionType>

<InstructionType code="GlobalAssignment" args="{identifier: String, operator: BinaryAssignmentOperator, unstack_len: usize}">
Assigns the value at the top of the stack to the given global variable `identifier` and removes `unstack_len` values from the stack.
</InstructionType>

<InstructionType code="LocalAssignment" args="{index: usize, operator: BinaryAssignmentOperator, unstack_len: usize}">
Assigns the value at the top of the stack to the local variable at the given index `index` and removes `unstack_len` values from the stack.
</InstructionType>

<InstructionType code="Declaration" args="{unstack_len: usize}">
Declares a variable in the current scope, initialized with the value at the top of the stack, and removes `unstack_len` values from the stack.
</InstructionType>

<InstructionType code="RunCall" args="{name: String, unstack_len: usize}">
Starts a new thread executing the program `name` with the value at the top of the stack as an argument, then removes `unstack_len` values from the stack. Finally, adds the thread's PID to the stack.
</InstructionType>

<InstructionType code="FnCall" args="{name: String, unstack_len: usize, variable_idx: Option<usize>, arguments: Option<Vec<usize>}>">
Calls the function `name` with local arguments at the given indexes, then removes `unstack_len` values from the stack. If `variable_idx` is provided, the function is a method of the object at the given index. Finally, the function result is added to the stack, if it returns a value.
</InstructionType>

<InstructionType code="JumpIf" args="{jump_false: i64, unstack_len: usize}">
Jumps to instruction `jump_false` if the value at the top of the stack is false, then removes `unstack_len` values from the stack. The jump value is relative to the current instruction.
</InstructionType>

<InstructionType code="Jump" args="{jump: i64}">
Jumps to instruction `jump`. The jump value is relative to the current instruction.
</InstructionType>

<InstructionType code="Break" args="{jump: i64, unstack_len: usize, stop_atomic: bool}">
Jumps to instruction `jump` while removing `unstack_len` values from the stack. If `stop_atomic` is true, stops atomic execution.
</InstructionType>

<InstructionType code="ChannelPeek" args="{channel_name: String}">
Checks if a message is available in the channel `channel_name`. If so, adds the message and `true` to the stack; otherwise, adds `false`.
</InstructionType>

<InstructionType code="ChannelPop" args="{channel_name: String}">
Removes the message from the channel `channel_name` (does not add it to the stack).
</InstructionType>

<InstructionType code="WaitStart" args="{dependencies: WaitDependency, start_atomic: bool}">
Starts a wait on a condition using the given dependencies. If `start_atomic` is true, begins an atomic section. Dependencies include global variables or channels used in the condition. This instruction does not modify the virtual machine state and merely indicates the start of a waiting zone.
</InstructionType>

<InstructionType code="Wait" args="{jump: i64, unstack_len: usize}">
If the top of the stack is `false`, jumps to `jump` (relative to the next instruction); otherwise, proceeds to the next instruction. In both cases, removes `unstack_len` values from the stack.
</InstructionType>

<InstructionType code="Send" args="{channel_name: String, unstack_len: usize}">
Sends the value at the top of the stack to the channel `channel_name`, then removes `unstack_len` values from the stack.
</InstructionType>

<InstructionType code="Connect" args="{sender_pid: Option<usize>, receiver_pid: Option<usize>, sender_channel: String, receiver_channel: String}">
Connects `sender_channel` and `receiver_channel` between the processes `sender_pid` and `receiver_pid`. If `sender_pid` or `receiver_pid` is `None`, the current process is used. If a `Send` has already occurred, the message is directly transferred.
</InstructionType>

<InstructionType code="AtomicStart" args="">
Begins an atomic section where processes cannot be interrupted, preventing concurrency issues. An atomic section ends with `AtomicEnd` and must not contain wait instructions, except at the beginning.
</InstructionType>

<InstructionType code="AtomicEnd" args="">
Ends an atomic section.
</InstructionType>

<InstructionType code="EndProgram" args="">
Terminates the current process.
</InstructionType>

<InstructionType code="Exit" args="">
Terminates all processes.
</InstructionType>

The `InstructionType` enum is defined in the file [/vm/src/instruction.rs](https://github.com/althread/althread/blob/main/interpreter/src/vm/instruction.rs#L12).

