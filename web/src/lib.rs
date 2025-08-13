use fastrand;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_wasm_bindgen;
use std::collections::HashMap;
use std::path::Path;
use wasm_bindgen::prelude::*;

use althread::ast::token::literal::Literal;
use althread::module_resolver::VirtualFileSystem;
use althread::vm::instruction::InstructionType;
use althread::vm::VM;
use althread::{ast::Ast, checker, error::AlthreadError, vm::GlobalAction};
use web_sys::console;
use console_error_panic_hook;

const SEND: u8 = b's';
const RECV: u8 = b'r';

fn error_to_js(err: AlthreadError) -> JsValue {
    serde_wasm_bindgen::to_value(&err).unwrap()
}

fn format_error_for_web(err: AlthreadError) -> String {
    format!("Althread Error: {:?}", err)
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

pub struct RunResult<'a> {
    debug: String,
    stdout: Vec<String>,
    message_flow_graph: Vec<MessageFlowEvent<'a>>,
    vm_states: Vec<VM<'a>>,
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
            match p {
                GlobalAction::Send(s_chan_name, opt_receiver_info) => {
                    let sender_name = get_prog_name(info.prog_id, &vm);
                    let receiver_id = opt_receiver_info
                        .as_ref()
                        .map(|ri| ri.program_id)
                        .unwrap_or(0);

                    let clock = vm
                        .running_programs
                        .iter()
                        .find(|p| p.id == pid)
                        .map(|p| p.clock)
                        .unwrap_or(0);

                    let event = MessageFlowEvent {
                        sender: pid,
                        receiver: receiver_id,
                        evt_type: SEND,
                        message: s_chan_name.clone(), // Channel name for SEND
                        number: clock,
                        actor_prog_name: sender_name,
                        vm_state: vm.clone(), // State after send
                    };
                    message_flow_graph.push(event);
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

    Ok(serde_wasm_bindgen::to_value(&RunResult {
        debug: result,
        stdout,
        message_flow_graph,
        vm_states,
    })
    .unwrap())
}

#[wasm_bindgen]
pub fn check(source: &str, filepath: &str, virtual_fs: JsValue) -> Result<JsValue, JsValue> {
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

    let checked = checker::check_program(&compiled_project).map_err(error_to_js)?;

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
        let current_state = vm.current_state();
        return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "states": [],
            "is_finished": true,
            "current_state": {
                "globals": current_state.0.iter().map(|(key, value)| {
                (key.clone(), format!("{:?}", value)) // Convert values to strings
                }).collect::<HashMap<_, _>>(),
                "channels": current_state.1.iter().map(|((pid, name), values)| {
                    serde_json::json!({
                        "key": format!("{},{}", pid, name), // Convert tuple key to a string
                        "values": values.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>() // Convert values to strings
                    })
                }).collect::<Vec<_>>(),
                "programs": current_state.2.iter().enumerate().map(|(index, (memory, instruction_pointer, clock))| {
                    let prog_name = vm.running_programs.get(index)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| format!("PID_{}", index));
                    serde_json::json!({
                        "pid": index,
                        "name": prog_name,
                        "memory": memory,
                        "instruction_pointer": instruction_pointer,
                        "clock": clock
                    })
                }).collect::<Vec<_>>()
            },
            "output": []
        })).unwrap());
    }
    
    // Convert the result to a more JS-friendly format
    let js_next_states: Vec<_> = next_states.into_iter().enumerate().map(|(index, (name, pid, instructions, _))| {
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

        serde_json::json!({
            "index": index,
            "prog_name": name,
            "prog_id": pid,
            "instruction_preview": instruction_strings.first().unwrap_or(&"No instruction".to_string()),
            "instructions": instruction_strings
        })
    }).collect();

    let current_state = vm.current_state();
    let state_info = serde_json::json!({
        "states": js_next_states,
        "is_finished": false,
        "current_state": {
            "globals": current_state.0.iter().map(|(key, value)| {
            (key.clone(), format!("{:?}", value)) // Convert values to strings
            }).collect::<HashMap<_, _>>(),
            "channels": current_state.1.iter().map(|((pid, name), values)| {
                serde_json::json!({
                    "key": format!("{},{}", pid, name), // Convert tuple key to a string
                    "values": values.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>() // Convert values to strings
                })
            }).collect::<Vec<_>>(),
            "programs": current_state.2.iter().enumerate().map(|(index, (memory, instruction_pointer, clock))| {
                let prog_name = vm.running_programs.get(index)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("PID_{}", index));
                serde_json::json!({
                    "pid": index,
                    "name": prog_name,
                    "memory": memory,
                    "instruction_pointer": instruction_pointer,
                    "clock": clock
                })
            }).collect::<Vec<_>>()
        },
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
        let current_state = vm.current_state();
        return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
            "states": [],
            "is_finished": true,
            "message": "No next state",
            "current_state": {
                "globals": current_state.0.iter().map(|(key, value)| {
                (key.clone(), format!("{:?}", value)) // Convert values to strings
                }).collect::<HashMap<_, _>>(),
                "channels": current_state.1.iter().map(|((pid, name), values)| {
                    serde_json::json!({
                        "key": format!("{},{}", pid, name), // Convert tuple key to a string
                        "values": values.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>() // Convert values to strings
                    })
                }).collect::<Vec<_>>(),
                "programs": current_state.2.iter().enumerate().map(|(index, (memory, instruction_pointer, clock))| {
                    let prog_name = vm.running_programs.get(index)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| format!("PID_{}", index));
                    serde_json::json!({
                        "pid": index,
                        "name": prog_name,
                        "memory": memory,
                        "instruction_pointer": instruction_pointer,
                        "clock": clock
                    })
                }).collect::<Vec<_>>()
            },
            "output": []
        })).unwrap());
    }
    
    // Convert the result to a more JS-friendly format with enhanced state display
    let js_next_states: Vec<_> = next_states.into_iter().enumerate().map(|(index, (name, pid, instructions, _))| {
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
        let state_preview = format!(
            "{}:{}:{}",
            name,
            pid,
            if instructions.get(0).and_then(|inst| inst.pos.as_ref()).is_some() {
                source
                    .lines()
                    .nth(instructions[0].pos.as_ref().unwrap().line)
                    .unwrap_or_default()
            } else {
                "?"
            }
        );

        serde_json::json!({
            "index": index,
            "prog_name": name,
            "prog_id": pid,
            "instruction_preview": instruction_strings.first().unwrap_or(&"No instruction".to_string()),
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
        "current_state": {
            "globals": current_state.0.iter().map(|(key, value)| {
                (key.clone(), format!("{:?}", value)) // Convert values to strings
            }).collect::<HashMap<_, _>>(),
            "channels": current_state.1.iter().map(|((pid, name), values)| {
                serde_json::json!({
                    "key": format!("{},{}", pid, name), // Convert tuple key to a string
                    "values": values.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>() // Convert values to strings
                })
            }).collect::<Vec<_>>(),
            "programs": current_state.2.iter().enumerate().map(|(index, (memory, instruction_pointer, clock))| {
                let prog_name = vm.running_programs.get(index)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("PID_{}", index));
                serde_json::json!({
                    "pid": index,
                    "name": prog_name,
                    "memory": memory.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>(), // Convert memory values to strings
                    "instruction_pointer": instruction_pointer,
                    "clock": clock
                })
            }).collect::<Vec<_>>()
        },
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

    // Execute the selected step
    let (name, pid, instructions, new_vm) = next_states.into_iter().nth(selected_index).unwrap();
    
    // Display comprehensive state information similar to run_interactive
    let current_state = new_vm.current_state();
    let mut state_display_info = Vec::new();
    
    // Add state information similar to run_interactive format
    state_display_info.push(format!("======= VM next ======="));
    state_display_info.push(format!(
        "{}:{}:{}",
        name,
        pid,
        if instructions.get(0).and_then(|inst| inst.pos.as_ref()).is_some() {
            source
                .lines()
                .nth(instructions[0].pos.as_ref().unwrap().line)
                .unwrap_or_default()
        } else {
            "?"
        }
    ));
    
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
    
    // To capture output, we need to execute this step and capture the actions
    // Let's execute one random step from the current vm state to get the actions
    let mut execution_vm = vm.clone();
    let step_output: Vec<String>;
    let mut step_debug = String::new();
    let mut message_flow_events: Vec<MessageFlowEvent> = Vec::new();
    
    match execution_vm.next_random() {
        Ok(info) => {
            // Check for invariant errors similar to run_interactive
            if info.invariant_error.is_err() {
                let error_msg = format!("Invariant error: {:?}", info.invariant_error.unwrap_err());
                return Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
                    "error": error_msg,
                    "invariant_violated": true
                })).unwrap());
            }
            
            // Capture debug info
            for inst in info.instructions.iter() {
                step_debug.push_str(&format!("#{}: {}\n", info.prog_id, inst));
            }
            
            // Capture output from actions
            step_output = info.actions.iter().filter_map(|action| {
                if let GlobalAction::Print(s_print) = action {
                    Some(s_print.clone())
                } else {
                    None
                }
            }).collect();
            
            // Capture message flow events
            let get_prog_name = |prog_id: usize, vm_instance: &althread::vm::VM| -> String {
                vm_instance
                    .running_programs
                    .iter()
                    .find(|p| p.id == prog_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("PID_{}", prog_id))
            };
            
            for action in info.actions.iter() {
                match action {
                    GlobalAction::Send(s_chan_name, opt_receiver_info) => {
                        let sender_name = get_prog_name(info.prog_id, &execution_vm);
                        let receiver_id = opt_receiver_info
                            .as_ref()
                            .map(|ri| ri.program_id)
                            .unwrap_or(0);

                        let clock = execution_vm
                            .running_programs
                            .iter()
                            .find(|p| p.id == pid)
                            .map(|p| p.clock)
                            .unwrap_or(0);

                        let event = MessageFlowEvent {
                            sender: pid,
                            receiver: receiver_id,
                            evt_type: SEND,
                            message: s_chan_name.clone(),
                            number: clock,
                            actor_prog_name: sender_name,
                            vm_state: execution_vm.clone(),
                        };
                        message_flow_events.push(event);
                    }
                    _ => {}
                }
            }
        },
        Err(_) => {
            step_output = Vec::new();
            for inst in instructions.iter() {
                step_debug.push_str(&format!("#{}: {}\n", pid, inst));
            }
        }
    }

    web_sys::console::log_1(&JsValue::from_str(&format!("current state: {:?}", current_state)));
    web_sys::console::log_1(&JsValue::from_str(&format!("new vm state: {:?}", new_vm)));

    let result = serde_json::json!({
        "executed_step": {
            "prog_name": name,
            "prog_id": pid,
            "instructions": instructions.iter().map(|inst| inst.to_string()).collect::<Vec<String>>()
        },
        "output": step_output,
        "debug": step_debug,
        // "message_flow_events": message_flow_events,
        // "vm_state": new_vm.clone(),
        // "state_display": state_display_info,
        // "new_state": {
        //     "globals": current_state.0.iter().map(|(key, value)| {
        //         (key.clone(), format!("{:?}", value)) // Convert values to strings
        //     }).collect::<HashMap<_, _>>(),
        //     "channels": current_state.1.iter().map(|((pid, name), values)| {
        //         serde_json::json!({
        //             "key": format!("{},{}", pid, name), // Convert tuple key to a string
        //             "values": values.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>() // Convert values to strings
        //         })
        //     }).collect::<Vec<_>>(),
        //     "programs": current_state.2.iter().enumerate().map(|(index, (memory, instruction_pointer, clock))| {
        //         let prog_name = new_vm.running_programs.get(index)
        //             .map(|p| p.name.clone())
        //             .unwrap_or_else(|| {
        //                 println!("error is here");
        //                 format!("PID_{}", index)
        //             });
        //         serde_json::json!({
        //             "pid": index,
        //             "name": prog_name,
        //             "memory": memory.iter().map(|v| format!("{:?}", v)).collect::<Vec<String>>(), // Convert memory values to strings
        //             "instruction_pointer": instruction_pointer,
        //             "clock": clock
        //         })
        //     }).collect::<Vec<_>>()
        // }
    });

    Ok(serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?)
}

