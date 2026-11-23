use fastrand;
use serde::Serialize;
use serde_wasm_bindgen;
use std::collections::HashMap;
use std::path::Path;
use wasm_bindgen::prelude::*;

use althread::ast::token::literal::Literal;
use althread::compiler::CompiledProject;
use althread::module_resolver::VirtualFileSystem;
use althread::vm::{instruction::Instruction, VM};
use althread::{checker, error::AlthreadError, vm::GlobalAction};
use console_error_panic_hook;

mod types;
use types::*;

const SEND: u8 = b's';
const RECV: u8 = b'r';
const WEB_RUN_MAX_STEPS: usize = 1_000;

/// Helper to serialize with json_compatible mode (no Maps, plain objects)
fn to_js<T: Serialize>(value: &T) -> JsValue {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap()
}

fn message_payload_string(message: &Literal) -> String {
    althread::vm::channels::parse_message_tuple(message)
        .map(|(_, _, content)| content)
        .unwrap_or_else(|| message.to_string())
}

fn error_to_js(err: AlthreadError) -> JsValue {
    to_js(&runtime_error_info(err))
}

fn web_pos_from_rc(pos: &std::rc::Rc<althread::error::Pos>) -> WebPos {
    WebPos {
        line: pos.line(),
        col: pos.column(),
        start: pos.start,
        end: pos.end,
        file_path: pos.file_path.clone(),
    }
}

fn runtime_error_info(err: AlthreadError) -> RuntimeErrorInfo {
    RuntimeErrorInfo {
        pos: err.pos.as_ref().map(web_pos_from_rc),
        message: err.message,
        error_type: format!("{:?}", err.error_type),
        stack: err.stack.iter().map(web_pos_from_rc).collect(),
    }
}

fn parse_virtual_fs(virtual_fs: JsValue) -> Result<HashMap<String, String>, JsValue> {
    serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))
}

fn compile_source(
    source: &str,
    filepath: &str,
    fs_map: HashMap<String, String>,
) -> Result<CompiledProject, AlthreadError> {
    let virtual_filesystem = VirtualFileSystem::new(fs_map);
    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    let ast = althread::parser::parse_ast(source, filepath)?;
    ast.compile(Path::new(filepath), virtual_filesystem, &mut input_map)
}

// Convert a VM Literal to a typed web Literal
fn value_to_literal(value: &althread::ast::token::literal::Literal) -> types::Literal {
    use althread::ast::token::literal::Literal as VmLiteral;

    match value {
        VmLiteral::Null => types::Literal::Null,
        VmLiteral::Int(n) => types::Literal::Int(*n),
        VmLiteral::Float(f) => types::Literal::Float(f.into_inner()),
        VmLiteral::String(s) => types::Literal::String(s.clone()),
        VmLiteral::Bool(b) => types::Literal::Bool(*b),
        VmLiteral::List(_, items) => {
            types::Literal::List(items.iter().map(value_to_literal).collect())
        }
        VmLiteral::Tuple(items) => {
            types::Literal::Tuple(items.iter().map(value_to_literal).collect())
        }
        VmLiteral::Process(name, id) => types::Literal::Process(name.clone(), *id),
    }
}

