use std::{
    collections::HashSet, env::var, error::Error, fs::{self, remove_dir_all}, io::{stdin, stdout, Read, Write}, path::{Path, PathBuf}, process::exit
};

mod args;
mod package;
mod resolver;
mod git;
use args::{CheckCommand, CliArguments, Command, CompileCommand, RandomSearchCommand, RunCommand, 
          InitCommand, AddCommand, RemoveCommand, UpdateCommand, InstallCommand};
use clap::Parser;
use owo_colors::{OwoColorize, Style};

use althread::{ast::Ast, checker};

use crate::package::{DependencySpec, Package};

fn main() {
    let cli_args = CliArguments::parse();

    match &cli_args.command {
        Command::Compile(command) => compile_command(&command.clone()),
        Command::Run(command) => run_command(&command.clone()),
        Command::RandomSearch(command) => random_search_command(&command.clone()),
        Command::Check(command) => check_command(&command.clone()),
        Command::Init(command) => init_command(&command.clone()),
        Command::Add(command) => add_command(&command.clone()),
        Command::Remove(command) => remove_command(&command.clone()),
        Command::Update(command) => update_command(&command.clone()),
        Command::Install(command) => install_command(&command.clone()),
    }
}

pub fn compile_command(cli_args: &CompileCommand) {
    // Read file
    let (source, path) = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
            (
                String::from_utf8(buf).expect("Could not read stdin"),
                PathBuf::from("./stdin.alt")
            )
        }
        args::Input::Path(path) => (
            fs::read_to_string(&path).expect("Could not read file"),
            path
        )
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

    let compiled_project = ast.compile(&path).unwrap_or_else(|e| {
        e.report(&source);
        exit(1);
    });

    println!("{}", compiled_project);
}

pub fn check_command(cli_args: &CheckCommand) {
    // Read file
    let (source, path) = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
            (
                String::from_utf8(buf).expect("Could not read stdin"),
                PathBuf::from("./stdin.alt")
            )
        }
        args::Input::Path(path) => (
            fs::read_to_string(&path).expect("Could not read file"),
            path
        )
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

    let compiled_project = ast.compile(&path).unwrap_or_else(|e| {
        e.report(&source);
        exit(1);
    });

    let checked = checker::check_program(&compiled_project).unwrap_or_else(|e| {
        e.report(&source);
        exit(1);
    });

    if checked.0.is_empty() {
        println!("No invariant violated");
        return;
    } else {
        println!("Invariant violated");
        for vm in checked.0.iter() {
            println!("========= {}#{:?}", vm.name, vm.pid);
        }
    }
}

