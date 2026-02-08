use api::SharedStore;
use axum::{
    body::Body,
    extract::State,
    http::Request,
    response::{IntoResponse, Response},
    Router,
};
use serde_json::Value;
use trace::{SpanBuilder, SpanKind};

/// Payload capture mode
#[derive(Debug, Clone)]
pub enum CaptureMode {
    Off,
    Preview(usize), // max chars
    Full,
}

impl Default for CaptureMode {
    fn default() -> Self {
        CaptureMode::Full
    }
}

#[derive(Clone)]
struct ProxyState {
    store: SharedStore,
    target_url: String,
    client: reqwest::Client,
    capture_mode: CaptureMode,
}

/// Detect provider from target URL
fn detect_provider(url: &str) -> Option<String> {
    if url.contains("localhost:11434") || url.contains("ollama") {
        Some("ollama".to_string())
    } else if url.contains("api.openai.com") {
        Some("openai".to_string())
    } else if url.contains("api.anthropic.com") {
        Some("anthropic".to_string())
    } else {
        None
    }
}

/// Extract model name from a JSON body
fn extract_model(body: &Value) -> Option<String> {
    body.get("model").and_then(|v| v.as_str()).map(String::from)
}

/// Extract token counts from response (provider-aware)
fn extract_tokens(body: &Value, provider: Option<&str>) -> (Option<u64>, Option<u64>) {
    match provider {
        Some("anthropic") => {
            let input = body
                .get("usage")
                .and_then(|u| u.get("input_tokens"))
                .and_then(|v| v.as_u64());
            let output = body
                .get("usage")
                .and_then(|u| u.get("output_tokens"))
                .and_then(|v| v.as_u64());
            (input, output)
        }
        Some("ollama") => {
            let input = body.get("prompt_eval_count").and_then(|v| v.as_u64());
            let output = body.get("eval_count").and_then(|v| v.as_u64());
            (input, output)
        }
        _ => {
            // OpenAI / generic
            let input = body
                .get("usage")
                .and_then(|u| u.get("prompt_tokens"))
                .and_then(|v| v.as_u64())
                .or_else(|| body.get("prompt_eval_count").and_then(|v| v.as_u64()));
            let output = body
                .get("usage")
                .and_then(|u| u.get("completion_tokens"))
                .and_then(|v| v.as_u64())
                .or_else(|| body.get("eval_count").and_then(|v| v.as_u64()));
            (input, output)
        }
    }
}

/// Truncate a string for preview mode
fn preview_string(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else {
        format!("{}...", &s[..max_chars])
    }
}

