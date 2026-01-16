use fastrand;
use serde::{Serialize, Serializer};
use serde::ser::{SerializeStruct};
use serde_wasm_bindgen;
use std::collections::HashMap;
use std::path::Path;
use wasm_bindgen::prelude::*;

use althread::ast::token::literal::Literal;
use althread::module_resolver::VirtualFileSystem;
use althread::vm::instruction::InstructionType;
use althread::vm::VM;
use althread::{ast::Ast, checker, error::AlthreadError, vm::GlobalAction};
use console_error_panic_hook;

const SEND: u8 = b's';
const RECV: u8 = b'r';

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

fn parse_message_tuple(msg: &Literal) -> Option<(usize, usize, String)> {
    // Expected format: ((sender_id, sender_clock), content)
    let Literal::Tuple(msg_tuple) = msg else {
        return None;
    };
    if msg_tuple.len() < 2 {
        return None;
    }
    let Literal::Tuple(sender_info) = msg_tuple.get(0)? else {
        return None;
    };
    if sender_info.len() < 2 {
        return None;
    }
    let Literal::Int(sender_id) = sender_info.get(0)? else {
        return None;
    };
    let Literal::Int(sender_clock) = sender_info.get(1)? else {
        return None;
    };

    let content = msg_tuple.get(1)?.to_string();
    Some((*sender_id as usize, *sender_clock as usize, content))
}

fn delivery_preview(prev_vm: &althread::vm::VM, next_vm: &althread::vm::VM) -> Option<String> {
    let prev_channels = prev_vm.current_state().1;
    let next_channels = next_vm.current_state().1;
    let ((receiver_pid, channel_name), msg) = find_delivered_message(prev_channels, next_channels)?;
    let (sender_pid, sender_clock, content) = parse_message_tuple(&msg).unwrap_or((0, 0, msg.to_string()));
    Some(format!(
        "deliver {},{} <- {} @{} : {}",
        receiver_pid, channel_name, sender_pid, sender_clock, content
    ))
}

fn error_to_js(err: AlthreadError) -> JsValue {
    serde_wasm_bindgen::to_value(&err).unwrap()
}


// Helper function to create VM state JSON object
fn create_vm_state_json(vm: &althread::vm::VM) -> serde_json::Value {
    let current_state = vm.current_state();

    serde_json::json!({
        "globals": current_state.0.iter().map(|(key, value)| {
            (key.clone(), format!("{:?}", value))
        }).collect::<HashMap<_, _>>(),
        "channels": current_state.1.iter().map(|((pid, name), values)| {
            serde_json::json!({
                "pid": pid,
                "name": name,
                "values": values.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>()
            })
        }).collect::<Vec<_>>(),
                "channel_connections": vm.channels.get_connections()
            .iter()
            .map(|((from_pid, from_channel), (to_pid, to_channel))| {
                serde_json::json!({
                    "from": { "pid": from_pid, "channel": from_channel },
                    "to": { "pid": to_pid, "channel": to_channel }
                })
            })
            .collect::<Vec<_>>(),
        "programs": current_state.2.iter().enumerate().map(|(index, (memory, instruction_pointer, clock))| {
            let prog_name = vm.running_programs.get(index)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("PID_{}", index));
            serde_json::json!({
                "pid": index,
                "name": prog_name,
                "memory": memory.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>(),
                "instruction_pointer": instruction_pointer,
                "clock": clock
            })
        }).collect::<Vec<_>>()
    })
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

    // parse code with pest
    let pairs = althread::parser::parse(&source, file_path).map_err(error_to_js)?;

    let ast = Ast::build(pairs, file_path).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast
        .compile(Path::new(file_path), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    println!("{}", compiled_project.to_string());
    Ok(format!("{}", compiled_project))
}

pub struct MessageFlowEvent<'a> {
    pub sender: usize,           // id of the sending process
    pub receiver: usize,         // id of the receiving process
    pub evt_type: u8,            //send or receive
    pub message: String,         // for SEND: channel name, for RECV: message content
    pub number: usize,           // message sequence number (nmsg_sent for SEND, clock for RECV)
    pub actor_prog_name: String, // Name of the program performing this action
    pub vm_state: VM<'a>,        //vm state associated with this event
}

