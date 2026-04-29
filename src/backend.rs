//! tower-lsp Backend implementation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::request::{GotoDeclarationParams, GotoDeclarationResponse};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::completions::{
    builtin_completions, scan_mod_items, static_keywords, variable_completions,
};
use crate::demorgan::{find_violations, violation_to_action, violations_to_diagnostics};
use crate::fold::folding_ranges;
use crate::format::format_document;
use crate::gamedir::{
    TIGER_CONF, find_game_directory_steam, find_paradox_directory, find_workshop_directory_steam,
};
use crate::hover::{hover_builtin, hover_diagnostic_code, hover_scripted, hover_variable};
use crate::references::{find_references, rename_edit};
use crate::symbols::{
    defs_to_locations, document_symbols, word_at, workspace_symbols,
};
use crate::validate::{DiagMap, ValidateConfig, validate_mod};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Settings {
    game_dir: Option<String>,
    workshop_dir: Option<String>,
}

#[derive(Debug)]
pub struct Backend {
    client: Client,
    settings: RwLock<Settings>,
    validation_semaphore: tokio::sync::Semaphore,
    published: Mutex<HashMap<Url, Vec<Diagnostic>>>,
    documents: RwLock<HashMap<Url, String>>,
    /// Completion items per mod root (tier 3).
    mod_completions: RwLock<HashMap<PathBuf, Vec<CompletionItem>>>,
    /// Definition locations + detail for scripted items per mod root.
    /// Value: (Location, detail_str) where detail is "scripted_effect", "event", etc.
    mod_definitions: RwLock<HashMap<PathBuf, HashMap<String, (Location, String)>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            settings: RwLock::new(Settings::default()),
            validation_semaphore: tokio::sync::Semaphore::new(1),
            published: Mutex::new(HashMap::new()),
            documents: RwLock::new(HashMap::new()),
            mod_completions: RwLock::new(HashMap::new()),
            mod_definitions: RwLock::new(HashMap::new()),
        }
    }

    async fn build_validate_config(&self) -> Option<ValidateConfig> {
        let settings = self.settings.read().await;
        let game_dir = if let Some(ref g) = settings.game_dir {
            PathBuf::from(g)
        } else {
            match find_game_directory_steam() {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Cannot locate game via Steam: {e}");
                    return None;
                }
            }
        };
        let workshop_dir = settings
            .workshop_dir
            .as_deref()
            .map(PathBuf::from)
            .or_else(|| find_workshop_directory_steam().ok());
        let paradox_dir = find_paradox_directory();
        Some(ValidateConfig { game_dir, workshop_dir, paradox_dir, config_file: None })
    }

    fn find_mod_root(file_path: &Path) -> Option<PathBuf> {
        let mut dir = file_path.parent()?;
        loop {
            if dir.join(TIGER_CONF).exists() {
                return Some(dir.to_path_buf());
            }
            dir = dir.parent()?;
        }
    }

    async fn run_validation(&self, file_uri: &Url) {
        let path = match file_uri.to_file_path() {
            Ok(p) => p,
            Err(()) => {
                tracing::warn!("Cannot convert URI to path: {file_uri}");
                return;
            }
        };
        let mod_root = match Self::find_mod_root(&path) {
            Some(r) => r,
            None => {
                tracing::debug!("No mod root found for {}", path.display());
                return;
            }
        };
        let cfg = match self.build_validate_config().await {
            Some(c) => c,
            None => return,
        };

        let _permit = match self.validation_semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => return,
        };

        let mod_root_clone = mod_root.clone();
        let result =
            tokio::task::spawn_blocking(move || validate_mod(&mod_root_clone, &cfg)).await;

        match result {
            Ok(Ok(diag_map)) => {
                self.publish_tiger_diagnostics(diag_map).await;
                self.refresh_mod_index(mod_root).await;
            }
            Ok(Err(e)) => {
                tracing::error!("Validation failed for {}: {e:#}", mod_root.display());
                self.client
                    .show_message(MessageType::ERROR, format!("pdxscript-lsp: {e}"))
                    .await;
            }
            Err(e) => tracing::error!("spawn_blocking panicked: {e}"),
        }
    }

    async fn refresh_mod_index(&self, mod_root: PathBuf) {
        let root_clone = mod_root.clone();
        let cfg = self.build_validate_config().await;
        let game_dir = cfg.as_ref().map(|c| c.game_dir.clone());
        let workshop_dir = cfg.as_ref().and_then(|c| c.workshop_dir.clone());

        let scan = tokio::task::spawn_blocking(move || {
            scan_mod_items(&root_clone, game_dir.as_deref(), workshop_dir.as_deref())
        })
        .await
        .unwrap_or_default();

        let raw_defs = scan.definitions.clone();
        let completions = scan.into_completion_items();
        let definitions = defs_to_locations(raw_defs);

        self.mod_completions.write().await.insert(mod_root.clone(), completions);
        self.mod_definitions.write().await.insert(mod_root, definitions);
    }

    async fn publish_tiger_diagnostics(&self, mut new_map: DiagMap) {
        let mut published = self.published.lock().await;
        for uri in published.keys() {
            new_map.entry(uri.clone()).or_default();
        }
        let docs = self.documents.read().await;
        for (uri, diags) in &mut new_map {
            if let Some(text) = docs.get(uri) {
                let lines: Vec<&str> = text.lines().collect();
                let violations = find_violations(&lines);
                diags.extend(violations_to_diagnostics(&violations));
            }
        }
        for (uri, diags) in &new_map {
            self.client.publish_diagnostics(uri.clone(), diags.clone(), None).await;
        }
        *published = new_map.into_iter().filter(|(_, v)| !v.is_empty()).collect();
    }

    async fn publish_demorgan(&self, uri: &Url, text: &str) {
        let lines: Vec<&str> = text.lines().collect();
        let violations = find_violations(&lines);
        let mut diags = violations_to_diagnostics(&violations);
        if let Some(existing) = self.published.lock().await.get(uri) {
            let tiger: Vec<_> = existing
                .iter()
                .filter(|d| {
                    d.code != Some(NumberOrString::String("de-morgan".to_owned()))
                })
                .cloned()
                .collect();
            diags.extend(tiger);
        }
        self.client.publish_diagnostics(uri.clone(), diags, None).await;
    }

    /// Collect all mod search roots for a given mod root (mod + workshop deps).
    async fn search_roots(&self, mod_root: &Path) -> Vec<PathBuf> {
        let cfg = self.build_validate_config().await;
        let mut roots = vec![mod_root.to_path_buf()];
        // Parse load_mod deps from tiger conf
        let workshop_dir = cfg.as_ref().and_then(|c| c.workshop_dir.as_deref().map(PathBuf::from));
        let conf_names = [
            "vic3-tiger.conf",
            "ck3-tiger.conf",
            "imperator-tiger.conf",
            "hoi4-tiger.conf",
            "eu5-tiger.conf",
        ];
        for name in &conf_names {
            let conf = mod_root.join(name);
            if conf.exists() {
                if let Ok(text) = std::fs::read_to_string(&conf) {
                    for dep in parse_load_mod_roots(&text, mod_root, workshop_dir.as_deref()) {
                        roots.push(dep);
                    }
                }
                break;
            }
        }
        if let Some(cfg) = cfg {
            roots.push(cfg.game_dir.join("game"));
        }
        roots
    }
}

