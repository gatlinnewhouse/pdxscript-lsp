//! De Morgan's law detection and transform for PDX script.
//!
//! Detects `NOT = { AND/OR = { ... } }` and rewrites to
//! `AND/OR = { NOT = { ... } ... }`.

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, DiagnosticSeverity,
    NumberOrString, Position, Range, TextEdit, Url, WorkspaceEdit,
};

use std::collections::HashMap;

/// A De Morgan violation found in a document.
#[derive(Debug, Clone)]
pub struct Violation {
    /// 0-based line of the `NOT = {` opener
    pub not_line: u32,
    /// 0-based line of the closing `}` of the NOT block
    pub not_close_line: u32,
    /// The inner operator: "AND" or "OR"
    pub inner_op: String,
    /// 0-based line of the `AND/OR = {` opener
    pub inner_line: u32,
    /// 0-based char offset of the `{` on inner_line
    pub inner_brace_col: u32,
    /// 0-based line of the closing `}` of the inner block
    pub inner_close_line: u32,
    /// 0-based char offset of the `}` on inner_close_line
    pub inner_close_col: u32,
}

/// Scan all lines and return every De Morgan violation.
pub fn find_violations(lines: &[&str]) -> Vec<Violation> {
    let mut violations = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("NOT") {
            continue;
        }
        // Must be `NOT = {` (with optional spaces around `=`)
        let after_not = trimmed["NOT".len()..].trim_start();
        if !after_not.starts_with('=') {
            continue;
        }
        let after_eq = after_not[1..].trim_start();
        if !after_eq.starts_with('{') {
            continue;
        }
        // Find the `{` column in the original line
        let not_brace_col = line.find('{').unwrap_or(0);

        let (not_close_line, not_close_col) =
            match find_close(lines, i, not_brace_col) {
                Some(v) => v,
                None => continue,
            };

        // Find the inner AND/OR = { immediately inside (only whitespace allowed between)
        // Returns (op, line, op_start_col, brace_col).
        let (inner_op, inner_line_idx, inner_op_col, inner_brace_col) =
            match find_inner_op(lines, i, not_brace_col, not_close_line, not_close_col) {
                Some(v) => v,
                None => continue,
            };

        let (inner_close_line, inner_close_col) =
            match find_close(lines, inner_line_idx, inner_brace_col) {
                Some(v) => v,
                None => continue,
            };

        // Verify nothing else sits inside the NOT block beside the inner op
        if has_other_content(
            lines,
            i,
            not_brace_col,
            not_close_line,
            not_close_col,
            inner_line_idx,
            inner_op_col,    // use op keyword start, not brace col
            inner_close_line,
            inner_close_col,
        ) {
            continue;
        }

        violations.push(Violation {
            not_line: i as u32,
            not_close_line: not_close_line as u32,
            inner_op,
            inner_line: inner_line_idx as u32,
            inner_brace_col: inner_brace_col as u32,
            inner_close_line: inner_close_line as u32,
            inner_close_col: inner_close_col as u32,
        });
    }

    violations
}

/// Build LSP hint diagnostics for a set of violations.
pub fn violations_to_diagnostics(violations: &[Violation]) -> Vec<Diagnostic> {
    violations
        .iter()
        .map(|v| {
            let new_op = if v.inner_op == "OR" { "AND" } else { "OR" };
            Diagnostic {
                range: Range {
                    start: Position { line: v.not_line, character: 0 },
                    end: Position { line: v.not_close_line, character: u32::MAX },
                },
                severity: Some(DiagnosticSeverity::HINT),
                code: Some(NumberOrString::String("de-morgan".to_owned())),
                source: Some("pdxscript-lsp".to_owned()),
                message: format!(
                    "[de-morgan] NOT {{ {op} {{ … }} }} → {new} {{ NOT {{ … }} … }}",
                    op = v.inner_op,
                    new = new_op,
                ),
                ..Default::default()
            }
        })
        .collect()
}

/// Build a code action for `violation` that produces a `WorkspaceEdit`.
pub fn violation_to_action(
    uri: &Url,
    lines: &[&str],
    violation: &Violation,
) -> CodeActionOrCommand {
    let edit = build_edit(uri, lines, violation);
    let new_op = if violation.inner_op == "OR" { "AND" } else { "OR" };
    CodeActionOrCommand::CodeAction(CodeAction {
        title: format!(
            "De Morgan: NOT {{ {op} {{…}} }} → {new} {{ NOT {{…}} … }}",
            op = violation.inner_op,
            new = new_op,
        ),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(edit),
        command: None,
        is_preferred: Some(true),
        ..Default::default()
    })
}

