//! 运行时选项。

/// 运行时选项。
#[derive(Clone, Debug)]
pub struct Options {
    /// 运行时栈大小。
    pub stack_size: usize,
    /// 跟踪规约过程。
    pub trace: bool,
    /// 输出效率信息。
    pub timing: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            stack_size: 1024,
            trace: false,
            timing: false,
        }
    }
}
