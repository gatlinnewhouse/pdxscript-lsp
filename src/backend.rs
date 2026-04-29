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
use crate::validate::{DiagMap, HintMap, ValidateConfig, validate_mod};

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
    /// Scope inlay hints per file URI, collected from tiger-lib scope annotations.
    inlay_hints: RwLock<HintMap>,
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
            inlay_hints: RwLock::new(HashMap::new()),
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
            Ok(Ok((diag_map, hint_map))) => {
                // Merge new hints into global store (replace entries for affected files).
                self.inlay_hints.write().await.extend(hint_map);
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
                document_highlight_provider: Some(OneOf::Left(true)),
                document_link_provider: Some(DocumentLinkOptions {
                    resolve_provider: Some(false),
                    work_done_progress_options: Default::default(),
                }),
                inlay_hint_provider: Some(OneOf::Left(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                        legend: SemanticTokensLegend {
                            token_types: vec![
                                SemanticTokenType::KEYWORD,
                                SemanticTokenType::FUNCTION,
                                SemanticTokenType::VARIABLE,
                                SemanticTokenType::STRING,
                                SemanticTokenType::COMMENT,
                                SemanticTokenType::NUMBER,
                            ],
                            token_modifiers: vec![
                                SemanticTokenModifier::DEFINITION,
                            ],
                        },
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        range: Some(false),
                        work_done_progress_options: Default::default(),
                    }),
                ),
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

    // ─── Document Highlight ──────────────────────────────────────────────────

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> LspResult<Option<Vec<DocumentHighlight>>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = &params.text_document_position_params.position;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t, None => return Ok(None) };
        let line = match text.lines().nth(pos.line as usize) { Some(l) => l, None => return Ok(None) };
        let (word, _, _) = match word_at(line, pos.character as usize) { Some(w) => w, None => return Ok(None) };
        drop(docs);

        // Find all occurrences in this document only.
        let path = match uri.to_file_path() { Ok(p) => p, Err(()) => return Ok(None) };
        let locs = crate::references::find_references(&word, &[&path]);
        if locs.is_empty() { return Ok(None); }
        let highlights = locs.into_iter()
            .map(|loc| DocumentHighlight { range: loc.range, kind: Some(DocumentHighlightKind::TEXT) })
            .collect();
        Ok(Some(highlights))
    }

    // ─── Document Link ───────────────────────────────────────────────────────

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> LspResult<Option<Vec<DocumentLink>>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t.clone(), None => return Ok(None) };
        drop(docs);

        let mut links = Vec::new();
        for (line_idx, line) in text.lines().enumerate() {
            // Extract http(s):// URLs from line.
            let mut search = line;
            let mut col_offset = 0usize;
            while let Some(start) = search.find("http://").or_else(|| search.find("https://")) {
                let url_start = col_offset + start;
                let rest = &search[start..];
                let end = rest.find(|c: char| c.is_whitespace() || c == ')' || c == '"' || c == '\'')
                    .unwrap_or(rest.len());
                let url_str = &rest[..end];
                if let Ok(target) = Url::parse(url_str) {
                    let range = Range {
                        start: Position { line: line_idx as u32, character: url_start as u32 },
                        end: Position { line: line_idx as u32, character: (url_start + end) as u32 },
                    };
                    links.push(DocumentLink {
                        range,
                        target: Some(target),
                        tooltip: None,
                        data: None,
                    });
                }
                col_offset += start + end;
                search = &search[start + end..];
            }
        }
        Ok(if links.is_empty() { None } else { Some(links) })
    }

    // ─── Inlay Hints ─────────────────────────────────────────────────────────

    async fn inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> LspResult<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let hints = self.inlay_hints.read().await;
        if let Some(file_hints) = hints.get(uri) {
            // Filter to the requested range.
            let range = &params.range;
            let filtered: Vec<InlayHint> = file_hints.iter()
                .filter(|h| h.position.line >= range.start.line && h.position.line <= range.end.line)
                .cloned()
                .collect();
            Ok(if filtered.is_empty() { None } else { Some(filtered) })
        } else {
            Ok(None)
        }
    }

    // ─── Code Lens ───────────────────────────────────────────────────────────

    async fn code_lens(&self, params: CodeLensParams) -> LspResult<Option<Vec<CodeLens>>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t.clone(), None => return Ok(None) };
        drop(docs);

        let path = match uri.to_file_path() { Ok(p) => p, Err(()) => return Ok(None) };
        let mod_root = match Self::find_mod_root(&path) { Some(r) => r, None => return Ok(None) };
        let defs = self.mod_definitions.read().await;
        let definitions = match defs.get(&mod_root) { Some(d) => d, None => return Ok(None) };

        let mut lenses = Vec::new();
        // For each top-level definition in this document, count references.
        for (line_idx, line) in text.lines().enumerate() {
            // Look for top-level `name = {` patterns.
            let t = line.trim_end();
            if t.ends_with('{') && !line.starts_with(|c: char| c.is_whitespace()) {
                let before_brace = t[..t.len()-1].trim_end();
                if let Some(name) = before_brace.strip_suffix('=').map(|s| s.trim_end()) {
                    if definitions.contains_key(name) {
                        let roots = self.search_roots(&mod_root).await;
                        let word = name.to_owned();
                        let ref_count = tokio::task::spawn_blocking({
                            let roots = roots.clone();
                            let word = word.clone();
                            move || crate::references::find_references(
                                &word,
                                &roots.iter().map(|p| p.as_path()).collect::<Vec<_>>(),
                            )
                        })
                        .await
                        .map(|r| r.len())
                        .unwrap_or(0);

                        let range = Range {
                            start: Position { line: line_idx as u32, character: 0 },
                            end: Position { line: line_idx as u32, character: t.len() as u32 },
                        };
                        lenses.push(CodeLens {
                            range,
                            command: Some(Command {
                                title: format!(
                                    "{ref_count} reference{}",
                                    if ref_count == 1 { "" } else { "s" }
                                ),
                                command: "editor.action.findReferences".to_owned(),
                                arguments: None,
                            }),
                            data: None,
                        });
                    }
                }
            }
        }
        Ok(if lenses.is_empty() { None } else { Some(lenses) })
    }

    // ─── Selection Range ─────────────────────────────────────────────────────

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> LspResult<Option<Vec<SelectionRange>>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t.clone(), None => return Ok(None) };
        drop(docs);

        let lines: Vec<&str> = text.lines().collect();
        let results = params.positions.iter().map(|pos| {
            selection_range_at(&lines, pos)
        }).collect();
        Ok(Some(results))
    }

    // ─── Semantic Tokens (full document) ────────────────────────────────────

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> LspResult<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) { Some(t) => t.clone(), None => return Ok(None) };
        drop(docs);

        let path = match uri.to_file_path() { Ok(p) => p, Err(()) => return Ok(None) };
        let mod_root = Self::find_mod_root(&path);
        let defs = self.mod_definitions.read().await;
        let definitions: Option<&HashMap<String, (Location, String)>> =
            mod_root.as_ref().and_then(|r| defs.get(r));

        let data = build_semantic_tokens(&text, definitions);
        if data.is_empty() { return Ok(None); }
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data })))
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

