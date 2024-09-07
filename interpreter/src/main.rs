use std::{cmp::max, fs, io::Read, process::exit};

mod args;
use args::{CliArguments, Command, Input, CompileCommand, RunCommand};
use clap::Parser;

mod parser;
mod ast;
mod compiler;
mod vm;

use ast::Ast;

mod error;
use error::AlthreadError;


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
    let pairs = parser::parse(&source).unwrap_or_else(|e| {
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
    let pairs = parser::parse(&source).unwrap_or_else(|e| {
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

    let mut vm = vm::VM::new(&compiled_project);

    vm.start();
    for i in 0..20 {
        //println!("STEP {} ----\n{}", i,  vm);
        vm.next().unwrap_or_else(|err| {
            println!("Error: {:?}", err);
            exit(1);
        });
    }

}