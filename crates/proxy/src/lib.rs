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
    encore_bridge: Option<EncoreBridgeConfig>,
}

#[derive(Clone)]
struct EncoreBridgeConfig {
    base_url: String,
    control_token: String,
    org_id: String,
    project_id: String,
}

impl EncoreBridgeConfig {
    fn from_env() -> Option<Self> {
        let mode = std::env::var("TRACEWAY_BACKEND_MODE")
            .or_else(|_| std::env::var("TRACEWAY_CONTROL_PLANE_MODE"))
            .unwrap_or_else(|_| "off".to_string())
            .to_ascii_lowercase();
        if mode != "on" {
            return None;
        }

        let base_url = std::env::var("TRACEWAY_BACKEND_URL")
            .or_else(|_| std::env::var("TRACEWAY_CONTROL_PLANE_URL"))
            .ok()?
            .trim_end_matches('/')
            .to_string();
        let control_token = std::env::var("TRACEWAY_BACKEND_TOKEN")
            .or_else(|_| std::env::var("TRACEWAY_CONTROL_PLANE_TOKEN"))
            .ok()?;
        let org_id = std::env::var("TRACEWAY_DEFAULT_ORG_ID").ok()?;
        let project_id = std::env::var("TRACEWAY_DEFAULT_PROJECT_ID").ok()?;
        Some(Self {
            base_url,
            control_token,
            org_id,
            project_id,
        })
    }
}

async fn bridge_create_trace(config: &EncoreBridgeConfig, client: &reqwest::Client, trace_id: trace::TraceId, name: &str) {
    let _ = client
        .post(format!("{}/traces", config.base_url))
        .header("x-traceway-control-token", &config.control_token)
        .header("x-traceway-org-id", &config.org_id)
        .header("x-traceway-project-id", &config.project_id)
        .json(&serde_json::json!({
            "id": trace_id.to_string(),
            "name": name,
            "tags": ["proxy"],
        }))
        .send()
        .await;
}

async fn bridge_create_span(
    config: &EncoreBridgeConfig,
    client: &reqwest::Client,
    span_id: trace::SpanId,
    trace_id: trace::TraceId,
    span_name: &str,
    kind: &SpanKind,
    input_payload: Option<Value>,
) {
    let _ = client
        .post(format!("{}/spans", config.base_url))
        .header("x-traceway-control-token", &config.control_token)
        .header("x-traceway-org-id", &config.org_id)
        .header("x-traceway-project-id", &config.project_id)
        .json(&serde_json::json!({
            "id": span_id.to_string(),
            "trace_id": trace_id.to_string(),
            "name": span_name,
            "kind": serde_json::to_value(kind).unwrap_or(serde_json::json!({"type": "llm_call"})),
            "input": input_payload,
        }))
        .send()
        .await;
}

async fn bridge_complete_span(config: &EncoreBridgeConfig, client: &reqwest::Client, span_id: trace::SpanId, output_payload: Option<Value>) {
    let _ = client
        .post(format!("{}/spans/{}/complete", config.base_url, span_id))
        .header("x-traceway-control-token", &config.control_token)
        .header("x-traceway-org-id", &config.org_id)
        .header("x-traceway-project-id", &config.project_id)
        .json(&serde_json::json!({"output": output_payload}))
        .send()
        .await;
}