// Helper function to create typed VM state from VM
fn create_vm_state(vm: &VM) -> VMState {
    let current_state = vm.current_state();

    let globals = current_state
        .0
        .iter()
        .map(|(key, value)| (key.clone(), value_to_literal(value)))
        .collect();

    let channels = current_state
        .1
        .iter()
        .map(|((pid, name), values)| ChannelState {
            pid: *pid,
            name: name.clone(),
            values: values.iter().map(value_to_literal).collect(),
        })
        .collect();

    let pending_deliveries = vm
        .channels
        .get_pending_deliveries()
        .iter()
        .map(|((f_pid, f_chan, t_pid, t_chan), values)| PendingDelivery {
            from_pid: *f_pid,
            from_channel: f_chan.clone(),
            to_pid: *t_pid,
            to_channel: t_chan.clone(),
            values: values.iter().map(value_to_literal).collect(),
        })
        .collect();

    let waiting_send = vm
        .channels
        .get_waiting_send()
        .iter()
        .map(|((pid, name), values)| WaitingSend {
            pid: *pid,
            name: name.clone(),
            values: values.iter().map(value_to_literal).collect(),
        })
        .collect();

    let channel_connections = vm
        .channels
        .get_connections()
        .iter()
        .map(
            |((from_pid, from_channel), (to_pid, to_channel))| ChannelConnection {
                from: ChannelEndpoint {
                    pid: *from_pid,
                    channel: from_channel.clone(),
                },
                to: ChannelEndpoint {
                    pid: *to_pid,
                    channel: to_channel.clone(),
                },
            },
        )
        .collect();

    let locals = current_state
        .2
        .iter()
        .enumerate()
        .map(|(index, (memory, instruction_pointer, clock))| {
            let prog_name = vm
                .running_programs
                .get(index)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("PID_{}", index));

            let line = vm
                .programs_code
                .get(&prog_name)
                .and_then(|code| code.instructions.get(*instruction_pointer))
                .and_then(|inst| inst.pos.as_ref())
                .map(|pos| pos.line());

            let debug_info = vm.program_debug_info.get(&prog_name);
            let call_stack_info = vm
                .running_programs
                .get(index)
                .map(|p| p.get_call_stack_info())
                .unwrap_or_default();

            // Build frames information with named variables
            let mut frames = Vec::new();

            // Current frame (top of call stack)
            if let Some((fp, ip, pos)) = call_stack_info.first() {
                let mut variables = HashMap::new();

                // Add variables for the current frame
                if let Some(debug) = debug_info {
                    for var_info in &debug.local_variables {
                        // Check if variable is in scope at current instruction pointer
                        if var_info.scope_start_ip <= *ip
                            && var_info.scope_end_ip.map_or(true, |end| *ip < end)
                        {
                            // Get the variable value from memory
                            if var_info.stack_index < memory.len() {
                                variables.insert(
                                    var_info.name.clone(),
                                    VariableInfo {
                                        value: value_to_literal(&memory[var_info.stack_index]),
                                        var_type: format!("{:?}", var_info.datatype),
                                    },
                                );
                            }
                        }
                    }
                }

                frames.push(CallFrame {
                    function: prog_name.clone(),
                    frame_pointer: *fp,
                    instruction_pointer: *ip,
                    line: pos.as_ref().map(|p| p.line()),
                    variables,
                });
            }

            ProgramState {
                pid: index,
                name: prog_name,
                memory: memory.iter().map(value_to_literal).collect(),
                instruction_pointer: *instruction_pointer,
                line,
                clock: *clock,
                frames,
            }
        })
        .collect();

    VMState {
        globals,
        channels,
        pending_deliveries,
        waiting_send,
        channel_connections,
        locals,
    }
}

