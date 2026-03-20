use fastrand;
use serde::Serialize;
use serde_wasm_bindgen;
use std::collections::HashMap;
use std::path::Path;
use wasm_bindgen::prelude::*;

use althread::ast::token::literal::Literal;
use althread::module_resolver::VirtualFileSystem;
use althread::vm::VM;
use althread::{checker, error::AlthreadError, vm::GlobalAction};
use console_error_panic_hook;

mod types;
use types::*;

const SEND: u8 = b's';
const RECV: u8 = b'r';

/// Helper to serialize with json_compatible mode (no Maps, plain objects)
fn to_js<T: Serialize>(value: &T) -> JsValue {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap()
}

fn find_delivered_message(
    prev_channels: &althread::vm::channels::ChannelsState,
    next_channels: &althread::vm::channels::ChannelsState,
) -> Option<((usize, String), Literal)> {
    for (key, next_state) in next_channels.iter() {
        let prev_len = prev_channels.get(key).map(|s| s.len()).unwrap_or(0);
        if next_state.len() == prev_len + 1 {
            if let Some(msg) = next_state.last() {
                return Some(((key.0, key.1.clone()), msg.clone()));
            }
        }
    }
    None
}

fn delivery_preview(prev_vm: &althread::vm::VM, next_vm: &althread::vm::VM) -> Option<String> {
    let prev_channels = prev_vm.current_state().1;
    let next_channels = next_vm.current_state().1;
    let ((receiver_pid, channel_name), msg) = find_delivered_message(prev_channels, next_channels)?;
    let (sender_pid, sender_clock, content) =
        althread::vm::channels::parse_message_tuple(&msg).unwrap_or((0, 0, msg.to_string()));
    Some(format!(
        "deliver {},{} <- {} @{} : {}",
        receiver_pid, channel_name, sender_pid, sender_clock, content
    ))
}

fn message_payload_string(message: &Literal) -> String {
    althread::vm::channels::parse_message_tuple(message)
        .map(|(_, _, content)| content)
        .unwrap_or_else(|| message.to_string())
}

fn error_to_js(err: AlthreadError) -> JsValue {
    to_js(&err)
}

