//! OpenTelemetry OTLP/HTTP JSON trace ingest.
//!
//! Accepts `POST /v1/traces` with `ExportTraceServiceRequest` JSON body,
//! converts OTel spans to Traceway spans, auto-creates traces, and emits
//! system events so SSE subscribers and capture rules work seamlessly.

use std::collections::HashMap;

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::Json;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use trace::{OrgId, Span, SpanId, SpanKind, SpanStatus, Trace, TraceId};

use crate::{capture, AppState, SystemEvent};

#[derive(Clone)]
struct EncoreTraceBridge {
    base_url: String,
    control_token: String,
}

impl EncoreTraceBridge {
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

        Some(Self {
            base_url,
            control_token,
        })
    }

    async fn post_json(
        &self,
        client: &reqwest::Client,
        path: &str,
        org_id: &str,
        project_id: &str,
        body: serde_json::Value,
    ) {
        let _ = client
            .post(format!("{}{}", self.base_url, path))
            .header("x-traceway-control-token", &self.control_token)
            .header("x-traceway-org-id", org_id)
            .header("x-traceway-project-id", project_id)
            .json(&body)
            .send()
            .await;
    }
}

// ---------------------------------------------------------------------------
// OTLP namespace UUID for deterministic ID derivation (UUID v5).
// Generated once, never changes. This is just a random UUID used as the
// namespace for hashing OTel hex IDs into Traceway UUIDs.
// ---------------------------------------------------------------------------

const OTLP_NAMESPACE: Uuid = Uuid::from_bytes([
    0x9a, 0x3f, 0x1c, 0x7b, 0x4e, 0x2d, 0x48, 0xa1, 0xb5, 0x6e, 0x3c, 0x8d, 0x9f, 0x0a, 0x7e,
    0x5b,
]);

// ---------------------------------------------------------------------------
// OTLP request/response types (JSON encoding)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTraceServiceRequest {
    #[serde(default)]
    pub resource_spans: Vec<ResourceSpans>,
}

#[derive(Debug, Serialize)]
pub struct ExportTraceServiceResponse {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSpans {
    #[serde(default)]
    pub resource: Option<OtlpResource>,
    #[serde(default)]
    pub scope_spans: Vec<ScopeSpans>,
}

#[derive(Debug, Deserialize)]
pub struct OtlpResource {
    #[serde(default)]
    pub attributes: Vec<OtlpKeyValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeSpans {
    #[serde(default)]
    pub scope: Option<OtlpScope>,
    #[serde(default)]
    pub spans: Vec<OtlpSpan>,
}

#[derive(Debug, Deserialize)]
pub struct OtlpScope {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtlpSpan {
    pub trace_id: String,
    pub span_id: String,
    #[serde(default)]
    pub parent_span_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    /// OTel span kind: 0=UNSPECIFIED, 1=INTERNAL, 2=SERVER, 3=CLIENT, 4=PRODUCER, 5=CONSUMER
    #[serde(default)]
    pub kind: u32,
    #[serde(default)]
    pub start_time_unix_nano: StringOrU64,
    #[serde(default)]
    pub end_time_unix_nano: StringOrU64,
    #[serde(default)]
    pub attributes: Vec<OtlpKeyValue>,
    #[serde(default)]
    pub status: Option<OtlpStatus>,
    #[serde(default)]
    pub events: Vec<OtlpEvent>,
}

/// OTel JSON encodes nanosecond timestamps as either strings or numbers.
#[derive(Debug, Clone, Default)]
pub struct StringOrU64(pub u64);

impl<'de> Deserialize<'de> for StringOrU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StringOrU64;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or integer representing nanoseconds")
            }

