use std::{fs, io::Read, process::exit};

mod args;
use args::{CliArguments, Command, CompileCommand, RandomSearchCommand, RunCommand};
use clap::Parser;

use althread::ast::Ast;


fn main() {
    let cli_args = CliArguments::parse();

    match &cli_args.command {
        Command::Compile(command) => compile_command(&command.clone()),
        Command::Run(command) => run_command(&command.clone()),
        Command::RandomSearch(command) => random_search_command(&command.clone()),
    }

}



pub fn compile_command(cli_args: &CompileCommand) {
    // Read file
    let source = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
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

    println!("{}", &ast);

    let compiled_project = ast.compile().unwrap_or_else(|e| {
        e.report(&source);
        exit(1);
    });

    println!("{}", compiled_project);

}

pub fn run_command(cli_args: &RunCommand) {
    // Read file
    let source = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
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

    vm.start(fastrand::u64(0..(1 << 63)));
    for _ in 0..100000 {
        if vm.is_finished() {
            break;
        }
        let info = vm.next().unwrap_or_else(|err| {
            err.report(&source);
            exit(1);
        });
        match vm.running_programs.iter()
            .find(|(id, _)| **id == info.prog_id) {
            Some((_, p)) => match p
                .current_instruction() {
                Some(i) => println!("{}_{}: stopped at {}", info.prog_name, info.prog_id, i),
                None => println!("{}_{}: stopped at ?", info.prog_name, info.prog_id),
            },
            None => {
                println!("Program  {} not found", info.prog_id);
            }
        }
    }

}


pub fn random_search_command(cli_args: &RandomSearchCommand) {
    // Read file
    let source = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
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

    for s in 0..10000 {
        println!("Seed: {}/10000", s);
        let mut vm = althread::vm::VM::new(&compiled_project);
        vm.start(s);
        for _ in 0..100000 {
            if vm.is_finished() {
                break;
            }
            let _info = vm.next().unwrap_or_else(|err| {
                println!("Error with seed {}:", s);
                err.report(&source);
                exit(1);
            });
            /*match vm.running_programs.iter()
                .find(|(id, _)| **id == info.prog_id) {
                Some((_, p)) => match p
                    .current_instruction() {
                    Some(i) => println!("{}_{}: stopped at {}", info.prog_name, info.prog_id, i),
                    None => println!("{}_{}: stopped at ?", info.prog_name, info.prog_id),
                },
                None => {}
            }*/
        }
    }

}