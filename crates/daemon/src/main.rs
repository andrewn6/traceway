mod config;
mod ingest;
mod pid;

#[cfg(feature = "cloud")]
mod cloud;

use std::net::TcpListener as StdTcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use tokio::sync::{watch, RwLock};
use tracing::{error, info, warn};

use storage::PersistentStore;
use storage_sqlite::SqliteBackend;

use crate::config::Config;
use crate::pid::PidFile;

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_COMPONENT_RESTARTS: u32 = 3;

#[derive(Parser, Debug)]
#[command(name = "llmtrace", about = "LLM trace daemon with transparent proxy")]
struct Args {
    /// API server address
    #[arg(long)]
    api_addr: Option<String>,

    /// Proxy server address
    #[arg(long)]
    proxy_addr: Option<String>,

    /// Target LLM server URL (Ollama, OpenAI-compatible, etc.)
    #[arg(long)]
    target_url: Option<String>,

    /// Path to SQLite database file
    #[arg(long)]
    db_path: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long)]
    log_level: Option<String>,

    /// Run in foreground (don't daemonize)
    #[arg(long)]
    foreground: bool,

    /// Daemonize (fork to background)
    #[arg(long, short = 'd')]
    daemon: bool,

    /// Path to config file
    #[arg(long)]
    config: Option<String>,

    /// Enable synthetic span ingest loop for development/testing
    #[arg(long)]
    dev_ingest: bool,

    /// Interval (seconds) between synthetic span bursts [default: 5]
    #[arg(long, default_value = "5")]
    dev_ingest_interval: u64,

    /// Run in cloud mode (load config from environment)
    #[arg(long)]
    cloud: bool,
}

/// Resolved configuration merging CLI args over config file over defaults.
struct ResolvedConfig {
    api_addr: String,
    proxy_addr: String,
    target_url: String,
    db_path: PathBuf,
    log_level: String,
    foreground: bool,
    dev_ingest: bool,
    dev_ingest_interval: u64,
}

impl ResolvedConfig {
    fn from_args_and_config(args: &Args, config: &Config) -> Self {
        Self {
            api_addr: args
                .api_addr
                .clone()
                .unwrap_or_else(|| config.api.addr.clone()),
            proxy_addr: args
                .proxy_addr
                .clone()
                .unwrap_or_else(|| config.proxy.addr.clone()),
            target_url: args
                .target_url
                .clone()
                .unwrap_or_else(|| config.proxy.target.clone()),
            db_path: args
                .db_path
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| config.db_path()),
            log_level: args
                .log_level
                .clone()
                .or_else(|| std::env::var("LLMTRACE_LOG").ok())
                .unwrap_or_else(|| config.logging.level.clone()),
            foreground: !args.daemon,
            dev_ingest: args.dev_ingest,
            dev_ingest_interval: args.dev_ingest_interval,
        }
    }
}

fn setup_logging(log_level: &str, foreground: bool) {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_new(log_level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let log_dir = Config::log_dir();
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(&log_dir, "daemon.log");

    if foreground {
        // Log to both file and stdout
        let stdout_layer = fmt::layer()
            .with_target(false)
            .with_thread_ids(false);
        let file_layer = fmt::layer()
            .json()
            .with_writer(file_appender);

        tracing_subscriber::registry()
            .with(filter)
            .with(stdout_layer)
            .with(file_layer)
            .init();
    } else {
        // Log to file only (daemonized)
        let file_layer = fmt::layer()
            .json()
            .with_writer(file_appender);

        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .init();
    }
}

/// Check if a port is available by attempting to bind.
fn check_port_available(addr: &str) -> Result<(), String> {
    match StdTcpListener::bind(addr) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            Err(format!(
                "port {} is already in use. Another daemon may be running.\n\
                 Check with: lsof -i :{}\n\
                 Or update the address in ~/.llmtrace/config.toml",
                addr,
                addr.split(':').last().unwrap_or(addr)
            ))
        }
        Err(e) => Err(format!("cannot bind to {}: {}", addr, e)),
    }
}

/// Create shutdown signal listener (SIGINT + SIGTERM).
async fn shutdown_signal(mut shutdown_rx: watch::Receiver<bool>) {
    shutdown_rx.changed().await.ok();
}