fn web_pos_from_rc(pos: &std::rc::Rc<althread::error::Pos>) -> WebPos {
    WebPos {
        line: pos.line,
        col: pos.col,
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
fn create_vm_state(vm: &althread::vm::VM) -> VMState {
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
                .map(|pos| pos.line);

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
                    line: pos.as_ref().map(|p| p.line),
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

#[wasm_bindgen]
pub fn initialize() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn compile(source: &str, file_path: &str, virtual_fs: JsValue) -> Result<String, JsValue> {
    // Convert the JS file system to a Rust HashMap
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))?;

    // Create virtual filesystem
    let virtual_filesystem = VirtualFileSystem::new(fs_map);

    let mut input_map = HashMap::new();
    input_map.insert(file_path.to_string(), source.to_string());

    let ast = althread::parser::parse_ast(&source, file_path).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast
        .compile(Path::new(file_path), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    println!("{}", compiled_project.to_string());
    Ok(format!("{}", compiled_project))
}

#[wasm_bindgen]
pub fn run(source: &str, filepath: &str, virtual_fs: JsValue) -> Result<JsValue, JsValue> {
    // Convert the JS file system to a Rust HashMap
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))?;

    // Create virtual filesystem
    let virtual_filesystem = VirtualFileSystem::new(fs_map);

    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    let ast = althread::parser::parse_ast(&source, filepath).map_err(error_to_js)?;

    println!("{}", &ast);

    // Use compile_with_filesystem instead of compile
    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    // Rest of the function stays exactly the same
    let mut vm = althread::vm::VM::new(&compiled_project);

    vm.start(fastrand::u64(0..(1 << 32)));

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
    let mut vm_history = vec![vm.clone()]; // For tracking channel states
    let mut i = 0; //index for nodes
    let mut runtime_error = None;

    for _ in 0..100000 {
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
            .filter_map(|inst| inst.pos.as_ref().map(|p| p.line))
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
        vm_history.push(vm.clone());

        // Helper to get program name by ID
        let get_prog_name = |prog_id: usize, vm_instance: &VM| -> String {
            vm_instance
                .running_programs
                .iter()
                .find(|p| p.id == prog_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("PID_{}", prog_id)) // Fallback
        };

        println!("{}", get_prog_name(info.prog_id, &vm));

        for inst in info.instructions.iter() {
            result.push_str(&format!("#{}: {}\n", info.prog_id, inst));
        }

        let instruction_lines: Vec<usize> = info
            .instructions
            .iter()
            .filter_map(|inst| inst.pos.as_ref().map(|p| p.line))
            .collect();

        for action in info.actions.iter() {
            match action {
                GlobalAction::Print(s_print) => stdout.push(s_print.clone()),
                GlobalAction::Send(info) => {
                    let receiver_id = vm
                        .channels
                        .get_connections()
                        .get(&(info.from.process_id, info.to.channel_name.clone()))
                        .map(|(pid, _chan)| *pid)
                        .unwrap_or(0);

                    message_flow_graph.push(MessageFlowEvent {
                        sender: info.from.process_id,
                        receiver: receiver_id,
                        evt_type: SEND,
                        message: info.to.channel_name.clone(),
                        number: info.n_msg,
                        actor_prog_name: info.from.process_name.clone(),
                        vm_state: create_vm_state(&vm),
                        lines: instruction_lines.clone(),
                    });
                }
                GlobalAction::Broadcast(send_infos) => {
                    for send_info in send_infos {
                        let receiver_id = vm
                            .channels
                            .get_connections()
                            .get(&(send_info.from.process_id, send_info.to.channel_name.clone()))
                            .map(|(pid, _chan)| *pid)
                            .unwrap_or(0);

                        message_flow_graph.push(MessageFlowEvent {
                            sender: send_info.from.process_id,
                            receiver: receiver_id,
                            evt_type: SEND,
                            message: send_info.to.channel_name.clone(),
                            number: send_info.n_msg,
                            actor_prog_name: send_info.from.process_name.clone(),
                            vm_state: create_vm_state(&vm),
                            lines: instruction_lines.clone(),
                        });
                    }
                }
                GlobalAction::Deliver(info) => {
                    message_flow_graph.push(MessageFlowEvent {
                        sender: info.from.process_id,
                        receiver: info.to.process_id,
                        evt_type: RECV,
                        message: message_payload_string(&info.message),
                        number: info.sender_clock,
                        actor_prog_name: info.to.process_name.clone(),
                        vm_state: create_vm_state(&vm),
                        lines: instruction_lines.clone(),
                    });
                }
                _ => {}
            }
        }
        if info.invariant_error.is_err() {
            let err = info.invariant_error.unwrap_err();
            result.push_str(&format!(
                "Invariant error at line {}: {}\n",
                err.pos.unwrap().line,
                err.message
            ));
            break;
        }
        i += 1;
    }

    // Collect step_lines from nodes
    let step_lines: Vec<Vec<usize>> = nodes
        .iter()
        .filter_map(|node| node.metadata.lines.clone())
        .collect();

    let result = RunResult {
        debug: result,
        stdout,
        message_flow_events: message_flow_graph,
        nodes,
        step_lines,
        runtime_error,
    };

    Ok(to_js(&result))
}