/// Extract mod roots from load_mod blocks (duplicate of completions.rs logic, but sync).
fn parse_load_mod_roots(
    conf_text: &str,
    mod_root: &Path,
    workshop_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let mut depth: i32 = 0;
    let mut in_load_mod = false;
    let mut cur_path: Option<String> = None;
    let mut cur_id: Option<String> = None;

    for line in conf_text.lines() {
        let t = line.trim();
        if t.starts_with('#') { continue; }
        if depth == 0 && t.starts_with("load_mod") && t.contains('{') {
            in_load_mod = true;
            cur_path = None;
            cur_id = None;
        }
        for ch in t.chars() {
            if ch == '{' { depth += 1; }
            if ch == '}' { depth -= 1; }
        }
        if in_load_mod && depth > 0 {
            if let Some(v) = extract_conf_value(t, "mod") { cur_path = Some(v); }
            if let Some(v) = extract_conf_value(t, "workshop_id") { cur_id = Some(v); }
        }
        if in_load_mod && depth == 0 {
            in_load_mod = false;
            if let Some(ref p) = cur_path {
                let resolved = if Path::new(p).is_absolute() {
                    PathBuf::from(p)
                } else {
                    mod_root.join(p)
                };
                if resolved.is_dir() { results.push(resolved); }
            } else if let Some(ref id) = cur_id {
                if let Some(ws) = workshop_dir {
                    let p = ws.join(id);
                    if p.is_dir() { results.push(p); }
                }
            }
        }
    }
    results
}

