//! Completion item sources for PDX script.
//!
//! Three tiers:
//!   1. Static keywords — always available
//!   1b. Built-in triggers/effects/iterators from tiger-lib tables
//!   2. @variable names — scanned from document text
//!   3. Scripted effects / triggers / events / @variables — scanned from mod filesystem
//!      including dependency mods listed in *-tiger.conf load_mod blocks

use std::fs;
use std::path::{Path, PathBuf};

use tiger_lib::{LspEntryKind, all_builtin_entries};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, InsertTextFormat,
};

// ─── Tier 1: Static keywords ─────────────────────────────────────────────────

pub fn static_keywords() -> Vec<CompletionItem> {
    let keywords: &[(&str, &str)] = &[
        ("AND", "logic"),
        ("OR", "logic"),
        ("NOT", "logic"),
        ("NOR", "logic"),
        ("NAND", "logic"),
        ("if", "conditional"),
        ("else_if", "conditional"),
        ("else", "conditional"),
        ("trigger_if", "conditional"),
        ("trigger_else_if", "conditional"),
        ("trigger_else", "conditional"),
        ("switch", "conditional"),
        ("limit", "block"),
        ("trigger", "block"),
        ("effect", "block"),
        ("option", "block"),
        ("immediate", "block"),
        ("after", "block"),
        ("modifier", "block"),
        ("on_accept", "block"),
        ("on_decline", "block"),
        ("on_pass", "block"),
        ("on_fail", "block"),
        ("ai_chance", "block"),
        ("yes", "value"),
        ("no", "value"),
    ];

    keywords
        .iter()
        .map(|&(label, detail)| CompletionItem {
            label: label.to_owned(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_owned()),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(format!(" {detail}")),
                description: None,
            }),
            ..Default::default()
        })
        .collect()
}

// ─── Tier 1b: Built-in triggers/effects/iterators from tiger-lib tables ──────

/// Engine built-ins from tiger-lib's compiled tables. Cached after first call.
pub fn builtin_completions() -> &'static [CompletionItem] {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<CompletionItem>> = OnceLock::new();
    CACHE.get_or_init(|| {
        all_builtin_entries()
            .into_iter()
            .map(|e| {
                let detail = match e.kind {
                    LspEntryKind::Trigger => "trigger",
                    LspEntryKind::Effect => "effect",
                    LspEntryKind::Iterator => "iterator",
                };
                // KEYWORD kind prevents clients from auto-inserting `()`.
                // insert_text appends ` = ` so the user types the value directly.
                let insert = format!("{} = ", e.name);
                CompletionItem {
                    label: e.name,
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(detail.to_owned()),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: Some(format!(" {detail}")),
                        description: None,
                    }),
                    insert_text: Some(insert),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                }
            })
            .collect()
    })
}

// ─── Tier 2: @variable names from document text ───────────────────────────────

/// Extract every `@name` defined in `text` (lines of the form `@name = value`).
pub fn variable_completions(text: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for line in text.lines() {
        if let Some(name) = parse_variable_def(line) {
            items.push(CompletionItem {
                label: format!("@{name}"),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("variable".to_owned()),
                ..Default::default()
            });
        }
    }
    items
}

fn parse_variable_def(line: &str) -> Option<&str> {
    let t = line.trim_start();
    let rest = t.strip_prefix('@')?;
    if rest.starts_with('[') || rest.starts_with(':') {
        return None;
    }
    let name = rest.split(|c: char| c == '=' || c.is_whitespace()).next()?;
    if name.is_empty() { None } else { Some(name) }
}

// ─── Tier 3: Mod filesystem scan ─────────────────────────────────────────────

/// Named items discovered by scanning the mod directory.
#[derive(Debug, Default)]
pub struct ModItems {
    pub scripted_effects: Vec<String>,
    pub scripted_triggers: Vec<String>,
    pub scripted_modifiers: Vec<String>,
    pub events: Vec<String>,
    /// @variable names found across all script files in the mod tree.
    pub at_variables: Vec<String>,
    /// GFX/sprite names from .gfx files.
    pub icons: Vec<String>,
    /// Game items from common/ subdirs: (name, subdir_name).
    pub game_items: Vec<(String, String)>,
    /// Definition locations: item name → (absolute path, 1-based line, detail).
    /// detail is one of: "scripted_effect", "scripted_trigger", "scripted_modifier", "event".
    pub definitions: std::collections::HashMap<String, (std::path::PathBuf, u32, String)>,
}