// ─── Internals ───────────────────────────────────────────────────────────────

/// Find the matching `}` for the `{` at (line_idx, brace_col).
/// Returns (line_idx, col) of the `}`, both 0-based.
fn find_close(lines: &[&str], start_line: usize, start_col: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    for l in start_line..lines.len() {
        let line = lines[l];
        let col_from = if l == start_line { start_col } else { 0 };
        let mut in_string = false;
        for (c, ch) in line.char_indices().filter(|(c, _)| *c >= col_from) {
            match ch {
                '"' => in_string = !in_string,
                '#' if !in_string => break,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((l, c));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

/// Look for `AND = {` or `OR = {` immediately after the NOT's `{`.
/// Only whitespace/comments are allowed between the NOT `{` and the inner op.
/// Returns `(op, line, op_keyword_col, brace_col)`.
fn find_inner_op(
    lines: &[&str],
    not_line: usize,
    not_brace_col: usize,
    not_close_line: usize,
    not_close_col: usize,
) -> Option<(String, usize, usize, usize)> {
    // Check same line after not_brace_col first, then subsequent lines.
    let search_lines: Vec<(usize, &str, usize)> = {
        let mut v = Vec::new();
        for l in not_line..=not_close_line {
            let line = lines[l];
            let col_from = if l == not_line { not_brace_col + 1 } else { 0 };
            let col_to = if l == not_close_line { not_close_col } else { line.len() };
            v.push((l, &line[..col_to.min(line.len())], col_from));
        }
        v
    };

    for (l, line, col_from) in search_lines {
        let segment = &line[col_from..];
        let trimmed = segment.trim_start();
        let spaces = segment.len() - trimmed.len();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Must be AND or OR followed by ` = {`
        for op in &["AND", "OR"] {
            if trimmed.starts_with(op) {
                let after_op = trimmed[op.len()..].trim_start();
                if after_op.starts_with('=') {
                    let after_eq = after_op[1..].trim_start();
                    if after_eq.starts_with('{') {
                        // Column of the op keyword start ("AND"/"OR").
                        let op_col = col_from + spaces;
                        // Column of the `{`.
                        let brace_col = op_col + op.len()
                            + (trimmed[op.len()..].len() - after_op.len())
                            + 1  // the `=`
                            + (after_op[1..].len() - after_eq.len());
                        return Some((op.to_string(), l, op_col, brace_col));
                    }
                }
            }
        }
        // Hit non-whitespace that isn't the inner op — bail
        break;
    }
    None
}

/// Return true if there is any non-whitespace content inside the NOT block
/// that is not part of the inner AND/OR block.
fn has_other_content(
    lines: &[&str],
    not_line: usize,
    not_brace_col: usize,
    not_close_line: usize,
    not_close_col: usize,
    inner_line: usize,
    inner_brace_col: usize,
    inner_close_line: usize,
    inner_close_col: usize,
) -> bool {
    for l in not_line..=not_close_line {
        let line = lines[l];
        let col_from = if l == not_line { not_brace_col + 1 } else { 0 };
        let col_to = if l == not_close_line { not_close_col } else { line.len() };

        for (c, ch) in line.char_indices() {
            if c < col_from || c >= col_to {
                continue;
            }
            // Skip the inner block's extent
            if l >= inner_line && l <= inner_close_line {
                let inner_start = if l == inner_line { inner_brace_col } else { 0 };
                let inner_end = if l == inner_close_line { inner_close_col + 1 } else { line.len() };
                if c >= inner_start && c < inner_end {
                    continue;
                }
            }
            match ch {
                ' ' | '\t' | '}' => {}
                '#' => break, // rest of line is comment
                _ => return true,
            }
        }
    }
    false
}

/// Extract the direct children of the block bounded by (open_line, open_col) .. (close_line, close_col).
/// Each child is a (start_line, start_col, end_line, end_col) span (0-based, inclusive).
fn extract_children(
    lines: &[&str],
    open_line: usize,
    open_col: usize,
    close_line: usize,
    close_col: usize,
) -> Vec<(usize, usize, usize, usize)> {
    let mut children = Vec::new();
    let mut depth = 0usize;
    let mut child_start: Option<(usize, usize)> = None;
    let mut in_string = false;

    for l in open_line..=close_line {
        let line = lines[l];
        let col_from = if l == open_line { open_col + 1 } else { 0 };
        let col_to = if l == close_line { close_col } else { line.len() };

        for (c, ch) in line.char_indices() {
            if c < col_from || c >= col_to {
                continue;
            }
            match ch {
                '"' => {
                    in_string = !in_string;
                    if depth == 0 && child_start.is_none() {
                        child_start = Some((l, c));
                    }
                }
                '#' if !in_string && depth == 0 => break,
                '{' if !in_string => {
                    if depth == 0 && child_start.is_none() {
                        child_start = Some((l, c));
                    }
                    depth += 1;
                }
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(start) = child_start.take() {
                            children.push((start.0, start.1, l, c));
                        }
                    }
                }
                ch if !in_string && !ch.is_whitespace() => {
                    if depth == 0 && child_start.is_none() {
                        child_start = Some((l, c));
                    }
                }
                _ => {}
            }
        }

        // Flush a non-brace child that ends at line boundary
        if depth == 0 {
            if let Some(start) = child_start.take() {
                // Only flush if line had actual non-space content
                let seg = if l == close_line {
                    &line[..close_col.min(line.len())]
                } else {
                    line
                };
                if start.1 <= seg.len() && seg[start.1..].trim_end().len() > 0 {
                    children.push((start.0, start.1, l, col_to.saturating_sub(1)));
                }
            }
        }
    }

    children
}

/// Collect the text of a child span as a single trimmed string.
fn child_text(lines: &[&str], sl: usize, sc: usize, el: usize, ec: usize) -> String {
    if sl == el {
        let line = lines[sl];
        let sc = sc.min(line.len());
        let ec = ec.min(line.len().saturating_sub(1));
        return if sc <= ec { line[sc..=ec].trim().to_owned() } else { String::new() };
    }
    let first = lines[sl];
    let sc = sc.min(first.len());
    let mut parts = vec![first[sc..].trim_end().to_owned()];
    for l in (sl + 1)..el {
        parts.push(lines[l].trim().to_owned());
    }
    let last = lines[el];
    let ec = ec.min(last.len().saturating_sub(1));
    parts.push(last[..=ec].trim().to_owned());
    parts.join("\n")
}

/// Build the `WorkspaceEdit` for a violation.
fn build_edit(uri: &Url, lines: &[&str], v: &Violation) -> WorkspaceEdit {
    let indent = {
        let not_line = lines[v.not_line as usize];
        let spaces = not_line.len() - not_line.trim_start().len();
        " ".repeat(spaces)
    };

    let new_op = if v.inner_op == "OR" { "AND" } else { "OR" };

    let children = extract_children(
        lines,
        v.inner_line as usize,
        v.inner_brace_col as usize,
        v.inner_close_line as usize,
        v.inner_close_col as usize,
    );

    let mut new_lines: Vec<String> = vec![format!("{indent}{new_op} = {{")];
    for (sl, sc, el, ec) in children {
        let text = child_text(lines, sl, sc, el, ec);
        if text.contains('\n') {
            new_lines.push(format!("{indent}    NOT = {{"));
            for part in text.lines() {
                new_lines.push(format!("{indent}        {part}"));
            }
            new_lines.push(format!("{indent}    }}"));
        } else {
            new_lines.push(format!("{indent}    NOT = {{ {text} }}"));
        }
    }
    new_lines.push(format!("{indent}}}"));

    let new_text = new_lines.join("\n");

    // Replace from start of NOT line to end of NOT close line (inclusive)
    let edit_range = Range {
        start: Position { line: v.not_line, character: 0 },
        end: Position {
            line: v.not_close_line,
            character: lines[v.not_close_line as usize].len() as u32,
        },
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![TextEdit { range: edit_range, new_text }]);

    WorkspaceEdit { changes: Some(changes), ..Default::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Vec<&str> {
        s.lines().collect()
    }

    #[test]
    fn detects_not_or() {
        let script = "NOT = {\n  OR = {\n    a = yes\n    b = yes\n  }\n}";
        let vs = find_violations(&lines(script));
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].inner_op, "OR");
        assert_eq!(vs[0].not_line, 0);
    }

    #[test]
    fn detects_not_and() {
        let script = "NOT = {\n  AND = {\n    x = yes\n  }\n}";
        let vs = find_violations(&lines(script));
        assert_eq!(vs.len(), 1);
        assert_eq!(vs[0].inner_op, "AND");
    }

    #[test]
    fn no_violation_for_plain_not() {
        let script = "NOT = {\n  a = yes\n  b = no\n}";
        assert!(find_violations(&lines(script)).is_empty());
    }

    #[test]
    fn no_violation_when_not_wraps_non_logic() {
        // NOT wrapping something that is neither AND nor OR.
        let script = "NOT = {\n  is_ruler = yes\n}";
        assert!(find_violations(&lines(script)).is_empty());
    }

    #[test]
    fn multiple_violations() {
        let script = concat!(
            "NOT = {\n  OR = {\n    a = yes\n  }\n}\n",
            "NOT = {\n  AND = {\n    b = yes\n  }\n}"
        );
        let vs = find_violations(&lines(script));
        assert_eq!(vs.len(), 2);
    }

    #[test]
    fn diagnostic_hint_severity() {
        let script = "NOT = {\n  OR = {\n    a = yes\n  }\n}";
        let vs = find_violations(&lines(script));
        let diags = violations_to_diagnostics(&vs);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::HINT));
    }

    #[test]
    fn diagnostic_message_mentions_de_morgan() {
        let script = "NOT = {\n  OR = {\n    a = yes\n  }\n}";
        let vs = find_violations(&lines(script));
        let diags = violations_to_diagnostics(&vs);
        assert!(diags[0].message.contains("de-morgan") || diags[0].message.contains("OR"));
    }

    // ─── Rewrite output tests ─────────────────────────────────────────────────

    fn rewrite_text(script: &str) -> String {
        let ls = lines(script);
        let vs = find_violations(&ls);
        assert!(!vs.is_empty(), "no violations found in: {script}");
        let uri = Url::parse("file:///test.txt").unwrap();
        let action = violation_to_action(&uri, &ls, &vs[0]);
        match action {
            CodeActionOrCommand::CodeAction(ca) => {
                let edit = ca.edit.unwrap();
                let changes = edit.changes.unwrap();
                let edits = changes.into_values().next().unwrap();
                edits.into_iter().next().unwrap().new_text
            }
            _ => panic!("expected CodeAction"),
        }
    }

    #[test]
    fn rewrite_not_or_produces_and_not() {
        let script = "NOT = {\n    OR = {\n        a = yes\n        b = yes\n    }\n}";
        let text = rewrite_text(script);
        assert!(text.contains("AND = {"), "missing AND: {text}");
        assert!(!text.contains("OR = {"), "OR not removed: {text}");
        assert!(!text.contains("NOT = {\n    OR"), "outer NOT not removed: {text}");
        // Both children wrapped in NOT
        assert_eq!(text.matches("NOT = {").count(), 2, "expected 2 NOT wrappers: {text}");
    }

    #[test]
    fn rewrite_not_and_produces_or_not() {
        let script = "NOT = {\n    AND = {\n        x = yes\n    }\n}";
        let text = rewrite_text(script);
        assert!(text.contains("OR = {"), "missing OR: {text}");
        assert!(!text.contains("AND = {"), "AND not removed: {text}");
        assert_eq!(text.matches("NOT = {").count(), 1, "expected 1 NOT wrapper: {text}");
    }

    #[test]
    fn rewrite_not_or_two_has_variable() {
        // Reproduces the real-world crash case: `has_variable = X` children inside OR.
        let script =
            "NOT = {\n    OR = {\n        has_variable = ismail_var\n        has_variable = hawduqo_var\n    }\n}";
        let text = rewrite_text(script);
        assert!(text.contains("AND = {"), "missing AND: {text}");
        assert!(text.contains("has_variable = ismail_var"), "first child lost: {text}");
        assert!(text.contains("has_variable = hawduqo_var"), "second child lost: {text}");
        assert_eq!(text.matches("NOT = {").count(), 2, "expected 2 NOT wrappers: {text}");
        assert!(!text.contains("AND = {\n}"), "empty AND block: {text}");
    }

    #[test]
    fn rewrite_preserves_indentation() {
        let script = "    NOT = {\n        OR = {\n            a = yes\n        }\n    }";
        let text = rewrite_text(script);
        assert!(text.starts_with("    AND = {"), "wrong indent: {text}");
    }

    #[test]
    fn rewrite_multiline_child_wraps_in_block() {
        // Child spanning multiple lines should produce NOT = { \n ... \n } block form.
        let script = "NOT = {\n    OR = {\n        trigger_if = {\n            limit = { a = yes }\n        }\n    }\n}";
        let text = rewrite_text(script);
        assert!(text.contains("AND = {"), "missing AND: {text}");
        assert!(text.contains("trigger_if"), "multiline child lost: {text}");
    }
}

