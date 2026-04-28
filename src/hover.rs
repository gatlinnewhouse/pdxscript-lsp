//! Hover text for scripted items and engine built-ins.

use std::collections::HashMap;
use std::fs;

use tiger_lib::{LspEntryKind, all_builtin_entries};
use tower_lsp::lsp_types::{Hover, HoverContents, Location, MarkupContent, MarkupKind};

use crate::wiki;

/// Hover for a scripted item — shows the first 20 lines of its definition.
pub fn hover_scripted(name: &str, detail: &str, loc: &Location) -> Option<Hover> {
    let path = loc.uri.to_file_path().ok()?;
    let text = fs::read_to_string(path).ok()?;
    let start = loc.range.start.line as usize;

    // Show up to 20 lines, stop early at a closing `}` at depth 0.
    let snippet: String = {
        let mut lines_out = Vec::new();
        let mut depth: i32 = 0;
        for line in text.lines().skip(start).take(20) {
            lines_out.push(line);
            for ch in line.chars() {
                if ch == '{' { depth += 1; }
                if ch == '}' { depth -= 1; }
            }
            if depth <= 0 && !lines_out.is_empty() { break; }
        }
        lines_out.join("\n")
    };

    let wiki_url = wiki::scripted_wiki_url(detail);
    let value = format!(
        "**{name}** — {detail}\n\n\
         ```pdxscript\n{snippet}\n```\n\n\
         [Wiki: {detail}]({wiki_url})"
    );

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
        range: None,
    })
}

/// Hover for an engine built-in trigger/effect/iterator.
pub fn hover_builtin(name: &str) -> Option<Hover> {
    builtin_index().get(name).map(|kind| {
        let wiki_url = wiki::builtin_wiki_url(kind);
        let value = match kind {
            LspEntryKind::Trigger => format!(
                "**{name}** — engine trigger\n\n[Wiki: Triggers]({wiki_url})"
            ),
            LspEntryKind::Effect => format!(
                "**{name}** — engine effect\n\n[Wiki: Effects]({wiki_url})"
            ),
            LspEntryKind::Iterator => format!(
                "**{name}** — engine iterator\n\n\
                 Expands to: `every_{name}`, `any_{name}`, `random_{name}`, `ordered_{name}`\n\n\
                 [Wiki: Scopes]({wiki_url})"
            ),
        };
        Hover {
            contents: HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value }),
            range: None,
        }
    })
}

/// Hover for an `@variable` — shows value if findable in the document text.
pub fn hover_variable(name: &str, doc_text: &str) -> Option<Hover> {
    let bare = name.trim_start_matches('@');
    for line in doc_text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix('@') {
            if rest.starts_with(bare) {
                let after = rest[bare.len()..].trim_start();
                if after.starts_with('=') || after.is_empty() {
                    let wiki_url = wiki::variables_url();
                    let value = format!(
                        "**@{bare}** — variable\n\n`{t}`\n\n[Wiki: Variables]({wiki_url})"
                    );
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value,
                        }),
                        range: None,
                    });
                }
            }
        }
    }
    None
}

fn builtin_index() -> &'static HashMap<String, LspEntryKind> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<HashMap<String, LspEntryKind>> = OnceLock::new();
    CACHE.get_or_init(|| {
        all_builtin_entries()
            .into_iter()
            .map(|e| (e.name, e.kind))
            .collect()
    })
}
