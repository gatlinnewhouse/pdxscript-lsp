mod backend;
mod completions;
mod demorgan;
mod fold;
mod format;
mod gamedir;
mod hover;
mod references;
mod symbols;
mod validate;
mod wiki;

use tower_lsp::{LspService, Server};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Log to file to avoid polluting stdio (LSP uses stdio for protocol)
    let log_dir = dirs_or_home();
    let file_appender = tracing_appender::rolling::never(&log_dir, "pdxscript-lsp.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with_writer(non_blocking)
        .init();

    tracing::info!("pdxscript-lsp starting");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(backend::Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn dirs_or_home() -> std::path::PathBuf {
    // Prefer $XDG_CACHE_HOME or ~/.cache, fall back to /tmp
    if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        return std::path::PathBuf::from(cache);
    }
    if let Some(home) = home::home_dir() {
        let cache = home.join(".cache");
        if cache.exists() {
            return cache;
        }
    }
    std::path::PathBuf::from("/tmp")
}
