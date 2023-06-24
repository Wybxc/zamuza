use std::ops::Range;

/// 从行列坐标（闭）返回对应的行，以及行内的起止位置（开）。
pub fn lines_span(
    source: &str,
    (line_start, col_start): (usize, usize),
    (line_end, col_end): (usize, usize),
) -> Option<(Range<usize>, (usize, usize))> {
    let mut lines = source.split_inclusive('\n');

    let mut lines_before = 0;
    for _ in 1..line_start {
        lines_before += lines.next()?.len();
    }

    let mut lines_middle = 0;
    for _ in line_start..line_end {
        lines_middle += lines.next()?.len();
    }

    let lines_after = lines.next()?.len();

    let start = lines_before;
    let end = lines_before + lines_middle + lines_after;

    let col_start = col_start - 1;
    let col_end = lines_middle + col_end;

    Some((start..end, (col_start, col_end)))
}
