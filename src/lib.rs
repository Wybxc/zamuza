//! An interaction nets compiler.
//!
//! Implements the algorithm described in [An Implementation Model for Interaction Nets](https://doi.org/10.48550/arXiv.1505.07164).

#![deny(missing_docs)]
#![deny(unreachable_pub)]

pub mod backend;
pub mod frontend;
pub mod options;
pub(crate) mod utils;

use anyhow::Result;
use backend::RuntimeBuilder;
use backend::{optimize, target::Target};
use frontend::{check, parser};
use options::Options;

/// 编译器上下文
#[derive(Default)]
pub struct Context {
    builder: RuntimeBuilder,
    options: Options,
}

impl Context {
    /// 创建一个新的编译器上下文。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置运行时选项。
    pub fn set_options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// 编译源代码。
    pub fn add_file(mut self, filename: &str, source: &str) -> Result<Self> {
        let module = match parser::parse(source, filename) {
            Ok(module) => module,
            Err(snippet) => anyhow::bail!("{}", snippet),
        };

        if let Err(e) = check::check_module(&module) {
            anyhow::bail!("{}", e.to_snippet());
        }

        self.builder.module(module.into_inner())?;
        Ok(self)
    }

    /// 输出到流。
    pub fn output_stream<T: Target>(self, output: impl std::io::Write) -> Result<()> {
        let mut runtime = self.builder.build()?;
        optimize::optimize(&mut runtime);

        T::write(output, runtime, &self.options)?;
        Ok(())
    }

    /// 输出到文件。
    pub fn output_file<T: Target>(self, output: impl AsRef<std::path::Path>) -> Result<()> {
        let mut runtime = self.builder.build()?;
        optimize::optimize(&mut runtime);

        T::write_to_file(output, runtime, &self.options)?;
        Ok(())
    }

    /// 运行。
    #[cfg(feature = "tinycc")]
    pub fn run(self) -> Result<()> {
        let mut output = std::io::Cursor::new(Vec::new());
        self.output_stream::<backend::target::C>(&mut output)?;
        let output = std::ffi::CString::new(output.into_inner())?;

        tinycc::Context::new(tinycc::OutputType::Memory)?
            .compile_string(&output)?
            .run(&[])?;

        Ok(())
    }
}
