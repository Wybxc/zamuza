//! An interaction nets compiler.
//!
//! Implements the algorithm described in [An Implementation Model for Interaction Nets](https://doi.org/10.48550/arXiv.1505.07164).

#![deny(missing_docs)]

extern crate pest;
#[macro_use]
extern crate pest_derive;

pub mod ast;
pub mod check;
pub mod options;
pub mod parser;
pub mod runtime;
pub(crate) mod utils;

use anyhow::Result;
use colorized::{Color, Colors};
use options::Options;
use runtime::vm::VM;
use runtime::RuntimeBuilder;

/// Compile a program to a runtime.
pub fn compile(
    source: &str,
    filename: &str,
    output: impl std::io::Write,
    options: &Options,
) -> Result<()> {
    let program = match parser::parse(source, filename) {
        Ok(program) => program,
        Err(snippet) => anyhow::bail!("{}", snippet),
    };

    if let Err(e) = check::check_program(&program) {
        anyhow::bail!("{}", e.to_snippet(source, filename));
    }

    let runtime = RuntimeBuilder::build_runtime(program)?;
    runtime.write::<runtime::target::C>(output, options)
}

/// Execute a program.
pub fn execute(source: &str, filename: &str, options: &Options) -> Result<()> {
    let program = match parser::parse(source, filename) {
        Ok(program) => program,
        Err(snippet) => anyhow::bail!("{}", snippet),
    };

    if let Err(e) = check::check_program(&program) {
        anyhow::bail!("{}", e.to_snippet(source, filename));
    }

    let runtime = RuntimeBuilder::build_runtime(program)?;
    let vm = VM::new(runtime, options);
    if let Err(e) = vm.run() {
        eprintln!("{}: {}", "error".color(Colors::RedFg), e);
        std::process::exit(1);
    }
    Ok(())
}