async fn proxy_handler(State(state): State<ProxyState>, req: Request<Body>) -> Response {
    let method = req.method().clone();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.to_string())
        .unwrap_or_else(|| "/".to_string());
    let span_name = format!("{} {}", method, path);

    let provider = detect_provider(&state.target_url);

    // Read request body
    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("failed to read request body: {}", e);
            return (axum::http::StatusCode::BAD_REQUEST, "Failed to read body").into_response();
        }
    };

    // Parse request JSON for model extraction
    let req_json = serde_json::from_slice::<Value>(&body_bytes).ok();
    let model = req_json
        .as_ref()
        .and_then(extract_model)
        .unwrap_or_else(|| "unknown".to_string());

    // Build input preview
    let input_preview = match &state.capture_mode {
        CaptureMode::Off => None,
        CaptureMode::Preview(max) => {
            let raw = String::from_utf8_lossy(&body_bytes);
            Some(preview_string(&raw, *max))
        }
        CaptureMode::Full => Some(String::from_utf8_lossy(&body_bytes).to_string()),
    };

    // Build span kind
    let kind = SpanKind::LlmCall {
        model: model.clone(),
        provider: provider.clone(),
        input_tokens: None,
        output_tokens: None,
        input_preview: input_preview.clone(),
        output_preview: None,
    };

    // Build input payload
    let input_payload = match &state.capture_mode {
        CaptureMode::Off => None,
        _ => req_json.clone(),
    };

    // Create and insert span
    let mut builder = SpanBuilder::new(
        trace::Trace::new(Some(span_name.clone())).id,
        &span_name,
        kind,
    );
    if let Some(input) = input_payload {
        builder = builder.input(input);
    }
    let span = builder.build();
    let span_id = span.id();
    let trace_id = span.trace_id();

    {
        let mut store = state.store.write().await;
        store.insert(span).await;
    }

    tracing::info!(%trace_id, %span_id, %span_name, %model, "proxying request");

    // Build target URL and request
    let target_url = format!("{}{}", state.target_url, path);
    let mut target_req = state.client.request(method, &target_url);
    for (name, value) in parts.headers.iter() {
        if name != "host" {
            target_req = target_req.header(name, value);
        }
    }

    let result = target_req.body(body_bytes.to_vec()).send().await;

    match result {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();

            match response.bytes().await {
                Ok(resp_bytes) => {
                    let resp_json = serde_json::from_slice::<Value>(&resp_bytes).ok();

                    // Extract tokens
                    let (input_tokens, output_tokens) = resp_json
                        .as_ref()
                        .map(|j| extract_tokens(j, provider.as_deref()))
                        .unwrap_or((None, None));

                    // Build output payload
                    let output_payload = match &state.capture_mode {
                        CaptureMode::Off => None,
                        CaptureMode::Preview(_) => resp_json.as_ref().map(|j| {
                            serde_json::json!({
                                "preview": preview_string(&j.to_string(), 500)
                            })
                        }),
                        CaptureMode::Full => resp_json.clone(),
                    };

                    // Build updated kind with token counts
                    // We need to complete the span with the output payload
                    // The span kind was set at creation; token counts go into kind
                    // but since spans are immutable, we capture tokens via the output field
                    let output_with_tokens = output_payload.map(|mut v| {
                        if let Some(obj) = v.as_object_mut() {
                            if let Some(it) = input_tokens {
                                obj.insert(
                                    "_input_tokens".to_string(),
                                    serde_json::Value::from(it),
                                );
                            }
                            if let Some(ot) = output_tokens {
                                obj.insert(
                                    "_output_tokens".to_string(),
                                    serde_json::Value::from(ot),
                                );
                            }
                        }
                        v
                    });

                    {
                        let mut store = state.store.write().await;
                        if status.is_success() {
                            store.complete_span(span_id, output_with_tokens).await;
                        } else {
                            store
                                .fail_span(span_id, format!("HTTP {}", status))
                                .await;
                        }
                    }

                    tracing::info!(%span_id, %status, ?input_tokens, ?output_tokens, "request completed");

                    let mut builder = Response::builder().status(status);
                    for (name, value) in headers.iter() {
                        builder = builder.header(name, value);
                    }
                    builder.body(Body::from(resp_bytes)).unwrap()
                }
                Err(e) => {
                    fail_span_helper(
                        &state.store,
                        span_id,
                        &format!("Failed to read response: {}", e),
                    )
                    .await;
                    (
                        axum::http::StatusCode::BAD_GATEWAY,
                        "Failed to read response",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            fail_span_helper(
                &state.store,
                span_id,
                &format!("Request failed: {}", e),
            )
            .await;
            (
                axum::http::StatusCode::BAD_GATEWAY,
                format!("Proxy error: {}", e),
            )
                .into_response()
        }
    }
}

async fn fail_span_helper(store: &SharedStore, span_id: trace::SpanId, error: &str) {
    let mut w = store.write().await;
    w.fail_span(span_id, error).await;
    tracing::warn!(%span_id, %error, "span failed");
}

pub fn router(store: SharedStore, target_url: String) -> Router {
    let state = ProxyState {
        store,
        target_url,
        client: reqwest::Client::new(),
        capture_mode: CaptureMode::default(),
    };

    Router::new().fallback(proxy_handler).with_state(state)
}

pub async fn serve(store: SharedStore, addr: &str, target_url: &str) -> std::io::Result<()> {
    serve_with_shutdown(store, addr, target_url, std::future::pending()).await
}

pub async fn serve_with_shutdown(
    store: SharedStore,
    addr: &str,
    target_url: &str,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let app = router(store, target_url.to_string());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("proxy listening on {} -> {}", addr, target_url);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
