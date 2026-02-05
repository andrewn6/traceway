use api::SharedStore;
use axum::{
    body::Body,
    extract::State,
    http::Request,
    response::{IntoResponse, Response},
    Router,
};
use serde_json::Value;
use trace::Span;
use uuid::Uuid;

#[derive(Clone)]
struct ProxyState {
    store: SharedStore,
    target_url: String,
    client: reqwest::Client,
}

/// Try to extract model name from a JSON body (common field across LLM APIs)
fn extract_model(body: &Value) -> Option<String> {
    body.get("model").and_then(|v| v.as_str()).map(String::from)
}

/// Try to extract token counts from response (varies by provider)
fn extract_tokens(body: &Value) -> (Option<u64>, Option<u64>) {
    // Ollama style
    let input = body
        .get("prompt_eval_count")
        .and_then(|v| v.as_u64())
        // OpenAI style
        .or_else(|| {
            body.get("usage")
                .and_then(|u| u.get("prompt_tokens"))
                .and_then(|v| v.as_u64())
        });

    let output = body
        .get("eval_count")
        .and_then(|v| v.as_u64())
        // OpenAI style
        .or_else(|| {
            body.get("usage")
                .and_then(|u| u.get("completion_tokens"))
                .and_then(|v| v.as_u64())
        });

    (input, output)
}

async fn proxy_handler(
    State(state): State<ProxyState>,
    req: Request<Body>,
) -> Response {
    let method = req.method().clone();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.to_string())
        .unwrap_or_else(|| "/".to_string());
    let span_name = format!("{} {}", method, path);

    let trace_id = Uuid::new_v4();
    let mut span = Span::new(trace_id, None, &span_name);
    let span_id = span.id;

    // Read request body
    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("failed to read request body: {}", e);
            return (axum::http::StatusCode::BAD_REQUEST, "Failed to read body").into_response();
        }
    };

    // Try to parse as JSON to extract model
    if let Ok(json) = serde_json::from_slice::<Value>(&body_bytes) {
        if let Some(model) = extract_model(&json) {
            span.metadata.model = Some(model);
        }
    }

    // Insert span
    {
        let mut store = state.store.write().await;
        store.insert(span).await;
    }

    tracing::info!(%trace_id, %span_id, %span_name, "proxying request");

    // Build target URL
    let target_url = format!("{}{}", state.target_url, path);

    // Build request to target
    let mut target_req = state.client.request(method, &target_url);

    // Forward headers (except host)
    for (name, value) in parts.headers.iter() {
        if name != "host" {
            target_req = target_req.header(name, value);
        }
    }

    // Send request
    let result = target_req.body(body_bytes.to_vec()).send().await;

    match result {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();

            match response.bytes().await {
                Ok(resp_bytes) => {
                    // Try to extract tokens from response
                    let (input, output) =
                        if let Ok(json) = serde_json::from_slice::<Value>(&resp_bytes) {
                            extract_tokens(&json)
                        } else {
                            (None, None)
                        };

                    {
                        let mut store = state.store.write().await;
                        if let Some(s) = store.get_mut(span_id) {
                            s.metadata.input_tokens = input;
                            s.metadata.output_tokens = output;
                        }
                        if status.is_success() {
                            store.complete(span_id).await;
                        } else {
                            store.fail(span_id, format!("HTTP {}", status)).await;
                        }
                    }

                    tracing::info!(%span_id, %status, "request completed");

                    // Build response
                    let mut builder = Response::builder().status(status);
                    for (name, value) in headers.iter() {
                        builder = builder.header(name, value);
                    }
                    builder.body(Body::from(resp_bytes)).unwrap()
                }
                Err(e) => {
                    fail_span(&state.store, span_id, &format!("Failed to read response: {}", e))
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
            fail_span(
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

async fn fail_span(store: &SharedStore, span_id: trace::SpanId, error: &str) {
    let mut w = store.write().await;
    w.fail(span_id, error).await;
    tracing::warn!(%span_id, %error, "span failed");
}

pub fn router(store: SharedStore, target_url: String) -> Router {
    let state = ProxyState {
        store,
        target_url,
        client: reqwest::Client::new(),
    };

    Router::new().fallback(proxy_handler).with_state(state)
}

pub async fn serve(store: SharedStore, addr: &str, target_url: &str) -> std::io::Result<()> {
    let app = router(store, target_url.to_string());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("proxy listening on {} -> {}", addr, target_url);
    axum::serve(listener, app)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
