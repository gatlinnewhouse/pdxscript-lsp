//! Run tiger-lib validation and collect LSP diagnostics.
//!
//! tiger-lib stores reports in a global static Mutex (`ERRORS`). Concurrent
//! calls to `take_reports()` would interleave results, so we serialize all
//! validation runs behind `VALIDATION_LOCK`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
#[cfg(any(feature = "vic3", feature = "eu5"))]
use tiger_lib::ModMetadata;
#[cfg(any(feature = "ck3", feature = "imperator", feature = "hoi4"))]
use tiger_lib::ModFile;
use tiger_lib::{
    Confidence, Everything, LspAnnotationKind, Severity,
    set_lsp_mode, take_annotations, take_reports,
};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, InlayHint, InlayHintKind, InlayHintLabel,
    NumberOrString, Position, Range, Url,
};

/// Serializes all validation runs — tiger-lib ERRORS is a global static.
static VALIDATION_LOCK: Mutex<()> = Mutex::new(());

/// Per-file diagnostics keyed by absolute path URI.
pub type DiagMap = HashMap<Url, Vec<Diagnostic>>;

/// Per-file inlay hints collected from tiger-lib scope annotations.
pub type HintMap = HashMap<Url, Vec<InlayHint>>;

/// Configuration paths needed to run validation.
#[derive(Debug, Clone)]
pub struct ValidateConfig {
    pub game_dir: PathBuf,
    pub workshop_dir: Option<PathBuf>,
    pub paradox_dir: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
}