#[wasm_bindgen]
pub fn execute_interactive_step_debug(source: &str, filepath: &str, virtual_fs: JsValue, execution_history: JsValue, selected_index: usize) -> Result<JsValue, JsValue> {
    // Step 1: Parse inputs
    let fs_map: HashMap<String, String> = serde_wasm_bindgen::from_value(virtual_fs)
        .map_err(|e| JsValue::from_str(&format!("Step 1 failed - parse virtual filesystem: {}", e)))?;

    let history: Vec<usize> = serde_wasm_bindgen::from_value(execution_history)
        .map_err(|e| JsValue::from_str(&format!("Step 1 failed - parse execution history: {}", e)))?;

    // Step 2: Create filesystem and input map
    let virtual_filesystem = VirtualFileSystem::new(fs_map);
    let mut input_map = HashMap::new();
    input_map.insert(filepath.to_string(), source.to_string());

    // Step 3: Parse code
    let pairs = althread::parser::parse(&source, filepath)
        .map_err(|e| JsValue::from_str(&format!("Step 3 failed - parse code: {}", format_error_for_web(e))))?;

    // Step 4: Build AST
    let ast = Ast::build(pairs, filepath)
        .map_err(|e| JsValue::from_str(&format!("Step 4 failed - build AST: {}", format_error_for_web(e))))?;

    // Step 5: Compile project
    let compiled_project = ast
        .compile(Path::new(filepath), virtual_filesystem, &mut input_map)
        .map_err(|e| JsValue::from_str(&format!("Step 5 failed - compile project: {}", format_error_for_web(e))))?;

    // Step 6: Create VM
    let mut vm = althread::vm::VM::new(&compiled_project);
    vm.start(0);

    // Step 7: Replay execution history
    for (i, &selected_index_in_history) in history.iter().enumerate() {
        let next_states = vm.next()
            .map_err(|e| JsValue::from_str(&format!("Step 7.{} failed - vm.next() during replay: {}", i, format_error_for_web(e))))?;
        
        if selected_index_in_history >= next_states.len() {
            return Err(JsValue::from_str(&format!("Step 7.{} failed - Invalid selection index {} in history", i, selected_index_in_history)));
        }
        
        let (_, _, _, new_vm) = next_states.into_iter().nth(selected_index_in_history)
            .ok_or_else(|| JsValue::from_str(&format!("Step 7.{} failed - Failed to get selected state", i)))?;
        vm = new_vm;
    }
    
    // Step 8: Get next possible states
    let next_states = vm.next()
        .map_err(|e| JsValue::from_str(&format!("Step 8 failed - vm.next() for current state: {}", format_error_for_web(e))))?;
    
    if selected_index >= next_states.len() {
        return Err(JsValue::from_str(&format!("Step 8 failed - Invalid selection index: {}", selected_index)));
    }
    
    // Step 9: Execute the selected step
    let (selected_name, selected_pid, selected_instructions, new_vm) = next_states.into_iter().nth(selected_index)
        .ok_or_else(|| JsValue::from_str("Step 9 failed - Failed to get selected next state"))?;
    
    vm = new_vm;
    
    // Step 10: Create result
    Ok(serde_wasm_bindgen::to_value(&serde_json::json!({
        "success": true,
        "debug": "All steps completed successfully",
        "executed_step": {
            "prog_name": selected_name,
            "prog_id": selected_pid
        }
    })).map_err(|e| JsValue::from_str(&format!("Step 10 failed - Serialization error: {}", e)))?)
}
