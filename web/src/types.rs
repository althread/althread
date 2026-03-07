use serde::Serialize;
use std::collections::HashMap;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct WebPos {
    pub line: usize,
    pub col: usize,
    pub start: usize,
    pub end: usize,
    pub file_path: String,
}

#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct RuntimeErrorInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<WebPos>,
    pub message: String,
    pub error_type: String,
    pub stack: Vec<WebPos>,
}

/// Represents a literal value with its type and content
#[derive(Serialize, Tsify, Clone, Debug)]
#[tsify(into_wasm_abi)]
#[serde(tag = "type", content = "value")]
pub enum Literal {
    Null,
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    List(Vec<Literal>),
    Tuple(Vec<Literal>),
    Process(String, usize),
}

/// Represents a variable in a call frame
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct VariableInfo {
    pub value: Literal,
    #[serde(rename = "type")]
    pub var_type: String,
}

/// Represents a call frame in a program's execution stack
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct CallFrame {
    pub function: String,
    pub frame_pointer: usize,
    pub instruction_pointer: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[tsify(type = "Record<string, VariableInfo>")]
    pub variables: HashMap<String, VariableInfo>,
}

/// Represents the state of a single program/process
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct ProgramState {
    pub pid: usize,
    pub name: String,
    pub instruction_pointer: usize,
    pub memory: Vec<Literal>,
    pub clock: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub frames: Vec<CallFrame>,
}

/// Represents a channel with its buffered values
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct ChannelState {
    pub pid: usize,
    pub name: String,
    pub values: Vec<Literal>,
}

/// Represents a message pending delivery between channels
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct PendingDelivery {
    pub from_pid: usize,
    pub from_channel: String,
    pub to_pid: usize,
    pub to_channel: String,
    pub values: Vec<Literal>,
}

/// Represents a message waiting to be sent (unconnected channel)
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct WaitingSend {
    pub pid: usize,
    pub name: String,
    pub values: Vec<Literal>,
}

/// Represents a connection between two channels
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct ChannelConnection {
    pub from: ChannelEndpoint,
    pub to: ChannelEndpoint,
}

#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct ChannelEndpoint {
    pub pid: usize,
    pub channel: String,
}

/// Complete VM state snapshot
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct VMState {
    #[tsify(type = "Record<string, Literal>")]
    pub globals: HashMap<String, Literal>,
    pub channels: Vec<ChannelState>,
    pub pending_deliveries: Vec<PendingDelivery>,
    pub waiting_send: Vec<WaitingSend>,
    pub channel_connections: Vec<ChannelConnection>,
    pub locals: Vec<ProgramState>,
}

/// Metadata about a node in the state graph
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct NodeMetadata {
    pub level: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successors: Option<Vec<Successor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<usize>>,
}

/// Represents a transition in the state graph
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct Successor {
    pub to_index: usize,
    pub lines: Vec<usize>,
    pub instructions: Vec<String>,
    pub actions: Vec<String>,
    pub pid: usize,
    pub name: String,
}

/// Unified graph node structure for both run and check modes
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct GraphNode {
    pub vm: VMState,
    pub metadata: NodeMetadata,
}

/// Information about an executed step in interactive mode
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct ExecutedStepInfo {
    pub prog_name: String,
    pub prog_id: usize,
    pub instructions: Vec<String>,
}

/// A message flow event for visualization
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct MessageFlowEvent {
    pub sender: usize,
    pub receiver: usize,
    pub evt_type: u8,
    pub message: String,
    pub number: usize,
    pub actor_prog_name: String,
    pub vm_state: VMState,
    pub lines: Vec<usize>,
}

/// Result from running a program
#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct RunResult {
    pub debug: String,
    pub stdout: Vec<String>,
    pub message_flow_events: Vec<MessageFlowEvent>,
    pub nodes: Vec<GraphNode>,
    pub step_lines: Vec<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_error: Option<RuntimeErrorInfo>,
}

/// Result from checking a program
#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct CheckResult {
    pub path: Vec<GraphNode>,
    pub nodes: Vec<GraphNode>,
    pub exhaustive: bool,
}

/// Result from an interactive step execution
#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct InteractiveStepResult {
    pub executed_step: ExecutedStepInfo,
    pub output: Vec<String>,
    pub debug: String,
    pub new_state: VMState,
    pub message_flow_events: Vec<MessageFlowEvent>,
    pub state_display: Vec<String>,
    pub lines: Vec<usize>,
}

/// Interactive session state information
#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi)]
pub struct InteractiveSessionState {
    pub next_states: Vec<NextStateOption>,
    pub current_state: VMState,
    pub state_display: Vec<String>,
    pub output: Vec<String>,
}

/// Option for next state in interactive mode
#[derive(Serialize, Tsify, Clone)]
#[tsify(into_wasm_abi)]
pub struct NextStateOption {
    pub prog_name: String,
    pub prog_id: usize,
    pub instructions: Vec<String>,
    pub lines: Vec<usize>,
}