pub struct InteractiveStepResult<'a> {
    executed_step: ExecutedStepInfo,
    output: Vec<String>,
    debug: String,
    current_state: VM<'a>,
    new_state: serde_json::Value,
    message_flow_events: Vec<MessageFlowEvent<'a>>,
    state_display: serde_json::Value,
}

#[derive(Serialize)]
pub struct ExecutedStepInfo {
    prog_name: String,
    prog_id: usize,
    instructions: Vec<String>,
}

pub struct RunResult<'a> {
    debug: String,
    stdout: Vec<String>,
    message_flow_graph: Vec<MessageFlowEvent<'a>>,
    vm_states: Vec<VM<'a>>,
}

impl<'a> Serialize for InteractiveStepResult<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("InteractiveStepResult", 7)?;
        state.serialize_field("executed_step", &self.executed_step)?;
        state.serialize_field("output", &self.output)?;
        state.serialize_field("debug", &self.debug)?;
        state.serialize_field("current_state", &self.current_state)?;
        state.serialize_field("new_state", &self.new_state)?;
        state.serialize_field("message_flow_events", &self.message_flow_events)?;
        state.serialize_field("state_display", &self.state_display)?;
        state.end()
    }
}

impl<'a> Serialize for MessageFlowEvent<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Updated field count to 7
        let mut state = serializer.serialize_struct("MessageFlowEvent", 7)?;
        state.serialize_field("sender", &self.sender)?;
        state.serialize_field("receiver", &self.receiver)?;
        state.serialize_field("evt_type", &self.evt_type)?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("number", &self.number)?;
        state.serialize_field("actor_prog_name", &self.actor_prog_name)?; // Added new field
        state.serialize_field("vm_state", &self.vm_state)?;
        state.end()
    }
}

