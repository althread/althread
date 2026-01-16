use std::{ffi::OsStr, path::PathBuf};

use clap::builder::TypedValueParser;
use clap::{Args, Parser, Subcommand, ValueHint};

/// An input that is either stdin or a real path.
#[derive(Debug, Clone)]
pub enum Input {
    /// Stdin, represented by `-`.
    Stdin,
    /// A non-empty path.
    Path(PathBuf),
}

/// The Typst compiler.
#[derive(Debug, Clone, Parser)]
#[clap(name = "althread")]
pub struct CliArguments {
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    #[command(subcommand)]
    pub command: Command,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Compiles an input file into a supported output format
    #[command(visible_alias = "p")]
    Compile(CompileCommand),

    /// Runs an input file
    #[command(visible_alias = "r")]
    Run(RunCommand),

    /// Compiles an input file into a supported output format
    #[command()]
    RandomSearch(RandomSearchCommand),

    /// Check the input program
    #[command()]
    Check(CheckCommand),

    /// Initialize a new Althread package
    #[command()]
    Init(InitCommand),

    /// Add a dependency to the current package
    #[command()]
    Add(AddCommand),

    /// Remove a dependency from the current package
    #[command()]
    Remove(RemoveCommand),

    /// Update dependencies
    #[command()]
    Update(UpdateCommand),

    /// Install/fetch all dependencies
    #[command()]
    Install(InstallCommand),
}

/// Compiles an input file into a supported output format
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Shared arguments
    #[clap(flatten)]
    pub common: SharedArgs,
}

/// Compiles an input file into a supported output format
#[derive(Debug, Clone, Parser)]
pub struct CheckCommand {
    /// Shared arguments
    #[clap(flatten)]
    pub common: SharedArgs,

    /// max number of states to explore
    #[clap(long, default_value_t = 100_000)]
    pub max_states: u64,
}

/// Compiles an input file into a supported output format
#[derive(Debug, Clone, Parser)]
pub struct RunCommand {
    /// Shared arguments
    #[clap(flatten)]
    pub common: SharedArgs,

    /// debug
    #[clap(long)]
    pub debug: bool,

    /// verbose
    #[clap(long)]
    pub verbose: bool,

    /// interactive
    #[clap(long)]
    pub interactive: bool,

    /// max number of steps
    #[clap(long, default_value_t = 100_000)]
    pub max_steps: u64,

    /// seed
    #[clap(long)]
    pub seed: Option<u64>,
}

/// Compiles an input file into a supported output format
#[derive(Debug, Clone, Parser)]
pub struct RandomSearchCommand {
    /// Shared arguments
    #[clap(flatten)]
    pub common: SharedArgs,

    /// max number of steps per seed
    #[clap(long, default_value_t = 100_000)]
    pub max_steps: u64,

    /// max number of seeds
    #[clap(long, default_value_t = 10_000)]
    pub max_seeds: u64,
}

/// Initialize a new Althread package
#[derive(Debug, Clone, Parser)]
pub struct InitCommand {
    /// Package name (defaults to current directory name)
    pub name: Option<String>,

    /// Package version
    #[clap(long, default_value = "0.1.0")]
    pub version: String,

    /// Package description
    #[clap(long)]
    pub description: Option<String>,

    /// Author name
    #[clap(long)]
    pub author: Option<String>,

    /// Force initialization even if alt.toml exists
    #[clap(long)]
    pub force: bool,
}

/// Add a dependency to the current package
#[derive(Debug, Clone, Parser)]
pub struct AddCommand {
    /// Dependency specification (e.g., "github.com/user/repo@v1.0.0")
    pub dependency: String,

    /// Add as development dependency
    #[clap(long)]
    pub dev: bool,
}

/// Remove a dependency from the current package
#[derive(Debug, Clone, Parser)]
pub struct RemoveCommand {
    /// Dependency name to remove
    pub dependency: String,
}

/// Update dependencies
#[derive(Debug, Clone, Parser)]
pub struct UpdateCommand {
    /// Update only specific dependency
    pub dependency: Option<String>,
}

/// Install/fetch all dependencies
#[derive(Debug, Clone, Parser)]
pub struct InstallCommand {
    /// Force reinstall even if already cached
    #[clap(long)]
    pub force: bool,
}

/// Common arguments of compile, watch, and query.
#[derive(Debug, Clone, Args)]
pub struct SharedArgs {
    /// Path to input Typst file. Use `-` to read input from stdin
    #[clap(value_parser = make_input_value_parser(), value_hint = ValueHint::FilePath)]
    pub input: Input,
}

/// The clap value parser used by `SharedArgs.input`
fn make_input_value_parser() -> impl TypedValueParser<Value = Input> {
    clap::builder::OsStringValueParser::new().try_map(|value| {
        if value.is_empty() {
            Err(clap::Error::new(clap::error::ErrorKind::InvalidValue))
        } else if value == "-" {
            Ok(Input::Stdin)
        } else {
            let path = PathBuf::from(value.clone());
            if path.extension() != Some(OsStr::new("alt")) {
                let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation);
                err.insert(
                    clap::error::ContextKind::InvalidValue,
                    clap::error::ContextValue::String(
                        "Input file must have .alt extension".to_owned(),
                    ),
                );
                return Err(err);
            }
            Ok(Input::Path(value.into()))
        }
    })
}
