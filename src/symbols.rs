//! Document and workspace symbol extraction.

use std::collections::HashMap;

use tower_lsp::lsp_types::{
    DocumentSymbol, Location, Position, Range, SymbolInformation, SymbolKind, Url,
};

/// Map a scan-time detail string (from `scan_mod_items`) to a SymbolKind.
pub fn kind_from_detail(detail: &str) -> SymbolKind {
    match detail {
        "scripted_effect"   => SymbolKind::FUNCTION,
        "scripted_trigger"  => SymbolKind::OPERATOR,
        "scripted_modifier" => SymbolKind::PROPERTY,
        "event"             => SymbolKind::EVENT,
        _                   => SymbolKind::OBJECT,
    }
}

/// Infer a SymbolKind from a file's `common/` subdirectory name or `events/` path.
/// Used to give document symbols richer icons than the generic FUNCTION fallback.
pub fn kind_from_path(path: &std::path::Path) -> SymbolKind {
    let components: Vec<_> = path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Walk backwards: first `events` dir → EVENT; then check the parent of the filename.
    for (i, &seg) in components.iter().enumerate() {
        if seg == "events" { return SymbolKind::EVENT; }
        if seg == "common" {
            // Next component is the subdir name.
            if let Some(&sub) = components.get(i + 1) {
                return match sub {
                    "scripted_effects"   => SymbolKind::FUNCTION,
                    "scripted_triggers"  => SymbolKind::OPERATOR,
                    "scripted_modifiers" => SymbolKind::PROPERTY,
                    "decisions"          => SymbolKind::ENUM_MEMBER,
                    "on_actions"         => SymbolKind::EVENT,
                    _                    => SymbolKind::MODULE,
                };
            }
        }
    }
    SymbolKind::OBJECT
}

/// Extract top-level `key = { ... }` blocks as document symbols.
pub fn document_symbols(text: &str, file_kind: SymbolKind) -> Vec<DocumentSymbol> {
    let lines: Vec<&str> = text.lines().collect();
    let mut symbols: Vec<DocumentSymbol> = Vec::new();
    let mut depth: i32 = 0;
    let mut pending: Option<(String, u32)> = None; // (name, start_line)

    for (i, &line) in lines.iter().enumerate() {
        let lnum = i as u32;

        // At depth 0, detect `key = {`
        if depth == 0 {
            if let Some(name) = top_level_key(line) {
                pending = Some((name.to_owned(), lnum));
            }
        }

        let mut in_str = false;
        for ch in line.chars() {
            match ch {
                '"' => in_str = !in_str,
                '#' if !in_str => break,
                '{' if !in_str => depth += 1,
                '}' if !in_str => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some((name, start)) = pending.take() {
                            let end_char = line.len() as u32;
                            // Events override the file-level kind.
                            let kind = if name.contains('.') {
                                SymbolKind::EVENT
                            } else {
                                file_kind
                            };
                            symbols.push(DocumentSymbol {
                                name: name.clone(),
                                kind,
                                range: Range {
                                    start: Position { line: start, character: 0 },
                                    end: Position { line: lnum, character: end_char },
                                },
                                selection_range: Range {
                                    start: Position { line: start, character: 0 },
                                    end: Position {
                                        line: start,
                                        character: lines[start as usize].len() as u32,
                                    },
                                },
                                detail: Some(kind_detail(kind)),
                                tags: None,
                                deprecated: None,
                                children: None,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    symbols
}

/// Search workspace symbol index by query string (case-insensitive substring).
pub fn workspace_symbols(
    query: &str,
    definitions: &HashMap<String, (Location, String)>,
) -> Vec<SymbolInformation> {
    let q = query.to_lowercase();
    definitions
        .iter()
        .filter(|(name, _)| name.to_lowercase().contains(&q))
        .map(|(name, (loc, detail))| {
            #[allow(deprecated)]
            SymbolInformation {
                name: name.clone(),
                kind: kind_from_detail(detail),
                location: loc.clone(),
                container_name: None,
                tags: None,
                deprecated: None,
            }
        })
        .collect()
}

fn kind_detail(kind: SymbolKind) -> String {
    match kind {
        SymbolKind::FUNCTION    => "scripted_effect".to_owned(),
        SymbolKind::OPERATOR    => "scripted_trigger".to_owned(),
        SymbolKind::PROPERTY    => "scripted_modifier".to_owned(),
        SymbolKind::EVENT       => "event".to_owned(),
        SymbolKind::ENUM_MEMBER => "decision".to_owned(),
        SymbolKind::MODULE      => "block".to_owned(),
        _                       => "item".to_owned(),
    }
}

/// `identifier = {` at column 0. Returns the identifier.
pub fn top_level_key(line: &str) -> Option<&str> {
    if line.starts_with(|c: char| c.is_whitespace() || c == '#') {
        return None;
    }
    let trimmed = line.trim_end();
    if !trimmed.ends_with('{') {
        return None;
    }
    let before = trimmed[..trimmed.len() - 1].trim_end();
    let ident = before.strip_suffix('=')?.trim_end();
    if !ident.is_empty() && ident.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
        Some(ident)
    } else {
        None
    }
}

/// Word (identifier) at a byte column in a line.
pub fn word_at(line: &str, col: usize) -> Option<(String, usize, usize)> {
    let col = col.min(line.len());
    let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == ':';
    if !line[col..].starts_with(is_ident) && col > 0 && !line[col - 1..].starts_with(is_ident) {
        return None;
    }
    let start = line[..col]
        .rfind(|c: char| !is_ident(c))
        .map_or(0, |i| i + 1);
    let end = col
        + line[col..]
            .find(|c: char| !is_ident(c))
            .unwrap_or(line.len() - col);
    if start >= end {
        return None;
    }
    Some((line[start..end].to_owned(), start, end))
}

/// Convert a `(PathBuf, 1-based-line, detail)` definition map to `(Location, detail)` map.
pub fn defs_to_locations(
    raw: HashMap<String, (std::path::PathBuf, u32, String)>,
) -> HashMap<String, (Location, String)> {
    raw.into_iter()
        .filter_map(|(name, (path, line, detail))| {
            let uri = Url::from_file_path(&path).ok()?;
            let l = line.saturating_sub(1); // 1-based → 0-based
            let range = Range {
                start: Position { line: l, character: 0 },
                end: Position { line: l, character: 0 },
            };
            Some((name, (Location { uri, range }, detail)))
        })
        .collect()
}
