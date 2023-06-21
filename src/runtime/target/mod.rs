//! 编译目标

use anyhow::Result;

use super::ir;

mod c;
pub use c::C;

/// 编译目标
pub trait Target {
    /// 将 IR 编译为目标代码并写入流。
    fn write(f: impl std::io::Write, program: ir::Program) -> Result<()>;
}