fn collect_step_actions(
    actions: &[GlobalAction],
    vm: &VM,
    lines: &[usize],
    stdout: &mut Vec<String>,
    message_flow_events: &mut Vec<MessageFlowEvent>,
) {
    let connections = vm.channels.get_connections();
    let mut cached_vm_state: Option<VMState> = None;

    for action in actions {
        match action {
            GlobalAction::Print(s_print) => stdout.push(s_print.clone()),
            GlobalAction::Send(info) => {
                let receiver_id = connections
                    .get(&(info.from.process_id, info.to.channel_name.clone()))
                    .map(|(pid, _chan)| *pid)
                    .unwrap_or(0);
                let vm_state = cached_vm_state
                    .get_or_insert_with(|| create_vm_state(vm))
                    .clone();

                message_flow_events.push(MessageFlowEvent {
                    sender: info.from.process_id,
                    receiver: receiver_id,
                    evt_type: SEND,
                    message: info.to.channel_name.clone(),
                    number: info.n_msg,
                    actor_prog_name: info.from.process_name.clone(),
                    vm_state,
                    lines: lines.to_vec(),
                });
            }
            GlobalAction::Broadcast(send_infos) => {
                for send_info in send_infos {
                    let receiver_id = connections
                        .get(&(send_info.from.process_id, send_info.to.channel_name.clone()))
                        .map(|(pid, _chan)| *pid)
                        .unwrap_or(0);
                    let vm_state = cached_vm_state
                        .get_or_insert_with(|| create_vm_state(vm))
                        .clone();

                    message_flow_events.push(MessageFlowEvent {
                        sender: send_info.from.process_id,
                        receiver: receiver_id,
                        evt_type: SEND,
                        message: send_info.to.channel_name.clone(),
                        number: send_info.n_msg,
                        actor_prog_name: send_info.from.process_name.clone(),
                        vm_state,
                        lines: lines.to_vec(),
                    });
                }
            }
            GlobalAction::Deliver(info) => {
                let vm_state = cached_vm_state
                    .get_or_insert_with(|| create_vm_state(vm))
                    .clone();

                message_flow_events.push(MessageFlowEvent {
                    sender: info.from.process_id,
                    receiver: info.to.process_id,
                    evt_type: RECV,
                    message: message_payload_string(&info.message),
                    number: info.sender_clock,
                    actor_prog_name: info.to.process_name.clone(),
                    vm_state,
                    lines: lines.to_vec(),
                });
            }
            _ => {}
        }
    }
}

fn format_state_display(vm: &VM) -> Vec<String> {
    let current_state = vm.current_state();
    let mut state_display_info = Vec::new();
    state_display_info.push(format!("global: {:?}", current_state.0));
    for ((channel_pid, cname), state) in current_state.1.iter() {
        state_display_info.push(format!("channel {},{}", channel_pid, cname));
        for v in state.iter() {
            state_display_info.push(format!("  * {}", v));
        }
    }
    for (local_pid, local_state) in current_state.2.iter().enumerate() {
        state_display_info.push(format!(
            "{} ({}): {:?}",
            local_pid,
            local_state.1,
            local_state
                .0
                .iter()
                .map(|v| format!("{}", v))
                .collect::<Vec<String>>()
                .join(", ")
        ));
    }
    state_display_info
}

