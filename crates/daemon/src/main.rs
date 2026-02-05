use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use storage::{PersistentStore, SqliteBackend};

#[derive(Parser, Debug)]
#[command(name = "llmtrace", about = "LLM trace daemon with transparent proxy")]
struct Args {
    /// API server address
    #[arg(long, default_value = "127.0.0.1:3000")]
    api_addr: String,

    /// Proxy server address
    #[arg(long, default_value = "127.0.0.1:3001")]
    proxy_addr: String,

    /// Target LLM server URL (Ollama, OpenAI-compatible, etc.)
    #[arg(long, default_value = "http://localhost:11434")]
    target_url: String,

    /// Path to SQLite database file
    #[arg(long)]
    db_path: Option<String>,
}

fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".llmtrace")
        .join("traces.db")
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber");

    info!("LLM trace daemon starting");

    let db_path = args
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(default_db_path);

    info!(path = %db_path.display(), "opening database");

    let backend = SqliteBackend::open(&db_path).expect("Failed to open database");
    let persistent = PersistentStore::open(backend)
        .await
        .expect("Failed to load data from database");
    let store = Arc::new(RwLock::new(persistent));

    // Start API server
    let api_store = store.clone();
    let api_addr = args.api_addr.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = api::serve(api_store, &api_addr).await {
            tracing::error!("api server error: {}", e);
        }
    });

    // Start Proxy server
    let proxy_store = store.clone();
    let proxy_addr = args.proxy_addr.clone();
    let target_url = args.target_url.clone();
    let proxy_handle = tokio::spawn(async move {
        if let Err(e) = proxy::serve(proxy_store, &proxy_addr, &target_url).await {
            tracing::error!("proxy server error: {}", e);
        }
    });

    info!(
        "daemon ready - api at http://{}, proxy at http://{} -> {}",
        args.api_addr, args.proxy_addr, args.target_url
    );

    tokio::signal::ctrl_c().await.ok();
    info!("shutting down");

    api_handle.abort();
    proxy_handle.abort();
}
