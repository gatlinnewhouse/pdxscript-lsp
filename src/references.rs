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

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    fn fake_uri() -> Url {
        Url::parse("file:///tmp/test.txt").unwrap()
    }

    fn occurrences(name: &str, text: &str) -> Vec<Location> {
        let uri = fake_uri();
        let mut out = Vec::new();
        collect_occurrences(name, text, uri, &mut out);
        out
    }

    #[test]
    fn finds_simple_occurrence() {
        let locs = occurrences("my_effect", "my_effect = yes\n");
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.character, 0);
    }

    #[test]
    fn finds_multiple_occurrences_same_line() {
        let locs = occurrences("foo", "foo = foo\n");
        assert_eq!(locs.len(), 2);
    }

    #[test]
    fn skips_comment_text() {
        let locs = occurrences("foo", "bar = yes # foo here\n");
        assert_eq!(locs.len(), 0, "should not find foo in a comment");
    }

    #[test]
    fn does_not_match_substring() {
        // "foo" should not match inside "foobar" or "myfoo"
        let locs = occurrences("foo", "foobar = yes\nmyfoo = yes\n");
        assert_eq!(locs.len(), 0);
    }

    #[test]
    fn matches_whole_word_boundary() {
        let locs = occurrences("foo", "a = foo\n");
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].range.start.character, 4);
    }

    #[test]
    fn rename_edit_produces_text_edits() {
        let uri = fake_uri();
        let loc = Location {
            uri: uri.clone(),
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 9 },
            },
        };
        let edit = rename_edit(&[loc], "new_name");
        let changes = edit.changes.unwrap();
        let edits = changes.get(&uri).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "new_name");
    }

    #[test]
    fn rename_edit_groups_by_file() {
        let uri1 = Url::parse("file:///tmp/a.txt").unwrap();
        let uri2 = Url::parse("file:///tmp/b.txt").unwrap();
        let make_loc = |uri: Url| Location {
            uri,
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 3 },
            },
        };
        let edit = rename_edit(&[make_loc(uri1.clone()), make_loc(uri2.clone()), make_loc(uri1.clone())], "x");
        let changes = edit.changes.unwrap();
        assert_eq!(changes[&uri1].len(), 2);
        assert_eq!(changes[&uri2].len(), 1);
    }
}
