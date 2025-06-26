use fastrand;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use wasm_bindgen::prelude::*;

use althread::{ast::Ast, checker, error::AlthreadError, vm::GlobalAction};
use althread::{vm::instruction::InstructionType};
use althread::{vm::VM};
use althread::ast::token::literal::Literal;

const SEND: u8 = b's';
const RECV: u8 = b'r';


fn error_to_js(err: AlthreadError) -> JsValue {
    serde_wasm_bindgen::to_value(&err).unwrap()
}

#[wasm_bindgen]
pub fn compile(source: &str) -> Result<String, JsValue> {
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(error_to_js)?;

    let ast = Ast::build(pairs).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast.compile(std::path::Path::new("")).map_err(error_to_js)?;
    println!("{}", compiled_project.to_string());
    Ok(format!("{}", compiled_project))
}

pub struct MessageFlowEvent<'a> {
    pub sender: usize, // id of the sending process
    pub receiver: usize,  // id of the receiving process
    pub evt_type: u8, //send or receive
    pub message: String, // for SEND: channel name, for RECV: message content
    pub number: usize, // message sequence number (nmsg_sent for SEND, clock for RECV)
    pub actor_prog_name: String, // Name of the program performing this action
    pub vm_state: VM<'a>, //vm state associated with this event
}

pub struct RunResult<'a> {
    debug: String,
    stdout: Vec<String>,
    message_flow_graph: Vec<MessageFlowEvent<'a>>,
    vm_states: Vec<VM<'a>>,
}

impl <'a> Serialize for MessageFlowEvent<'a> {
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

impl <'a> Serialize for RunResult<'a> {
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
pub fn run(source: &str) -> Result<JsValue, JsValue> {
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(error_to_js)?;

    let ast = Ast::build(pairs).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast.compile(std::path::Path::new("")).map_err(error_to_js)?;

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
            vm_instance.running_programs
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
                    let previous_vm_state = vm_states.get(i-1); // The state before this pop
                    if let Some(prev_vm) = previous_vm_state {
                        if let Some(chan_content_vec) = prev_vm.channels.get_states().get(&(info.prog_id, s.to_string())) {
                            if let Some(Literal::Tuple(ref msg_tuple)) = chan_content_vec.get(0) { // Message popped
                                if msg_tuple.len() >= 2 { // Ensure msg_tuple has at least sender_info and content
                                    if let Some(Literal::Tuple(ref sender_info_tuple)) = msg_tuple.get(0) {
                                        if sender_info_tuple.len() >= 2 { // Ensure sender_info_tuple has senderid and clock
                                            if let (Some(Literal::Int(senderid)), Some(Literal::Int(received_clock))) = 
                                                (sender_info_tuple.get(0), sender_info_tuple.get(1)) {
                                                if let Some(actual_message_content) = msg_tuple.get(1) {
                                                    let receiver_clock = vm.running_programs
                                                        .iter()
                                                        .find(|p| p.id == pid)
                                                        .map(|p| p.clock)
                                                        .unwrap_or(0);
                                                    

                                                    let max_clock = std::cmp::max(*received_clock as usize, receiver_clock as usize) + 1;

                                                    vm.running_programs
                                                        .iter_mut()
                                                        .find(|p| p.id == pid)
                                                        .map(|p| p.clock = max_clock);

                                                    let receiver_name = get_prog_name(info.prog_id, &vm);
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
                    let receiver_id = opt_receiver_info.as_ref().map(|ri| ri.program_id).unwrap_or(0);

                    let clock = vm.running_programs
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
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    // parse code with pest
    let pairs = althread::parser::parse(&source).map_err(error_to_js)?;

    let ast = Ast::build(pairs).map_err(error_to_js)?;

    println!("{}", &ast);

    let compiled_project = ast.compile(std::path::Path::new("")).map_err(error_to_js)?;

    let checked = checker::check_program(&compiled_project).map_err(error_to_js)?;

    Ok(serde_wasm_bindgen::to_value(&checked).unwrap())
}
