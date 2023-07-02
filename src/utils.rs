use std::{fmt::Display, ops::Deref};

use annotate_snippets::snippet::{AnnotationType, Slice, SourceAnnotation};

/// 位置信息片段
#[derive(Debug, Clone, PartialEq)]
pub struct Span<'a, T>
where
    T: 'a,
{
    inner: T,
    pub filename: &'a str,
    pub source: &'a str,
    pub start: usize,
    pub end: usize,
}

impl<'a, T: Display> Display for Span<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'a, T> Deref for Span<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> AsRef<T> for Span<'a, T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<'a, T> Span<'a, T> {
    /// 创建一个新的 `Span`。
    pub fn new(inner: T, filename: &'a str, source: &'a str, start: usize, end: usize) -> Self {
        Self {
            inner,
            filename,
            source,
            start,
            end,
        }
    }

    pub fn from_pest(inner: T, filename: &'a str, source: &'a str, span: pest::Span<'a>) -> Self {
        Self::new(inner, filename, source, span.start(), span.end())
    }

    /// 转换为内部类型。
    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn lines(&self) -> Option<LinesInfo<'a>> {
        let mut lines = self.source.split_inclusive('\n').enumerate();

        let mut start = 0;
        let mut line_start = 0;
        for (i, line) in lines.by_ref() {
            let next = start + line.len();
            if next > self.start {
                line_start = i + 1;
                break;
            }
            start = next;
        }
        if line_start == 0 {
            return None;
        }

        let mut end = start;
        for (_, line) in lines {
            let next = end + line.len();
            if next > self.end {
                break;
            }
            end = next;
        }

        Some(LinesInfo {
            filename: self.filename,
            source: &self.source[start..end],
            line_start,
            range: (self.start - start, self.end - start),
        })
    }
}

/// 所在行的信息
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LinesInfo<'a> {
    /// 文件名
    pub filename: &'a str,
    /// 包含 Span 的某几行
    pub source: &'a str,
    /// 起始行号
    pub line_start: usize,
    /// Span 在 source 中的切片
    pub range: (usize, usize),
}

impl<'a> LinesInfo<'a> {
    pub fn as_annotation(&self, message: &'a str, annotation_type: AnnotationType) -> Slice<'a> {
        Slice {
            source: self.source,
            line_start: self.line_start,
            origin: Some(self.filename),
            annotations: vec![SourceAnnotation {
                range: self.range,
                label: message,
                annotation_type,
            }],
            fold: true,
        }
    }
}
