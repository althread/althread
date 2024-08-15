use std::{cell::RefCell, rc::Rc};

use crate::env::symbol_table::symbol_table_stack::SymbolTableStack;

#[derive(Debug)]
pub struct Process {
    pub symbol_table: Rc<RefCell<SymbolTableStack>>,
    pub position: usize,             // the current position in the AST
    pub child: Option<Box<Process>>, // the child scope environment
}

impl Process {
    pub fn new(symbol_table: &Rc<RefCell<SymbolTableStack>>) -> Self {
        Self {
            position: 0,
            child: None,
            symbol_table: Rc::clone(symbol_table),
        }
    }

    pub fn consume(&mut self) {
        self.clean();
        self.position += 1;
    }

    pub fn reset(&mut self) {
        self.clean();
        self.position = 0;
    }

    pub fn clean(&mut self) {
        self.child = None;
    }

    pub fn get_child(&mut self) -> &mut Process {
        if self.child.is_none() {
            self.child = Some(Box::new(Self::new(&self.symbol_table)));
        }

        self.child.as_mut().unwrap()
    }
}

// #[derive(Debug)]
// pub struct Env {
//     pub process_table: Rc<RefCell<ProcessTable>>,
//     pub global_table: Rc<RefCell<SymbolTable>>,
//     pub running_process: Rc<RefCell<RunningProcess>>,
// }

// impl Env {
//     pub fn new() -> Self {
//         Self {
//             process_table: Rc::new(RefCell::new(ProcessTable::new())),
//             global_table: Rc::new(RefCell::new(SymbolTable::new())),
//             running_process: Rc::new(RefCell::new(RunningProcess::new())),
//         }
//     }

//     pub fn run(&mut self, ast: &Ast) {
//         if let Some(_global_block) = &ast.global_block {
//             println!("Run global block");
//         }

//         for (name, _block) in &ast.condition_blocks {
//             println!("Run condition block {}", name);
//         }

//         for (name, _block) in &ast.process_blocks {
//             let process = Process::new(
//                 &self.global_table,
//                 &self.process_table,
//                 &self.running_process,
//             );

//             self.process_table
//                 .borrow_mut()
//                 .insert(name.clone(), process);
//         }

//         self.running_process
//             .borrow_mut()
//             .push("main".to_string(), &self.process_table);

//         println!("{}", self.process_table.borrow());
//         println!("{:?}", self.running_process.borrow());

//         // TODO : Boucle principale
//     }
// }
