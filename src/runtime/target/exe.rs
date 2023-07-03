//! 编译到可执行文件的运行时

use std::path::Path;

use crate::{options::Options, runtime::Program};

use super::Target;

/// 编译到可执行文件的运行时
pub struct Exe;

impl Target for Exe {
    fn write(
        _f: impl std::io::Write,
        _program: Program,
        _options: &Options,
    ) -> Result<(), super::Error> {
        Err(super::Error::UnsupportedWriteToStream)
    }

    fn write_to_file(
        filename: impl AsRef<Path>,
        program: Program,
        options: &Options,
    ) -> Result<(), super::Error> {
        let mut buf = std::io::Cursor::new(Vec::new());
        super::C::write(&mut buf, program, options)?;
        let c_code = unsafe { std::ffi::CString::from_vec_unchecked(buf.into_inner()) };

        compile(filename, &c_code)?;
        Ok(())
    }
}

fn compile(filename: impl AsRef<Path>, c_code: &std::ffi::CStr) -> Result<(), anyhow::Error> {
    tinycc::Context::new(tinycc::OutputType::Exe)?
        .compile_string(c_code)?
        .output_file(filename)?;
    Ok(())
}
