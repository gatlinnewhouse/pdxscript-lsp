//! Completion item sources for PDX script.
//!
//! Three tiers:
//!   1. Static keywords — always available
//!   2. @variable names — scanned from document text
//!   3. Scripted effects / triggers / events — scanned from mod filesystem

use std::fs;
use std::path::Path;

use tiger_lib::{LspEntryKind, all_builtin_entries};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

// ─── Tier 1: Static keywords ─────────────────────────────────────────────────

pub fn static_keywords() -> Vec<CompletionItem> {
    // (label, detail)
    let keywords: &[(&str, &str)] = &[
        // Logic operators
        ("AND", "logic"),
        ("OR", "logic"),
        ("NOT", "logic"),
        ("NOR", "logic"),
        ("NAND", "logic"),
        // Conditionals
        ("if", "conditional"),
        ("else_if", "conditional"),
        ("else", "conditional"),
        ("trigger_if", "conditional"),
        ("trigger_else_if", "conditional"),
        ("trigger_else", "conditional"),
        ("switch", "conditional"),
        // Common block keys
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
        // Common value keywords
        ("yes", "value"),
        ("no", "value"),
    ];

    keywords
        .iter()
        .map(|&(label, detail)| CompletionItem {
            label: label.to_owned(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_owned()),
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
                // Use KEYWORD so clients don't auto-insert `()`.
                // insert_text appends ` = ` so the user just types the value.
                let insert = format!("{} = ", e.name);
                CompletionItem {
                    label: e.name,
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(detail.to_owned()),
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
    // Must start with `@`, not `@[` (calc) or `@:` (directive)
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
}

impl ModItems {
    pub fn into_completion_items(self) -> Vec<CompletionItem> {
        let mut out = Vec::new();
        for name in self.scripted_effects {
            out.push(item(name, CompletionItemKind::FUNCTION, "scripted_effect"));
        }
        for name in self.scripted_triggers {
            out.push(item(name, CompletionItemKind::FUNCTION, "scripted_trigger"));
        }
        for name in self.scripted_modifiers {
            out.push(item(name, CompletionItemKind::FUNCTION, "scripted_modifier"));
        }
        for name in self.events {
            out.push(item(name, CompletionItemKind::MODULE, "event"));
        }
        out
    }
}

fn item(label: String, kind: CompletionItemKind, detail: &str) -> CompletionItem {
    CompletionItem {
        label,
        kind: Some(kind),
        detail: Some(detail.to_owned()),
        ..Default::default()
    }
}

/// Blocking — call from `tokio::task::spawn_blocking`.
///
/// Scans both the mod directory and, if provided, the game directory so that
/// vanilla scripted triggers/effects (e.g. `has_law_or_variant`) are included.
pub fn scan_mod_items(mod_root: &Path, game_dir: Option<&Path>) -> ModItems {
    let mut items = ModItems {
        scripted_effects: scan_top_level_keys(&mod_root.join("common/scripted_effects")),
        scripted_triggers: scan_top_level_keys(&mod_root.join("common/scripted_triggers")),
        scripted_modifiers: scan_top_level_keys(&mod_root.join("common/scripted_modifiers")),
        events: scan_event_ids(&mod_root.join("events")),
    };

    if let Some(game) = game_dir {
        let game_base = game.join("game");
        items.scripted_effects.extend(
            scan_top_level_keys(&game_base.join("common/scripted_effects"))
        );
        items.scripted_triggers.extend(
            scan_top_level_keys(&game_base.join("common/scripted_triggers"))
        );
        items.scripted_modifiers.extend(
            scan_top_level_keys(&game_base.join("common/scripted_modifiers"))
        );
        items.events.extend(
            scan_event_ids(&game_base.join("events"))
        );
    }

    // Deduplicate — mod items shadow game items but we keep both for now.
    items.scripted_effects.sort_unstable();
    items.scripted_effects.dedup();
    items.scripted_triggers.sort_unstable();
    items.scripted_triggers.dedup();
    items.scripted_modifiers.sort_unstable();
    items.scripted_modifiers.dedup();
    items.events.sort_unstable();
    items.events.dedup();

    items
}

/// Collect `identifier = {` keys at column 0 from every `.txt` in `dir`.
fn scan_top_level_keys(dir: &Path) -> Vec<String> {
    let mut keys = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else { return keys };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else { continue };
        for line in text.lines() {
            if let Some(k) = parse_top_level_key(line) {
                keys.push(k.to_owned());
            }
        }
    }
    keys
}

/// `identifier = {` at column 0. Returns the identifier.
fn parse_top_level_key(line: &str) -> Option<&str> {
    // Reject blank lines, comments, and indented lines
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

/// Collect `namespace.id = {` event ids at column 0.
fn scan_event_ids(events_dir: &Path) -> Vec<String> {
    scan_top_level_keys(events_dir)
        .into_iter()
        .filter(|k| k.contains('.'))
        .collect()
}