#[wasm_bindgen]
pub fn check(
    source: &str,
    filepath: &str,
    virtual_fs: JsValue,
    max_states: Option<usize>,
) -> Result<JsValue, JsValue> {
    const WEB_GRAPH_DETAILS_THRESHOLD: usize = 200;

    // Convert the JS file system to a Rust HashMap
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))?;

    // Create virtual filesystem
    let virtual_filesystem = VirtualFileSystem::new(fs_map);

    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    let ast = althread::parser::parse_ast(&source, filepath).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    let (path, state_graph) =
        checker::check_program(&compiled_project, max_states).map_err(error_to_js)?;
    let omit_transition_details = state_graph.nodes.len() > WEB_GRAPH_DETAILS_THRESHOLD;

    // Convert path to GraphNode structure
    let path_nodes: Vec<GraphNode> = path
        .iter()
        .enumerate()
        .map(|(idx, state_link)| GraphNode {
            vm: create_vm_state(state_graph.vm(state_link.to)),
            metadata: NodeMetadata {
                level: idx,
                step_index: None,
                successors: None,
                lines: Some(state_link.lines.clone()),
            },
        })
        .collect();

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

    let result = CheckResult {
        path: path_nodes,
        nodes: graph_nodes,
        exhaustive: state_graph.exhaustive,
    };

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
    // Convert the JS file system to a Rust HashMap
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))?;

    // Create virtual filesystem
    let virtual_filesystem = VirtualFileSystem::new(fs_map);

    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    let ast = althread::parser::parse_ast(&source, filepath).map_err(error_to_js)?;

    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    let mut vm = althread::vm::VM::new(&compiled_project);
    vm.start(0); // Use deterministic seed for interactive mode

    // Get initial next states
    let next_states = vm.next().map_err(error_to_js)?;

    if next_states.is_empty() {
        let result = InteractiveSessionState {
            next_states: vec![],
            current_state: create_vm_state(&vm),
            state_display: vec![],
            output: vec![],
        };
        return Ok(to_js(&result));
    }

    // Convert the result to NextStateOption format
    let next_state_options: Vec<NextStateOption> = next_states
        .into_iter()
        .map(|(name, pid, instructions, _actions, _nvm)| {
            let lines: Vec<usize> = instructions
                .iter()
                .filter_map(|inst| inst.pos.as_ref().map(|p| p.line))
                .collect();

            let instruction_strings: Vec<String> =
                instructions.iter().map(|inst| inst.to_string()).collect();

            NextStateOption {
                prog_name: name,
                prog_id: pid,
                instructions: instruction_strings,
                lines,
            }
        })
        .collect();

    let result = InteractiveSessionState {
        next_states: next_state_options,
        current_state: create_vm_state(&vm),
        state_display: vec![],
        output: vec![],
    };

    Ok(to_js(&result))
}