const MAIN_STYLE: Style = Style::new().red().on_bright_black();
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
        for (name, pid, insts, nvm) in next_states.iter() {
            println!("======= VM next =======");
            println!(
                "{}:{}:{}",
                name,
                pid,
                if insts[0].pos.is_some() {
                    source
                        .lines()
                        .nth(insts[0].pos.unwrap().line)
                        .unwrap_or_default()
                } else {
                    "?"
                }
            );

            let s = nvm.current_state();
            println!("global: {:?}", s.0);
            for ((pid, cname), state) in s.1.iter() {
                println!("channel {},{}", pid, cname);
                for v in state.iter() {
                    println!("  * {}", v);
                }
            }
            for (pid, local_state) in s.2.iter().enumerate() {
                println!(
                    "{} ({}): {:?}",
                    pid,
                    local_state.1,
                    local_state
                        .0
                        .iter()
                        .map(|v| format!("{}", v))
                        .collect::<Vec<String>>()
                        .join(", ")
                );
            }
        }
        //read an integer from the user
        let mut selected: i32 = -1;
        while selected < 0 || selected >= next_states.len() as i32 {
            println!("Enter an integer between 0 and {}:", next_states.len() - 1);
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
    let (source, path) = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
            (
                String::from_utf8(buf).expect("Could not read stdin"),
                PathBuf::from("./stdin.alt")
            )
        }
        args::Input::Path(path) =>  (
            fs::read_to_string(&path).expect("Could not read file"),
            path
        )
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

    let compiled_project = ast.compile(&path).unwrap_or_else(|e| {
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
                if inst.pos.unwrap_or_default().line != 0
                    && prev_line != inst.pos.unwrap_or_default().line
                {
                    println!(
                        "#{}:{} {}",
                        info.prog_id,
                        inst.pos.unwrap_or_default().line,
                        source
                            .lines()
                            .nth(inst.pos.unwrap_or_default().line - 1)
                            .unwrap_or_default()
                            .style(if info.prog_id == 0 {
                                MAIN_STYLE
                            } else {
                                PROCESS_PALETTE
                                    [((info.prog_id - 1) as usize) % PROCESS_PALETTE.len()]
                            })
                    );
                    prev_line = inst.pos.unwrap_or_default().line;
                }
                if cli_args.verbose {
                    println!("\t\t\t#{}:{}", info.prog_id, inst);
                }
            }
            match vm.running_programs.get(info.prog_id) {
                Some(p) => match p.current_instruction() {
                    Ok(i) => println!("{}_{}: stopped at {}", info.prog_name, info.prog_id, i),
                    _ => println!("{}_{}: stopped at ?", info.prog_name, info.prog_id),
                },
                None => println!("{}_{}: prog not found", info.prog_name, info.prog_id),
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
                println!(
                    "{} ({}): {:?}",
                    pid,
                    local_state.1,
                    local_state
                        .0
                        .iter()
                        .map(|v| format!("{}", v))
                        .collect::<Vec<String>>()
                        .join(", ")
                );
            }
        }
    }
}

pub fn random_search_command(cli_args: &RandomSearchCommand) {
    // Read file
    let (source, path) = match cli_args.common.input.clone() {
        args::Input::Stdin => {
            let mut buf = Vec::new();
            let _ = std::io::stdin().read_to_end(&mut buf);
            (
                String::from_utf8(buf).expect("Could not read stdin"),
                PathBuf::from("./stdin.alt")
            )
        }
        args::Input::Path(path) => (
            fs::read_to_string(&path).expect("Could not read file"),
            path
    )
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

    let compiled_project = ast.compile(&path).unwrap_or_else(|e| {
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
            None => println!("{}_{}: prog not found", info.prog_name, info.prog_id),
        }*/
        }
    }
}

// Package management command implementations
pub fn init_command(cli_args: &InitCommand) {
    use std::path::Path;
    
    let alt_toml_path = Path::new("alt.toml");
    
    // Check if alt.toml already exists
    if alt_toml_path.exists() && !cli_args.force {
        eprintln!("Error: alt.toml already exists. Use --force to overwrite.");
        std::process::exit(1);
    }
    
    // Determine package name
    let package_name = cli_args.name.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "althread-project".to_string())
    });
    
    // Create package info
    let mut package = package::Package::new(package_name.clone(), cli_args.version.clone());
    
    if let Some(description) = &cli_args.description {
        package.package.description = Some(description.clone());
    }
    
    if let Some(author) = &cli_args.author {
        package.package.authors = Some(vec![author.clone()]);
    }
    
    // Save package to alt.toml
    if let Err(e) = package.save_to_path(alt_toml_path) {
        eprintln!("Error creating alt.toml: {}", e);
        std::process::exit(1);
    }
    
    println!("✓ Created alt.toml for package '{}'", package_name);
    println!("  Version: {}", cli_args.version);
    if let Some(desc) = &cli_args.description {
        println!("  Description: {}", desc);
    }
    if let Some(author) = &cli_args.author {
        println!("  Author: {}", author);
    }
}