impl<'a> Serialize for RunResult<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Corrected field count to 4
        let mut state = serializer.serialize_struct("RunResult", 4)?;
        state.serialize_field("debug", &self.debug)?;
        state.serialize_field("stdout", &self.stdout)?;
        state.serialize_field("message_flow_graph", &self.message_flow_graph)?;
        state.serialize_field("vm_states", &self.vm_states)?;
        state.end()
    }
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

    // parse code with pest
    let pairs = althread::parser::parse(&source, filepath).map_err(error_to_js)?;

    let ast = Ast::build(pairs, filepath).map_err(error_to_js)?;

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
    let mut vm_states = Vec::new();
    let mut i = 0; //index for vm_states

    for _ in 0..100000 {
        if vm.is_finished() {
            break;
        }
        let info = vm.next_random().map_err(error_to_js)?;
        vm_states.push(vm.clone());

        let pid = info.prog_id;

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

            if let InstructionType::ChannelPop(ref s) = &inst.control {
                if i > 0 {
                    let previous_vm_state = vm_states.get(i - 1); // The state before this pop
                    if let Some(prev_vm) = previous_vm_state {
                        if let Some(chan_content_vec) = prev_vm
                            .channels
                            .get_states()
                            .get(&(info.prog_id, s.to_string()))
                        {
                            if let Some(Literal::Tuple(ref msg_tuple)) = chan_content_vec.get(0) {
                                // Message popped
                                if msg_tuple.len() >= 2 {
                                    // Ensure msg_tuple has at least sender_info and content
                                    if let Some(Literal::Tuple(ref sender_info_tuple)) =
                                        msg_tuple.get(0)
                                    {
                                        if sender_info_tuple.len() >= 2 {
                                            // Ensure sender_info_tuple has senderid and clock
                                            if let (
                                                Some(Literal::Int(senderid)),
                                                Some(Literal::Int(received_clock)),
                                            ) =
                                                (sender_info_tuple.get(0), sender_info_tuple.get(1))
                                            {
                                                if let Some(actual_message_content) =
                                                    msg_tuple.get(1)
                                                {
                                                    let receiver_clock = vm
                                                        .running_programs
                                                        .iter()
                                                        .find(|p| p.id == pid)
                                                        .map(|p| p.clock)
                                                        .unwrap_or(0);

                                                    let max_clock = std::cmp::max(
                                                        *received_clock as usize,
                                                        receiver_clock as usize,
                                                    ) + 1;

                                                    vm.running_programs
                                                        .iter_mut()
                                                        .find(|p| p.id == pid)
                                                        .map(|p| p.clock = max_clock);

                                                    let receiver_name =
                                                        get_prog_name(info.prog_id, &vm);
                                                    let event = MessageFlowEvent {
                                                        sender: *senderid as usize,
                                                        receiver: pid,
                                                        evt_type: RECV,
                                                        message: actual_message_content.to_string(),
                                                        number: max_clock, // Using max_clock as message number
                                                        actor_prog_name: receiver_name,
                                                        vm_state: vm.clone(), // State after receive
                                                    };
                                                    message_flow_graph.push(event);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for p in info.actions.iter() {
            if let GlobalAction::Print(s_print) = p {
                stdout.push(s_print.clone());
            }
        }

        // Send events are now derived from executed instructions (Send is no longer a GlobalAction).
        for inst in info.instructions.iter() {
            if let InstructionType::Send { channel_name, .. } = &inst.control {
                let sender_id = info.prog_id;
                let sender_name = get_prog_name(sender_id, &vm);

                let receiver_id = vm
                    .channels
                    .get_connections()
                    .get(&(sender_id, channel_name.clone()))
                    .map(|(pid, _chan)| *pid)
                    .unwrap_or(0);

                let clock = vm
                    .running_programs
                    .iter()
                    .find(|p| p.id == sender_id)
                    .map(|p| p.clock)
                    .unwrap_or(0);

                let event = MessageFlowEvent {
                    sender: sender_id,
                    receiver: receiver_id,
                    evt_type: SEND,
                    message: channel_name.clone(),
                    number: clock,
                    actor_prog_name: sender_name,
                    vm_state: vm.clone(),
                };
                message_flow_graph.push(event);
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

    Ok(serde_wasm_bindgen::to_value(&RunResult {
        debug: result,
        stdout,
        message_flow_graph,
        vm_states,
    })
    .unwrap())
}

#[wasm_bindgen]
pub fn check(source: &str, filepath: &str, virtual_fs: JsValue, max_states: Option<usize>) -> Result<JsValue, JsValue> {
    // Convert the JS file system to a Rust HashMap
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))?;

    // Create virtual filesystem
    let virtual_filesystem = VirtualFileSystem::new(fs_map);

    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    // parse code with pest
    let pairs = althread::parser::parse(&source, filepath).map_err(error_to_js)?;

    let ast = Ast::build(pairs, filepath).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    let checked = checker::check_program(&compiled_project, max_states).map_err(error_to_js)?;

    Ok(serde_wasm_bindgen::to_value(&checked).unwrap())
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
pub fn start_interactive_session(source: &str, filepath: &str, virtual_fs: JsValue) -> Result<JsValue, JsValue> {
    // Convert the JS file system to a Rust HashMap
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse virtual filesystem: {}", e)))?;

    // Create virtual filesystem
    let virtual_filesystem = VirtualFileSystem::new(fs_map);

    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    // parse code with pest
    let pairs = althread::parser::parse(&source, filepath).map_err(error_to_js)?;

    let ast = Ast::build(pairs, filepath).map_err(error_to_js)?;

    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    let mut vm = althread::vm::VM::new(&compiled_project);
    vm.start(0); // Use deterministic seed for interactive mode

    // Get initial next states
    let next_states = vm.next().map_err(error_to_js)?;
    
    if next_states.is_empty() {
        return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "states": [],
            "is_finished": true,
            "current_state": create_vm_state_json(&vm),
            "output": []
        })).unwrap());
    }
    
    // Convert the result to a more JS-friendly format
    let js_next_states: Vec<_> = next_states.into_iter().enumerate().map(|(index, (name, pid, instructions, nvm))| {
        let instruction_strings: Vec<String> = instructions.iter().map(|inst| {
            if let Some(pos) = &inst.pos {
                let line_content = source
                    .lines()
                    .nth(pos.line.saturating_sub(1))
                    .unwrap_or("?");
                format!("{}:{}: {}", name, pid, line_content.trim())
            } else {
                format!("{}:{}: {}", name, pid, inst)
            }
        }).collect();

        let preview = if let Some(first) = instruction_strings.first() {
            first.clone()
        } else if name.starts_with("__deliver__") {
            delivery_preview(&vm, &nvm).unwrap_or_else(|| "deliver <unknown>".to_string())
        } else {
            "No instruction".to_string()
        };

        serde_json::json!({
            "index": index,
            "prog_name": name,
            "prog_id": pid,
            "instruction_preview": preview,
            "instructions": instruction_strings
        })
    }).collect();

    let state_info = serde_json::json!({
        "states": js_next_states,
        "is_finished": false,
        "current_state": create_vm_state_json(&vm),
        "output": [] // No output for initial state
    });

    Ok(serde_wasm_bindgen::to_value(&state_info).unwrap())
}

#[wasm_bindgen]
pub fn get_next_interactive_states(source: &str, filepath: &str, virtual_fs: JsValue, execution_history: JsValue) -> Result<JsValue, JsValue> {
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

    // parse code with pest - web-safe error handling
    let pairs = althread::parser::parse(&source, filepath).map_err(error_to_js)?;

    let ast = Ast::build(pairs, filepath).map_err(error_to_js)?;

    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(error_to_js)?;

    let mut vm = althread::vm::VM::new(&compiled_project);
    vm.start(0); // Use deterministic seed for interactive mode

    // Replay execution history
    for &selected_index in &history {
        let next_states = vm.next().map_err(error_to_js)?;
        if selected_index >= next_states.len() {
            return Err(JsValue::from_str(&format!("Invalid selection index {} in history", selected_index)));
        }
        let (_, _, _, new_vm) = next_states.into_iter().nth(selected_index).unwrap();
        vm = new_vm;
    }
    
    // Get next possible states - web-safe error handling
    let next_states = vm.next().map_err(error_to_js)?;
    
    if next_states.is_empty() {
        return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "states": [],
            "is_finished": true,
            "message": "No next state",
            "current_state": create_vm_state_json(&vm),
            "output": []
        })).unwrap());
    }
    
    // Convert the result to a more JS-friendly format with enhanced state display
    let js_next_states: Vec<_> = next_states.into_iter().enumerate().map(|(index, (name, pid, instructions, nvm))| {
        let instruction_strings: Vec<String> = instructions.iter().map(|inst| {
            if let Some(pos) = &inst.pos {
                let line_content = source
                    .lines()
                    .nth(pos.line.saturating_sub(1))
                    .unwrap_or("?");
                format!("{}:{}: {}", name, pid, line_content.trim())
            } else {
                format!("{}:{}: {}", name, pid, inst)
            }
        }).collect();

        // Add state info similar to run_interactive format
        let state_preview = if let Some(inst) = instructions.first() {
            if let Some(pos) = &inst.pos {
                let line = source
                    .lines()
                    .nth(pos.line.saturating_sub(1))
                    .unwrap_or_default();
                format!("{}:{}:{}", name, pid, line)
            } else {
                format!("{}:{}:?", name, pid)
            }
        } else if name.starts_with("__deliver__") {
            format!("{}:{}:{}", name, pid, delivery_preview(&vm, &nvm).unwrap_or_else(|| "deliver <unknown>".to_string()))
        } else {
            format!("{}:{}:?", name, pid)
        };

        let preview = if let Some(first) = instruction_strings.first() {
            first.clone()
        } else if name.starts_with("__deliver__") {
            delivery_preview(&vm, &nvm).unwrap_or_else(|| "deliver <unknown>".to_string())
        } else {
            "No instruction".to_string()
        };

        serde_json::json!({
            "index": index,
            "prog_name": name,
            "prog_id": pid,
            "instruction_preview": preview,
            "instructions": instruction_strings,
            "state_preview": state_preview
        })
    }).collect();

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
    
    let state_info = serde_json::json!({
        "states": js_next_states,
        "is_finished": false,
        "state_display": state_display_info,
        "current_state": create_vm_state_json(&vm),
        "output": [] // This is the accumulated output up to this point
    });

    Ok(serde_wasm_bindgen::to_value(&state_info).unwrap())
}

#[wasm_bindgen]
pub fn execute_interactive_step(source: &str, filepath: &str, virtual_fs: JsValue, execution_history: JsValue, selected_index: usize) -> Result<JsValue, JsValue> {
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

    // parse code with pest - web-safe error handling
    let pairs = althread::parser::parse(&source, filepath).map_err(error_to_js)?;

    let ast = Ast::build(pairs, filepath).map_err(error_to_js)?;

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
            return Err(JsValue::from_str(&format!("Invalid selection index {} in history", step_index)));
        }
        let (_, _, _, new_vm) = next_states.into_iter().nth(step_index).unwrap();
        vm = new_vm;
    }

    // Get next possible states for this step - web-safe error handling
    let next_states = vm.next().map_err(error_to_js)?;
    
    if next_states.is_empty() {
        return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "finished": true,
            "message": "No next state"
        })).unwrap());
    }
    
    if selected_index >= next_states.len() {
        return Err(JsValue::from_str(&format!("Invalid selection index {}", selected_index)));
    }

    // Execute the selected transition.
    // NOTE: vm.next() contains both program steps and delivery steps.
    let (name, pid, instructions, new_vm) = next_states.into_iter().nth(selected_index).unwrap();

    let is_delivery_choice = name.starts_with("__deliver__") && instructions.is_empty();

    // Store the VM state BEFORE execution for event detection
    let vm_before_step = vm.clone();

    // Apply the selected transition.
    let mut execution_vm = vm.clone();
    let mut message_flow_events: Vec<MessageFlowEvent> = Vec::new();
    let mut step_output: Vec<String> = Vec::new();
    let mut step_debug = String::new();

    if is_delivery_choice {
        // vm.next() already computed the successor VM, including wake-ups.
        execution_vm = new_vm.clone();

        // Emit a SEND-like event for the delivered message (more stable than inferring from send()).
        if let Some((_key, msg)) = find_delivered_message(
            vm_before_step.current_state().1,
            execution_vm.current_state().1,
        ) {
            if let Some((sender_id, sender_clock, content)) = parse_message_tuple(&msg) {
                let actor_name = execution_vm
                    .running_programs
                    .iter()
                    .find(|p| p.id == sender_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("PID_{}", sender_id));
                message_flow_events.push(MessageFlowEvent {
                    sender: sender_id,
                    receiver: pid,
                    evt_type: SEND,
                    message: content,
                    number: sender_clock,
                    actor_prog_name: actor_name,
                    vm_state: execution_vm.clone(),
                });
            }
        }
    } else {
        // Program step: re-execute deterministically to get actions/output.
        let step_info = match execution_vm.next_step_pid(pid) {
            Ok(Some(info)) => info,
            Ok(None) => {
                return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
                    "finished": true,
                    "message": "Program has terminated"
                })).unwrap());
            }
            Err(e) => {
                return Err(error_to_js(e));
            }
        };

        // Check for invariant errors
        if step_info.invariant_error.is_err() {
            let error_msg = format!("Invariant error: {:?}", step_info.invariant_error.unwrap_err());
            return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
                "error": error_msg,
                "invariant_violated": true
            })).unwrap());
        }

        // Capture debug info from the actual executed step
        for inst in step_info.instructions.iter() {
            step_debug.push_str(&format!("#{}: {}\n", step_info.prog_id, inst));
        }

        // Capture output from actions
        step_output = step_info
            .actions
            .iter()
            .filter_map(|action| {
                if let GlobalAction::Print(s_print) = action {
                    Some(s_print.clone())
                } else {
                    None
                }
            })
            .collect();

        // Helper to get program name by ID
        let get_prog_name = |prog_id: usize, vm_instance: &althread::vm::VM| -> String {
            vm_instance
                .running_programs
                .iter()
                .find(|p| p.id == prog_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("PID_{}", prog_id))
        };

        // Receive events (based on vm_before_step)
        for inst in step_info.instructions.iter() {
            if let InstructionType::ChannelPop(ref s) = &inst.control {
                if let Some(chan_content_vec) = vm_before_step
                    .channels
                    .get_states()
                    .get(&(step_info.prog_id, s.to_string()))
                {
                    if let Some(Literal::Tuple(ref msg_tuple)) = chan_content_vec.get(0) {
                        if msg_tuple.len() >= 2 {
                            if let Some(Literal::Tuple(ref sender_info_tuple)) = msg_tuple.get(0) {
                                if sender_info_tuple.len() >= 2 {
                                    if let (
                                        Some(Literal::Int(senderid)),
                                        Some(Literal::Int(received_clock)),
                                    ) = (sender_info_tuple.get(0), sender_info_tuple.get(1))
                                    {
                                        if let Some(actual_message_content) = msg_tuple.get(1) {
                                            let receiver_clock = execution_vm
                                                .running_programs
                                                .iter()
                                                .find(|p| p.id == step_info.prog_id)
                                                .map(|p| p.clock)
                                                .unwrap_or(0);

                                            let max_clock = std::cmp::max(
                                                *received_clock as usize,
                                                receiver_clock as usize,
                                            ) + 1;

                                            let receiver_name =
                                                get_prog_name(step_info.prog_id, &execution_vm);
                                            message_flow_events.push(MessageFlowEvent {
                                                sender: *senderid as usize,
                                                receiver: step_info.prog_id,
                                                evt_type: RECV,
                                                message: actual_message_content.to_string(),
                                                number: max_clock,
                                                actor_prog_name: receiver_name,
                                                vm_state: execution_vm.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Send events from executed instructions
        for inst in step_info.instructions.iter() {
            if let InstructionType::Send { channel_name, .. } = &inst.control {
                let sender_id = step_info.prog_id;
                let sender_name = get_prog_name(sender_id, &execution_vm);

                let receiver_id = execution_vm
                    .channels
                    .get_connections()
                    .get(&(sender_id, channel_name.clone()))
                    .map(|(pid, _chan)| *pid)
                    .unwrap_or(0);

                let clock = execution_vm
                    .running_programs
                    .iter()
                    .find(|p| p.id == sender_id)
                    .map(|p| p.clock)
                    .unwrap_or(0);

                message_flow_events.push(MessageFlowEvent {
                    sender: sender_id,
                    receiver: receiver_id,
                    evt_type: SEND,
                    message: channel_name.clone(),
                    number: clock,
                    actor_prog_name: sender_name,
                    vm_state: execution_vm.clone(),
                });
            }
        }
    }

    // If we executed a delivery step, provide a useful executed_step string.
    let executed_instructions = if instructions.is_empty() && name.starts_with("__deliver__") {
        vec![delivery_preview(&vm_before_step, &execution_vm).unwrap_or_else(|| "deliver <unknown>".to_string())]
    } else {
        instructions
            .iter()
            .map(|inst| inst.to_string())
            .collect::<Vec<String>>()
    };

    let result = InteractiveStepResult {
        executed_step: ExecutedStepInfo {
            prog_name: name,
            prog_id: pid,
            instructions: executed_instructions,
        },
        output: step_output,
        debug: step_debug,
        current_state: execution_vm.clone(),
        new_state: create_vm_state_json(&execution_vm),
        message_flow_events,
        state_display: create_vm_state_json(&execution_vm),
    };

    Ok(serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?)
}