impl ModItems {
    pub fn into_completion_items(self) -> Vec<CompletionItem> {
        let mut out = Vec::new();
        for name in self.scripted_effects {
            out.push(scripted_item(name, "scripted_effect"));
        }
        for name in self.scripted_triggers {
            out.push(scripted_item(name, "scripted_trigger"));
        }
        for name in self.scripted_modifiers {
            out.push(scripted_item(name, "scripted_modifier"));
        }
        for name in self.events {
            // Events are referenced by id, not as `id = ` assignments, so no insert_text suffix.
            out.push(CompletionItem {
                label: name,
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("event".to_owned()),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(" event".to_owned()),
                    description: None,
                }),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
        for name in self.at_variables {
            out.push(CompletionItem {
                label: format!("@{name}"),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("@variable".to_owned()),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(" @var".to_owned()),
                    description: None,
                }),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
        for name in self.icons {
            out.push(CompletionItem {
                label: format!("@{name}!"),
                kind: Some(CompletionItemKind::COLOR),
                detail: Some("icon".to_owned()),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(" icon".to_owned()),
                    description: None,
                }),
                insert_text: Some(format!("@{name}!")),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
        for (name, subdir) in self.game_items {
            out.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: Some(subdir.clone()),
                label_details: Some(CompletionItemLabelDetails {
                    detail: Some(format!(" {subdir}")),
                    description: None,
                }),
                insert_text: Some(name),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
        out
    }
}

/// Completion item for a scripted trigger/effect/modifier.
/// FUNCTION kind for correct icon/semantics. Explicit insert_text prevents client `()` insertion.
fn scripted_item(label: String, detail: &str) -> CompletionItem {
    let insert = format!("{label} = ");
    CompletionItem {
        label,
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(detail.to_owned()),
        label_details: Some(CompletionItemLabelDetails {
            detail: Some(format!(" {detail}")),
            description: None,
        }),
        insert_text: Some(insert),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    }
}

/// Blocking — call from `tokio::task::spawn_blocking`.
///
/// Scans the mod, any dependency mods listed in `*-tiger.conf` load_mod blocks,
/// and the game directory for scripted items and @variables.
pub fn scan_mod_items(
    mod_root: &Path,
    game_dir: Option<&Path>,
    workshop_dir: Option<&Path>,
) -> ModItems {
    // Collect all roots to scan: primary mod + dependencies + game dir.
    let dep_roots = parse_load_mod_paths(mod_root, workshop_dir);

    let mut items = scan_single_mod(mod_root);

    for dep in &dep_roots {
        merge_items(&mut items, scan_single_mod(dep));
    }

    if let Some(game) = game_dir {
        let game_base = game.join("game");
        merge_items(&mut items, scan_single_mod(&game_base));
    }

    dedup_items(&mut items);
    items
}

/// Subdirectories of `common/` already handled as scripted items — skip to avoid duplication.
const SCRIPTED_SUBDIRS: &[&str] = &[
    "scripted_effects",
    "scripted_triggers",
    "scripted_modifiers",
];

fn scan_single_mod(root: &Path) -> ModItems {
    let effects_locs  = scan_top_level_keys_with_locs(&root.join("common/scripted_effects"));
    let triggers_locs = scan_top_level_keys_with_locs(&root.join("common/scripted_triggers"));
    let mods_locs     = scan_top_level_keys_with_locs(&root.join("common/scripted_modifiers"));
    let events_locs   = scan_event_ids_with_locs(&root.join("events"));

    let mut definitions = std::collections::HashMap::new();
    for (name, path, line) in &effects_locs {
        definitions.entry(name.clone()).or_insert_with(|| (path.clone(), *line, "scripted_effect".to_owned()));
    }
    for (name, path, line) in &triggers_locs {
        definitions.entry(name.clone()).or_insert_with(|| (path.clone(), *line, "scripted_trigger".to_owned()));
    }
    for (name, path, line) in &mods_locs {
        definitions.entry(name.clone()).or_insert_with(|| (path.clone(), *line, "scripted_modifier".to_owned()));
    }
    for (name, path, line) in &events_locs {
        definitions.entry(name.clone()).or_insert_with(|| (path.clone(), *line, "event".to_owned()));
    }

    // Collect icons and game items.
    let icons = collect_icon_names(root);
    let game_items = collect_game_item_names(root);

    ModItems {
        scripted_effects:  effects_locs.into_iter().map(|(n, _, _)| n).collect(),
        scripted_triggers: triggers_locs.into_iter().map(|(n, _, _)| n).collect(),
        scripted_modifiers: mods_locs.into_iter().map(|(n, _, _)| n).collect(),
        events: events_locs.into_iter().map(|(n, _, _)| n).collect(),
        at_variables: scan_at_variables_in_tree(root),
        icons,
        game_items,
        definitions,
    }
}

/// Collect GFX sprite names from root/gfx/.
fn collect_icon_names(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    scan_gfx_dir(&root.join("gfx"), &mut names);
    names
}

/// Collect game items from root/common/ (excluding already-handled scripted dirs).
fn collect_game_item_names(root: &Path) -> Vec<(String, String)> {
    let mut items = Vec::new();
    let common = root.join("common");
    let Ok(entries) = fs::read_dir(&common) else { return items };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let subdir = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_owned(),
            None => continue,
        };
        if SCRIPTED_SUBDIRS.contains(&subdir.as_str()) { continue; }
        for (name, _, _) in scan_top_level_keys_with_locs(&path) {
            items.push((name, subdir.clone()));
        }
    }
    items
}

fn merge_items(dst: &mut ModItems, src: ModItems) {
    dst.scripted_effects.extend(src.scripted_effects);
    dst.scripted_triggers.extend(src.scripted_triggers);
    dst.scripted_modifiers.extend(src.scripted_modifiers);
    dst.events.extend(src.events);
    dst.at_variables.extend(src.at_variables);
    dst.icons.extend(src.icons);
    dst.game_items.extend(src.game_items);
    for (k, v) in src.definitions {
        // Mod definitions take precedence over dependency/game definitions.
        dst.definitions.entry(k).or_insert(v);
    }
}

fn dedup_items(items: &mut ModItems) {
    for v in [
        &mut items.scripted_effects,
        &mut items.scripted_triggers,
        &mut items.scripted_modifiers,
        &mut items.events,
        &mut items.at_variables,
        &mut items.icons,
    ] {
        v.sort_unstable();
        v.dedup();
    }
    items.game_items.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    items.game_items.dedup_by(|a, b| a.0 == b.0);
    // definitions HashMap deduplicates naturally.
}

// ─── Tiger conf dependency parsing ───────────────────────────────────────────

/// Parse `load_mod` blocks from `*-tiger.conf` and return resolved mod root paths.
///
/// Handles two forms:
///   `mod = "/absolute/or/relative/path"`
///   `workshop_id = 123456789`  (resolved via `workshop_dir`)
fn parse_load_mod_paths(mod_root: &Path, workshop_dir: Option<&Path>) -> Vec<PathBuf> {
    // Find the tiger conf — whichever one exists.
    let conf_names = [
        "vic3-tiger.conf",
        "ck3-tiger.conf",
        "imperator-tiger.conf",
        "hoi4-tiger.conf",
        "eu5-tiger.conf",
    ];
    let conf_path = conf_names
        .iter()
        .map(|n| mod_root.join(n))
        .find(|p| p.exists());
    let conf_path = match conf_path {
        Some(p) => p,
        None => return vec![],
    };
    let text = match fs::read_to_string(&conf_path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    let mut results = Vec::new();
    // Walk through load_mod = { ... } blocks.
    let mut depth: i32 = 0;
    let mut in_load_mod = false;
    let mut current_mod_path: Option<String> = None;
    let mut current_workshop_id: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with('#') { continue; }

        // Detect load_mod block start
        if depth == 0 && trimmed.starts_with("load_mod") && trimmed.contains('{') {
            in_load_mod = true;
            current_mod_path = None;
            current_workshop_id = None;
        }

        // Count braces
        for ch in trimmed.chars() {
            if ch == '{' { depth += 1; }
            if ch == '}' { depth -= 1; }
        }

        if in_load_mod && depth > 0 {
            // Parse `mod = "path"` or `workshop_id = 123`
            if let Some(val) = extract_value(trimmed, "mod") {
                current_mod_path = Some(val);
            }
            if let Some(val) = extract_value(trimmed, "workshop_id") {
                current_workshop_id = Some(val);
            }
        }

        // Block closed
        if in_load_mod && depth == 0 {
            in_load_mod = false;
            // `mod` path takes precedence over workshop_id per tiger docs.
            if let Some(ref p) = current_mod_path {
                let resolved = if Path::new(p).is_absolute() {
                    PathBuf::from(p)
                } else {
                    mod_root.join(p)
                };
                if resolved.is_dir() {
                    results.push(resolved);
                }
            } else if let Some(ref id) = current_workshop_id {
                if let Some(ws) = workshop_dir {
                    let path = ws.join(id);
                    if path.is_dir() {
                        results.push(path);
                    }
                }
            }
        }
    }

    results
}

/// Extract the value from a line like `key = "value"` or `key = value`.
fn extract_value(line: &str, key: &str) -> Option<String> {
    // Match `key = ...` allowing optional whitespace and optional quotes.
    let rest = line.trim_start();
    let rest = rest.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    // Strip comment suffix
    let rest = rest.split('#').next().unwrap_or(rest).trim();
    // Strip surrounding quotes
    let val = if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
        &rest[1..rest.len() - 1]
    } else {
        rest
    };
    if val.is_empty() { None } else { Some(val.to_owned()) }
}

// ─── @variable scanning ───────────────────────────────────────────────────────

/// Walk all `.txt` files under `root` and collect `@name = value` variable names.
/// Searches common/ and events/ subdirectories only to avoid binary/gfx dirs.
fn scan_at_variables_in_tree(root: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for subdir in &["common", "events", "scripted_guis"] {
        scan_at_variables_in_dir(&root.join(subdir), &mut names);
    }
    names
}

fn scan_at_variables_in_dir(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_at_variables_in_dir(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            for line in text.lines() {
                if let Some(name) = parse_variable_def(line) {
                    out.push(name.to_owned());
                }
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Collect `identifier = {` keys with their definition location (path, 1-based line).
fn scan_top_level_keys_with_locs(dir: &Path) -> Vec<(String, PathBuf, u32)> {
    let mut out = Vec::new();
    scan_keys_recursive(dir, &mut out);
    out
}

fn scan_keys_recursive(dir: &Path, out: &mut Vec<(String, PathBuf, u32)>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_keys_recursive(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            for (i, line) in text.lines().enumerate() {
                if let Some(k) = parse_top_level_key(line) {
                    out.push((k.to_owned(), path.clone(), (i + 1) as u32));
                }
            }
        }
    }
}

/// `identifier = {` at column 0. Returns the identifier.
fn parse_top_level_key(line: &str) -> Option<&str> {
    if line.starts_with(|c: char| c.is_whitespace() || c == '#') {
        return None;
    }
    let trimmed = line.trim_end();
    if !trimmed.ends_with('{') {
        return None;
    }
    let before_brace = trimmed[..trimmed.len() - 1].trim_end();
    let ident = before_brace.strip_suffix('=')?.trim_end();
    if !ident.is_empty()
        && ident.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        Some(ident)
    } else {
        None
    }
}

fn scan_event_ids_with_locs(events_dir: &Path) -> Vec<(String, PathBuf, u32)> {
    scan_top_level_keys_with_locs(events_dir)
        .into_iter()
        .filter(|(k, _, _)| k.contains('.'))
        .collect()
}

// ─── Localization key completions ────────────────────────────────────────────

/// Return completion items for localization keys (`$KEY$` macro syntax).
/// Scans the mod's `localization/` dir and, if provided, the game's `localization/` dir.
/// Only returns keys from the same mod to avoid flooding with 250k+ vanilla keys.
pub fn loca_completions(mod_root: &Path, game_dir: Option<&Path>) -> Vec<CompletionItem> {
    let mut keys = Vec::new();
    scan_loca_dir(&mod_root.join("localization"), &mut keys);
    // Game dir keys are intentionally excluded — too many (250k+).
    // Only include them if the mod has no localization of its own.
    if keys.is_empty() {
        if let Some(game) = game_dir {
            scan_loca_dir(&game.join("game").join("localization"), &mut keys);
        }
    }
    keys.sort_unstable();
    keys.dedup();
    keys.into_iter()
        .map(|key| CompletionItem {
            label: key.clone(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some("localization key".to_owned()),
            label_details: Some(CompletionItemLabelDetails {
                detail: Some(" loca".to_owned()),
                description: None,
            }),
            insert_text: Some(key),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        })
        .collect()
}

fn scan_loca_dir(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_loca_dir(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("yml") {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            for line in text.lines() {
                if let Some(key) = parse_loca_key(line) {
                    out.push(key.to_owned());
                }
            }
        }
    }
}

/// Parse a localization key from a line of the form `KEY: 0 "text"` or `KEY: "text"`.
/// Skips language headers (`l_english:`) and comment lines.
fn parse_loca_key(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') { return None; }
    // Language header lines like `l_english:` — skip
    if t.starts_with("l_") && t.ends_with(':') { return None; }
    // Key is everything before the first colon, no spaces allowed
    let key = t.split(':').next()?.trim();
    if key.is_empty() || key.contains(' ') || key.starts_with('"') {
        None
    } else {
        Some(key)
    }
}

// ─── Icon / sprite name completions ──────────────────────────────────────────

fn scan_gfx_dir(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_gfx_dir(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("gfx") {
            let Ok(text) = fs::read_to_string(&path) else { continue };
            for line in text.lines() {
                if let Some(name) = parse_gfx_name(line) {
                    out.push(name.to_owned());
                }
            }
        }
    }
}

/// Extract the value of `name = "GFX_..."` lines in .gfx files.
fn parse_gfx_name(line: &str) -> Option<&str> {
    let t = line.trim();
    if t.starts_with('#') { return None; }
    // Match lines of the form `name = "value"` (after any block key)
    let rest = t.strip_prefix("name")?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    let name = if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
        &rest[1..rest.len() - 1]
    } else {
        rest.split_whitespace().next()?
    };
    if name.is_empty() { None } else { Some(name) }
}

