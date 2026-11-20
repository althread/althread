use core::panic;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    rc::Rc,
};

use channels::{ChannelLinkKey, Channels, ChannelsState};
use fastrand::Rng;

use instruction::{Instruction, InstructionType, ProgramCode};
use running_program::RunningProgramState;
use serde::{ser::SerializeStruct, Serialize, Serializer};

use crate::{
    ast::{
        statement::{expression::LocalExpressionNode, waiting_case::WaitDependency},
        token::{datatype::DataType, literal::Literal},
    },
    compiler::{stdlib::Stdlib, CompiledProject, FunctionDefinition},
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
};

pub mod channels;
pub mod instruction;
pub mod running_program;

pub type Memory = Vec<Literal>;
pub type GlobalMemory = BTreeMap<String, Literal>;

#[derive(Debug)]
pub struct ExecutionStepInfo<I = Instruction> {
    pub prog_name: String,
    pub prog_id: usize,
    pub instructions: Vec<I>,
    pub invariant_error: AlthreadResult<i32>,
    pub actions: Vec<GlobalAction>,
}

impl ExecutionStepInfo<&Instruction> {
    fn into_owned(self) -> ExecutionStepInfo {
        ExecutionStepInfo {
            prog_name: self.prog_name,
            prog_id: self.prog_id,
            instructions: self.instructions.into_iter().cloned().collect(),
            invariant_error: self.invariant_error,
            actions: self.actions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScheduledStep {
    Program(usize),
    Deliver(ChannelLinkKey),
}

fn str_to_expr_error(pos: Option<Pos>) -> impl Fn(String) -> AlthreadError {
    return move |msg| AlthreadError::new(ErrorType::ExpressionError, pos.clone(), msg);
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct ProcessInfo {
    pub process_id: usize,
    pub process_name: String,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct ChannelInfo {
    pub channel_name: String,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct SendInfo {
    pub from: ProcessInfo,
    pub to: ChannelInfo,
    pub message: Literal,
    pub n_msg: usize,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct DeliverInfo {
    pub from: ProcessInfo,
    pub to: ProcessInfo,
    pub channel_name: String,
    pub message: Literal,
    pub sender_clock: usize,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum GlobalAction {
    StartProgram(String, usize, Literal, Option<usize>, Option<Pos>),
    Print(String),
    Write(String),
    Send(SendInfo),
    Broadcast(Vec<SendInfo>),
    Deliver(DeliverInfo),
    Connect(usize, String),
    EndProgram,
    Wait,
    Exit,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GlobalActions {
    pub actions: Vec<GlobalAction>,
    pub wait: bool,
    pub end: bool,
}

#[derive(Debug, Clone)]
pub struct VM<'a> {
    pub globals: GlobalMemory,
    pub channels: Channels,
    pub running_programs: Vec<RunningProgramState<'a>>,
    pub programs_code: &'a HashMap<String, ProgramCode>,
    pub user_funcs: &'a HashMap<String, FunctionDefinition>,
    pub executable_programs: BTreeSet<usize>, // needs to be sorted to have a deterministic behavior
    pub always_conditions: &'a Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>,
    pub program_debug_info: &'a HashMap<String, crate::compiler::ProgramDebugInfo>,

    /// The programs that are waiting for a condition to be true
    /// The condition depends on the global variables that are in the HashSet
    waiting_programs: HashMap<usize, WaitDependency>,
    next_program_id: usize,
    rng: Rng,

    pub stdlib: Rc<Stdlib>,
}

impl<'a> VM<'a> {
    pub fn new(compiled_project: &'a CompiledProject) -> Self {
        Self {
            globals: compiled_project.global_memory.clone(),
            channels: Channels::new(),
            running_programs: Vec::new(),
            executable_programs: BTreeSet::new(),
            programs_code: &compiled_project.programs_code,
            user_funcs: &compiled_project.user_functions,
            always_conditions: &compiled_project.always_conditions,
            program_debug_info: &compiled_project.program_debug_info,
            next_program_id: 0,
            waiting_programs: HashMap::new(),
            rng: Rng::new(),
            stdlib: compiled_project.stdlib.clone(),
        }
    }

    fn process_runtime_actions(&mut self, actions: &[GlobalAction]) -> bool {
        let mut need_to_check_invariants = false;

        for action in actions {
            match action {
                GlobalAction::Wait => {
                    unreachable!("await action should not be in the list of actions");
                }
                GlobalAction::Deliver(_) => {
                    unreachable!("Deliver is VM-generated and cannot come from a program step")
                }
                GlobalAction::Connect(sender_id, sender_channel) => {
                    if let Some(dependency) = self.waiting_programs.get(sender_id) {
                        if dependency.channels_connection.contains(sender_channel) {
                            self.waiting_programs.remove(sender_id);
                            self.executable_programs.insert(*sender_id);
                        }
                    }
                }
                GlobalAction::Write(var_name) => {
                    self.waiting_programs.retain(|prog_id, dependencies| {
                        if dependencies.variables.contains(var_name) {
                            self.executable_programs.insert(*prog_id);
                            return false;
                        }
                        true
                    });

                    need_to_check_invariants = true;
                }
                GlobalAction::StartProgram(name, pid, args, caller_program_id, call_site_pos) => {
                    self.run_program(
                        name,
                        *pid,
                        args.clone(),
                        *caller_program_id,
                        call_site_pos.clone(),
                    );
                }
                GlobalAction::EndProgram => {
                    panic!("EndProgram action should not be in the list of actions");
                }
                GlobalAction::Exit => self.running_programs.clear(),
                GlobalAction::Print(_) => {}
                GlobalAction::Send(_) => {}
                GlobalAction::Broadcast(_) => {}
            }
        }

        need_to_check_invariants
    }

    fn run_program(
        &mut self,
        program_name: &str,
        pid: usize,
        args: Literal,
        caller_program_id: Option<usize>,
        call_site_pos: Option<Pos>,
    ) {
        assert!(
            self.running_programs.get(pid).is_none(),
            "program with id {} already exists",
            pid
        );

        let mut new_program = RunningProgramState::new(
            pid,
            program_name.to_string(),
            &self.programs_code[program_name],
            self.user_funcs,
            args,
            self.stdlib.clone(),
        );

        // Set the caller context
        new_program.caller_program_id = caller_program_id;
        new_program.call_site_pos = call_site_pos;

        self.running_programs.insert(pid, new_program);
        self.executable_programs.insert(pid);
    }

    pub fn start(&mut self, seed: u64) {
        self.rng = Rng::with_seed(seed);
        self.next_program_id = 1;
        self.run_program("main", 0, Literal::empty_tuple(), None, None); // No caller for main
    }

    fn wait_dependencies_satisfied_now(
        &self,
        program_id: usize,
        dependencies: &WaitDependency,
    ) -> bool {
        dependencies
            .channels_state
            .iter()
            .any(|channel_name| self.channels.has_buffered_message(program_id, channel_name))
            || dependencies
                .channels_connection
                .iter()
                .any(|channel_name| self.channels.has_connection_from(program_id, channel_name))
    }

    pub fn next_random(&mut self) -> AlthreadResult<ExecutionStepInfo> {
        let mut candidates: Vec<ScheduledStep> = self
            .executable_programs
            .iter()
            .copied()
            .map(ScheduledStep::Program)
            .collect();

        candidates.extend(
            self.channels
                .pending_links()
                .into_iter()
                .map(ScheduledStep::Deliver),
        );

        if candidates.is_empty() {
            return Err(AlthreadError::new(
                ErrorType::RuntimeError,
                None,
                format!(
                    "All programs are waiting, deadlock:\n{}",
                    self.waiting_programs
                        .iter()
                        .map(|(id, dep)| format!(
                            "-{}#{} at line {}: {:?}",
                            self.running_programs.get(*id).unwrap().name,
                            id,
                            self.running_programs
                                .get(*id)
                                .unwrap()
                                .current_instruction()
                                .unwrap()
                                .pos
                                .as_ref()
                                .unwrap()
                                .line(),
                            dep
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            ));
        }

        let choice_idx = self.rng.usize(0..candidates.len());
        let choice = candidates.swap_remove(choice_idx);

        let program_id = match choice {
            ScheduledStep::Program(pid) => pid,
            ScheduledStep::Deliver(link) => return Ok(self.step_delivery(link).into_owned()),
        };

        let program = self
            .running_programs
            .get_mut(program_id)
            .expect("program is executable but not found in running programs");

        let mut exec_info = ExecutionStepInfo {
            prog_name: program.name.clone(),
            prog_id: program_id,
            instructions: Vec::new(),
            actions: Vec::new(),
            invariant_error: Ok(0),
        };

        let (actions, executed_instructions) = program.next_global(
            &mut self.globals,
            &mut self.channels,
            &mut self.next_program_id,
        )?;
        // maybe should be replace to avoid recurrent calls
        if actions.wait {
            if !actions.actions.is_empty() {
                return Err(AlthreadError::new(
                    ErrorType::RuntimeError,
                    None,
                    "A process reached `wait` after producing global actions in the same step. Wait guards must stay side-effect free, and successful wait branches must complete before a later wait is reported.".to_string(),
                ));
            }

            let dependencies = match &self
                .running_programs
                .get(program_id)
                .expect("program is waiting but not found in running programs")
                .current_instruction()
                .expect("waiting on no instruction")
                .control
            {
                InstructionType::WaitStart { dependencies, .. } => dependencies.clone(),
                _ => unreachable!("waiting on an instruction that is not a WaitStart instruction"),
            };

            if self.wait_dependencies_satisfied_now(program_id, &dependencies) {
                return self.next_random();
            }

            self.executable_programs.remove(&program_id);
            self.waiting_programs.insert(program_id, dependencies);
            return self.next_random();
        }

        let need_to_check_invariants = self.process_runtime_actions(&actions.actions);
        if actions.end {
            let remove_id = program_id;
            self.executable_programs.remove(&remove_id);
            self.waiting_programs.remove(&remove_id);
        }

        // TODO this method should be modified so eventually violation generate an error,
        // for example by having a encounterd eventually counter, if the final VM's counter is == 0 no block validated eventually and path is wrong
        if need_to_check_invariants {
            exec_info.invariant_error = self.check_invariants();
        }

        exec_info.instructions = executed_instructions;
        exec_info.actions = actions.actions;

        Ok(exec_info)
    }

    pub fn next_step_pid(&mut self, pid: usize) -> AlthreadResult<Option<ExecutionStepInfo>> {
        Ok(self
            .step_program(pid, true)?
            .map(ExecutionStepInfo::into_owned))
    }

    fn step_program(
        &mut self,
        pid: usize,
        check_invariants: bool,
    ) -> AlthreadResult<Option<ExecutionStepInfo<&'a Instruction>>> {
        let program = self
            .running_programs
            .get_mut(pid)
            .expect("program is executable but not found in running programs");

        if program.has_terminated() {
            return Ok(None);
        }

        let mut exec_info = ExecutionStepInfo {
            prog_name: program.name.clone(),
            prog_id: pid,
            instructions: Vec::new(),
            actions: Vec::new(),
            invariant_error: Ok(0),
        };

        let (actions, executed_instructions) = program.next_global_borrowed(
            &mut self.globals,
            &mut self.channels,
            &mut self.next_program_id,
        )?;
        // maybe should be replace to avoid recurrent calls
        if actions.wait {
            if !actions.actions.is_empty() {
                return Err(AlthreadError::new(
                    ErrorType::RuntimeError,
                    None,
                    "A process reached `wait` after producing global actions in the same step. Wait guards must stay side-effect free, and successful wait branches must complete before a later wait is reported.".to_string(),
                ));
            }

            let dependencies = match &self
                .running_programs
                .get(pid)
                .expect("program is waiting but not found in running programs")
                .current_instruction()
                .expect("waiting on no instruction")
                .control
            {
                InstructionType::WaitStart { dependencies, .. } => dependencies.clone(),
                _ => unreachable!("waiting on an instruction that is not a WaitStart instruction"),
            };

            if self.wait_dependencies_satisfied_now(pid, &dependencies) {
                return self.step_program(pid, check_invariants);
            }

            self.executable_programs.remove(&pid);
            self.waiting_programs.insert(pid, dependencies);
            return Ok(None);
        }

        let need_to_check_invariants = self.process_runtime_actions(&actions.actions);
        if actions.end {
            let remove_id = pid;
            self.executable_programs.remove(&remove_id);
            self.waiting_programs.remove(&remove_id);
        }

        exec_info.instructions = executed_instructions;
        exec_info.actions = actions.actions;
        if check_invariants && need_to_check_invariants {
            exec_info.invariant_error = self.check_invariants();
        }

        Ok(Some(exec_info))
    }

    pub fn get_program(&self, pid: usize) -> &RunningProgramState<'_> {
        self.running_programs.get(pid).expect("program not found")
    }

    /**
     * List all the next possible state of the VM
     */
    pub fn next(
        &self,
    ) -> AlthreadResult<Vec<(String, usize, Vec<Instruction>, Vec<GlobalAction>, VM<'a>)>> {
        Ok(self
            .successors()?
            .into_iter()
            .map(|(_, step, vm)| {
                let step = step.into_owned();
                (
                    step.prog_name,
                    step.prog_id,
                    step.instructions,
                    step.actions,
                    vm,
                )
            })
            .collect())
    }

    pub(crate) fn successors(
        &self,
    ) -> AlthreadResult<Vec<(ScheduledStep, ExecutionStepInfo<&'a Instruction>, Self)>> {
        if self.running_programs.is_empty() {
            return Ok(Vec::new());
        }
        let steps = self
            .executable_programs
            .iter()
            .filter(|pid| !self.waiting_programs.contains_key(pid))
            .copied()
            .map(ScheduledStep::Program)
            .chain(
                self.channels
                    .pending_links()
                    .into_iter()
                    .map(ScheduledStep::Deliver),
            );
        let mut successors = Vec::new();
        for step in steps {
            let mut vm = self.clone();
            if let Some(info) = vm.advance(&step)? {
                successors.push((step, info, vm));
            }
        }
        Ok(successors)
    }

    pub(crate) fn advance(
        &mut self,
        step: &ScheduledStep,
    ) -> AlthreadResult<Option<ExecutionStepInfo<&'a Instruction>>> {
        match step {
            ScheduledStep::Program(pid) => self.step_program(*pid, false),
            ScheduledStep::Deliver(link) => Ok(Some(self.step_delivery(link.clone()))),
        }
    }

    fn step_delivery(&mut self, link: ChannelLinkKey) -> ExecutionStepInfo<&'a Instruction> {
        let delivery_info = self
            .channels
            .deliver_one(link)
            .expect("pending link must have a deliverable message");

        if let Some(dependency) = self.waiting_programs.get(&delivery_info.to.program_id) {
            if dependency
                .channels_state
                .contains(&delivery_info.to.channel_name)
            {
                self.waiting_programs.remove(&delivery_info.to.program_id);
                self.executable_programs.insert(delivery_info.to.program_id);
            }
        }

        let from_name = self
            .running_programs
            .get(delivery_info.from_program_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("PID_{}", delivery_info.from_program_id));
        let to_name = self
            .running_programs
            .get(delivery_info.to.program_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("PID_{}", delivery_info.to.program_id));

        let (sender_id, sender_clock) = channels::message_trace_id(&delivery_info.message)
            .unwrap_or((delivery_info.from_program_id, 0));

        ExecutionStepInfo {
            prog_name: format!(
                "__deliver__ {}#{}",
                delivery_info.to.channel_name, delivery_info.to.program_id
            ),
            prog_id: delivery_info.to.program_id,
            instructions: Vec::new(),
            invariant_error: Ok(0),
            actions: vec![GlobalAction::Deliver(crate::vm::DeliverInfo {
                from: crate::vm::ProcessInfo {
                    process_id: sender_id,
                    process_name: from_name,
                },
                to: crate::vm::ProcessInfo {
                    process_id: delivery_info.to.program_id,
                    process_name: to_name,
                },
                channel_name: delivery_info.to.channel_name,
                message: delivery_info.message,
                sender_clock,
            })],
        }
    }

    pub fn is_finished(&self) -> bool {
        self.executable_programs.is_empty()
            && !self.channels.has_pending_deliveries()
            && self.waiting_programs.is_empty()
    }

    pub fn new_memory() -> Memory {
        Vec::<Literal>::new()
    }

    pub fn current_state(
        &self,
    ) -> (
        &GlobalMemory,
        &ChannelsState,
        Vec<(&Vec<Literal>, usize, usize)>,
    ) {
        let local_states = self
            .running_programs
            .iter()
            .map(|prog| prog.current_state())
            .collect();

        (&self.globals, self.channels.state(), local_states)
    }

    //42 this checks invariants (the always block conditions)
    // return OK(1) if all invariants hold
    pub fn check_invariants(&self) -> AlthreadResult<i32> {
        for (_deps, read_vars, expr, pos) in self.always_conditions.iter() {
            //if _deps.contains(&var_name) { //TODO improve by checking if the variable is in the dependencies
            // Check if the condition is true
            // create a small memory stack with the value of the variables
            let mut memory = Vec::new();
            for var_name in read_vars.iter() {
                if let Some(proc_name) = var_name.strip_prefix("$.procs.") {
                    let values = self
                        .running_programs
                        .iter()
                        .filter(|p| p.name == proc_name)
                        .map(|p| Literal::Process(p.name.clone(), p.id))
                        .collect::<Vec<_>>();
                    memory.push(Literal::List(
                        DataType::Process(proc_name.to_string()),
                        values,
                    ));
                } else {
                    memory.push(
                        self.globals
                            .get(var_name)
                            .expect(format!("global variable '{}' not found", var_name).as_str())
                            .clone(),
                    );
                }
            }
            match expr.eval_with_scope(&memory, read_vars, self) {
                Ok(cond) => {
                    if !cond.is_true() {
                        return Err(AlthreadError::new(
                            ErrorType::InvariantError,
                            Some(pos.clone()),
                            "The invariant is not respected".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(pos.clone()),
                        e,
                    ));
                }
            }

            //}
        }

        Ok(1)
    }
}

impl<'a> fmt::Display for VM<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Globals:")?;
        for (name, val) in self.globals.iter() {
            writeln!(f, "  {}: {}", name, val)?;
        }
        writeln!(f, "'main' stack:")?;
        /*for val in self.running_programs.get(&0).expect("no program is not running, cannot print the VM").memory.iter() {
            writeln!(f," - {}", val)?;
        }*/
        Ok(())
    }
}

impl VM<'_> {
    fn hash_execution_state<H: Hasher>(&self, state: &mut H) {
        self.globals.hash(state);
        self.executable_programs.hash(state);
        self.next_program_id.hash(state);
        // The dependency sets are checked by equality.
        let mut waiting_ids: Vec<_> = self.waiting_programs.keys().collect();
        waiting_ids.sort_unstable();
        waiting_ids.hash(state);
    }

    fn same_execution_state(&self, other: &Self) -> bool {
        self.globals == other.globals
            && self.executable_programs == other.executable_programs
            && self.waiting_programs == other.waiting_programs
            && self.next_program_id == other.next_program_id
    }
}

impl Hash for VM<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_execution_state(state);
        self.channels.hash(state);
        self.running_programs.hash(state);
    }
}

impl PartialEq for VM<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.same_execution_state(other)
            && self.channels == other.channels
            && self.running_programs == other.running_programs
            && (std::ptr::eq(self.programs_code, other.programs_code)
                || self.programs_code == other.programs_code)
    }
}

impl Eq for VM<'_> {}

/// State identity within one compiled project, ignoring message trace metadata.
pub(crate) struct CheckerState<'a>(pub(crate) Rc<VM<'a>>);

