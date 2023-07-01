//! 编译目标

use std::path::Path;

use anyhow::Result;

use crate::options::Options;

use super::Program;

mod c;
mod exe;
pub use c::C;
pub use exe::Exe;

/// 编译目标
pub trait Target {
    /// 将 IR 编译为目标代码并写入流。
    ///
    /// 具体的实现可能只支持写入文件，而不支持写入流。
    fn write(f: impl std::io::Write, program: Program, options: &Options) -> Result<()>;

    /// 将 IR 编译为目标代码并写入文件。
    fn write_to_file(
        filename: impl AsRef<Path>,
        program: Program,
        options: &Options,
    ) -> Result<()> {
        let mut f = std::fs::File::create(filename)?;
        Self::write(&mut f, program, options)
    }
}