fn build_next_state_options(
    next_states: &[(String, usize, Vec<Instruction>, Vec<GlobalAction>, VM<'_>)],
) -> Vec<NextStateOption> {
    next_states
        .iter()
        .map(|(name, pid, instructions, actions, _nvm)| {
            let lines: Vec<usize> = instructions
                .iter()
                .filter_map(|inst| inst.pos.as_ref().map(|p| p.line()))
                .collect();

            let instruction_strings: Vec<String> = if instructions.is_empty() {
                let delivery = actions.iter().find_map(|action| match action {
                    GlobalAction::Deliver(info) => Some(format!(
                        "DELIVER {} <- {}#{} : {}",
                        info.channel_name,
                        info.from.process_name,
                        info.from.process_id,
                        message_payload_string(&info.message)
                    )),
                    _ => None,
                });
                delivery.into_iter().collect()
            } else {
                instructions.iter().map(|inst| inst.to_string()).collect()
            };

            NextStateOption {
                prog_name: name.clone(),
                prog_id: *pid,
                instructions: instruction_strings,
                lines,
            }
        })
        .collect()
}

fn run_core(
    source: &str,
    filepath: &str,
    fs_map: HashMap<String, String>,
    seed: u64,
) -> Result<RunResult, AlthreadError> {
    let compiled_project = compile_source(source, filepath, fs_map)?;

    let mut vm = VM::new(&compiled_project);
    vm.start(seed);

    let mut result = String::new();
    let mut stdout = vec![];
    let mut message_flow_graph = Vec::new();
    let initial_vm_state = create_vm_state(&vm);
    let mut nodes = vec![GraphNode {
        vm: initial_vm_state,
        metadata: NodeMetadata {
            level: 0,
            step_index: Some(0),
            successors: None,
            lines: None,
        },
    }];
    let mut i = 0; // index for nodes
    let mut runtime_error = None;
    let mut max_steps_reached = false;

    for _ in 0..WEB_RUN_MAX_STEPS {
        if vm.is_finished() {
            break;
        }
        let info = match vm.next_random() {
            Ok(info) => info,
            Err(err) => {
                runtime_error = Some(runtime_error_info(err));
                break;
            }
        };

        let lines: Vec<usize> = info
            .instructions
            .iter()
            .filter_map(|inst| inst.pos.as_ref().map(|p| p.line()))
            .collect();

        let new_vm_state = create_vm_state(&vm);
        nodes.push(GraphNode {
            vm: new_vm_state,
            metadata: NodeMetadata {
                level: i + 1,
                step_index: Some(i + 1),
                successors: None,
                lines: Some(lines.clone()),
            },
        });

        for inst in info.instructions.iter() {
            result.push_str(&format!("#{}: {}\n", info.prog_id, inst));
        }

        collect_step_actions(
            &info.actions,
            &vm,
            &lines,
            &mut stdout,
            &mut message_flow_graph,
        );

        if let Err(err) = info.invariant_error {
            let line = err.pos.as_ref().map_or(0, |pos| pos.line());
            result.push_str(&format!("Invariant error at line {}: {}\n", line, err.message));
            runtime_error = Some(runtime_error_info(err));
            break;
        }
        i += 1;
    }

    if !vm.is_finished() && runtime_error.is_none() {
        max_steps_reached = true;
    }

    // Collect step_lines from nodes
    let step_lines: Vec<Vec<usize>> = nodes
        .iter()
        .filter_map(|node| node.metadata.lines.clone())
        .collect();

    Ok(RunResult {
        debug: result,
        stdout,
        message_flow_events: message_flow_graph,
        nodes,
        step_lines,
        max_steps_reached,
        runtime_error,
    })
}

fn check_core(
    source: &str,
    filepath: &str,
    fs_map: HashMap<String, String>,
    max_states: Option<usize>,
) -> Result<CheckResult, AlthreadError> {
    const WEB_GRAPH_DETAILS_THRESHOLD: usize = 200;

    let compiled_project = compile_source(source, filepath, fs_map)?;

    let checked = checker::check(&compiled_project, max_states)?;
    let state_graph = checked.graph;
    let omit_transition_details = state_graph.nodes.len() > WEB_GRAPH_DETAILS_THRESHOLD;

    let mut path_nodes = Vec::new();
    let mut violation_path = Vec::new();
    let mut path_stdout = Vec::new();
    let mut path_debug = String::new();
    let mut path_message_flow_events = Vec::new();

    let (verdict, cycle_start) = match checked.verdict {
        checker::Verdict::Verified => (CheckVerdict::Verified, None),
        checker::Verdict::Inconclusive => (CheckVerdict::Inconclusive, None),
        checker::Verdict::Violated { path, cycle_start } => {
            let initial = state_graph.initial_state;
            violation_path.push(initial);
            if !path[0].is_initial_marker() {
                path_nodes.push(GraphNode {
                    vm: create_vm_state(state_graph.vm(initial)),
                    metadata: NodeMetadata {
                        level: 0,
                        step_index: None,
                        successors: None,
                        lines: None,
                    },
                });
            }
            state_graph.replay_path(&path, |link, vm, actions| {
                violation_path.push(link.to);
                for inst in &link.instructions {
                    path_debug.push_str(&format!("#{}: {}\n", link.pid, inst));
                }
                collect_step_actions(
                    actions,
                    vm,
                    &link.lines,
                    &mut path_stdout,
                    &mut path_message_flow_events,
                );
                path_nodes.push(GraphNode {
                    vm: create_vm_state(vm),
                    metadata: NodeMetadata {
                        level: path_nodes.len(),
                        step_index: None,
                        successors: None,
                        lines: Some(link.lines.clone()),
                    },
                });
            })?;
            (CheckVerdict::Violated, cycle_start)
        }
    };

    // Convert state graph nodes to GraphNode structure
    let graph_nodes: Vec<GraphNode> = state_graph
        .nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let successors = if omit_transition_details {
                None
            } else {
                Some(
                    node.successors
                        .iter()
                        .map(|succ| Successor {
                            to_index: succ.to,
                            lines: succ.lines.clone(),
                            instructions: succ
                                .instructions
                                .iter()
                                .map(|i| format!("{:?}", i))
                                .collect(),
                            actions: succ.actions.iter().map(|a| format!("{:?}", a)).collect(),
                            pid: succ.pid,
                            name: succ.name.clone(),
                        })
                        .collect(),
                )
            };

            GraphNode {
                vm: create_vm_state(state_graph.vm(idx)),
                metadata: NodeMetadata {
                    level: node.level,
                    step_index: None,
                    successors,
                    lines: None,
                },
            }
        })
        .collect();

    Ok(CheckResult {
        verdict,
        violation_path,
        path: path_nodes,
        path_stdout,
        path_debug,
        path_message_flow_events,
        nodes: graph_nodes,
        exhaustive: state_graph.exhaustive,
        violation_cycle_start: cycle_start,
    })
}

