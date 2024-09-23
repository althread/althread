use std::{collections::HashSet, fs, io::{stdin, Read}, process::exit};

mod args;
use args::{CliArguments, Command, CompileCommand, RandomSearchCommand, RunCommand};
use clap::Parser;
use owo_colors::{OwoColorize, Style};

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
        }
        args::Input::Path(path) => fs::read_to_string(&path).expect("Could not read file"),
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



const MAIN_STYLE: Style = Style::new()
    .red()
    .on_bright_black();
const PROCESS_PALETTE: [Style; 6] = [
    Style::new().green(),
    Style::new().yellow(),
    Style::new().blue(),
    Style::new().magenta(),
    Style::new().cyan(),
    Style::new().red(),
];

pub fn run_interactive(source: String, compiled_project: althread::compiler::CompiledProject) {
    
    let mut vm = althread::vm::VM::new(&compiled_project);

    vm.start(0); 

    loop {
        let next_states = vm.next().unwrap_or_else(|e| {
            e.report(&source);
            exit(1);
        });
        if next_states.is_empty() {
            println!("No next state");
            return;
        }
        for (name, pid, pos, nvm) in next_states.iter() {
            println!("======= VM next =======");
            println!("{}:{}:{}", name, pid, source.lines().nth(pos.line).unwrap_or_default());

            let s = nvm.current_state();
            println!("global: {:?}", s.0);
            for ((pid, cname), state) in s.1.iter() {
                println!("channel {},{}", pid, cname);
                for v in state.iter() {
                    println!("  * {}", v);
                }
            }
            for (pid, local_state) in s.2.iter().enumerate() {
                println!("{} ({}): {:?}", pid, local_state.1, local_state.0.iter().map(|v| format!("{}", v)).collect::<Vec<String>>().join(", "));
            }
        }
        //read an integer from the user
        let mut selected: i32 = -1;
        while selected < 0 || selected >= next_states.len() as i32 {
            println!("Enter an integer between 0 and {}:", next_states.len()-1);
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            selected = input.trim().parse().unwrap();
        }
        let (_, _, _, nvm) = next_states.get(selected as usize).unwrap();
        vm = nvm.clone();
    }
}

pub fn run_command(cli_args: &RunCommand) {
    // Read file
    let source = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
            String::from_utf8(buf).expect("Could not read stdin")
        }
        args::Input::Path(path) => fs::read_to_string(&path).expect("Could not read file"),
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

    if cli_args.interactive {
        run_interactive(source, compiled_project);
        return;
    }

    let mut vm_execution: Vec<althread::vm::VM> = Vec::new();
    let mut vm_set: HashSet<althread::vm::VM> = HashSet::new();
    let mut vm = althread::vm::VM::new(&compiled_project);

    vm.start(cli_args.seed.unwrap_or(fastrand::u64(0..(1 << 63))));
    for _ in 0..100000 {
        if vm.is_finished() {
            break;
        }
        let info = vm.next_random().unwrap_or_else(|err| {
            err.report(&source);
            exit(1);
        });

        if cli_args.verbose || cli_args.debug {
            let mut prev_line = 0;
            for inst in info.instructions.iter() {
                if inst.pos.unwrap_or_default().line != 0 && prev_line != inst.pos.unwrap_or_default().line {
                    println!("#{}:{} {}", 
                        info.prog_id, 
                        inst.pos.unwrap_or_default().line,
                        source.lines().nth(inst.pos.unwrap_or_default().line - 1).unwrap_or_default().style(if info.prog_id == 0 { MAIN_STYLE } else {
                            PROCESS_PALETTE[((info.prog_id - 1) as usize) % PROCESS_PALETTE.len()]
                        })
                    );
                    prev_line = inst.pos.unwrap_or_default().line;
                }
                if cli_args.verbose {
                    println!(
                        "\t\t\t#{}:{}",
                        info.prog_id,
                        inst
                    );
                }
            }
            match vm
                .running_programs
                .get(info.prog_id)
            {
                Some(p) => match p.current_instruction() {
                    Ok(i) => println!("{}_{}: stopped at {}", info.prog_name, info.prog_id, i),
                    _ => println!("{}_{}: stopped at ?", info.prog_name, info.prog_id),
                },
                None => {
                    println!("Program {} stopped", info.prog_id);
                }
            }
        }
        if info.invariant_error.is_err() {
            info.invariant_error.unwrap_err().report(&source);
            break;
        }

        vm_execution.push(vm.clone());
        if vm_set.contains(&vm) {
            println!("===== Loop detected =====");
            vm_set.insert(vm.clone());
            break;
        }
        vm_set.insert(vm.clone());

    }
    if cli_args.verbose {
        for v in vm_execution.iter() {
            println!("======= VM step =======");
            let s = v.current_state();
            println!("global: {:?}", s.0);
            for ((pid, cname), state) in s.1.iter() {
                println!("channel {},{}", pid, cname);
                for v in state.iter() {
                    println!("  * {}", v);
                }
            }
            for (pid, local_state) in s.2.iter().enumerate() {
                println!("{} ({}): {:?}", pid, local_state.1, local_state.0.iter().map(|v| format!("{}", v)).collect::<Vec<String>>().join(", "));
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
        }
        args::Input::Path(path) => fs::read_to_string(&path).expect("Could not read file"),
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
            let info = vm.next_random().unwrap_or_else(|err| {
                println!("Error with seed {}:", s);
                err.report(&source);
                exit(1);
            });
            if info.invariant_error.is_err() {
                println!("Error with seed {}:", s);
                info.invariant_error.unwrap_err().report(&source);
                exit(1);
            }
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
