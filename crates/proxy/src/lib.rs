use api::SharedStore;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use trace::{Span, SpanMetadata};
use uuid::Uuid;

#[derive(Clone)]
struct ProxyState {
    store: SharedStore,
    ollama_url: String,
    client: reqwest::Client,
}

// Ollama request/response types

#[derive(Debug, Deserialize, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    #[serde(default)]
    stream: Option<bool>,
    #[serde(flatten)]
    extra: Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: Option<bool>,
    #[serde(flatten)]
    extra: Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GenerateResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(flatten)]
    extra: Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatResponse {
    message: Option<ChatMessage>,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
    #[serde(default)]
    eval_count: Option<u64>,
    #[serde(flatten)]
    extra: Value,
}

async fn proxy_generate(
    State(state): State<ProxyState>,
    Json(req): Json<GenerateRequest>,
) -> Result<Response, StatusCode> {
    let trace_id = Uuid::new_v4();
    let model = req.model.clone();

    // Create span before making request
    let mut span = Span::new(trace_id, None, "ollama-generate");
    span.metadata = SpanMetadata {
        model: Some(model.clone()),
        input_tokens: None,
        output_tokens: None,
    };
    let span_id = span.id;

    {
        let mut store = state.store.write().await;
        store.insert(span);
    }

    tracing::info!(%trace_id, %span_id, model = %model, "proxying generate request");

    // Force non-streaming for simplicity
    let mut request_body = serde_json::to_value(&req).unwrap();
    request_body["stream"] = serde_json::Value::Bool(false);

    let url = format!("{}/api/generate", state.ollama_url);
    let result = state.client.post(&url).json(&request_body).send().await;

    match result {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                match response.json::<GenerateResponse>().await {
                    Ok(gen_resp) => {
                        // Complete span with token counts
                        {
                            let mut store = state.store.write().await;
                            if let Some(s) = store.get_mut(span_id) {
                                s.metadata.input_tokens = gen_resp.prompt_eval_count;
                                s.metadata.output_tokens = gen_resp.eval_count;
                                s.complete();
                            }
                        }
                        tracing::info!(%span_id, "generate completed");

                        // Return full response
                        let resp_json = serde_json::to_value(&gen_resp).unwrap();
                        Ok(Json(resp_json).into_response())
                    }
                    Err(e) => {
                        fail_span(&state.store, span_id, &format!("Failed to parse response: {}", e)).await;
                        Err(StatusCode::BAD_GATEWAY)
                    }
                }
            } else {
                let error_text = response.text().await.unwrap_or_default();
                fail_span(&state.store, span_id, &format!("Ollama error {}: {}", status, error_text)).await;
                Err(StatusCode::BAD_GATEWAY)
            }
        }
        Err(e) => {
            fail_span(&state.store, span_id, &format!("Request failed: {}", e)).await;
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

async fn proxy_chat(
    State(state): State<ProxyState>,
    Json(req): Json<ChatRequest>,
) -> Result<Response, StatusCode> {
    let trace_id = Uuid::new_v4();
    let model = req.model.clone();

    // Create span before making request
    let mut span = Span::new(trace_id, None, "ollama-chat");
    span.metadata = SpanMetadata {
        model: Some(model.clone()),
        input_tokens: None,
        output_tokens: None,
    };
    let span_id = span.id;

    {
        let mut store = state.store.write().await;
        store.insert(span);
    }

    tracing::info!(%trace_id, %span_id, model = %model, "proxying chat request");

    // Force non-streaming for simplicity
    let mut request_body = serde_json::to_value(&req).unwrap();
    request_body["stream"] = serde_json::Value::Bool(false);

    let url = format!("{}/api/chat", state.ollama_url);
    let result = state.client.post(&url).json(&request_body).send().await;

    match result {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                match response.json::<ChatResponse>().await {
                    Ok(chat_resp) => {
                        // Complete span with token counts
                        {
                            let mut store = state.store.write().await;
                            if let Some(s) = store.get_mut(span_id) {
                                s.metadata.input_tokens = chat_resp.prompt_eval_count;
                                s.metadata.output_tokens = chat_resp.eval_count;
                                s.complete();
                            }
                        }
                        tracing::info!(%span_id, "chat completed");

                        // Return full response
                        let resp_json = serde_json::to_value(&chat_resp).unwrap();
                        Ok(Json(resp_json).into_response())
                    }
                    Err(e) => {
                        fail_span(&state.store, span_id, &format!("Failed to parse response: {}", e)).await;
                        Err(StatusCode::BAD_GATEWAY)
                    }
                }
            } else {
                let error_text = response.text().await.unwrap_or_default();
                fail_span(&state.store, span_id, &format!("Ollama error {}: {}", status, error_text)).await;
                Err(StatusCode::BAD_GATEWAY)
            }
        }
        Err(e) => {
            fail_span(&state.store, span_id, &format!("Request failed: {}", e)).await;
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

async fn fail_span(store: &SharedStore, span_id: trace::SpanId, error: &str) {
    let mut w = store.write().await;
    if let Some(s) = w.get_mut(span_id) {
        s.fail(error);
    }
    tracing::warn!(%span_id, %error, "span failed");
}

pub fn router(store: SharedStore, ollama_url: String) -> Router {
    let state = ProxyState {
        store,
        ollama_url,
        client: reqwest::Client::new(),
    };

    Router::new()
        .route("/api/generate", post(proxy_generate))
        .route("/api/chat", post(proxy_chat))
        .with_state(state)
}

pub async fn serve(store: SharedStore, addr: &str, ollama_url: &str) -> std::io::Result<()> {
    let app = router(store, ollama_url.to_string());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("proxy listening on {} -> {}", addr, ollama_url);
    axum::serve(listener, app)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