            fn visit_u64<E>(self, v: u64) -> Result<StringOrU64, E> {
                Ok(StringOrU64(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<StringOrU64, E> {
                Ok(StringOrU64(v as u64))
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<StringOrU64, E> {
                if v.is_empty() {
                    return Ok(StringOrU64(0));
                }
                v.parse::<u64>()
                    .map(StringOrU64)
                    .map_err(serde::de::Error::custom)
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct OtlpStatus {
    /// 0 = UNSET, 1 = OK, 2 = ERROR
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtlpEvent {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub time_unix_nano: StringOrU64,
    #[serde(default)]
    pub attributes: Vec<OtlpKeyValue>,
}

#[derive(Debug, Deserialize)]
pub struct OtlpKeyValue {
    pub key: String,
    #[serde(default)]
    pub value: Option<OtlpAnyValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtlpAnyValue {
    #[serde(default)]
    pub string_value: Option<String>,
    #[serde(default)]
    pub int_value: Option<StringOrU64>,
    #[serde(default)]
    pub double_value: Option<f64>,
    #[serde(default)]
    pub bool_value: Option<bool>,
    #[serde(default)]
    pub array_value: Option<OtlpArrayValue>,
    #[serde(default)]
    pub kvlist_value: Option<OtlpKvList>,
    #[serde(default)]
    pub bytes_value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OtlpArrayValue {
    #[serde(default)]
    pub values: Vec<OtlpAnyValue>,
}

#[derive(Debug, Deserialize)]
pub struct OtlpKvList {
    #[serde(default)]
    pub values: Vec<OtlpKeyValue>,
}

// ---------------------------------------------------------------------------
// ID conversion: OTel hex → Traceway UUID v5
// ---------------------------------------------------------------------------

/// Convert an OTel trace_id (32 hex chars) to a deterministic Traceway UUID.
fn otel_trace_id_to_uuid(hex: &str) -> Result<TraceId, String> {
    if hex.len() != 32 {
        return Err(format!("invalid trace_id length: {} (expected 32)", hex.len()));
    }
    // Use the raw hex string as the name within our OTLP namespace
    Ok(Uuid::new_v5(&OTLP_NAMESPACE, hex.as_bytes()))
}

/// Convert an OTel span_id (16 hex chars) to a deterministic Traceway UUID.
/// Scoped to the trace by including trace_id in the hash input.
fn otel_span_id_to_uuid(trace_hex: &str, span_hex: &str) -> Result<SpanId, String> {
    if span_hex.len() != 16 {
        return Err(format!(
            "invalid span_id length: {} (expected 16)",
            span_hex.len()
        ));
    }
    // Combine trace + span hex to ensure uniqueness across traces
    let combined = format!("{}{}", trace_hex, span_hex);
    Ok(Uuid::new_v5(&OTLP_NAMESPACE, combined.as_bytes()))
}

// ---------------------------------------------------------------------------
// Attribute helpers
// ---------------------------------------------------------------------------

/// Convert an OtlpAnyValue to a serde_json::Value.
fn otel_value_to_json(v: &OtlpAnyValue) -> serde_json::Value {
    if let Some(ref s) = v.string_value {
        return serde_json::Value::String(s.clone());
    }
    if let Some(ref i) = v.int_value {
        return serde_json::Value::Number(serde_json::Number::from(i.0 as i64));
    }
    if let Some(d) = v.double_value {
        return serde_json::Number::from_f64(d)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Some(b) = v.bool_value {
        return serde_json::Value::Bool(b);
    }
    if let Some(ref arr) = v.array_value {
        let values: Vec<serde_json::Value> = arr.values.iter().map(otel_value_to_json).collect();
        return serde_json::Value::Array(values);
    }
    if let Some(ref kv) = v.kvlist_value {
        let mut map = serde_json::Map::new();
        for item in &kv.values {
            let val = item
                .value
                .as_ref()
                .map(otel_value_to_json)
                .unwrap_or(serde_json::Value::Null);
            map.insert(item.key.clone(), val);
        }
        return serde_json::Value::Object(map);
    }
    if let Some(ref b) = v.bytes_value {
        return serde_json::Value::String(b.clone());
    }
    serde_json::Value::Null
}

/// Extract a string attribute by key.
fn extract_string_attr(attrs: &[OtlpKeyValue], key: &str) -> Option<String> {
    attrs.iter().find(|kv| kv.key == key).and_then(|kv| {
        kv.value
            .as_ref()
            .and_then(|v| v.string_value.clone())
    })
}

/// Extract an integer attribute by key.
fn extract_int_attr(attrs: &[OtlpKeyValue], key: &str) -> Option<u64> {
    attrs.iter().find(|kv| kv.key == key).and_then(|kv| {
        kv.value.as_ref().and_then(|v| v.int_value.as_ref().map(|i| i.0))
    })
}

/// Extract a double attribute by key.
fn extract_double_attr(attrs: &[OtlpKeyValue], key: &str) -> Option<f64> {
    attrs.iter().find(|kv| kv.key == key).and_then(|kv| {
        kv.value.as_ref().and_then(|v| v.double_value)
    })
}

/// Convert OTel span kind integer to a human-readable name.
fn otel_span_kind_name(kind: u32) -> &'static str {
    match kind {
        1 => "internal",
        2 => "server",
        3 => "client",
        4 => "producer",
        5 => "consumer",
        _ => "unspecified",
    }
}

/// Convert nanosecond timestamp to DateTime<Utc>. Returns None for 0.
fn nanos_to_datetime(nanos: u64) -> Option<DateTime<Utc>> {
    if nanos == 0 {
        return None;
    }
    let secs = (nanos / 1_000_000_000) as i64;
    let subsec_nanos = (nanos % 1_000_000_000) as u32;
    Utc.timestamp_opt(secs, subsec_nanos).single()
}

// ---------------------------------------------------------------------------
// Span conversion: OtlpSpan → Traceway Span
// ---------------------------------------------------------------------------

fn convert_otlp_span(
    otel_span: &OtlpSpan,
    resource_attrs: &[OtlpKeyValue],
    org_id: OrgId,
) -> Result<Span, String> {
    let trace_id = otel_trace_id_to_uuid(&otel_span.trace_id)?;
    let span_id = otel_span_id_to_uuid(&otel_span.trace_id, &otel_span.span_id)?;

    let parent_id = match otel_span.parent_span_id.as_deref() {
        Some(p) if !p.is_empty() => Some(otel_span_id_to_uuid(&otel_span.trace_id, p)?),
        _ => None,
    };

    let name = otel_span
        .name
        .clone()
        .unwrap_or_else(|| "unnamed".to_string());

    // Detect gen_ai semantic conventions
    let model = extract_string_attr(&otel_span.attributes, "gen_ai.request.model")
        .or_else(|| extract_string_attr(&otel_span.attributes, "gen_ai.response.model"));
    let system = extract_string_attr(&otel_span.attributes, "gen_ai.system");

    let kind = if model.is_some() || system.is_some() {
        // This is an LLM call span
        let model_str = model.unwrap_or_else(|| "unknown".to_string());
        let provider = system.or_else(|| {
            // Try to infer provider from resource attributes
            extract_string_attr(resource_attrs, "gen_ai.system")
        });
        let input_tokens = extract_int_attr(&otel_span.attributes, "gen_ai.usage.input_tokens");
        let output_tokens = extract_int_attr(&otel_span.attributes, "gen_ai.usage.output_tokens");
        let cost = extract_double_attr(&otel_span.attributes, "gen_ai.usage.cost");

        SpanKind::LlmCall {
            model: model_str,
            provider,
            input_tokens,
            output_tokens,
            cost,
            input_preview: None,
            output_preview: None,
        }
        .with_estimated_cost()
    } else {
        // Generic span → Custom kind with all attributes preserved
        let kind_name = otel_span_kind_name(otel_span.kind).to_string();
        let mut attributes = HashMap::new();
        for kv in &otel_span.attributes {
            if let Some(ref v) = kv.value {
                attributes.insert(kv.key.clone(), otel_value_to_json(v));
            }
        }
        // Also store resource attributes (service.name, etc.) under a prefix
        for kv in resource_attrs {
            if let Some(ref v) = kv.value {
                attributes.insert(format!("resource.{}", kv.key), otel_value_to_json(v));
            }
        }
        SpanKind::Custom {
            kind: kind_name,
            attributes,
        }
    };

    // Status mapping
    let status = match otel_span.status.as_ref().map(|s| s.code).unwrap_or(0) {
        2 => {
            let error_msg = otel_span
                .status
                .as_ref()
                .and_then(|s| s.message.clone())
                .unwrap_or_else(|| "error".to_string());
            SpanStatus::Failed { error: error_msg }
        }
        _ => SpanStatus::Completed, // UNSET (0) and OK (1) both mean success
    };

    // Timestamps
    let started_at = nanos_to_datetime(otel_span.start_time_unix_nano.0)
        .unwrap_or_else(Utc::now);
    let ended_at = nanos_to_datetime(otel_span.end_time_unix_nano.0);

    Ok(Span::from_parts(
        span_id,
        trace_id,
        Some(org_id),
        parent_id,
        name,
        kind,
        status,
        started_at,
        ended_at,
        None, // input — OTel doesn't have a structured input concept
        None, // output — same
    ))
}

// ---------------------------------------------------------------------------
// Handler: POST /v1/traces
// ---------------------------------------------------------------------------

pub async fn ingest_traces(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ExportTraceServiceRequest>,
) -> Result<Json<ExportTraceServiceResponse>, (StatusCode, Json<serde_json::Value>)> {
    // ---- Auth: extract API key from Authorization header ----
    let ctx = extract_otlp_auth(&state, &headers).await?;
    let org_id = ctx.org_id;
    let project_id = ctx.project_id;
    let org_id_str = org_id.to_string();
    let project_id_str = project_id.to_string();

    let store = state
        .store_for_project(org_id, project_id)
        .await
        .map_err(|e| {
            (
                e.0,
                Json(serde_json::json!({ "error": e.1 })),
            )
        })?;

    // ---- Convert all spans, grouped by trace ----
    // Map: traceway_trace_id → (earliest_started_at, root_span_name, Vec<Span>)
    let mut traces_map: HashMap<TraceId, (DateTime<Utc>, Option<String>, Vec<Span>)> =
        HashMap::new();
    let mut conversion_errors: Vec<String> = Vec::new();

    for resource_spans in &req.resource_spans {
        let resource_attrs = resource_spans
            .resource
            .as_ref()
            .map(|r| r.attributes.as_slice())
            .unwrap_or(&[]);

        for scope_spans in &resource_spans.scope_spans {
            for otel_span in &scope_spans.spans {
                match convert_otlp_span(otel_span, resource_attrs, org_id) {
                    Ok(span) => {
                        let entry = traces_map
                            .entry(span.trace_id())
                            .or_insert_with(|| (span.started_at(), None, Vec::new()));

                        // Track earliest start time
                        if span.started_at() < entry.0 {
                            entry.0 = span.started_at();
                        }
                        // Track root span name (no parent)
                        if span.parent_id().is_none() && entry.1.is_none() {
                            entry.1 = Some(span.name().to_string());
                        }
                        entry.2.push(span);
                    }
                    Err(e) => {
                        conversion_errors.push(e);
                    }
                }
            }
        }
    }

    if !conversion_errors.is_empty() {
        tracing::warn!(
            errors = conversion_errors.len(),
            first = %conversion_errors[0],
            "OTLP ingest: some spans failed conversion"
        );
    }

    // ---- Create traces + insert spans ----
    let mut w = store.write().await;

    // Derive service.name from the first resource (used for trace naming)
    let service_name = req
        .resource_spans
        .first()
        .and_then(|rs| rs.resource.as_ref())
        .and_then(|r| extract_string_attr(&r.attributes, "service.name"));

    for (trace_id, (earliest_start, root_name, spans)) in &traces_map {
        // Always save the trace (INSERT OR REPLACE is idempotent).
        // If the trace already exists in the backend, this is a no-op update.
        let trace_name = root_name
            .clone()
            .or_else(|| service_name.clone())
            .unwrap_or_else(|| "otlp-trace".to_string());

        let trace = Trace {
            id: *trace_id,
            org_id: Some(org_id),
            name: Some(trace_name),
            tags: vec!["otlp".to_string()],
            started_at: *earliest_start,
            ended_at: None,
            machine_id: None,
        };

        if let Err(e) = w.save_trace(trace).await {
            tracing::error!(%trace_id, "OTLP: failed to save trace: {e}");
            continue;
        }

        // Insert all spans for this trace
        for span in spans {
            if let Err(e) = w.insert(span.clone()).await {
                tracing::error!(span_id = %span.id(), "OTLP: failed to insert span: {e}");
            }
        }
    }
    drop(w);

    // ---- Mirror traces/spans into Encore product API (daemon bridge) ----
    if let Some(bridge) = EncoreTraceBridge::from_env() {
        let client = reqwest::Client::new();
        for (trace_id, (_earliest_start, root_name, spans)) in &traces_map {
            let trace_name = root_name
                .clone()
                .or_else(|| service_name.clone())
                .unwrap_or_else(|| "otlp-trace".to_string());

            bridge
                .post_json(
                    &client,
                    "/traces",
                    &org_id_str,
                    &project_id_str,
                    serde_json::json!({
                        "id": trace_id.to_string(),
                        "name": trace_name,
                        "tags": ["otlp"],
                    }),
                )
                .await;

            for span in spans {
                bridge
                    .post_json(
                        &client,
                        "/spans",
                        &org_id_str,
                        &project_id_str,
                        serde_json::json!({
                            "id": span.id().to_string(),
                            "trace_id": span.trace_id().to_string(),
                            "parent_id": span.parent_id().map(|p| p.to_string()),
                            "name": span.name(),
                            "kind": serde_json::to_value(span.kind()).unwrap_or(serde_json::json!({"type": "custom"})),
                            "input": serde_json::Value::Null,
                        }),
                    )
                    .await;

                match span.status() {
                    SpanStatus::Failed { error } => {
                        bridge
                            .post_json(
                                &client,
                                &format!("/spans/{}/fail", span.id()),
                                &org_id_str,
                                &project_id_str,
                                serde_json::json!({"error": error}),
                            )
                            .await;
                    }
                    _ => {
                        bridge
                            .post_json(
                                &client,
                                &format!("/spans/{}/complete", span.id()),
                                &org_id_str,
                                &project_id_str,
                                serde_json::json!({"output": serde_json::Value::Null}),
                            )
                            .await;
                    }
                }
            }
        }
    }

    // ---- Emit events (outside write lock) ----
    for (trace_id, (earliest_start, root_name, spans)) in traces_map {
        // Emit TraceCreated — harmless if trace already existed (UI deduplicates).
        let trace_name = root_name
            .or_else(|| service_name.clone())
            .unwrap_or_else(|| "otlp-trace".to_string());
        let trace = Trace {
            id: trace_id,
            org_id: Some(org_id),
            name: Some(trace_name),
            tags: vec!["otlp".to_string()],
            started_at: earliest_start,
            ended_at: None,
            machine_id: None,
        };
        state.emit_event(SystemEvent::TraceCreated { trace }, &org_id_str);

        for span in spans {
            let span_clone = span.clone();
            // Emit appropriate event based on status
            match span.status() {
                SpanStatus::Failed { .. } => {
                    state.emit_event(SystemEvent::SpanFailed { span }, &org_id_str);
                }
                _ => {
                    state.emit_event(
                        SystemEvent::SpanCompleted { span: span.clone() },
                        &org_id_str,
                    );
                }
            }

            // Process capture rules for completed/failed spans
            if span_clone.status().is_terminal() {
                let store_clone = store.clone();
                let events_tx = state.events_tx.clone();
                let event_log = state.event_log.clone();
                let org_id_str2 = org_id_str.clone();
                tokio::spawn(async move {
                    capture::process_capture_rules(
                        &store_clone,
                        &span_clone,
                        &events_tx,
                        &event_log,
                        &org_id_str2,
                    )
                    .await;
                });
            }
        }
    }

    tracing::debug!(
        resource_spans = req.resource_spans.len(),
        "OTLP: trace ingest complete"
    );

    Ok(Json(ExportTraceServiceResponse {}))
}

// ---------------------------------------------------------------------------
// Auth helper for OTLP endpoints
// ---------------------------------------------------------------------------

/// Extract auth context from the OTLP request.
/// Supports: `Authorization: Bearer tw_sk_...` (API key) and local mode bypass.
async fn extract_otlp_auth(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<auth::AuthContext, (StatusCode, Json<serde_json::Value>)> {
    // In local mode, skip auth entirely
    if state.auth_config.local_mode {
        return Ok(auth::AuthContext::local());
    }

    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let token = auth_header
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Missing or invalid Authorization header. Use: Authorization: Bearer tw_sk_..."
                })),
            )
        })?;

    if !token.starts_with("tw_sk_") || token.len() < 16 {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid API key format" })),
        ));
    }

    let prefix = &token[..16];
    let (org_id, project_id, key_hash, scopes) = state
        .api_key_lookup
        .lookup_api_key(prefix)
        .await
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Unknown API key" })),
            )
        })?;

    if !auth::verify_api_key(token, &key_hash) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid API key" })),
        ));
    }

    Ok(auth::AuthContext::from_api_key(org_id, project_id, scopes))
}
