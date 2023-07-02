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
use options::Options;
use runtime::target::{self, Target};
use runtime::RuntimeBuilder;

/// Compile a program to a runtime.
pub fn compile<T: Target>(
    source: &str,
    filename: &str,
    output: impl std::io::Write,
    options: &Options,
) -> Result<()> {
    let program = match parser::parse(source, filename) {
        Ok(program) => program,
        Err(snippet) => anyhow::bail!("{}", snippet),
    };

    if let Err(e) = check::check_module(&program) {
        anyhow::bail!("{}", e.to_snippet());
    }

    let runtime = RuntimeBuilder::build_runtime(program.into_inner())?;
    runtime.write::<T>(output, options)
}

/// Compile a program to a runtime and write it to a file.
pub fn compile_to_file<T: Target>(
    source: &str,
    filename: &str,
    output: impl AsRef<std::path::Path>,
    options: &Options,
) -> Result<()> {
    let program = match parser::parse(source, filename) {
        Ok(program) => program,
        Err(snippet) => anyhow::bail!("{}", snippet),
    };

    if let Err(e) = check::check_module(&program) {
        anyhow::bail!("{}", e.to_snippet());
    }

    let runtime = RuntimeBuilder::build_runtime(program.into_inner())?;
    runtime.write_to_file::<T>(output, options)
}

/// Run a program.
pub fn run(source: &str, filename: &str, options: &Options) -> Result<()> {
    let mut output = std::io::Cursor::new(Vec::new());
    compile::<target::C>(source, filename, &mut output, options)?;
    let output = std::ffi::CString::new(output.into_inner())?;

    tinycc::Context::new(tinycc::OutputType::Memory)?
        .compile_string(&output)?
        .run(&[])?;

    Ok(())
}
