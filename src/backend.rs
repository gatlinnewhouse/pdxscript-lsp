//! tower-lsp Backend implementation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::completions::{builtin_completions, scan_mod_items, static_keywords, variable_completions};
use crate::demorgan::{find_violations, violation_to_action, violations_to_diagnostics};
use crate::gamedir::{TIGER_CONF, find_game_directory_steam, find_paradox_directory, find_workshop_directory_steam};
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
    /// Tier 3 completions per mod root, refreshed after each validation run.
    mod_completions: RwLock<HashMap<PathBuf, Vec<CompletionItem>>>,
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
                    tracing::warn!("Cannot locate Victoria 3 via Steam: {e}");
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
            Err(()) => { tracing::warn!("Cannot convert URI to path: {file_uri}"); return; }
        };
        let mod_root = match Self::find_mod_root(&path) {
            Some(r) => r,
            None => { tracing::debug!("No mod root found for {}", path.display()); return; }
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
        let result = tokio::task::spawn_blocking(move || validate_mod(&mod_root_clone, &cfg)).await;

        match result {
            Ok(Ok(diag_map)) => {
                self.publish_tiger_diagnostics(diag_map).await;
                // Refresh tier-3 completions after each successful validation.
                self.refresh_mod_completions(mod_root).await;
            }
            Ok(Err(e)) => {
                tracing::error!("Validation failed for {}: {e:#}", mod_root.display());
                self.client.show_message(MessageType::ERROR, format!("pdxscript-lsp: {e}")).await;
            }
            Err(e) => tracing::error!("spawn_blocking panicked: {e}"),
        }
    }

    async fn refresh_mod_completions(&self, mod_root: PathBuf) {
        let root_clone = mod_root.clone();
        let game_dir = self.build_validate_config().await.map(|c| c.game_dir);
        let items = tokio::task::spawn_blocking(move || {
            scan_mod_items(&root_clone, game_dir.as_deref()).into_completion_items()
        })
        .await
        .unwrap_or_default();
        self.mod_completions.write().await.insert(mod_root, items);
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
                .filter(|d| d.code != Some(NumberOrString::String("de-morgan".to_owned())))
                .cloned()
                .collect();
            diags.extend(tiger);
        }
        self.client.publish_diagnostics(uri.clone(), diags, None).await;
    }
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
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "@".to_owned(),
                        " ".to_owned(),
                        "=".to_owned(),
                    ]),
                    ..Default::default()
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

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(()) => return Ok(None),
        };

        // Tier 1a: static keywords (AND/OR/if/limit/…)
        let mut items = static_keywords();
        // Tier 1b: engine built-ins from tiger-lib tables (triggers, effects, iterators)
        items.extend_from_slice(builtin_completions());

        // Tier 2: @variables from current document
        let docs = self.documents.read().await;
        if let Some(text) = docs.get(uri) {
            items.extend(variable_completions(text));
        }
        drop(docs);

        // Tier 3: mod filesystem items
        if let Some(mod_root) = Self::find_mod_root(&path) {
            if let Some(cached) = self.mod_completions.read().await.get(&mod_root) {
                items.extend(cached.clone());
            } else {
                // First open before any validation — do a quick blocking scan.
                let root_clone = mod_root.clone();
                let game_dir = self.build_validate_config().await.map(|c| c.game_dir);
                if let Ok(scanned) = tokio::task::spawn_blocking(move || {
                    scan_mod_items(&root_clone, game_dir.as_deref()).into_completion_items()
                })
                .await
                {
                    self.mod_completions.write().await.insert(mod_root, scanned.clone());
                    items.extend(scanned);
                }
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn code_action(&self, params: CodeActionParams) -> LspResult<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().await;
        let text = match docs.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };
        let lines: Vec<&str> = text.lines().collect();
        let violations = find_violations(&lines);
        let request_start = params.range.start.line;
        let request_end = params.range.end.line;
        let actions: Vec<CodeActionOrCommand> = violations
            .iter()
            .filter(|v| v.not_line <= request_end && v.not_close_line >= request_start)
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
