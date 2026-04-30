//! Call hierarchy: prepare + incoming/outgoing calls for scripted items.

use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    Location, Position, Range, SymbolKind,
};

use crate::references::find_references;
use crate::symbols::{top_level_key, word_at};

/// Build a CallHierarchyItem for a scripted definition.
pub fn make_item(name: &str, loc: &Location, detail: &str) -> CallHierarchyItem {
    let kind = match detail {
        "scripted_effect"   => SymbolKind::FUNCTION,
        "scripted_trigger"  => SymbolKind::OPERATOR,
        "scripted_modifier" => SymbolKind::PROPERTY,
        "event"             => SymbolKind::EVENT,
        _                   => SymbolKind::OBJECT,
    };
    CallHierarchyItem {
        name: name.to_owned(),
        kind,
        tags: None,
        detail: Some(detail.to_owned()),
        uri: loc.uri.clone(),
        range: loc.range,
        selection_range: loc.range,
        data: None,
    }
}

/// `textDocument/prepareCallHierarchy`: return a CallHierarchyItem if cursor is on a
/// known scripted item name.
pub fn prepare(
    params: &CallHierarchyPrepareParams,
    text: &str,
    definitions: &HashMap<String, (Location, String)>,
) -> Option<Vec<CallHierarchyItem>> {
    let pos = &params.text_document_position_params.position;
    let line = text.lines().nth(pos.line as usize)?;
    let (word, _, _) = word_at(line, pos.character as usize)?;
    let (loc, detail) = definitions.get(&word)?;
    Some(vec![make_item(&word, loc, detail)])
}

/// `callHierarchy/incomingCalls`: who calls this item?
/// Groups all references by the top-level scripted item that contains them.
pub fn incoming_calls(
    params: &CallHierarchyIncomingCallsParams,
    definitions: &HashMap<String, (Location, String)>,
    roots: &[PathBuf],
) -> Vec<CallHierarchyIncomingCall> {
    let name = &params.item.name;
    let refs = find_references(name, &roots.iter().map(|p| p.as_path()).collect::<Vec<_>>());

    // Group reference locations by their containing file + top-level symbol.
    let mut caller_map: HashMap<String, (CallHierarchyItem, Vec<Range>)> = HashMap::new();

    for loc in refs {
        // Skip the definition itself (same location as the item's range).
        if Some(&loc) == definitions.get(name).map(|(l, _)| l) {
            continue;
        }

        let file_path = match loc.uri.to_file_path() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let text = match std::fs::read_to_string(&file_path) {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Find the top-level symbol containing this reference.
        let container = containing_symbol(&text, loc.range.start.line);
        let caller_name = match container {
            Some(n) => n,
            None => continue, // reference is not inside any named block — skip
        };

        let key = format!("{}#{}", loc.uri, caller_name);
        let entry = caller_map.entry(key).or_insert_with(|| {
            // Try to get the definition location of the caller for richer info.
            let caller_item = if let Some((caller_loc, detail)) = definitions.get(&caller_name) {
                make_item(&caller_name, caller_loc, detail)
            } else {
                // Caller not in definitions — build a minimal item from the reference file.
                let caller_line = find_top_level_key_line(&text, &caller_name).unwrap_or(0);
                let range = Range {
                    start: Position { line: caller_line, character: 0 },
                    end: Position { line: caller_line, character: caller_name.len() as u32 },
                };
                let caller_loc = Location { uri: loc.uri.clone(), range };
                make_item(&caller_name, &caller_loc, "block")
            };
            (caller_item, Vec::new())
        });
        entry.1.push(loc.range);
    }

    caller_map
        .into_values()
        .map(|(item, from_ranges)| CallHierarchyIncomingCall { from: item, from_ranges })
        .collect()
}

/// `callHierarchy/outgoingCalls`: what scripted items does this item call?
/// Scans the body of the item for identifiers that are in `definitions`.
pub fn outgoing_calls(
    params: &CallHierarchyOutgoingCallsParams,
    text: &str,
    definitions: &HashMap<String, (Location, String)>,
) -> Vec<CallHierarchyOutgoingCall> {
    let item = &params.item;
    let start_line = item.range.start.line as usize;
    let end_line = item.range.end.line as usize;

    let lines: Vec<&str> = text.lines().collect();
    // Collect all identifier mentions within the item's line range.
    let mut callees: HashMap<String, Vec<Range>> = HashMap::new();

    for (li, line) in lines.iter().enumerate() {
        if li < start_line || li > end_line { continue; }
        // Skip the first line (it's the definition header `name = {`).
        if li == start_line { continue; }

        let effective = line.split('#').next().unwrap_or(line);
        let mut col = 0usize;
        let chars: Vec<(usize, char)> = effective.char_indices().collect();
        let mut ci = 0;
        while ci < chars.len() {
            let (byte_i, ch) = chars[ci];
            if ch.is_alphanumeric() || ch == '_' {
                let id_start = byte_i;
                let mut np = ci + 1;
                while np < chars.len() && (chars[np].1.is_alphanumeric() || chars[np].1 == '_') {
                    np += 1;
                }
                let id_end = chars.get(np).map(|(b, _)| *b).unwrap_or(effective.len());
                let word = &effective[id_start..id_end];
                if definitions.contains_key(word) {
                    let range = Range {
                        start: Position { line: li as u32, character: id_start as u32 },
                        end: Position { line: li as u32, character: id_end as u32 },
                    };
                    callees.entry(word.to_owned()).or_default().push(range);
                }
                col = id_end;
                ci = np;
                continue;
            }
            col = byte_i + ch.len_utf8();
            ci += 1;
        }
        let _ = col;
    }

    callees
        .into_iter()
        .filter_map(|(callee_name, from_ranges)| {
            let (loc, detail) = definitions.get(&callee_name)?;
            let to = make_item(&callee_name, loc, detail);
            Some(CallHierarchyOutgoingCall { to, from_ranges })
        })
        .collect()
}

/// Find the name of the top-level `name = {` block that contains `line_idx`.
/// Walks backwards from `line_idx` at depth 0 to find the opening `name = {`.
fn containing_symbol(text: &str, line_idx: u32) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut depth: i32 = 0;

    for i in (0..=(line_idx as usize).min(lines.len().saturating_sub(1))).rev() {
        let line = lines[i];
        let opens: i32 = line.chars().filter(|&c| c == '{').count() as i32;
        let closes: i32 = line.chars().filter(|&c| c == '}').count() as i32;

        if i == line_idx as usize {
            // On the target line, count from right to get depth above this line.
            depth += closes - opens;
        } else {
            depth += closes - opens;
        }

        if depth < 0 {
            // We've crossed an unmatched `{` — this is the enclosing block.
            if i == 0 {
                return top_level_key(line).map(str::to_owned);
            }
            depth = 0;
        }

        if i == 0 {
            return top_level_key(line).map(str::to_owned);
        }
    }
    None
}

/// Find the 0-based line number of `name = {` at column 0.
fn find_top_level_key_line(text: &str, name: &str) -> Option<u32> {
    for (i, line) in text.lines().enumerate() {
        if top_level_key(line) == Some(name) {
            return Some(i as u32);
        }
    }
    None
}