enum InteractiveError {
    Althread(AlthreadError),
    Message(String),
}

impl From<AlthreadError> for InteractiveError {
    fn from(err: AlthreadError) -> Self {
        Self::Althread(err)
    }
}

impl InteractiveError {
    fn into_js(self) -> JsValue {
        match self {
            Self::Althread(err) => error_to_js(err),
            Self::Message(msg) => JsValue::from_str(&msg),
        }
    }
}

fn replay_interactive_history<'a>(
    vm: &mut VM<'a>,
    history: &[usize],
) -> Result<(), InteractiveError> {
    for &selected_index in history {
        let next_states = vm.next()?;
        if next_states.is_empty() {
            return Err(InteractiveError::Message(
                "No next state during replay".to_string(),
            ));
        }
        if selected_index >= next_states.len() {
            return Err(InteractiveError::Message(format!(
                "Invalid selection index {} in history",
                selected_index
            )));
        }
        let (_, _, _, _, new_vm) = next_states.into_iter().nth(selected_index).unwrap();
        *vm = new_vm;
    }
    Ok(())
}

fn start_interactive_core(
    source: &str,
    filepath: &str,
    fs_map: HashMap<String, String>,
) -> Result<InteractiveSessionState, AlthreadError> {
    let compiled_project = compile_source(source, filepath, fs_map)?;

    let mut vm = VM::new(&compiled_project);
    vm.start(0); // Use deterministic seed for interactive mode

    let next_states = vm.next()?;
    if next_states.is_empty() && !vm.is_finished() {
        return Err(vm.deadlock_error());
    }

    let next_state_options = build_next_state_options(&next_states);

    Ok(InteractiveSessionState {
        next_states: next_state_options,
        current_state: create_vm_state(&vm),
        state_display: vec![],
        output: vec![],
    })
}

fn get_next_interactive_core(
    source: &str,
    filepath: &str,
    fs_map: HashMap<String, String>,
    history: &[usize],
) -> Result<InteractiveSessionState, InteractiveError> {
    let compiled_project = compile_source(source, filepath, fs_map)?;

    let mut vm = VM::new(&compiled_project);
    vm.start(0);
    replay_interactive_history(&mut vm, history)?;

    let next_states = vm.next()?;
    if next_states.is_empty() && !vm.is_finished() {
        return Err(InteractiveError::Althread(vm.deadlock_error()));
    }

    let next_state_options = build_next_state_options(&next_states);

    Ok(InteractiveSessionState {
        next_states: next_state_options,
        current_state: create_vm_state(&vm),
        state_display: format_state_display(&vm),
        output: vec![],
    })
}

