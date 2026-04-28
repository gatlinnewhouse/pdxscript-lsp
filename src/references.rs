//! Find references and rename across mod files.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tower_lsp::lsp_types::{Location, Position, Range, TextEdit, Url, WorkspaceEdit};

/// Find all file locations where `name` appears as a whole token.
/// `roots` should include the mod dir, dependency dirs, and optionally the game dir.
pub fn find_references(name: &str, roots: &[&Path]) -> Vec<Location> {
    let mut out = Vec::new();
    for root in roots {
        scan_dir(name, root, &mut out);
    }
    out
}

fn scan_dir(name: &str, dir: &Path, out: &mut Vec<Location>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(name, &path, out);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("txt") | Some("gui")
        ) {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            let Ok(uri) = Url::from_file_path(&path) else { continue };
            collect_occurrences(name, &text, uri, out);
        }
    }
}

fn collect_occurrences(name: &str, text: &str, uri: Url, out: &mut Vec<Location>) {
    let nlen = name.len();
    for (line_idx, line) in text.lines().enumerate() {
        // Skip comments
        let effective = line.split('#').next().unwrap_or(line);
        let bytes = effective.as_bytes();
        let mut start = 0usize;
        while let Some(rel) = effective[start..].find(name) {
            let col = start + rel;
            let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'.';
            let before_ok = col == 0 || !is_ident(bytes[col - 1]);
            let after_ok = col + nlen >= bytes.len() || !is_ident(bytes[col + nlen]);
            if before_ok && after_ok {
                out.push(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: line_idx as u32, character: col as u32 },
                        end: Position {
                            line: line_idx as u32,
                            character: (col + nlen) as u32,
                        },
                    },
                });
            }
            start = col + 1;
        }
    }
}

/// Build a WorkspaceEdit that replaces every reference to `old_name` with `new_name`.
pub fn rename_edit(refs: &[Location], new_name: &str) -> WorkspaceEdit {
    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for loc in refs {
        changes
            .entry(loc.uri.clone())
            .or_default()
            .push(TextEdit { range: loc.range, new_text: new_name.to_owned() });
    }
    WorkspaceEdit { changes: Some(changes), ..Default::default() }
}
