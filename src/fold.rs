//! Folding ranges from brace structure.

use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

pub fn folding_ranges(text: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    let mut stack: Vec<u32> = Vec::new();

    for (i, line) in text.lines().enumerate() {
        let lnum = i as u32;
        let mut in_str = false;
        for ch in line.chars() {
            match ch {
                '"' => in_str = !in_str,
                '#' if !in_str => break,
                '{' if !in_str => stack.push(lnum),
                '}' if !in_str => {
                    if let Some(start) = stack.pop() {
                        if lnum > start {
                            ranges.push(FoldingRange {
                                start_line: start,
                                end_line: lnum,
                                kind: Some(FoldingRangeKind::Region),
                                ..Default::default()
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    ranges
}