fn execute_interactive_step_core(
    source: &str,
    filepath: &str,
    fs_map: HashMap<String, String>,
    history: &[usize],
    selected_index: usize,
) -> Result<InteractiveStepResult, InteractiveError> {
    let compiled_project = compile_source(source, filepath, fs_map)?;

    let mut vm = VM::new(&compiled_project);
    vm.start(0);
    replay_interactive_history(&mut vm, history)?;

    let next_states = vm.next()?;
    if selected_index >= next_states.len() {
        return Err(InteractiveError::Message(format!(
            "Invalid selection index {}",
            selected_index
        )));
    }

    let (name, pid, instructions, actions, execution_vm) =
        next_states.into_iter().nth(selected_index).unwrap();

    let mut message_flow_events: Vec<MessageFlowEvent> = Vec::new();
    let mut step_output: Vec<String> = Vec::new();
    let mut step_debug = String::new();

    let step_lines: Vec<usize> = instructions
        .iter()
        .filter_map(|inst| inst.pos.as_ref().map(|p| p.line()))
        .collect();

    for inst in &instructions {
        step_debug.push_str(&format!("#{}: {}\n", pid, inst));
    }

    collect_step_actions(
        &actions,
        &execution_vm,
        &step_lines,
        &mut step_output,
        &mut message_flow_events,
    );

    let executed_instructions = if instructions.is_empty() && name.starts_with("__deliver__") {
        let delivery = message_flow_events.iter().find_map(|action| {
            (action.evt_type == RECV).then(|| {
                format!(
                    "deliver {} <- {} : {}",
                    action.receiver, action.sender, action.message
                )
            })
        });
        vec![delivery.unwrap_or_else(|| "deliver <unknown>".to_string())]
    } else {
        instructions.iter().map(|inst| inst.to_string()).collect()
    };

    let (following_states, deadlock) = match execution_vm.next() {
        Ok(states) if states.is_empty() && !execution_vm.is_finished() => {
            (vec![], Some(runtime_error_info(execution_vm.deadlock_error())))
        }
        Ok(states) => (build_next_state_options(&states), None),
        Err(err) => return Err(InteractiveError::Althread(err)),
    };

    Ok(InteractiveStepResult {
        executed_step: ExecutedStepInfo {
            prog_name: name,
            prog_id: pid,
            instructions: executed_instructions,
        },
        output: step_output,
        debug: step_debug,
        new_state: create_vm_state(&execution_vm),
        message_flow_events,
        state_display: format_state_display(&execution_vm),
        lines: step_lines,
        next_states: following_states,
        deadlock,
    })
}

#[wasm_bindgen]
pub fn initialize() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn compile(source: &str, file_path: &str, virtual_fs: JsValue) -> Result<String, JsValue> {
    let fs_map = parse_virtual_fs(virtual_fs)?;
    let compiled_project = compile_source(source, file_path, fs_map).map_err(error_to_js)?;
    Ok(format!("{}", compiled_project))
}

#[wasm_bindgen]
pub fn run(source: &str, filepath: &str, virtual_fs: JsValue) -> Result<JsValue, JsValue> {
    let fs_map = parse_virtual_fs(virtual_fs)?;
    let seed = fastrand::u64(0..(1 << 32));
    let result = run_core(source, filepath, fs_map, seed).map_err(error_to_js)?;
    Ok(to_js(&result))
}

#[wasm_bindgen]
pub fn check(
    source: &str,
    filepath: &str,
    virtual_fs: JsValue,
    max_states: Option<usize>,
) -> Result<JsValue, JsValue> {
    let fs_map = parse_virtual_fs(virtual_fs)?;
    let result = check_core(source, filepath, fs_map, max_states).map_err(error_to_js)?;
    Ok(to_js(&result))
}

// Package management utilities for web editor
#[wasm_bindgen]
pub fn create_alt_toml(package_name: &str, version: &str) -> String {
    format!(
        r#"[package]
name = "{}"
version = "{}"

[dependencies]

[dev-dependencies]

"#,
        package_name, version
    )
}