/// Validate the mod at `mod_root` and return per-file diagnostics + scope inlay hints.
///
/// Blocks the calling thread — intended to be run inside
/// `tokio::task::spawn_blocking`.
pub fn validate_mod(mod_root: &Path, cfg: &ValidateConfig) -> Result<(DiagMap, HintMap)> {
    let _lock = VALIDATION_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    // vic3 and eu5 use a metadata folder; ck3/imperator/hoi4 use a .mod file.
    #[cfg(any(feature = "vic3", feature = "eu5"))]
    let mut everything = {
        let metadata = ModMetadata::read(mod_root)?;
        Everything::new(
            cfg.config_file.as_deref(),
            Some(&cfg.game_dir),
            cfg.workshop_dir.as_deref(),
            cfg.paradox_dir.as_deref(),
            mod_root,
            metadata.replace_paths(),
        )?
    };

    #[cfg(any(feature = "ck3", feature = "imperator", feature = "hoi4"))]
    let mut everything = {
        // Look for descriptor.mod in mod_root, or a .mod file next to it.
        let mod_file_path = if mod_root.join("descriptor.mod").exists() {
            mod_root.join("descriptor.mod")
        } else {
            // Try one level up for a .mod file matching the directory name
            let parent = mod_root.parent().unwrap_or(mod_root);
            let stem = mod_root.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("mod");
            let candidate = parent.join(format!("{stem}.mod"));
            if candidate.exists() { candidate } else { mod_root.join("descriptor.mod") }
        };
        let modfile = ModFile::read(&mod_file_path)?;
        Everything::new(
            cfg.config_file.as_deref(),
            Some(&cfg.game_dir),
            cfg.workshop_dir.as_deref(),
            cfg.paradox_dir.as_deref(),
            &modfile.modpath(),
            modfile.replace_paths(),
        )?
    };

    everything.load_output_settings(false);
    everything.load_config_filtering_rules();
    everything.load_all();
    // Enable scope annotations so inlay hints are populated.
    set_lsp_mode(true);
    everything.validate_all();
    set_lsp_mode(false);

    // Drain both stores before releasing the lock.
    let raw = take_reports();
    let annotations = take_annotations();

    let mut map: DiagMap = HashMap::new();

    for (meta, pointer_set) in &raw {
        for pointers in pointer_set {
            // Use the primary pointer (first element) as the diagnostic location.
            let Some(primary) = pointers.first() else {
                continue;
            };

            let fullpath = primary.loc.fullpath();
            let uri = match path_to_uri(fullpath) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let range = loc_to_range(primary.loc.line, primary.loc.column, primary.length);

            // Prefix with confidence tag for weak reports so modders know it may be a false positive.
            let conf_prefix = if meta.confidence == Confidence::Weak {
                "[weak] "
            } else {
                ""
            };
            let mut message = format!("{conf_prefix}[{}] {}", meta.key, meta.msg);
            if meta.confidence == Confidence::Weak {
                message.push_str("\n(low confidence — likely a false positive)");
            }
            if let Some(info) = &meta.info {
                message.push('\n');
                message.push_str(info);
            }
            if let Some(wiki) = &meta.wiki {
                message.push('\n');
                message.push_str(wiki);
            } else if let Some(url) = crate::wiki::fallback_wiki_url(&meta.key.to_string()) {
                message.push('\n');
                message.push_str(url);
            }

            // Attach related locations for secondary pointers.
            let related = if pointers.len() > 1 {
                let related: Vec<_> = pointers[1..]
                    .iter()
                    .filter_map(|p| {
                        let uri = path_to_uri(p.loc.fullpath()).ok()?;
                        let range = loc_to_range(p.loc.line, p.loc.column, p.length);
                        let msg = p.msg.clone().unwrap_or_else(|| "related".to_owned());
                        Some(tower_lsp::lsp_types::DiagnosticRelatedInformation {
                            location: tower_lsp::lsp_types::Location { uri, range },
                            message: msg,
                        })
                    })
                    .collect();
                if related.is_empty() { None } else { Some(related) }
            } else {
                None
            };

            let confidence_str = match meta.confidence {
                Confidence::Weak       => "weak",
                Confidence::Reasonable => "reasonable",
                Confidence::Strong     => "strong",
            };
            let diag = Diagnostic {
                range,
                severity: Some(severity_to_lsp(meta.severity)),
                code: Some(NumberOrString::String(meta.key.to_string())),
                source: Some("pdxscript-lsp".to_owned()),
                message,
                related_information: related,
                // Stash confidence in data so hover can display the real value.
                data: Some(serde_json::json!({ "confidence": confidence_str })),
                ..Default::default()
            };

            map.entry(uri).or_default().push(diag);
        }
    }

    // Convert scope annotations to LSP inlay hints.
    let mut hint_map: HintMap = HashMap::new();
    for ann in annotations {
        let fullpath = ann.loc.fullpath();
        let uri = match path_to_uri(fullpath) {
            Ok(u) => u,
            Err(_) => continue,
        };
        if ann.loc.line == 0 { continue; }
        let LspAnnotationKind::Scope(scope_str) = ann.kind;
        // Show scope at start of the block line (column 0 of that line).
        let pos = Position {
            line: ann.loc.line.saturating_sub(1),
            character: 0,
        };
        let hint = InlayHint {
            position: pos,
            label: InlayHintLabel::String(format!("⟨{scope_str}⟩")),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: None,
            padding_left: Some(false),
            padding_right: Some(true),
            data: None,
        };
        hint_map.entry(uri).or_default().push(hint);
    }

    Ok((map, hint_map))
}

fn path_to_uri(path: &Path) -> Result<Url> {
    Ok(Url::from_file_path(path).map_err(|()| anyhow::anyhow!("bad path: {}", path.display()))?)
}

/// Convert 1-based (line, column) + length to a zero-based LSP `Range`.
/// `line == 0` means file-level — map to the first character.
fn loc_to_range(line: u32, col: u32, length: usize) -> Range {
    let (l, c) = if line == 0 { (0, 0) } else { (line - 1, col.saturating_sub(1)) };
    let end_col = c + u32::try_from(length).unwrap_or(1).max(1);
    Range {
        start: Position { line: l, character: c },
        end: Position { line: l, character: end_col },
    }
}

fn severity_to_lsp(sev: Severity) -> DiagnosticSeverity {
    match sev {
        Severity::Fatal | Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Untidy => DiagnosticSeverity::INFORMATION,
        Severity::Tips => DiagnosticSeverity::HINT,
    }
}
