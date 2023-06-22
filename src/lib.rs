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

use anyhow::Result;
use options::Options;
use runtime::vm::VM;
use runtime::RuntimeBuilder;

/// Compile a program to a runtime.
pub fn compile(input: &str, output: impl std::io::Write, options: &Options) -> Result<()> {
    let program = parser::parse(input)?;
    check::check_program(&program)?;

    let runtime = RuntimeBuilder::build_runtime(program)?;
    runtime.write::<runtime::target::C>(output, options)
}

/// Execute a program.
pub fn execute(input: &str, options: &Options) -> Result<()> {
    let program = parser::parse(input)?;
    check::check_program(&program)?;

    let runtime = RuntimeBuilder::build_runtime(program)?;
    let vm = VM::new(runtime, options);
    vm.run()
}
