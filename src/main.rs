use anyhow::Result;
use clap::Parser;
use std::io::Read;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Source file, or pass "-" to read from stdin
    #[clap(short, long, value_parser)]
    file: clio::Input,

    /// Output file, or pass "-" to write to stdout
    #[clap(short, long, value_parser)]
    output: clio::Output,
}

fn main() -> Result<()> {
    let mut args = Cli::parse();

    let mut program = String::new();
    args.file.read_to_string(&mut program)?;

    zamuza::compile(&program, args.output)?;

    Ok(())
}
