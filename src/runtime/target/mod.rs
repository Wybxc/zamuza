//! 编译目标

use super::Program;
use crate::options::Options;
use std::path::Path;
use thiserror::Error;

mod c;
pub use c::C;

#[cfg(feature = "tinycc")]
mod exe;
#[cfg(feature = "tinycc")]
pub use exe::Exe;

#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("target does not support writing to stream")]
    UnsupportedWriteToStream,

    #[error("formatting error")]
    Fmt(#[from] std::fmt::Error),

    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("ffi error")]
    Ffi(#[from] anyhow::Error),
}

/// 编译目标
pub trait Target {
    /// 将 IR 编译为目标代码并写入流。
    ///
    /// 具体的实现可能只支持写入文件，而不支持写入流。
    fn write(f: impl std::io::Write, program: Program, options: &Options) -> Result<(), Error>;

    /// 将 IR 编译为目标代码并写入文件。
    fn write_to_file(
        filename: impl AsRef<Path>,
        program: Program,
        options: &Options,
    ) -> Result<(), Error> {
        let mut f = std::fs::File::create(filename)?;
        Self::write(&mut f, program, options)
    }
}
