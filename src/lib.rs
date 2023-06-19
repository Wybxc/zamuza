//! An interaction nets compiler.
//!
//! Implements the algorithm described in [An Implementation Model for Interaction Nets](https://doi.org/10.48550/arXiv.1505.07164).

#![deny(missing_docs)]

extern crate pest;
#[macro_use]
extern crate pest_derive;

pub mod ast;
pub mod check;
pub mod parser;
pub mod runtime;

use anyhow::Result;

/// Compile a program to a runtime.
pub fn compile(input: &str) -> Result<String> {
    let program = parser::parse(input)?;
    check::check_program(&program)?;

    let mut runtime_builder = runtime::RuntimeBuilder::new();
    runtime_builder.program(program)?;
    let runtime = runtime_builder.build()?;

    Ok(runtime)
}