async fn bridge_fail_span(config: &EncoreBridgeConfig, client: &reqwest::Client, span_id: trace::SpanId, error: String) {
    let _ = client
        .post(format!("{}/spans/{}/fail", config.base_url, span_id))
        .header("x-traceway-control-token", &config.control_token)
        .header("x-traceway-org-id", &config.org_id)
        .header("x-traceway-project-id", &config.project_id)
        .json(&serde_json::json!({"error": error}))
        .send()
        .await;
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

/// Truncate a string for preview mode (character-aware, safe for multi-byte UTF-8)
fn preview_string(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_string_ascii() {
        assert_eq!(preview_string("hello world", 5), "hello...");
        assert_eq!(preview_string("hello", 5), "hello");
        assert_eq!(preview_string("hi", 10), "hi");
    }

    #[test]
    fn preview_string_emoji() {
        // "Hello 🌍 World" — 🌍 is 4 bytes but 1 char
        assert_eq!(preview_string("Hello 🌍 World", 7), "Hello 🌍...");
        assert_eq!(preview_string("Hello 🌍 World", 100), "Hello 🌍 World");
        // Truncate right at the emoji
        assert_eq!(preview_string("🌍🌍🌍", 2), "🌍🌍...");
    }

    #[test]
    fn preview_string_cjk() {
        // Each CJK char is 3 bytes
        assert_eq!(preview_string("日本語テスト", 3), "日本語...");
        assert_eq!(preview_string("日本語テスト", 6), "日本語テスト");
    }

    #[test]
    fn preview_string_empty() {
        assert_eq!(preview_string("", 10), "");
        assert_eq!(preview_string("", 0), "");
    }

    #[test]
    fn preview_string_zero_max() {
        assert_eq!(preview_string("hello", 0), "...");
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
        cost: None,
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
        if let Err(e) = store.insert(span).await {
            tracing::error!(%span_id, "failed to insert proxy span: {e}");
        }
    }

    if let Some(config) = &state.encore_bridge {
        bridge_create_trace(config, &state.client, trace_id, &span_name).await;
        bridge_create_span(
            config,
            &state.client,
            span_id,
            trace_id,
            &span_name,
            &SpanKind::LlmCall {
                model: model.clone(),
                provider: provider.clone(),
                input_tokens: None,
                output_tokens: None,
                cost: None,
                input_preview: input_preview.clone(),
                output_preview: None,
            },
            req_json.clone(),
        )
        .await;
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

                    // Build output preview for the updated kind
                    let output_preview = match &state.capture_mode {
                        CaptureMode::Off => None,
                        CaptureMode::Preview(max) => resp_json
                            .as_ref()
                            .map(|j| preview_string(&j.to_string(), *max)),
                        CaptureMode::Full => resp_json
                            .as_ref()
                            .map(|j| j.to_string()),
                    };

                    // Build updated SpanKind with actual token counts + estimated cost
                    let updated_kind = SpanKind::LlmCall {
                        model: model.clone(),
                        provider: provider.clone(),
                        input_tokens,
                        output_tokens,
                        cost: None,
                        input_preview: input_preview.clone(),
                        output_preview,
                    }.with_estimated_cost();

                    {
                        let mut store = state.store.write().await;
                        if status.is_success() {
                            if let Err(e) = store
                                .complete_span_with_kind(span_id, updated_kind, output_payload.clone())
                                .await
                            {
                                tracing::error!(%span_id, "failed to complete proxy span: {e}");
                            }
                        } else {
                            if let Err(e) = store
                                .fail_span(span_id, format!("HTTP {}", status))
                                .await
                            {
                                tracing::error!(%span_id, "failed to fail proxy span: {e}");
                            }
                        }
                    }

                    if let Some(config) = &state.encore_bridge {
                        if status.is_success() {
                            bridge_complete_span(config, &state.client, span_id, output_payload.clone()).await;
                        } else {
                            bridge_fail_span(config, &state.client, span_id, format!("HTTP {}", status)).await;
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
    if let Err(e) = w.fail_span(span_id, error).await {
        tracing::error!(%span_id, "failed to record span failure: {e}");
    }
    tracing::warn!(%span_id, %error, "span failed");
}

pub fn router(store: SharedStore, target_url: String) -> Router {
    let state = ProxyState {
        store,
        target_url,
        client: reqwest::Client::new(),
        capture_mode: CaptureMode::default(),
        encore_bridge: EncoreBridgeConfig::from_env(),
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
