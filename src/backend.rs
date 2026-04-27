//! tower-lsp Backend implementation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::gamedir::{find_game_directory_steam, find_paradox_directory, find_workshop_directory_steam};
use crate::validate::{DiagMap, ValidateConfig, validate_mod};

/// Settings from `initializationOptions` or workspace config.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Settings {
    /// Absolute path to the Victoria 3 game directory.
    game_dir: Option<String>,
    /// Absolute path to the Steam workshop directory (optional, auto-detected).
    workshop_dir: Option<String>,
}

#[derive(Debug)]
pub struct Backend {
    client: Client,
    settings: RwLock<Settings>,
    /// Serializes concurrent validation requests (one validation at a time).
    validation_semaphore: tokio::sync::Semaphore,
    /// Last known set of diagnostics per URI, used to clear stale diagnostics.
    published: Mutex<HashMap<Url, Vec<Diagnostic>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            settings: RwLock::new(Settings::default()),
            validation_semaphore: tokio::sync::Semaphore::new(1),
            published: Mutex::new(HashMap::new()),
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

        Some(ValidateConfig {
            game_dir,
            workshop_dir,
            paradox_dir,
            config_file: None,
        })
    }

    /// Find the mod root (directory containing `vic3-tiger.conf`) by walking
    /// up from `file_path`.
    fn find_mod_root(file_path: &Path) -> Option<PathBuf> {
        let mut dir = file_path.parent()?;
        loop {
            if dir.join("vic3-tiger.conf").exists() {
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

        // Acquire permit — ensures only one validation runs at a time.
        let _permit = match self.validation_semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => return,
        };

        let mod_root_clone = mod_root.clone();
        let result = tokio::task::spawn_blocking(move || validate_mod(&mod_root_clone, &cfg)).await;

        match result {
            Ok(Ok(diag_map)) => self.publish_diagnostics(diag_map).await,
            Ok(Err(e)) => {
                tracing::error!("Validation failed for {}: {e:#}", mod_root.display());
                self.client
                    .show_message(MessageType::ERROR, format!("pdxscript-lsp: {e}"))
                    .await;
            }
            Err(e) => tracing::error!("spawn_blocking panicked: {e}"),
        }
    }

    /// Publish a fresh set of diagnostics and clear any files that are now
    /// clean (i.e. were in the previous map but are absent from the new one).
    async fn publish_diagnostics(&self, mut new_map: DiagMap) {
        let mut published = self.published.lock().await;

        // Clear diagnostics for files that are now clean.
        for uri in published.keys() {
            new_map.entry(uri.clone()).or_default();
        }

        for (uri, diags) in &new_map {
            self.client.publish_diagnostics(uri.clone(), diags.clone(), None).await;
        }

        *published = new_map.into_iter().filter(|(_, v)| !v.is_empty()).collect();
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        // Apply initial settings if provided.
        if let Some(opts) = params.initialization_options {
            if let Ok(s) = serde_json::from_value::<Settings>(opts) {
                *self.settings.write().await = s;
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::NONE, // we only care about save events
                )),
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
        self.client
            .log_message(MessageType::INFO, "pdxscript-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.run_validation(&params.text_document.uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.run_validation(&params.text_document.uri).await;
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        if let Ok(s) = serde_json::from_value::<Settings>(params.settings) {
            *self.settings.write().await = s;
        }
    }
}