/// Expand selection outward from `pos` using brace structure.
/// Returns a chain of nested SelectionRange: word → line → enclosing block → outer block → ...
fn selection_range_at(lines: &[&str], pos: &Position) -> SelectionRange {
    let li = pos.line as usize;
    let ci = pos.character as usize;

    // Innermost: word under cursor.
    let line = lines.get(li).copied().unwrap_or("");
    let word_range = if let Some((_, start, end)) = crate::symbols::word_at(line, ci) {
        Range {
            start: Position { line: pos.line, character: start as u32 },
            end: Position { line: pos.line, character: end as u32 },
        }
    } else {
        Range {
            start: *pos,
            end: Position { line: pos.line, character: ci.saturating_add(1) as u32 },
        }
    };

    // Build enclosing brace ranges by walking backward/forward.
    let mut ranges = vec![word_range];
    // Line range.
    let line_range = Range {
        start: Position { line: pos.line, character: 0 },
        end: Position { line: pos.line, character: line.len() as u32 },
    };
    if line_range != *ranges.last().unwrap() {
        ranges.push(line_range);
    }

    // Enclosing brace ranges — up to 10 levels.
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (line, col) of each `{`

    'outer: for (brace_li, brace_line) in lines.iter().enumerate() {
        let char_iter: Vec<(usize, char)> = brace_line.char_indices().collect();
        for (brace_ci, ch) in &char_iter {
            // Only consider positions before cursor for `{`, after for `}`.
            let before_cursor = brace_li < li || (brace_li == li && *brace_ci <= ci);
            match ch {
                '{' => {
                    if before_cursor {
                        stack.push((brace_li, *brace_ci));
                    }
                }
                '}' => {
                    if !before_cursor {
                        if let Some((open_li, open_ci)) = stack.pop() {
                            let r = Range {
                                start: Position { line: open_li as u32, character: open_ci as u32 },
                                end: Position { line: brace_li as u32, character: (brace_ci + 1) as u32 },
                            };
                            ranges.push(r);
                            if ranges.len() >= 12 { break 'outer; }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Build linked chain from innermost to outermost.
    ranges.dedup();
    ranges.into_iter().rev().fold(None::<SelectionRange>, |parent, range| {
        Some(SelectionRange {
            range,
            parent: parent.map(Box::new),
        })
    }).unwrap_or(SelectionRange { range: word_range, parent: None })
}

/// Build semantic token data for the full document.
/// Token types: 0=keyword, 1=function, 2=variable, 3=string, 4=comment, 5=number
/// Token modifiers: 0x1=definition
fn build_semantic_tokens(
    text: &str,
    definitions: Option<&HashMap<String, (Location, String)>>,
) -> Vec<SemanticToken> {
    use std::sync::OnceLock;
    use tiger_lib::all_builtin_entries;

    static TRIGGER_SET: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    let trigger_set = TRIGGER_SET.get_or_init(|| {
        all_builtin_entries().into_iter().map(|e| e.name).collect()
    });

    let def_set: std::collections::HashSet<&str> = definitions
        .map(|d| d.keys().map(|k| k.as_str()).collect())
        .unwrap_or_default();

    let mut tokens: Vec<SemanticToken> = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for (li, line) in text.lines().enumerate() {
        let li = li as u32;
        let trimmed = line.trim_start();

        // Whole-line comment.
        if trimmed.starts_with('#') {
            let start = line.find('#').unwrap_or(0) as u32;
            tokens.push(SemanticToken {
                delta_line: li - prev_line,
                delta_start: if li == prev_line { start - prev_start } else { start },
                length: (line.len() - line.find('#').unwrap_or(0)) as u32,
                token_type: 4, // comment
                token_modifiers_bitset: 0,
            });
            prev_line = li;
            prev_start = start;
            continue;
        }

        // Scan tokens on the line.
        let chars: Vec<(usize, char)> = line.char_indices().collect();
        let mut char_pos = 0;

        while char_pos < chars.len() {
            let (byte_i, ch) = chars[char_pos];

            // Skip to inline comment.
            if ch == '#' { break; }

            // Quoted string.
            if ch == '"' {
                let end = chars[char_pos+1..].iter()
                    .find(|(_, c)| *c == '"')
                    .map(|(i, _)| char_pos + 1 + chars[char_pos+1..].iter().position(|(bi, _)| *bi == *i + byte_i + 1).unwrap_or(0) + 1)
                    .unwrap_or(chars.len());
                let start = byte_i as u32;
                let len = chars.get(end).map(|(bi, _)| *bi).unwrap_or(line.len()) as u32 - byte_i as u32;
                if len > 0 {
                    tokens.push(SemanticToken {
                        delta_line: li - prev_line,
                        delta_start: if li == prev_line { start - prev_start } else { start },
                        length: len,
                        token_type: 3, // string
                        token_modifiers_bitset: 0,
                    });
                    prev_line = li;
                    prev_start = start;
                }
                char_pos = end;
                continue;
            }

            // Number literal.
            if ch.is_ascii_digit() || (ch == '-' && chars.get(char_pos+1).map(|(_, c)| c.is_ascii_digit()).unwrap_or(false)) {
                let num_start = byte_i;
                let mut np = char_pos + 1;
                while np < chars.len() && (chars[np].1.is_ascii_digit() || chars[np].1 == '.') {
                    np += 1;
                }
                let num_end = chars.get(np).map(|(bi, _)| *bi).unwrap_or(line.len());
                let start = num_start as u32;
                let len = (num_end - num_start) as u32;
                if len > 0 {
                    tokens.push(SemanticToken {
                        delta_line: li - prev_line,
                        delta_start: if li == prev_line { start - prev_start } else { start },
                        length: len,
                        token_type: 5, // number
                        token_modifiers_bitset: 0,
                    });
                    prev_line = li;
                    prev_start = start;
                }
                char_pos = np;
                continue;
            }

            // Identifier.
            if ch.is_alphanumeric() || ch == '_' || ch == '@' {
                let id_start = byte_i;
                let mut np = char_pos + 1;
                while np < chars.len() && (chars[np].1.is_alphanumeric() || chars[np].1 == '_' || chars[np].1 == '.' || chars[np].1 == ':') {
                    np += 1;
                }
                let id_end = chars.get(np).map(|(bi, _)| *bi).unwrap_or(line.len());
                let word = &line[id_start..id_end];

                let (tok_type, modifier) = if word.starts_with('@') {
                    (2u32, 0u32) // variable
                } else if word == "yes" || word == "no" || word == "AND" || word == "OR"
                    || word == "NOT" || word == "NOR" || word == "NAND"
                    || word == "if" || word == "else" || word == "else_if"
                    || word == "trigger_if" || word == "trigger_else" || word == "trigger_else_if"
                    || word == "switch" || word == "limit"
                {
                    (0u32, 0u32) // keyword
                } else if trigger_set.contains(word) {
                    (1u32, 0u32) // function (builtin trigger/effect)
                } else if def_set.contains(word) {
                    // Check if this is a definition site (at column 0, before `=`)
                    let is_def = byte_i == 0 || line[..byte_i].trim().is_empty();
                    (1u32, if is_def { 1u32 } else { 0u32 }) // function ± definition modifier
                } else {
                    char_pos = np;
                    continue;
                };

                let start = id_start as u32;
                let len = (id_end - id_start) as u32;
                tokens.push(SemanticToken {
                    delta_line: li - prev_line,
                    delta_start: if li == prev_line { start - prev_start } else { start },
                    length: len,
                    token_type: tok_type,
                    token_modifiers_bitset: modifier,
                });
                prev_line = li;
                prev_start = start;
                char_pos = np;
                continue;
            }

            char_pos += 1;
        }
    }

    tokens
}