#[wasm_bindgen]
pub fn add_dependency_to_toml(
    toml_content: &str,
    package_name: &str,
    version: &str,
) -> Result<String, JsValue> {
    let mut lines: Vec<&str> = toml_content.lines().collect();
    let mut dependencies_section_found = false;
    let mut insert_index = lines.len();

    // Find the [dependencies] section
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "[dependencies]" {
            dependencies_section_found = true;
            // Find where to insert (before next section or at end)
            for j in (i + 1)..lines.len() {
                if lines[j].trim().starts_with('[') {
                    insert_index = j;
                    break;
                }
            }
            break;
        }
    }

    if !dependencies_section_found {
        return Err(JsValue::from_str(
            "No [dependencies] section found in alt.toml",
        ));
    }

    // Create the new dependency line
    let new_dependency = format!(r#""{}" = "{}""#, package_name, version);

    // Insert the new dependency
    lines.insert(insert_index, &new_dependency);

    Ok(lines.join("\n"))
}

#[wasm_bindgen]
pub fn parse_dependencies_from_toml(toml_content: &str) -> Result<JsValue, JsValue> {
    let mut dependencies = std::collections::HashMap::new();
    let lines: Vec<&str> = toml_content.lines().collect();
    let mut in_dependencies_section = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed == "[dependencies]" {
            in_dependencies_section = true;
            continue;
        }

        if trimmed.starts_with('[') && trimmed != "[dependencies]" {
            in_dependencies_section = false;
            continue;
        }

        if in_dependencies_section && trimmed.contains('=') {
            // Parse dependency line: "package.name" = "version"
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                let package_name = parts[0].trim().trim_matches('"').trim();
                let version = parts[1].trim().trim_matches('"').trim();
                dependencies.insert(package_name.to_string(), version.to_string());
            }
        }
    }

    serde_wasm_bindgen::to_value(&dependencies)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize dependencies: {}", e)))
}

#[wasm_bindgen]
pub fn validate_package_name(package_name: &str) -> bool {
    // Basic validation for package names like github.com/user/repo
    if package_name.contains('/') {
        let parts: Vec<&str> = package_name.split('/').collect();
        return parts.len() >= 3 && !parts.iter().any(|p| p.is_empty());
    }
    // Allow simple names too
    !package_name.is_empty() && !package_name.contains(' ')
}

// Interactive mode functionality - replay-based approach
#[wasm_bindgen]
pub fn start_interactive_session(
    source: &str,
    filepath: &str,
    virtual_fs: JsValue,
) -> Result<JsValue, JsValue> {
    let fs_map = parse_virtual_fs(virtual_fs)?;
    let result = start_interactive_core(source, filepath, fs_map).map_err(error_to_js)?;
    Ok(to_js(&result))
}

#[wasm_bindgen]
pub fn get_next_interactive_states(
    source: &str,
    filepath: &str,
    virtual_fs: JsValue,
    execution_history: JsValue,
) -> Result<JsValue, JsValue> {
    let fs_map = parse_virtual_fs(virtual_fs)?;
    let history: Vec<usize> = serde_wasm_bindgen::from_value(execution_history)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse execution history: {}", e)))?;

    let result = get_next_interactive_core(source, filepath, fs_map, &history)
        .map_err(InteractiveError::into_js)?;
    Ok(to_js(&result))
}