#[wasm_bindgen]
pub fn get_next_interactive_states(
    source: &str,
    filepath: &str,
    virtual_fs: JsValue,
    execution_history: JsValue,
) -> Result<JsValue, JsValue> {
    // Convert the JS file system to a Rust HashMap
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))?;

    // Parse execution history
    let history: Vec<usize> = serde_wasm_bindgen::from_value(execution_history)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse execution history: {}", e)))?;

    // Create virtual filesystem
    let virtual_filesystem = VirtualFileSystem::new(fs_map);

    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    let ast = althread::parser::parse_ast(&source, filepath).map_err(error_to_js)?;

    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    let mut vm = althread::vm::VM::new(&compiled_project);
    vm.start(0); // Use deterministic seed for interactive mode

    // Replay execution history
    for &selected_index in &history {
        let next_states = vm.next().map_err(error_to_js)?;
        if selected_index >= next_states.len() {
            return Err(JsValue::from_str(&format!(
                "Invalid selection index {} in history",
                selected_index
            )));
        }
        let (_, _, _, _, new_vm) = next_states.into_iter().nth(selected_index).unwrap();
        vm = new_vm;
    }

    // Get next possible states - web-safe error handling
    let next_states = vm.next().map_err(error_to_js)?;

    if next_states.is_empty() {
        return Err(JsValue::from_str(
            "No next states available (execution finished)",
        ));
    }

    // Convert next states to NextStateOption format
    let next_state_options: Vec<NextStateOption> = next_states
        .iter()
        .map(|(name, pid, instructions, _actions, _nvm)| {
            let lines: Vec<usize> = instructions
                .iter()
                .filter_map(|inst| inst.pos.as_ref().map(|p| p.line))
                .collect();

            let instruction_strings: Vec<String> =
                instructions.iter().map(|inst| inst.to_string()).collect();

            NextStateOption {
                prog_name: name.clone(),
                prog_id: *pid,
                instructions: instruction_strings,
                lines,
            }
        })
        .collect();

    let current_state = vm.current_state();

    // Generate state display information similar to run_interactive
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

    let result = InteractiveSessionState {
        next_states: next_state_options,
        current_state: create_vm_state(&vm),
        state_display: state_display_info,
        output: vec![],
    };

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
    // Convert the JS file system to a Rust HashMap
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))?;

    // Parse execution history
    let history: Vec<usize> = serde_wasm_bindgen::from_value(execution_history)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse execution history: {}", e)))?;

    // Create virtual filesystem
    let virtual_filesystem = VirtualFileSystem::new(fs_map);

    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    let ast = althread::parser::parse_ast(&source, filepath).map_err(error_to_js)?;

    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    let mut vm = althread::vm::VM::new(&compiled_project);
    vm.start(0); // Use deterministic seed for interactive mode

    // Replay execution history
    for &step_index in &history {
        let next_states = vm.next().map_err(error_to_js)?;
        if next_states.is_empty() {
            return Err(JsValue::from_str("No next state during replay"));
        }
        if step_index >= next_states.len() {
            return Err(JsValue::from_str(&format!(
                "Invalid selection index {} in history",
                step_index
            )));
        }
        let (_, _, _, _, new_vm) = next_states.into_iter().nth(step_index).unwrap();
        vm = new_vm;
    }

    // Get next possible states for this step - web-safe error handling
    let next_states = vm.next().map_err(error_to_js)?;

    if next_states.is_empty() {
        return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "finished": true,
            "message": "No next state"
        }))
        .unwrap());
    }

    if selected_index >= next_states.len() {
        return Err(JsValue::from_str(&format!(
            "Invalid selection index {}",
            selected_index
        )));
    }

    // Execute the selected transition.
    // NOTE: vm.next() contains both program steps and delivery steps.
    let (name, pid, instructions, actions, new_vm) =
        next_states.into_iter().nth(selected_index).unwrap();

    let execution_vm = new_vm.clone();
    let mut message_flow_events: Vec<MessageFlowEvent> = Vec::new();
    let mut step_output: Vec<String> = Vec::new();
    let mut step_debug = String::new();

    let step_lines: Vec<usize> = instructions
        .iter()
        .filter_map(|inst| inst.pos.as_ref().map(|p| p.line))
        .collect();

    // Capture debug info from the instructions
    for inst in instructions.iter() {
        step_debug.push_str(&format!("#{}: {}\n", pid, inst));
    }

    for action in actions {
        match action {
            GlobalAction::Print(s) => step_output.push(s),
            GlobalAction::Send(info) => {
                let receiver_id = execution_vm
                    .channels
                    .get_connections()
                    .get(&(info.from.process_id, info.to.channel_name.clone()))
                    .map(|(pid, _chan)| *pid)
                    .unwrap_or(0);

                message_flow_events.push(MessageFlowEvent {
                    sender: info.from.process_id,
                    receiver: receiver_id,
                    evt_type: SEND,
                    message: info.to.channel_name,
                    number: info.n_msg,
                    actor_prog_name: info.from.process_name,
                    vm_state: create_vm_state(&execution_vm),
                    lines: step_lines.clone(),
                });
            }
            GlobalAction::Deliver(info) => {
                message_flow_events.push(MessageFlowEvent {
                    sender: info.from.process_id,
                    receiver: info.to.process_id,
                    evt_type: RECV,
                    message: message_payload_string(&info.message),
                    number: info.sender_clock,
                    actor_prog_name: info.to.process_name,
                    vm_state: create_vm_state(&execution_vm),
                    lines: step_lines.clone(),
                });
            }
            _ => {}
        }
    }

    // If we executed a delivery step, provide a useful executed_step string.
    let executed_instructions = if instructions.is_empty() && name.starts_with("__deliver__") {
        let mut delivery = None;
        for action in &message_flow_events {
            if action.evt_type == RECV {
                delivery = Some(format!(
                    "deliver {} <- {} : {}",
                    action.receiver, action.sender, action.message
                ));
                break;
            }
        }
        vec![delivery.unwrap_or_else(|| "deliver <unknown>".to_string())]
    } else {
        instructions
            .iter()
            .map(|inst| inst.to_string())
            .collect::<Vec<String>>()
    };

    // Generate state display information
    let current_state = execution_vm.current_state();
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

    let result = InteractiveStepResult {
        executed_step: ExecutedStepInfo {
            prog_name: name,
            prog_id: pid,
            instructions: executed_instructions,
        },
        output: step_output,
        debug: step_debug,
        new_state: create_vm_state(&execution_vm),
        message_flow_events,
        state_display: state_display_info,
        lines: step_lines,
    };

    Ok(to_js(&result))
}
