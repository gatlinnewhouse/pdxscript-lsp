//! LSP document formatting — port of pdxscript.nvim's format_lines.

use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// Return a whole-document TextEdit if the text needs reformatting, else None.
pub fn format_document(text: &str) -> Option<Vec<TextEdit>> {
    let lines: Vec<&str> = text.lines().collect();
    let formatted = format_lines(&lines);

    // Preserve trailing newline
    let mut new_text = formatted.join("\n");
    if text.ends_with('\n') {
        new_text.push('\n');
    }

    if new_text == text {
        return None;
    }

    let end_line = lines.len().saturating_sub(1) as u32;
    let end_char = lines.last().map_or(0, |l| l.len()) as u32;

    Some(vec![TextEdit {
        range: Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: end_line, character: end_char },
        },
        new_text,
    }])
}

fn format_lines(lines: &[&str]) -> Vec<String> {
    let mut result = Vec::with_capacity(lines.len());
    let mut depth: i32 = 0;

    for &line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            result.push(String::new());
            continue;
        }

        let mut opens: i32 = 0;
        let mut closes: i32 = 0;
        let mut leading_closes: i32 = 0;
        let mut in_string = false;
        let mut found_non_brace = false;

        for ch in trimmed.chars() {
            match ch {
                '"' => {
                    in_string = !in_string;
                    found_non_brace = true;
                }
                _ if in_string => {}
                '#' => break,
                '{' => {
                    opens += 1;
                    found_non_brace = true;
                }
                '}' => {
                    closes += 1;
                    if !found_non_brace {
                        leading_closes += 1;
                    }
                }
                c if !c.is_whitespace() => {
                    found_non_brace = true;
                }
                _ => {}
            }
        }

        let indent = (depth - leading_closes).max(0) as usize;
        result.push(format!("{}{trimmed}", "  ".repeat(indent)));
        depth = (depth + opens - closes).max(0);
    }

    result
}
