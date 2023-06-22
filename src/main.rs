use anyhow::Result;
use clap::{Args, Parser};
use std::io::Read;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Source file, or pass "-" to read from stdin
    #[clap(short, long, value_parser)]
    file: clio::Input,

    /// Output file, or pass "-" to write to stdout, or pass nothing to execute directly
    #[clap(short, long, value_parser)]
    output: Option<clio::Output>,

    #[clap(flatten)]
    options: Options,
}

#[derive(Args)]
struct Options {
    /// Runtime stack size
    #[clap(short, long, default_value = "1024")]
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

fn main() -> Result<()> {
    let mut args = Cli::parse();

    let mut program = String::new();
    args.file.read_to_string(&mut program)?;

    if let Some(output) = args.output {
        zamuza::compile(&program, output, &args.options.into())?;
    } else {
        zamuza::execute(&program, &args.options.into())?;
    }

    Ok(())
}
