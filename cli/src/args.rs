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

    /// Compiles an input file into a supported output format
    #[command(visible_alias = "r")]
    Run(RunCommand),

    /// Compiles an input file into a supported output format
    #[command()]
    RandomSearch(RandomSearchCommand),

    /// Check the input program
    #[command()]
    Check(CheckCommand),
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
