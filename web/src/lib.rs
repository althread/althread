use fastrand;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use wasm_bindgen::prelude::*;

use althread::{ast::Ast, checker, error::AlthreadError, vm::GlobalAction};
use althread::{vm::instruction::InstructionType};
use althread::{vm::VM};
use althread::ast::token::literal::Literal; ///////////////

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

    let compiled_project = ast.compile().map_err(error_to_js)?;

    Ok(format!("{}", compiled_project))
}

pub struct MessageFlowEvent<'a> {
    sender: usize, // id of the sending process
    receiver: Option<usize>,  // id of the receiving process
    evt_type: u8, //send or receive
    message: String, // the channel
    number: usize, // number of the message
    vm_state: VM<'a>, //vm state associated with this event
}

pub struct RunResult<'a> {
    debug: String,
    stdout: Vec<String>,
    messageFlow_graph: Vec<MessageFlowEvent<'a>>,
    vm_states: Vec<VM<'a>>,

}

impl <'a> Serialize for MessageFlowEvent<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("MessageFlowEvent", 6)?;
        state.serialize_field("sender", &self.sender)?;
        state.serialize_field("receiver", &self.receiver)?;
        state.serialize_field("evt_type", &self.evt_type)?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("number", &self.number)?;
        state.serialize_field("vm_state", &self.vm_state)?;
        state.end()
    }
}

impl <'a> Serialize for RunResult<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("RunResult", 3)?;
        state.serialize_field("debug", &self.debug)?;
        state.serialize_field("stdout", &self.stdout)?;
        state.serialize_field("messageFlow_graph", &self.messageFlow_graph)?;
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

    let compiled_project = ast.compile().map_err(error_to_js)?;

    let mut vm = althread::vm::VM::new(&compiled_project);

    vm.start(fastrand::u64(0..(1 << 32)));

    let mut result = String::new();
    let mut stdout = vec![];
    let mut messageFlow_graph = Vec::new();
    let mut nmsg_sent:usize = 0;
    let mut vm_states = Vec::new(); /////
    let mut i = 0; //index for vm_states

    for _ in 0..100000 {
        
        if vm.is_finished() {
            break;
        }
        let info = vm.next_random().map_err(error_to_js)?;
        vm_states.push(vm.clone()); /////////////////////
        for inst in info.instructions.iter() {
            result.push_str(&format!("#{}: {}\n", info.prog_id, inst));
            /////////////////////////////////////////////////////////////////
            if let InstructionType::ChannelPop(ref s) = &inst.control{
                if i>0 { //first vm shouldn't be able to read anything 
                    let previous_vm = vm_states.get(i-1);
                    if let Some(chan_content) = previous_vm.unwrap().channels.getStates().get(&(info.prog_id, s.to_string())){
                        if let Some(Literal::Tuple(ref msg)) = chan_content.get(0){ //messsage popped
                            if let Some(Literal::Tuple(ref sender_info)) = msg.get(0){
                                if let Some(Literal::Int(senderid)) = sender_info.get(0){ //sender id
                                    if let Some(Literal::Int(clock)) = sender_info.get(1){
                                        if let Some(content) = msg.get(1){ //message content
                                            let event = MessageFlowEvent{
                                                sender: *senderid as usize,
                                                receiver: Some(info.prog_id),
                                                evt_type: RECV,
                                                message: content.to_string(),
                                                number: *clock as usize,
                                                vm_state: vm.clone(),
                                            };
                                            messageFlow_graph.push(event);
                                        }
                                    }  
                                }
                            }
                        }
                    }  
                }       
            }
            ///////////////////////////////////////////////////////////////////     
        }
    
        for p in info.actions.iter() {
            if let GlobalAction::Print(s) = p {
                stdout.push(s.clone());
            }
            match p{
                GlobalAction::Send(s, Some(receiverinfos)) => {
                  //  if i>0 {
                    //    let previous_vm = vm_states.get(i-1);
                   // if let Some(chan_content) = previous_vm.unwrap().channels.getStates().get(&(info.prog_id, s.to_string())){
                      //  if let Some(Literal::Tuple(ref msg)) = chan_content.get(0){ //messsage popped
                        //    if let Some(Literal::Tuple(ref sender_info)) = msg.get(0){
                          //      if let Some(Literal::Int(clock)) = sender_info.get(1){
                                    nmsg_sent+=1;
                                    let event = MessageFlowEvent {
                                        sender: info.prog_id, 
                                        receiver: Some(receiverinfos.program_id),
                                        evt_type: SEND,
                                        message: s.clone(), //channel name, just to fill the field
                                        number : nmsg_sent,
                                        vm_state: vm.clone(),
                                    };
                                    messageFlow_graph.push(event);
                            //    }
                            //}
                        //}
                    //}
                
                }
                GlobalAction::Send(s, None) => { //broadcast
                    nmsg_sent+=1;
                    let event = MessageFlowEvent {
                        sender: info.prog_id,
                        receiver: None,
                        evt_type: SEND,
                        message: s.clone(), //channel name,  just to fill the field
                        number: nmsg_sent,
                        vm_state: vm.clone(),
                    };
                    messageFlow_graph.push(event);
                    
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
        i+=1;
    }
    
    Ok(serde_wasm_bindgen::to_value(&RunResult {
        debug: result,
        stdout,
        messageFlow_graph,
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

    let compiled_project = ast.compile().map_err(error_to_js)?;

    let checked = checker::check_program(&compiled_project).map_err(error_to_js)?;

    Ok(serde_wasm_bindgen::to_value(&checked).unwrap())
}