/// Run the API server with supervision (restart on crash).
async fn run_api_supervised(
    store: Arc<RwLock<PersistentStore<SqliteBackend>>>,
    addr: String,
    start_time: Instant,
    config_json: serde_json::Value,
    config_path: String,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
) {
    let mut restarts = 0u32;
    let mut backoff = Duration::from_secs(1);

    loop {
        let api_store = store.clone();
        let api_addr = addr.clone();
        let api_start_time = start_time;
        let api_config = config_json.clone();
        let api_config_path = config_path.clone();
        let api_shutdown_tx = shutdown_tx.clone();
        let rx = shutdown_rx.clone();

        info!("starting api server on {}", api_addr);

        let result = tokio::spawn(async move {
            api::serve_with_shutdown(api_store, &api_addr, api_start_time, api_config, api_config_path, Some(api_shutdown_tx), shutdown_signal(rx)).await
        })
        .await;

        // Check if we've been asked to shut down
        if *shutdown_rx.borrow() {
            info!("api server stopped (shutdown requested)");
            return;
        }

        match result {
            Ok(Ok(())) => {
                info!("api server exited cleanly");
                return;
            }
            Ok(Err(e)) => {
                error!("api server error: {}", e);
            }
            Err(e) => {
                error!("api server panicked: {}", e);
            }
        }

        restarts += 1;
        if restarts > MAX_COMPONENT_RESTARTS {
            error!("api server exceeded max restarts ({}), giving up", MAX_COMPONENT_RESTARTS);
            return;
        }

        warn!(
            restarts,
            backoff_secs = backoff.as_secs(),
            "restarting api server after failure"
        );
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

/// Run the proxy server with supervision (restart on crash).
async fn run_proxy_supervised(
    store: Arc<RwLock<PersistentStore<SqliteBackend>>>,
    addr: String,
    target_url: String,
    shutdown_rx: watch::Receiver<bool>,
) {
    let mut restarts = 0u32;
    let mut backoff = Duration::from_secs(1);

    loop {
        let proxy_store = store.clone();
        let proxy_addr = addr.clone();
        let proxy_target = target_url.clone();
        let rx = shutdown_rx.clone();

        info!("starting proxy server on {} -> {}", proxy_addr, proxy_target);

        let result = tokio::spawn(async move {
            proxy::serve_with_shutdown(proxy_store, &proxy_addr, &proxy_target, shutdown_signal(rx))
                .await
        })
        .await;

        // Check if we've been asked to shut down
        if *shutdown_rx.borrow() {
            info!("proxy server stopped (shutdown requested)");
            return;
        }

        match result {
            Ok(Ok(())) => {
                info!("proxy server exited cleanly");
                return;
            }
            Ok(Err(e)) => {
                error!("proxy server error: {}", e);
            }
            Err(e) => {
                error!("proxy server panicked: {}", e);
            }
        }

        restarts += 1;
        if restarts > MAX_COMPONENT_RESTARTS {
            error!(
                "proxy server exceeded max restarts ({}), giving up",
                MAX_COMPONENT_RESTARTS
            );
            return;
        }

        warn!(
            restarts,
            backoff_secs = backoff.as_secs(),
            "restarting proxy server after failure"
        );
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(30));
    }
}

/// Daemonize by re-executing the current binary with --foreground in the background.
/// This avoids the complexity of fork() after tokio runtime starts.
fn daemonize(args: &Args) -> ! {
    use std::process::Command;

    let exe = std::env::current_exe().expect("failed to get current executable path");

    let mut cmd = Command::new(exe);
    cmd.arg("--foreground");

    if let Some(ref addr) = args.api_addr {
        cmd.arg("--api-addr").arg(addr);
    }
    if let Some(ref addr) = args.proxy_addr {
        cmd.arg("--proxy-addr").arg(addr);
    }
    if let Some(ref url) = args.target_url {
        cmd.arg("--target-url").arg(url);
    }
    if let Some(ref path) = args.db_path {
        cmd.arg("--db-path").arg(path);
    }
    if let Some(ref level) = args.log_level {
        cmd.arg("--log-level").arg(level);
    }
    if let Some(ref config) = args.config {
        cmd.arg("--config").arg(config);
    }
    if args.dev_ingest {
        cmd.arg("--dev-ingest");
        cmd.arg("--dev-ingest-interval")
            .arg(args.dev_ingest_interval.to_string());
    }

    // Redirect stdio to /dev/null for the background process
    use std::process::Stdio;
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    match cmd.spawn() {
        Ok(child) => {
            eprintln!("daemon started (pid {})", child.id());
            eprintln!("logs: {}", Config::log_dir().display());
            eprintln!("pid file: {}", Config::pid_path().display());
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("failed to daemonize: {}", e);
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Cloud mode: load all config from environment
    #[cfg(feature = "cloud")]
    if args.cloud {
        run_cloud_mode().await;
        return;
    }

    // Load config file
    let config = match &args.config {
        Some(path) => Config::load_from(std::path::Path::new(path)),
        None => Config::load(),
    };

    let resolved = ResolvedConfig::from_args_and_config(&args, &config);

    // --- Daemonize (re-exec with --foreground in background) ---
    if !resolved.foreground {
        daemonize(&args);
    }

    // Setup logging (needs to happen before any tracing calls)
    setup_logging(&resolved.log_level, resolved.foreground);

    info!("llmtrace daemon starting");

    // --- PID file ---
    let pid_file = PidFile::new(Config::pid_path());
    if let Err(e) = pid_file.acquire() {
        error!("{}", e);
        std::process::exit(1);
    }
    // Keep pid_file alive — it removes itself on Drop

    // --- Port conflict detection ---
    if let Err(e) = check_port_available(&resolved.api_addr) {
        error!("api: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = check_port_available(&resolved.proxy_addr) {
        error!("proxy: {}", e);
        std::process::exit(1);
    }

    // --- Ordered startup ---
    let start_time = Instant::now();

    // 1. Storage
    info!(path = %resolved.db_path.display(), "opening database");
    if let Some(parent) = resolved.db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let backend = match SqliteBackend::open(&resolved.db_path) {
        Ok(b) => b,
        Err(e) => {
            error!("failed to open database: {}", e);
            std::process::exit(1);
        }
    };
    let persistent = match PersistentStore::open(backend).await {
        Ok(p) => p,
        Err(e) => {
            error!("failed to load data: {}", e);
            std::process::exit(1);
        }
    };
    let store = Arc::new(RwLock::new(persistent));
    info!("storage ready");

    // 2. Shutdown signal channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Serialize config to JSON for the API layer
    let config_json = serde_json::to_value(&config).unwrap_or_default();
    let config_path_str = args.config
        .as_ref()
        .map(|p| p.to_string())
        .unwrap_or_else(|| Config::default_path().to_string_lossy().to_string());

    // 3. API server (supervised)
    let api_handle = tokio::spawn(run_api_supervised(
        store.clone(),
        resolved.api_addr.clone(),
        start_time,
        config_json,
        config_path_str,
        shutdown_tx.clone(),
        shutdown_rx.clone(),
    ));

    // Small delay to let API bind before proxy
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 4. Proxy server (supervised)
    let proxy_handle = tokio::spawn(run_proxy_supervised(
        store.clone(),
        resolved.proxy_addr.clone(),
        resolved.target_url.clone(),
        shutdown_rx.clone(),
    ));

    // 5. Dev ingest loop (optional synthetic span generation for testing)
    let ingest_handle = if resolved.dev_ingest {
        let interval = Duration::from_secs(resolved.dev_ingest_interval);
        info!(
            interval_secs = resolved.dev_ingest_interval,
            "starting synthetic ingest loop"
        );
        Some(tokio::spawn(ingest::run_synthetic_ingest(
            store.clone(),
            interval,
            shutdown_rx.clone(),
        )))
    } else {
        None
    };

    info!(
        "daemon ready — api http://{} | proxy http://{} -> {}",
        resolved.api_addr, resolved.proxy_addr, resolved.target_url
    );

    // --- Wait for shutdown signal ---
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("received SIGINT"),
        _ = terminate => info!("received SIGTERM"),
    }

    // --- Graceful shutdown ---
    info!("initiating graceful shutdown");

    // Signal all components to stop
    let _ = shutdown_tx.send(true);

    // Wait for components with timeout
    let shutdown_result = tokio::time::timeout(
        SHUTDOWN_TIMEOUT,
        async {
            let _ = tokio::join!(api_handle, proxy_handle);
            if let Some(h) = ingest_handle {
                let _ = h.await;
            }
        },
    )
    .await;

    match shutdown_result {
        Ok(()) => info!("all components stopped gracefully"),
        Err(_) => warn!("shutdown timed out after {} seconds, forcing exit", SHUTDOWN_TIMEOUT.as_secs()),
    }

    // PID file is removed by Drop on pid_file
    drop(pid_file);

    info!("daemon stopped");
}

/// Run in cloud mode - configuration loaded from environment variables
#[cfg(feature = "cloud")]
async fn run_cloud_mode() {
    use crate::cloud::{setup_cloud_logging, CloudConfig};

    let cloud_config = CloudConfig::from_env();
    setup_cloud_logging(&cloud_config);
    cloud_config.log_config();

    info!("llmfs cloud daemon starting");

    let start_time = Instant::now();

    // Initialize storage based on configuration
    let store = match cloud_config.storage_backend {
        cloud::StorageBackendType::Sqlite => {
            // Use in-memory or ephemeral SQLite for cloud
            let db_path = std::env::var("DB_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/tmp/llmfs.db"));

            info!(path = %db_path.display(), "Using SQLite storage");

            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            let backend = match SqliteBackend::open(&db_path) {
                Ok(b) => b,
                Err(e) => {
                    error!("Failed to open database: {}", e);
                    std::process::exit(1);
                }
            };

            match PersistentStore::open(backend).await {
                Ok(p) => Arc::new(RwLock::new(p)),
                Err(e) => {
                    error!("Failed to load data: {}", e);
                    std::process::exit(1);
                }
            }
        }
        cloud::StorageBackendType::Turbopuffer => {
            // Turbopuffer backend would be initialized here
            // For now, fall back to SQLite with a warning
            warn!("Turbopuffer backend selected but not yet integrated - using SQLite");

            let db_path = PathBuf::from("/tmp/llmfs.db");
            let backend = SqliteBackend::open(&db_path).expect("Failed to open SQLite");
            let persistent = PersistentStore::open(backend).await.expect("Failed to load data");
            Arc::new(RwLock::new(persistent))
        }
    };

    info!("Storage ready");

    // Shutdown signal handling
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Empty config for cloud mode (config comes from env)
    let config_json = serde_json::json!({
        "mode": "cloud",
        "storage": format!("{:?}", cloud_config.storage_backend),
        "redis": cloud_config.has_redis(),
        "region": cloud_config.region,
    });

    let addr = cloud_config.bind_addr();
    info!(addr = %addr, "Starting API server");

    // Start API server
    let api_handle = tokio::spawn({
        let store = store.clone();
        let shutdown_rx = shutdown_rx.clone();
        let shutdown_tx = shutdown_tx.clone();
        let addr = addr.clone();
        async move {
            api::serve_with_shutdown(
                store,
                &addr,
                start_time,
                config_json,
                String::new(),
                Some(shutdown_tx),
                shutdown_signal(shutdown_rx),
            )
            .await
        }
    });

    info!("Cloud daemon ready on http://{}", addr);

    // Wait for shutdown signal
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received SIGINT"),
        _ = terminate => info!("Received SIGTERM"),
    }

    info!("Initiating graceful shutdown");
    let _ = shutdown_tx.send(true);

    // Wait for API server with timeout
    let shutdown_result = tokio::time::timeout(SHUTDOWN_TIMEOUT, api_handle).await;

    match shutdown_result {
        Ok(Ok(Ok(()))) => info!("API server stopped gracefully"),
        Ok(Ok(Err(e))) => error!("API server error: {}", e),
        Ok(Err(e)) => error!("API server panicked: {}", e),
        Err(_) => warn!("Shutdown timed out"),
    }

    info!("Cloud daemon stopped");
}