pub fn add_command(cli_args: &AddCommand) {
    use std::path::Path;
    
    let alt_toml_path = Path::new("alt.toml");
    
    // Parse dependency first
    let dep_info = match package::DependencyInfo::parse(&cli_args.dependency) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("Error parsing dependency '{}': {}", cli_args.dependency, e);
            std::process::exit(1);
        }
    };
    
    // For local dependencies, just give guidance
    if dep_info.is_local {
        println!("Local dependency '{}' doesn't need to be added to alt.toml", dep_info.url);
        println!("Just use: import [{}] in your code", dep_info.url);
        return;
    }
    
    // For remote dependencies, ensure alt.toml exists
    let mut package = if alt_toml_path.exists() {
        match package::Package::load_from_path(alt_toml_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error reading alt.toml: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Create a new package if alt.toml doesn't exist
        println!("No alt.toml found. Creating one...");
        let package_name = std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "althread-project".to_string());
        package::Package::new(package_name, "0.1.0".to_string())
    };
    
    println!("Adding remote dependency: {} ({})", dep_info.name, dep_info.url);
    
    // Add dependency - use the full URL as the key
    let dep_spec = if dep_info.version == "latest" {
        package::DependencySpec::Simple("*".to_string())
    } else {
        package::DependencySpec::Simple(dep_info.version.clone())
    };
    
    if cli_args.dev {
        package.add_dev_dependency(dep_info.url.clone(), dep_spec); // Use URL as key
    } else {
        package.add_dependency(dep_info.url.clone(), dep_spec); // Use URL as key
    }
    
    // Save updated package
    if let Err(e) = package.save_to_path(alt_toml_path) {
        eprintln!("Error saving alt.toml: {}", e);
        std::process::exit(1);
    }
    
    println!("✓ Added {} to {} dependencies", dep_info.url, if cli_args.dev { "dev" } else { "runtime" });
    println!("  Run 'althread install' to fetch the dependency");
}

pub fn remove_command(cli_args: &RemoveCommand) {
    println!("Removing dependency: {}", cli_args.dependency);

    let alt_toml_path = Path::new("alt.toml");

    if !alt_toml_path.exists() {
        eprintln!("Error: No alt.toml found in current directory.");
        exit(1);
    }

    let mut package = match package::Package::load_from_path(alt_toml_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error reading alt.toml: {}", e);
            exit(1);
        }
    };

    let removed = package.remove_dependency(&cli_args.dependency);
    
    if !removed {
        eprintln!("Error: Dependency '{}' not found in alt.toml.", cli_args.dependency);
        exit(1);
    }

    if let Err(e) = package.save_to_path(alt_toml_path) {
        eprintln!("Error saving alt.toml : {}", e);
        exit(1);
    }

    println!("✓ Removed dependency: {}", cli_args.dependency);

    if should_clean_cache() {
        if let Err(e) = clean_dependency_cache(&cli_args.dependency) {
            eprintln!("Warning: Failed to clean cache: {}", e);
        } else {
            println!("✓ Cleaned up cached files");
        }
    }
}

pub fn update_command(cli_args: &UpdateCommand) {
    let alt_toml_path = Path::new("alt.toml");

    if !alt_toml_path.exists() {
        eprintln!("Error: No alt.toml found in current directory.");
        exit(1);
    }

    let mut package = match Package::load_from_path(alt_toml_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error reading alt.toml: {}", e);
            exit(1);
        }
    };

    let dependencies_to_update = if let Some(specific_dep) = &cli_args.dependency {
        // Update only the specified dependency
        vec![specific_dep.clone()]
    } else {
        // Update all dependencies
        let mut all_deps = package.dependencies.keys().cloned().collect::<Vec<_>>();
        all_deps.extend(package.dev_dependencies.keys().cloned());
        all_deps
    };

    if dependencies_to_update.is_empty() {
        println!("No dependencies to update.");
        return;
    }

    println!("Updating {} dependencies...", dependencies_to_update.len());

    let mut updated_count = 0;

    for dep_name in dependencies_to_update {
        match update_single_dependency(&mut package, &dep_name) {
            Ok(updated) => {
                if updated {
                    updated_count += 1;
                    println!("✓ Updated {}", dep_name);
                } else {
                    println!("- {} already up to date", dep_name);
                }
            }
            Err(e) => {
                eprintln!("✗ Failed to update {}: {}", dep_name, e);
            }
        }
    }

    if updated_count > 0 {
        // Save the updated package
        if let Err(e) = package.save_to_path(alt_toml_path) {
            eprintln!("Error saving alt.toml: {}", e);
            exit(1);
        }

        println!("✓ Updated {} dependencies in alt.toml", updated_count);
        println!(" Run 'althread install' to fetch the updated dependencies");
    } else {
        println!("All dependencies are already up to date.");
    }

}

