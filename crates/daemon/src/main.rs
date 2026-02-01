use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

use storage::SpanStore;
use trace::Span;

pub type SharedStore = Arc<RwLock<SpanStore>>;

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber");

    info!("LLM trace daemon starting");

    let store: SharedStore = Arc::new(RwLock::new(SpanStore::new()));

    {
        let trace_id = Uuid::new_v4();
        let mut w = store.write().await;

        let span = Span::new(trace_id, None, "agent-run");
        let parent_id = span.id;
        w.insert(span);
        info!(%trace_id, "created trace");

        let child = Span::new(trace_id, Some(parent_id), "llm-call");
        let child_id = child.id;
        w.insert(child);

        w.complete(child_id);
        w.complete(parent_id);
        info!("completed spans");
    }

    {
        let r = store.read().await;
        info!(traces = r.trace_count(), spans = r.span_count(), "store stats");
    }
    
    info!("daemon ready");

    tokio::signal::ctrl_c().await.ok();
    info!("shutting down")


}
