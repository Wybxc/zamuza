use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::io::Read;
use zamuza::runtime::target;

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
    #[cfg(feature = "tinycc")]
    Run {
        /// Source file, pass "-" to read from stdin
        #[clap(value_parser)]
        inputs: Vec<clio::Input>,

        #[clap(flatten)]
        options: Options,
    },
    /// Compile a program
    #[command(visible_alias = "c")]
    Compile {
        /// Source file, pass "-" to read from stdin
        #[clap(value_parser)]
        inputs: Vec<clio::Input>,

        /// Output file, pass "-" to write to stdout
        #[clap(short, long, value_parser)]
        output: clio::ClioPath,

        /// Output format
        #[cfg(feature = "tinycc")]
        #[clap(short, long, default_value = "exe")]
        format: OutputFormat,

        /// Output format
        #[cfg(not(feature = "tinycc"))]
        #[clap(short, long, default_value = "c")]
        format: OutputFormat,

        #[clap(flatten)]
        options: Options,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// C source code
    C,
    /// Executable
    #[cfg(feature = "tinycc")]
    Exe,
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
        #[cfg(feature = "tinycc")]
        Commands::Run { inputs, options } => {
            let mut context = zamuza::Context::new().set_options(options.into());

            for mut input in inputs {
                let filename = get_filename(&input);
                let mut program = String::new();
                input.read_to_string(&mut program)?;
                context = context.add_file(&filename, &program)?;
            }

            context.run()?;
        }
        Commands::Compile {
            inputs,
            output,
            format,
            options,
        } => {
            let mut context = zamuza::Context::new().set_options(options.into());

            for mut input in inputs {
                let filename = get_filename(&input);
                let mut program = String::new();
                input.read_to_string(&mut program)?;
                context = context.add_file(&filename, &program)?;
            }

            if output.is_file() {
                // write to file
                match format {
                    #[cfg(feature = "tinycc")]
                    OutputFormat::Exe => context.output_file::<target::Exe>(output.as_os_str())?,
                    OutputFormat::C => context.output_file::<target::C>(output.as_os_str())?,
                }
            } else {
                // write to stream
                let output = output.create()?;
                match format {
                    #[cfg(feature = "tinycc")]
                    OutputFormat::Exe => context.output_stream::<target::Exe>(output)?,
                    OutputFormat::C => context.output_stream::<target::C>(output)?,
                }
            }
        }
    };

    Ok(())
}
