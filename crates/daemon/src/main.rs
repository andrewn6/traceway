use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use storage::SpanStore;

pub type SharedStore = Arc<RwLock<SpanStore>>;

#[derive(Parser, Debug)]
#[command(name = "llmtrace", about = "LLM trace daemon with Ollama proxy")]
struct Args {
    /// API server address
    #[arg(long, default_value = "127.0.0.1:3000")]
    api_addr: String,

    /// Proxy server address
    #[arg(long, default_value = "127.0.0.1:3001")]
    proxy_addr: String,

    /// Ollama server URL
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber");

    info!("LLM trace daemon starting");

    let store: SharedStore = Arc::new(RwLock::new(SpanStore::new()));

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
    let ollama_url = args.ollama_url.clone();
    let proxy_handle = tokio::spawn(async move {
        if let Err(e) = proxy::serve(proxy_store, &proxy_addr, &ollama_url).await {
            tracing::error!("proxy server error: {}", e);
        }
    });

    info!(
        "daemon ready - api at http://{}, proxy at http://{} -> {}",
        args.api_addr, args.proxy_addr, args.ollama_url
    );

    tokio::signal::ctrl_c().await.ok();
    info!("shutting down");

    api_handle.abort();
    proxy_handle.abort();
}