impl Hash for CheckerState<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let vm = &self.0;
        vm.hash_execution_state(state);
        vm.channels.hash_execution_state(state);
        vm.running_programs.len().hash(state);
        for program in &vm.running_programs {
            program.hash_execution_state(state);
        }
    }
}

impl PartialEq for CheckerState<'_> {
    fn eq(&self, other: &Self) -> bool {
        let left = &self.0;
        let right = &other.0;
        std::ptr::eq(left.programs_code, right.programs_code)
            && left.same_execution_state(right)
            && left.channels.same_execution_state(&right.channels)
            && left.running_programs.len() == right.running_programs.len()
            && left
                .running_programs
                .iter()
                .zip(&right.running_programs)
                .all(|(left, right)| left.same_execution_state(right))
    }
}

impl Eq for CheckerState<'_> {}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::Path};

    use crate::{module_resolver::VirtualFileSystem, parser};

    use super::*;

    fn compile_vm(source: &str) -> VM<'static> {
        let mut input_map = HashMap::new();
        input_map.insert("main.alt".to_string(), source.to_string());

        let ast = parser::parse_ast(source, "main.alt").unwrap();
        let compiled_project = Box::new(
            ast.compile(
                Path::new("main.alt"),
                VirtualFileSystem::new(input_map.clone()),
                &mut input_map,
            )
            .unwrap(),
        );
        let compiled_project = Box::leak(compiled_project);

        let mut vm = VM::new(compiled_project);
        vm.start(0);
        vm
    }

    fn step_program_to_wait_start(vm: &mut VM<'_>, pid: usize) {
        loop {
            let instruction = vm
                .get_program(pid)
                .current_instruction()
                .unwrap()
                .control
                .clone();
            if matches!(instruction, InstructionType::WaitStart { .. }) {
                break;
            }
            assert!(vm.next_step_pid(pid).unwrap().is_some());
        }
    }

    fn deliver_pending_message(vm: &mut VM<'_>, to_pid: usize, to_channel: &str) {
        let link = vm
            .channels
            .pending_links()
            .into_iter()
            .find(|(_, _, pid, channel)| *pid == to_pid && channel == to_channel)
            .unwrap();
        vm.channels.deliver_one(link).unwrap();

        if let Some(dependency) = vm.waiting_programs.get(&to_pid) {
            if dependency.channels_state.contains(to_channel) {
                vm.waiting_programs.remove(&to_pid);
                vm.executable_programs.insert(to_pid);
            }
        }
    }

    #[test]
    fn wait_first_evaluates_all_receive_guards_atomically() {
        let source = r#"
program sender() {}

main {
    let a = run sender();
    let b = run sender();

    channel a.out (int)> self.fromA;
    channel b.out (int)> self.fromB;

  loop await first {
    receive fromA(v) => print("A", v);
    receive fromB(v) => print("B", v);
  }
}
        "#;

        let mut vm = compile_vm(source);
        step_program_to_wait_start(&mut vm, 0);

        let program = vm.running_programs.get_mut(0).unwrap();
        let (actions, executed_instructions) = program
            .next_global(&mut vm.globals, &mut vm.channels, &mut vm.next_program_id)
            .unwrap();

        assert!(actions.wait);
        assert!(actions.actions.is_empty());
        assert!(executed_instructions
            .iter()
            .any(|inst| { inst.control == InstructionType::ChannelPeek("fromA".to_string()) }));
        assert!(executed_instructions
            .iter()
            .any(|inst| { inst.control == InstructionType::ChannelPeek("fromB".to_string()) }));
    }

    #[test]
    fn wait_does_not_park_when_a_watched_channel_already_has_data() {
        let source = r#"
program sender() {
  send out(1);
}

main {
  let a = run sender();
  let b = run sender();

  channel a.out (int)> self.fromA;
  channel b.out (int)> self.fromB;

  loop await first {
    receive fromA(v) => print("A", v);
    receive fromB(v) => print("B", v);
  }
}
        "#;

        let mut vm = compile_vm(source);
        step_program_to_wait_start(&mut vm, 0);

        let _sender_step = vm.next_step_pid(1).unwrap();
        deliver_pending_message(&mut vm, 0, "fromA");

        let step = vm.next_step_pid(0).unwrap();

        assert!(step.is_some());
        assert!(vm.executable_programs.contains(&0));
        assert!(!vm.waiting_programs.contains_key(&0));
        assert!(step
            .as_ref()
            .unwrap()
            .actions
            .iter()
            .any(|action| matches!(action, GlobalAction::Print(message) if message == "A 1")));
    }

    #[test]
    fn wait_seq_restarts_atomic_evaluation_after_a_successful_block() {
        let source = r#"
shared {
    let Ready = true;
}

main {
    channel self.out (string)> self.in;
    send out("tail");

    await seq {
        (Ready) => {
            print("CASE 1");
            Ready = false;
        }
        receive in(msg) => {
            print("TAIL", msg);
        }
    }
}
                "#;

        let mut vm = compile_vm(source);
        step_program_to_wait_start(&mut vm, 0);
        deliver_pending_message(&mut vm, 0, "in");

        let first_step = vm.next_step_pid(0).unwrap().unwrap();

        assert!(first_step.actions.iter().any(|action| matches!(
            action,
            GlobalAction::Print(message) if message == "CASE 1"
        )));

        let mut saw_tail = first_step.actions.iter().any(|action| {
            matches!(
                action,
                GlobalAction::Print(message) if message == "TAIL tail"
            )
        });
        for _ in 0..4 {
            if saw_tail {
                break;
            }
            let Some(step) = vm.next_step_pid(0).unwrap() else {
                break;
            };
            if step.actions.iter().any(|action| {
                matches!(
                    action,
                    GlobalAction::Print(message) if message == "TAIL tail"
                )
            }) {
                saw_tail = true;
            }
        }

        assert!(saw_tail);
    }

    #[test]
    fn wait_seq_does_not_skip_a_blocked_matched_case() {
        let source = r#"
shared {
    let Ready = true;
}

main {
    channel self.block_out (string)> self.block_in;
    channel self.tail_out (string)> self.tail_in;
    send tail_out("tail");

    await seq {
        (Ready) => {
            await receive block_in(msg) => {
                print("BLOCK", msg);
            }
            Ready = false;
        }
        receive tail_in(msg) => {
            print("TAIL", msg);
        }
    }
}
                "#;

        let mut vm = compile_vm(source);
        step_program_to_wait_start(&mut vm, 0);
        deliver_pending_message(&mut vm, 0, "tail_in");

        let mut observed_actions = Vec::new();
        let mut blocked = false;
        for _ in 0..4 {
            match vm.next_step_pid(0).unwrap() {
                Some(step) => observed_actions.extend(step.actions),
                None => {
                    blocked = true;
                    break;
                }
            }
        }

        assert!(blocked);
        assert!(vm.waiting_programs.contains_key(&0));
        assert!(!observed_actions.iter().any(|action| matches!(
            action,
            GlobalAction::Print(message) if message == "TAIL tail"
        )));

        let _ = vm.channels.send(
            0,
            "block_out".to_string(),
            Literal::Tuple(vec![Literal::String("go".to_string())]),
            1,
        );
        deliver_pending_message(&mut vm, 0, "block_in");

        let mut resumed_actions = Vec::new();
        for _ in 0..8 {
            let Some(step) = vm.next_step_pid(0).unwrap() else {
                break;
            };
            resumed_actions.extend(step.actions);
        }

        let print_messages = resumed_actions
            .iter()
            .filter_map(|action| match action {
                GlobalAction::Print(message) => Some(message.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        let block_index = print_messages
            .iter()
            .position(|message| *message == "BLOCK go")
            .unwrap();
        let tail_index = print_messages
            .iter()
            .position(|message| *message == "TAIL tail")
            .unwrap();

        assert!(block_index < tail_index);
    }
}

#[derive(Serialize)]
struct SerializableRunningProgramStateForJs<'b> {
    pid: usize,
    name: &'b str,
    memory: &'b Vec<Literal>,   // The program's stack
    instruction_pointer: usize, // The program's PC
    clock: usize,               // Program's logical clock (if you have one)
    line: usize,                // Current line number
                                // Add any other per-program fields you want to expose
}

impl<'a> Serialize for VM<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (globals, channels, _locals) = self.current_state();

        // Number of fields in the serialized VM struct
        let mut s = serializer.serialize_struct("VM_JS", 6)?; // Using "VM_JS" for clarity

        s.serialize_field("globals", globals)?;
        s.serialize_field("channels", channels)?;

        let serializable_program_states: Vec<SerializableRunningProgramStateForJs> = self
            .running_programs // Iterate over all currently running programs
            .iter()
            .map(|prog_state| {
                let (memory, instruction_pointer, clock) = prog_state.current_state();
                let line = self
                    .programs_code
                    .get(&prog_state.name)
                    .and_then(|code| code.instructions.get(instruction_pointer))
                    .and_then(|inst| inst.pos.as_ref())
                    .map(|pos| pos.line())
                    .unwrap_or(0);

                SerializableRunningProgramStateForJs {
                    pid: prog_state.id,
                    name: &prog_state.name,
                    memory,
                    instruction_pointer,
                    clock,
                    line,
                }
            })
            .collect();

        s.serialize_field("locals", &serializable_program_states)?;
        s.serialize_field(
            "pending_deliveries",
            &self.channels.get_pending_deliveries(),
        )?;
        s.serialize_field("waiting_send", &self.channels.get_waiting_send())?;
        s.serialize_field("channel_connections", &self.channels.get_connections())?;

        s.end()
    }
}
