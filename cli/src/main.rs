use std::{fs, io::Read, process::exit};

mod args;
use args::{CliArguments, Command, CompileCommand, RunCommand};
use clap::Parser;

use althread::ast::Ast;
use althread::error::AlthreadError;


fn main() {
    let cli_args = CliArguments::parse();

    match &cli_args.command {
        Command::Compile(command) => compile_command(&command.clone()),
        Command::Run(command) => run_command(&command.clone()),
    }

}



pub fn compile_command(cli_args: &CompileCommand) {
    // Read file
    let source = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf);
            String::from_utf8(buf).expect("Could not read stdin")
        },
        args::Input::Path(path) => {
            fs::read_to_string(&path).expect("Could not read file")
        },
        
    };

    // parse code with pest
    let pairs = althread::parser::parse(&source).unwrap_or_else(|e| {
        println!("{:?}", e);
        exit(1);
    });

    let ast = Ast::build(pairs).unwrap_or_else(|e| {
        println!("{:?}", e);
        exit(1);
    });

    println!("{}", &ast);

    let compiled_project = ast.compile().unwrap_or_else(|e| {
        println!("{:?}", e);
        exit(1);
    });

    println!("{}", compiled_project);

}

pub fn run_command(cli_args: &RunCommand) {
    // Read file
    let source = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf);
            String::from_utf8(buf).expect("Could not read stdin")
        },
        args::Input::Path(path) => {
            fs::read_to_string(&path).expect("Could not read file")
        },
        
    };

    // parse code with pest
    let pairs = althread::parser::parse(&source).unwrap_or_else(|e| {
        e.report(&source);
        exit(1);
    });

    let ast = Ast::build(pairs).unwrap_or_else(|e| {
        e.report(&source);
        exit(1);
    });

    let compiled_project = ast.compile().unwrap_or_else(|e| {
        e.report(&source);
        exit(1);
    });

    let mut vm = althread::vm::VM::new(&compiled_project);

    vm.start();
    for i in 0..100000 {
        let info = vm.next().unwrap_or_else(|err| {
            println!("Error: {:?}", err);
            exit(1);
        });
        //println!("{}_{}: {} stopped at {}", info.prog_name, info.prog_id, info.instruction_count, vm.running_programs.get(vm.running_programs.iter().position(|p| p.id == info.prog_id).unwrap()).unwrap().id);
    }

}