pub fn install_command(cli_args: &InstallCommand) {
    let alt_toml_path = Path::new("alt.toml");
    
    if !alt_toml_path.exists() {
        eprintln!("Error: No alt.toml found. Run 'althread init' or 'althread add <dependency>' first.");
        exit(1);
    }
    
    let mut package = load_package_or_exit(alt_toml_path);
    
    println!("Installing dependencies for package '{}'...", package.package.name);
    
    // Check if there are any dependencies to install
    let dep_count = package.dependencies.len() + package.dev_dependencies.len();
    if dep_count == 0 {
        println!("No dependencies to install.");
        return;
    }
    
    // Use the resolver to handle both resolution and installation in a single pass
    let mut resolver = resolver::DependencyResolver::new();
    match resolver.resolve_and_install_dependencies(&package, cli_args.force) {
        Ok((resolved_deps, version_changes)) => {
            // Update package versions if needed
            if !version_changes.is_empty() {
                update_package_versions(&mut package, &version_changes);
                save_package_or_exit(&package, alt_toml_path);
                println!("✓ Updated alt.toml with actual versions");
            }
            
            // Display results
            println!("✓ Successfully installed {} dependencies:", resolved_deps.len());
            for resolved in &resolved_deps {
                println!("  - {}@{}", resolved.name, resolved.version);
            }
            
            if !resolved_deps.is_empty() {
                println!("✓ Dependency resolution successful");
                println!("  Resolved {} total dependencies (including transitive)", resolved_deps.len());
                
                // Show resolved dependencies
                for resolved in &resolved_deps {
                    println!("  {} @ {}", resolved.name, resolved.version);
                }
            }
        }
        Err(e) => {
            eprintln!("Error during dependency resolution and installation: {}", e);
            exit(1);
        }
    }
}

// Helper functions to reduce repetitive code
fn load_package_or_exit(alt_toml_path: &Path) -> Package {
    match Package::load_from_path(alt_toml_path) {
        Ok(package) => package,
        Err(e) => {
            eprintln!("Error reading alt.toml: {}", e);
            exit(1);
        }
    }
}

fn save_package_or_exit(package: &Package, alt_toml_path: &Path) {
    if let Err(e) = package.save_to_path(alt_toml_path) {
        eprintln!("Error saving alt.toml: {}", e);
        exit(1);
    }
}

fn update_package_versions(package: &mut Package, version_changes: &[(String, String, String)]) {
    println!("Updating alt.toml with actual versions used...");
    
    for (dep_url, original_version, actual_version) in version_changes {
        println!("  {}: {} -> {}", dep_url, original_version, actual_version);
        
        let new_spec = if package.dependencies.contains_key(dep_url) {
            match package.dependencies.get(dep_url).unwrap() {
                DependencySpec::Simple(_) => DependencySpec::Simple(actual_version.clone()),
                DependencySpec::Detailed { features, optional, .. } => {
                    DependencySpec::Detailed {
                        version: actual_version.clone(),
                        features: features.clone(),
                        optional: *optional,
                    }
                }
            }
        } else {
            match package.dev_dependencies.get(dep_url).unwrap() {
                DependencySpec::Simple(_) => DependencySpec::Simple(actual_version.clone()),
                DependencySpec::Detailed { features, optional, .. } => {
                    DependencySpec::Detailed {
                        version: actual_version.clone(),
                        features: features.clone(),
                        optional: *optional,
                    }
                }
            }
        };
        
        if package.dependencies.contains_key(dep_url) {
            package.dependencies.insert(dep_url.clone(), new_spec);
        } else {
            package.dev_dependencies.insert(dep_url.clone(), new_spec);
        }
    }
}

fn should_clean_cache() -> bool {
    print!("Remove cached files for this dependency? (y/N): ");
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    let input = input.trim().to_lowercase();
    input == "y" || input == "yes"
}

