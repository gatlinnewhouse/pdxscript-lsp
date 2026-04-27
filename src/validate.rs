//! Run tiger-lib validation and collect LSP diagnostics.
//!
//! tiger-lib stores reports in a global static Mutex (`ERRORS`). Concurrent
//! calls to `take_reports()` would interleave results, so we serialize all
//! validation runs behind `VALIDATION_LOCK`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
use tiger_lib::{Everything, ModMetadata, Severity, take_reports};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, Url,
};

/// Serializes all validation runs — tiger-lib ERRORS is a global static.
static VALIDATION_LOCK: Mutex<()> = Mutex::new(());

/// Per-file diagnostics keyed by absolute path URI.
pub type DiagMap = HashMap<Url, Vec<Diagnostic>>;

/// Configuration paths needed to run validation.
#[derive(Debug, Clone)]
pub struct ValidateConfig {
    pub game_dir: PathBuf,
    pub workshop_dir: Option<PathBuf>,
    pub paradox_dir: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
}

/// Validate the mod at `mod_root` and return per-file diagnostics.
///
/// Blocks the calling thread — intended to be run inside
/// `tokio::task::spawn_blocking`.
pub fn validate_mod(mod_root: &Path, cfg: &ValidateConfig) -> Result<DiagMap> {
    let _lock = VALIDATION_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let metadata = ModMetadata::read(mod_root)?;

    let mut everything = Everything::new(
        cfg.config_file.as_deref(),
        Some(&cfg.game_dir),
        cfg.workshop_dir.as_deref(),
        cfg.paradox_dir.as_deref(),
        mod_root,
        metadata.replace_paths(),
    )?;

    everything.load_output_settings(false);
    everything.load_config_filtering_rules();
    everything.load_all();
    everything.validate_all();

    // Drain the global report store before releasing the lock.
    let raw = take_reports();

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

            let mut message = meta.msg.clone();
            if let Some(info) = &meta.info {
                message.push('\n');
                message.push_str(info);
            }
            if let Some(wiki) = &meta.wiki {
                message.push('\n');
                message.push_str(wiki);
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

            let diag = Diagnostic {
                range,
                severity: Some(severity_to_lsp(meta.severity)),
                code: Some(NumberOrString::String(meta.key.to_string())),
                source: Some("pdxscript-lsp".to_owned()),
                message,
                related_information: related,
                ..Default::default()
            };

            map.entry(uri).or_default().push(diag);
        }
    }

    Ok(map)
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