fn extract_conf_value(line: &str, key: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    let rest = rest.split('#').next().unwrap_or(rest).trim();
    let val = if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
        &rest[1..rest.len() - 1]
    } else {
        rest
    };
    if val.is_empty() { None } else { Some(val.to_owned()) }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        if let Some(opts) = params.initialization_options {
            if let Ok(s) = serde_json::from_value::<Settings>(opts) {
                *self.settings.write().await = s;
            }
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                declaration_provider: Some(DeclarationCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "@".to_owned(),
                        "$".to_owned(),
                        " ".to_owned(),
                        "=".to_owned(),
                    ]),
                    ..Default::default()
                }),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["{".to_owned(), " ".to_owned()]),
                    retrigger_characters: Some(vec!["\n".to_owned()]),
                    work_done_progress_options: Default::default(),
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "pdxscript-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("pdxscript-lsp initialized");
        self.client.log_message(MessageType::INFO, "pdxscript-lsp ready").await;
    }

    async fn shutdown(&self) -> LspResult<()> { Ok(()) }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        self.documents.write().await.insert(uri.clone(), text.clone());
        self.publish_demorgan(&uri, &text).await;
        self.run_validation(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            self.documents.write().await.insert(uri.clone(), text.clone());
            self.publish_demorgan(&uri, &text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.run_validation(&params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.write().await.remove(&params.text_document.uri);
    }

    // ─── Hover ───────────────────────────────────────────────────────────────

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = &params.text_document_position_params.position;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t, None => return Ok(None) };
        let line = match text.lines().nth(pos.line as usize) { Some(l) => l, None => return Ok(None) };
        let (word, _, _) = match word_at(line, pos.character as usize) { Some(w) => w, None => return Ok(None) };
        drop(docs);

        // @variable hover — check current document first.
        if word.starts_with('@') {
            let docs = self.documents.read().await;
            if let Some(text) = docs.get(uri) {
                return Ok(hover_variable(&word, text));
            }
        }

        // Diagnostic code hover — if the cursor is inside a published diagnostic range,
        // show severity/confidence explanation for the tiger error key.
        {
            let published = self.published.lock().await;
            if let Some(diags) = published.get(uri) {
                for diag in diags {
                    let r = diag.range;
                    if r.start.line == pos.line
                        && pos.character >= r.start.character
                        && pos.character <= r.end.character
                    {
                        if let Some(NumberOrString::String(ref key)) = diag.code {
                            let sev = match diag.severity {
                                Some(DiagnosticSeverity::ERROR)       => "error",
                                Some(DiagnosticSeverity::WARNING)     => "warning",
                                Some(DiagnosticSeverity::INFORMATION) => "untidy",
                                Some(DiagnosticSeverity::HINT)        => "tips",
                                _ => "unknown",
                            };
                            // Extract wiki link from message (last line starting with http)
                            let wiki = diag.message.lines()
                                .find(|l| l.starts_with("http"))
                                .map(|l| l.trim());
                            let conf = diag.data.as_ref()
                                .and_then(|d| d.get("confidence"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("reasonable");
                            return Ok(Some(hover_diagnostic_code(key, sev, conf, wiki)));
                        }
                    }
                }
            }
        }

        let path = match uri.to_file_path() { Ok(p) => p, Err(()) => return Ok(None) };
        if let Some(mod_root) = Self::find_mod_root(&path) {
            let defs = self.mod_definitions.read().await;
            if let Some(definitions) = defs.get(&mod_root) {
                if let Some((loc, detail)) = definitions.get(&word) {
                    return Ok(hover_scripted(&word, detail, loc));
                }
            }
        }

        Ok(hover_builtin(&word))
    }

    // ─── Go to Definition ────────────────────────────────────────────────────

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = &params.text_document_position_params.position;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t, None => return Ok(None) };
        let line = match text.lines().nth(pos.line as usize) { Some(l) => l, None => return Ok(None) };
        let (word, _, _) = match word_at(line, pos.character as usize) { Some(w) => w, None => return Ok(None) };
        drop(docs);

        let path = match uri.to_file_path() { Ok(p) => p, Err(()) => return Ok(None) };
        if let Some(mod_root) = Self::find_mod_root(&path) {
            let defs = self.mod_definitions.read().await;
            if let Some(definitions) = defs.get(&mod_root) {
                if let Some((loc, _detail)) = definitions.get(&word) {
                    return Ok(Some(GotoDefinitionResponse::Scalar(loc.clone())));
                }
            }
        }
        Ok(None)
    }

    // ─── Declaration (alias to definition) ──────────────────────────────────

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> LspResult<Option<GotoDeclarationResponse>> {
        // PDX script has no separate declaration/definition distinction.
        let def_params = GotoDefinitionParams {
            text_document_position_params: params.text_document_position_params,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        match self.goto_definition(def_params).await? {
            Some(GotoDefinitionResponse::Scalar(loc)) =>
                Ok(Some(GotoDeclarationResponse::Scalar(loc))),
            Some(GotoDefinitionResponse::Array(locs)) =>
                Ok(Some(GotoDeclarationResponse::Array(locs))),
            Some(GotoDefinitionResponse::Link(links)) =>
                Ok(Some(GotoDeclarationResponse::Link(links))),
            None => Ok(None),
        }
    }

    // ─── Find References ─────────────────────────────────────────────────────

    async fn references(&self, params: ReferenceParams) -> LspResult<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = &params.text_document_position.position;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t, None => return Ok(None) };
        let line = match text.lines().nth(pos.line as usize) { Some(l) => l, None => return Ok(None) };
        let (word, _, _) = match word_at(line, pos.character as usize) { Some(w) => w, None => return Ok(None) };
        drop(docs);

        let path = match uri.to_file_path() { Ok(p) => p, Err(()) => return Ok(None) };
        let mod_root = match Self::find_mod_root(&path) { Some(r) => r, None => return Ok(None) };
        let roots = self.search_roots(&mod_root).await;
        let root_refs: Vec<&Path> = roots.iter().map(PathBuf::as_path).collect();

        let refs = tokio::task::spawn_blocking({
            let word = word.clone();
            let roots: Vec<PathBuf> = roots.clone();
            move || find_references(&word, &roots.iter().map(|p| p.as_path()).collect::<Vec<_>>())
        })
        .await
        .unwrap_or_default();

        drop(root_refs);
        Ok(if refs.is_empty() { None } else { Some(refs) })
    }

    // ─── Rename ──────────────────────────────────────────────────────────────

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> LspResult<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let pos = &params.position;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t, None => return Ok(None) };
        let line = match text.lines().nth(pos.line as usize) { Some(l) => l, None => return Ok(None) };
        let (_, start, end) = match word_at(line, pos.character as usize) { Some(w) => w, None => return Ok(None) };

        let range = Range {
            start: Position { line: pos.line, character: start as u32 },
            end: Position { line: pos.line, character: end as u32 },
        };
        Ok(Some(PrepareRenameResponse::Range(range)))
    }

    async fn rename(&self, params: RenameParams) -> LspResult<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = &params.text_document_position.position;
        let new_name = &params.new_name;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t, None => return Ok(None) };
        let line = match text.lines().nth(pos.line as usize) { Some(l) => l, None => return Ok(None) };
        let (word, _, _) = match word_at(line, pos.character as usize) { Some(w) => w, None => return Ok(None) };
        drop(docs);

        let path = match uri.to_file_path() { Ok(p) => p, Err(()) => return Ok(None) };
        let mod_root = match Self::find_mod_root(&path) { Some(r) => r, None => return Ok(None) };
        let roots = self.search_roots(&mod_root).await;

        let refs = tokio::task::spawn_blocking({
            let word = word.clone();
            let roots = roots.clone();
            move || find_references(&word, &roots.iter().map(|p| p.as_path()).collect::<Vec<_>>())
        })
        .await
        .unwrap_or_default();

        if refs.is_empty() { return Ok(None); }
        Ok(Some(rename_edit(&refs, new_name)))
    }

    // ─── Signature Help ──────────────────────────────────────────────────────

    async fn signature_help(&self, params: SignatureHelpParams) -> LspResult<Option<SignatureHelp>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = &params.text_document_position_params.position;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t.clone(), None => return Ok(None) };
        drop(docs);

        // Walk backward from cursor to find the enclosing `name = {` block opener.
        let lines: Vec<&str> = text.lines().collect();
        let cursor_line = pos.line as usize;
        let cursor_col  = pos.character as usize;

        let trigger_name = find_enclosing_block_trigger(&lines, cursor_line, cursor_col);
        let name = match trigger_name { Some(n) => n, None => return Ok(None) };

        let schema = tiger_lib::block_schema(&name);
        let fields = match schema { Some(f) => f, None => return Ok(None) };

        // Build parameter list: "required_field: type" or "?optional_field: type"
        let params_info: Vec<ParameterInformation> = fields.iter().map(|f| {
            let label = if f.required {
                format!("{}: {}", f.name, f.type_hint)
            } else {
                format!("?{}: {}", f.name, f.type_hint)
            };
            ParameterInformation {
                label: ParameterLabel::Simple(label),
                documentation: None,
            }
        }).collect();

        let param_list = fields.iter().map(|f| {
            if f.required { format!("{}: {}", f.name, f.type_hint) }
            else { format!("?{}: {}", f.name, f.type_hint) }
        }).collect::<Vec<_>>().join("  |  ");

        let sig = SignatureInformation {
            label: format!("{name} = {{ {param_list} }}"),
            documentation: None,
            parameters: Some(params_info),
            active_parameter: None,
        };

        Ok(Some(SignatureHelp {
            signatures: vec![sig],
            active_signature: Some(0),
            active_parameter: None,
        }))
    }

    // ─── Document Symbols ────────────────────────────────────────────────────

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> LspResult<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t.clone(), None => return Ok(None) };
        drop(docs);
        let syms = document_symbols(&text);
        if syms.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DocumentSymbolResponse::Nested(syms)))
        }
    }

    // ─── Workspace Symbols ───────────────────────────────────────────────────

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> LspResult<Option<Vec<SymbolInformation>>> {
        let query = &params.query;
        // Merge definitions from all known mod roots.
        let all_defs = self.mod_definitions.read().await;
        let mut merged: HashMap<String, (Location, String)> = HashMap::new();
        for defs in all_defs.values() {
            for (name, entry) in defs {
                merged.entry(name.clone()).or_insert_with(|| entry.clone());
            }
        }
        drop(all_defs);
        let syms = workspace_symbols(query, &merged);
        if syms.is_empty() { Ok(None) } else { Ok(Some(syms)) }
    }

    // ─── Folding Ranges ──────────────────────────────────────────────────────

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> LspResult<Option<Vec<FoldingRange>>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t.clone(), None => return Ok(None) };
        drop(docs);
        let ranges = folding_ranges(&text);
        if ranges.is_empty() { Ok(None) } else { Ok(Some(ranges)) }
    }

    // ─── Formatting ──────────────────────────────────────────────────────────

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> LspResult<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t.clone(), None => return Ok(None) };
        drop(docs);
        Ok(format_document(&text))
    }

    // ─── Completion ──────────────────────────────────────────────────────────

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let path = match uri.to_file_path() { Ok(p) => p, Err(()) => return Ok(None) };

        // When triggered by `$`, only return localization key completions.
        let trigger = params.context.as_ref()
            .and_then(|c| c.trigger_character.as_deref());
        if trigger == Some("$") {
            let mut loca_items = Vec::new();
            if let Some(mod_root) = Self::find_mod_root(&path) {
                let cfg = self.build_validate_config().await;
                let game_dir = cfg.as_ref().map(|c| c.game_dir.clone());
                let keys = tokio::task::spawn_blocking(move || {
                    crate::completions::loca_completions(&mod_root, game_dir.as_deref())
                })
                .await
                .unwrap_or_default();
                loca_items.extend(keys);
            }
            return Ok(Some(CompletionResponse::List(CompletionList {
                is_incomplete: false,
                items: loca_items,
            })));
        }

        let mut items = static_keywords();
        items.extend_from_slice(builtin_completions());

        let docs = self.documents.read().await;
        if let Some(text) = docs.get(uri) {
            items.extend(variable_completions(text));
        }
        drop(docs);

        if let Some(mod_root) = Self::find_mod_root(&path) {
            if let Some(cached) = self.mod_completions.read().await.get(&mod_root) {
                items.extend(cached.clone());
            } else {
                let root_clone = mod_root.clone();
                let cfg = self.build_validate_config().await;
                let game_dir = cfg.as_ref().map(|c| c.game_dir.clone());
                let workshop_dir = cfg.as_ref().and_then(|c| c.workshop_dir.clone());
                if let Ok(scan) = tokio::task::spawn_blocking(move || {
                    scan_mod_items(&root_clone, game_dir.as_deref(), workshop_dir.as_deref())
                })
                .await
                {
                    let raw_defs = scan.definitions.clone();
                    let scanned = scan.into_completion_items();
                    let definitions = defs_to_locations(raw_defs);
                    self.mod_completions.write().await.insert(mod_root.clone(), scanned.clone());
                    self.mod_definitions.write().await.insert(mod_root, definitions);
                    items.extend(scanned);
                }
            }
        }

        Ok(Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items,
        })))
    }

    // ─── Code Actions ────────────────────────────────────────────────────────

    async fn code_action(&self, params: CodeActionParams) -> LspResult<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t, None => return Ok(None) };
        let lines: Vec<&str> = text.lines().collect();
        let violations = find_violations(&lines);
        let start = params.range.start.line;
        let end = params.range.end.line;
        let actions: Vec<CodeActionOrCommand> = violations
            .iter()
            .filter(|v| v.not_line <= end && v.not_close_line >= start)
            .map(|v| violation_to_action(uri, &lines, v))
            .collect();
        if actions.is_empty() { Ok(None) } else { Ok(Some(actions)) }
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        if let Ok(s) = serde_json::from_value::<Settings>(params.settings) {
            *self.settings.write().await = s;
        }
    }
}