fn clean_dependency_cache(dependency: &str) -> Result<(), Box<dyn Error>> {
    let home_dir = var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let cache_dir = PathBuf::from(home_dir).join(".althread/cache");

    // if the dependency is a URL like "github.com/user/repo"
    let sanitized_url = dependency.replace("://", "/");
    let dep_cache_dir = cache_dir.join(&sanitized_url);

    if dep_cache_dir.exists() {
        remove_dir_all(&dep_cache_dir)?;
        println!("✓ Removed cache directory: {}", dep_cache_dir.display());
    } else {
        println!("  No cache found for dependency: {}", dependency);
    }

    Ok(())
}

fn update_single_dependency(package: &mut Package, dep_name: &str) -> Result<bool, Box<dyn Error>> {
    println!("Checking {} for updates...", dep_name);

    // Check if the dependency is present
    let current_spec = package.dependencies.get(dep_name)
        .or_else(|| package.dev_dependencies.get(dep_name));

    let current_spec = match current_spec {
        Some(spec) => spec, 
        None => {
            return Err(format!("Dependency '{}' not found in alt.toml", dep_name).into());
        }
    };

    let current_version = match current_spec {
        DependencySpec::Simple(v) => v,
        DependencySpec::Detailed { version, .. } => version,
    };

    println!("  Current version: {}", current_version);

    let latest_version = match resolver::VersionResolver::get_latest_version_from_repo(dep_name) {
        Ok(version) => {
            println!("  Latest version: {}", version);
            version
        }
        Err(e) => {
            println!("  Warning: Could not check the latest version: {}", e);
            return Ok(false);
        }
    };

    let needs_update = should_update_dependency(current_version, &latest_version);
    
    if needs_update {
        println!("  Update available: {} -> {}", current_version, latest_version);

        let new_spec = match current_spec {
            DependencySpec::Simple(_) => {
                DependencySpec::Simple(latest_version)
            }
            DependencySpec::Detailed { features, optional , ..} => {
                DependencySpec::Detailed {
                    version: latest_version,
                    features: features.clone(),
                    optional: *optional,
                }
            }
        };

        if package.dependencies.contains_key(dep_name) {
            package.dependencies.insert(dep_name.to_string(), new_spec);
        } else {
            package.dev_dependencies.insert(dep_name.to_string(), new_spec);
        }

        Ok(true)
    } else {
        println!("  No update needed");
        Ok(false)
    }
}

fn should_update_dependency(current: &str, latest: &str) -> bool {
    // If both are the same, no update needed
    if current == latest {
        return false;
    }

    // If current is * or 'latest', and latest is also 'latest', no update needed
    if (current == "*" || current == "latest") && latest == "latest" {
        return false;
    }

    // If current is * or 'latest', and we have a specific version (tag or commit), update
    if (current == "*" || current == "latest") && latest != "latest" {
        return true;
    }

    // Check if we're dealing with commit hashes (8 character hex strings)
    let current_is_commit = is_commit_hash(current);
    let latest_is_commit = is_commit_hash(latest);

    // If both are commit hashes, they're different commits, so update
    if current_is_commit && latest_is_commit {
        return true;
    }

    // If current is a commit hash and latest is a tag, update to the tag
    if current_is_commit && !latest_is_commit {
        return true;
    }

    // If current is a tag and latest is a commit hash, update to the commit
    if !current_is_commit && latest_is_commit {
        return true;
    }

    // If we have specific semantic versions, compare them
    if let (Ok(current_ver), Ok(latest_ver)) = (
        resolver::VersionResolver::parse_version_tag(current), 
        resolver::VersionResolver::parse_version_tag(latest)
    ) {
        return latest_ver > current_ver;
    }

    // If we can't parse versions, but they're different, consider it an update
    current != latest
}

fn is_commit_hash(s: &str) -> bool {
    // A commit hash is typically 8 characters (short hash) or 40 characters (full hash)
    // and contains only hexadecimal characters
    (s.len() == 8 || s.len() == 40) && s.chars().all(|c| c.is_ascii_hexdigit())
}