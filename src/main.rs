use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::io::Read;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a program
    #[command(visible_alias = "r")]
    Run {
        /// Source file, or pass "-" to read from stdin
        #[clap(value_parser)]
        input: clio::Input,

        #[clap(flatten)]
        options: Options,
    },
    /// Compile a program
    #[command(visible_alias = "c")]
    Compile {
        /// Source file, or pass "-" to read from stdin
        #[clap(value_parser)]
        input: clio::Input,

        /// Output file, pass "-" to write to stdout
        #[clap(short, long, value_parser)]
        output: clio::Output,

        #[clap(flatten)]
        options: Options,
    },
}

#[derive(Args)]
struct Options {
    /// Runtime stack size
    #[clap(long, default_value = "1024")]
    stack_size: usize,

    /// Trace reduction
    #[clap(long)]
    trace: bool,

    /// Output timing information
    #[clap(long)]
    timing: bool,
}

impl From<Options> for zamuza::options::Options {
    fn from(options: Options) -> Self {
        Self {
            stack_size: options.stack_size,
            trace: options.trace,
            timing: options.timing,
        }
    }
}

fn get_filename(input: &clio::Input) -> String {
    if input.is_std() {
        "<stdin>".to_string()
    } else {
        input.path().to_string()
    }
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Run { mut input, options } => {
            let filename = get_filename(&input);

            let mut program = String::new();
            input.read_to_string(&mut program)?;

            zamuza::execute(&program, &filename, &options.into())?;
        }
        Commands::Compile {
            mut input,
            output,
            options,
        } => {
            let filename = get_filename(&input);

            let mut program = String::new();
            input.read_to_string(&mut program)?;

            zamuza::compile(&program, &filename, output, &options.into())?;
        }
    };

    Ok(())
}