#[wasm_bindgen]
pub fn execute_interactive_step(
    source: &str,
    filepath: &str,
    virtual_fs: JsValue,
    execution_history: JsValue,
    selected_index: usize,
) -> Result<JsValue, JsValue> {
    let fs_map = parse_virtual_fs(virtual_fs)?;
    let history: Vec<usize> = serde_wasm_bindgen::from_value(execution_history)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse execution history: {}", e)))?;

    let result = execute_interactive_step_core(source, filepath, fs_map, &history, selected_index)
        .map_err(InteractiveError::into_js)?;
    Ok(to_js(&result))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fs_for(source: &str) -> HashMap<String, String> {
        HashMap::from([("main.alt".to_string(), source.to_string())])
    }

    #[test]
    fn run_reports_invariant_violation_as_runtime_error_not_step_limit() {
        let source = r#"
shared { let X = 0; }
always { X == 0; }
main {
    X = 1;
    X = 2;
}
"#;
        let res = run_core(source, "main.alt", fs_for(source), 0).unwrap();
        assert!(!res.max_steps_reached);
        let err = res
            .runtime_error
            .expect("invariant violation should populate runtime_error");
        assert_eq!(err.error_type, "InvariantError");
        assert!(err.message.contains("invariant"));
    }

    #[test]
    fn check_counterexample_collects_message_flow_and_stdout() {
        let source = r#"
shared { let Done = false; }
program Worker() {
    await receive inp(v) => {
        print("got", v);
        Done = true;
    }
}
main {
    let w = run Worker();
    channel self.out (int)> w.inp;
    send out(42);
}
always { !Done; }
"#;
        let res = check_core(source, "main.alt", fs_for(source), Some(200)).unwrap();
        assert!(matches!(res.verdict, CheckVerdict::Violated));
        assert!(res.path_stdout.iter().any(|line| line.contains("got 42")));
        assert!(res
            .path_message_flow_events
            .iter()
            .any(|evt| evt.evt_type == SEND));
        assert!(res
            .path_message_flow_events
            .iter()
            .any(|evt| evt.evt_type == RECV));
        assert!(!res.path_debug.is_empty());
    }

    #[test]
    fn interactive_step_finishes_cleanly_without_error() {
        let source = "main { print(1); }";
        let initial = start_interactive_core(source, "main.alt", fs_for(source)).unwrap();
        assert!(!initial.next_states.is_empty());

        let mut history = Vec::new();
        let mut last_next = initial.next_states;
        while !last_next.is_empty() {
            let step = execute_interactive_step_core(source, "main.alt", fs_for(source), &history, 0)
                .unwrap_or_else(|_| panic!("step failed"));
            assert!(step.deadlock.is_none());
            history.push(0);
            last_next = step.next_states;
        }

        let after = get_next_interactive_core(source, "main.alt", fs_for(source), &history)
            .unwrap_or_else(|_| panic!("get_next_interactive_core should succeed at completion"));
        assert!(after.next_states.is_empty());
    }

    #[test]
    fn interactive_mode_detects_deadlock_and_describes_deliveries() {
        let deadlock_source = r#"
shared { let Ready = false; }
main {
    await Ready;
}
"#;
        let err = start_interactive_core(deadlock_source, "main.alt", fs_for(deadlock_source))
            .err()
            .expect("initial deadlock should be reported");
        assert!(err.message.to_lowercase().contains("deadlock"));

        let msg_source = r#"
program Worker() {
    await receive inp(v) => { print(v); }
}
main {
    let w = run Worker();
    channel self.out (int)> w.inp;
    send out(7);
}
"#;
        let mut history = Vec::new();
        let mut saw_delivery_choice = false;
        let mut next = start_interactive_core(msg_source, "main.alt", fs_for(msg_source))
            .unwrap()
            .next_states;
        for _ in 0..10 {
            if let Some(choice) = next
                .iter()
                .find(|c| c.prog_name.starts_with("__deliver__"))
            {
                assert!(choice
                    .instructions
                    .iter()
                    .any(|inst| inst.starts_with("DELIVER ")));
                saw_delivery_choice = true;
                break;
            }
            if next.is_empty() {
                break;
            }
            let step =
                execute_interactive_step_core(msg_source, "main.alt", fs_for(msg_source), &history, 0)
                    .unwrap_or_else(|_| panic!("step failed"));
            history.push(0);
            next = step.next_states;
        }
        assert!(saw_delivery_choice);
    }
}