/// Walk backward from `(line, col)` to find the identifier before the nearest
/// unclosed `= {`.  Returns `None` if the cursor isn't inside a block.
///
/// Scans upward one line at a time, tracking brace depth.  When depth reaches
/// -1 (we've passed an unmatched `{`), we look at the text immediately before
/// the `{` for a `name = {` or `name = {` pattern and extract the name.
fn find_enclosing_block_trigger(lines: &[&str], cursor_line: usize, cursor_col: usize) -> Option<String> {
    let mut depth: i32 = 0;

    for li in (0..=cursor_line).rev() {
        let line = lines.get(li)?;
        // For the cursor line, only look at chars before the cursor.
        let slice = if li == cursor_line { &line[..cursor_col.min(line.len())] } else { line };

        // Scan right-to-left to find the first unmatched `{`.
        for (ci, ch) in slice.char_indices().rev() {
            match ch {
                '}' => depth += 1,
                '{' => {
                    if depth == 0 {
                        // Found the enclosing brace.  Look for `word = {` before it.
                        let before = slice[..ci].trim_end();
                        // Strip trailing `=` and whitespace.
                        let before = before.trim_end_matches(|c: char| c == '=' || c.is_whitespace());
                        // Extract the last identifier-like word.
                        let word: String = before.chars().rev()
                            .take_while(|c| c.is_alphanumeric() || *c == '_')
                            .collect::<String>()
                            .chars().rev().collect();
                        return if word.is_empty() { None } else { Some(word) };
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
    }
    None
}
