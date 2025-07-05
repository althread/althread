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
    if let Some(dep) = &cli_args.dependency {
        println!("Updating dependency: {}", dep);
    } else {
        println!("Updating all dependencies...");
    }
    // TODO: Implement dependency updates
    // 1. Check for newer versions
    // 2. Update alt.toml
    // 3. Refetch dependencies
    eprintln!("update command not yet implemented");
}

pub fn install_command(cli_args: &InstallCommand) {
    use std::path::Path;
    
    let alt_toml_path = Path::new("alt.toml");
    
    // Check if alt.toml exists
    if !alt_toml_path.exists() {
        eprintln!("Error: No alt.toml found. Run 'althread init' or 'althread add <dependency>' first.");
        std::process::exit(1);
    }
    
    // Load package
    let package = match package::Package::load_from_path(alt_toml_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error reading alt.toml: {}", e);
            std::process::exit(1);
        }
    };
    
    println!("Installing dependencies for package '{}'...", package.package.name);
    
    // Create cache directory if it doesn't exist
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let cache_dir = std::path::PathBuf::from(home_dir).join(".althread/cache");
    
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        eprintln!("Error creating cache directory: {}", e);
        std::process::exit(1);
    }
    
    // First, install all dependencies that we need to fetch
    let mut to_install = Vec::new();
    
    // Collect runtime dependencies
    for (name, spec) in &package.dependencies {
        let dep_info = match spec_to_dependency_info(name, spec) {
            Ok(info) => info,
            Err(e) => {
                eprintln!("Error parsing dependency {}: {}", name, e);
                std::process::exit(1);
            }
        };
        to_install.push(dep_info);
    }
    
    // Collect dev dependencies  
    for (name, spec) in &package.dev_dependencies {
        let dep_info = match spec_to_dependency_info(name, spec) {
            Ok(info) => info,
            Err(e) => {
                eprintln!("Error parsing dependency {}: {}", name, e);
                std::process::exit(1);
            }
        };
        to_install.push(dep_info);
    }
    
    if to_install.is_empty() {
        println!("No dependencies to install.");
        return;
    }
    
    // Install each dependency
    for dep in &to_install {
        if let Err(e) = fetch_and_cache_dependency(dep, cli_args.force, &cache_dir) {
            eprintln!("Error installing dependency {}: {}", dep.name, e);
            std::process::exit(1);
        }
    }
    
    println!("✓ Successfully installed {} dependencies:", to_install.len());
    for dep in &to_install {
        println!("  - {}@{}", dep.name, dep.version);
    }
    
    // Now resolve dependencies to check for conflicts
    let mut resolver = resolver::DependencyResolver::new();
    match resolver.resolve_dependencies(&package) {
        Ok(resolved_deps) => {
            println!("✓ Dependency resolution successful");
            if resolved_deps.len() != to_install.len() {
                println!("  Note: Resolved {} total dependencies (including transitive)", resolved_deps.len());
            }
        }
        Err(e) => {
            eprintln!("Warning: Dependency resolution failed: {}", e);
            eprintln!("Dependencies were installed but may have conflicts");
        }
    }
}

fn spec_to_dependency_info(name: &str, spec: &package::DependencySpec) -> Result<package::DependencyInfo, String> {
    let version = match spec {
        package::DependencySpec::Simple(v) => v.clone(),
        package::DependencySpec::Detailed { version, .. } => version.clone(),
    };

    // The name is actually the full URL (e.g., "github.com/user/repo")
    // Extract the actual package name from the URL
    let package_name = name.split('/').last().unwrap_or(name).to_string();

    Ok(package::DependencyInfo {
        name: package_name,
        url: name.to_string(), // name is the full URL
        version,
        is_local: false,
    })
}

fn fetch_and_cache_dependency(dep: &package::DependencyInfo, force: bool, cache_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching {}@{}...", dep.name, dep.version);
    
    // Resolve version
    let resolved_version = if dep.version == "*" || dep.version == "latest" {
        "main".to_string()
    } else {
        dep.version.clone()
    };
    
    // Create cache path structure: ~/.althread/cache/github.com/user/repo/v1.0.0
    let sanitized_url = dep.url.replace("://", "/");
    let repo_cache_dir = cache_dir.join(&sanitized_url);
    let version_cache_dir = repo_cache_dir.join(&resolved_version);
    
    // Check if already cached
    if version_cache_dir.exists() && !force {
        // Verify it has the necessary files
        if version_cache_dir.join("alt.toml").exists() {
            println!("  Already cached, skipping (use --force to reinstall)");
            return Ok(());
        } else {
            println!("  Cache exists but is incomplete, re-fetching...");
            std::fs::remove_dir_all(&version_cache_dir)?;
        }
    }
    
    // Convert import URL to git URL
    let git_url = git::GitRepository::url_from_import_path(&dep.url);
    println!("  Cloning from: {}", git_url);
    
    // Create temporary clone location
    let temp_clone_dir = repo_cache_dir.join("_temp_clone");
    if temp_clone_dir.exists() {
        std::fs::remove_dir_all(&temp_clone_dir)?;
    }
    
    // Clone the repository
    let git_repo = git::GitRepository::new(&git_url, temp_clone_dir.clone());
    git_repo.clone_if_needed()?;
    
    // Checkout the specific version
    git_repo.checkout_version(&resolved_version)?;
    
    // Verify this is a valid Althread package
    let alt_toml_path = temp_clone_dir.join("alt.toml");
    if !alt_toml_path.exists() {
        return Err(format!(
            "Invalid Althread package: {} does not contain alt.toml", 
            dep.url
        ).into());
    }
    
    // Create the version-specific cache directory
    std::fs::create_dir_all(&version_cache_dir)?;
    
    // Copy the entire repository contents to the versioned cache
    copy_dir_all(&temp_clone_dir, &version_cache_dir)?;
    
    // Clean up temporary clone
    std::fs::remove_dir_all(&temp_clone_dir)?;
    
    println!("  ✓ Cached to {}", version_cache_dir.display());
    
    // Validate the package structure
    validate_package_structure(&version_cache_dir)?;
    
    Ok(())
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dst)?;
    
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        // Skip .git directory and other hidden files
        if let Some(name) = entry.file_name().to_str() {
            if name.starts_with('.') {
                continue;
            }
        }
        
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

fn validate_package_structure(package_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let alt_toml_path = package_dir.join("alt.toml");
    
    // Parse and validate alt.toml
    let package = package::Package::load_from_path(&alt_toml_path)?;
    
    println!("  Package: {} v{}", package.package.name, package.package.version);
    if let Some(desc) = &package.package.description {
        println!("  Description: {}", desc);
    }
    
    // Look for .alt files
    let alt_files = find_alt_files(package_dir)?;
    if alt_files.is_empty() {
        println!("  Warning: No .alt files found in package");
    } else {
        println!("  Found {} .alt files", alt_files.len());
        for file in alt_files.iter().take(5) { // Show first 5
            let relative_path = file.strip_prefix(package_dir)
                .unwrap_or(file)
                .display();
            println!("    - {}", relative_path);
        }
        if alt_files.len() > 5 {
            println!("    ... and {} more", alt_files.len() - 5);
        }
    }
    
    Ok(())
}

fn find_alt_files(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut alt_files = Vec::new();
    
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // Recursively search subdirectories
            alt_files.extend(find_alt_files(&path)?);
        } else if let Some(extension) = path.extension() {
            if extension == "alt" {
                alt_files.push(path);
            }
        }
    }
    
    Ok(alt_files)
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
