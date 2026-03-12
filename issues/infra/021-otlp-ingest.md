# OpenTelemetry OTLP Trace Ingest

**Labels:** `enhancement`, `backend`, `P1`, `hard`
**Priority:** P1
**Difficulty:** Hard

## Summary

Add a `POST /v1/traces` endpoint that accepts OpenTelemetry OTLP/HTTP JSON-encoded trace data. This lets any OTel-instrumented application (Python, Node, Go, Java, etc.) send traces directly to Traceway without a custom SDK — just point the `OTEL_EXPORTER_OTLP_ENDPOINT` at Traceway and traces show up.

## Why

Right now Traceway only accepts traces via its own REST API (`POST /api/traces`, `POST /api/spans`, `POST /api/spans/:id/complete`). This means users need to use the Traceway SDK or build custom integration. OTel is the industry standard — every major LLM framework (LangChain, LlamaIndex, Vercel AI SDK, OpenAI Agents SDK) either ships with or has community OTel instrumentation. Supporting OTLP ingest makes Traceway a drop-in replacement for Jaeger/Honeycomb/Datadog for LLM observability, massively expanding the addressable user base with zero SDK work.

## Scope

### In scope
- `POST /v1/traces` — OTLP/HTTP JSON (`application/json`) ingest
- API key authentication (same `tw_sk_` keys, via `Authorization: Bearer` header)
- Mapping OTel spans → Traceway Spans + auto-created Traces
- Gen AI semantic convention detection (`gen_ai.*` attributes → `LlmCall` SpanKind)
- Emitting SystemEvents for ingested spans (so SSE, capture rules, etc. all work)
- Returning proper `ExportTraceServiceResponse` JSON

### Out of scope (for now)
- OTLP/gRPC (protobuf over HTTP/2) — JSON-only for v1
- OTLP/HTTP protobuf (`application/x-protobuf`) — JSON-only for v1
- `POST /v1/metrics` and `POST /v1/logs` — traces only
- Partial success responses (all-or-nothing per batch for v1)

## Data Model Mapping

### IDs
OTel uses raw bytes encoded as hex strings: `trace_id` is 32 hex chars (16 bytes), `span_id` is 16 hex chars (8 bytes). Traceway uses UUIDv7.

**Strategy: deterministic UUID derivation.** Convert OTel hex IDs to Traceway UUIDs using a deterministic mapping so that:
- The same OTel trace_id always maps to the same Traceway TraceId
- Parent-child relationships are preserved (parent_span_id lookups work)
- Re-sending the same span is idempotent

Conversion approach:
- `trace_id` (16 bytes hex) → pad/hash into UUID v5 (namespace: a fixed Traceway OTLP namespace UUID)
- `span_id` (8 bytes hex) → combine with trace_id bytes to form UUID v5

This means OTel span IDs are globally unique within a trace context, which matches Traceway's model.

### Span fields

| OTel field | Traceway field | Notes |
|---|---|---|
| `name` | `name` | Direct copy |
| `traceId` (hex) | `trace_id` | UUID v5 derivation |
| `spanId` (hex) | `id` | UUID v5 derivation (scoped to trace) |
| `parentSpanId` (hex) | `parent_id` | UUID v5 derivation, `None` if empty/missing |
| `startTimeUnixNano` | `started_at` | Nanos → `DateTime<Utc>` |
| `endTimeUnixNano` | `ended_at` | Nanos → `DateTime<Utc>`, `None` if 0 |
| `status.code` | `status` | See status mapping below |
| `status.message` | `status` (error string) | Only used when status = ERROR |
| `kind` | Ignored | OTel span kind (CLIENT/SERVER/etc.) stored as attribute |
| `attributes` | `kind` (SpanKind) | Gen AI detection, else `Custom` |

### Status mapping

| OTel `status.code` | Value | Traceway SpanStatus |
|---|---|---|
| `STATUS_CODE_UNSET` | 0 | `Completed` (OTel default = success) |
| `STATUS_CODE_OK` | 1 | `Completed` |
| `STATUS_CODE_ERROR` | 2 | `Failed { error: status.message }` |

Note: OTel sends completed spans — there is no "Running" state. All OTLP-ingested spans arrive already finished.

### SpanKind detection

When processing OTel span attributes, check for Gen AI semantic conventions:

```
gen_ai.system         → provider (e.g., "openai", "anthropic")
gen_ai.request.model  → model
gen_ai.usage.input_tokens  → input_tokens
gen_ai.usage.output_tokens → output_tokens
gen_ai.usage.cost          → cost (if available, rare)
```

If `gen_ai.request.model` OR `gen_ai.system` is present → map to `SpanKind::LlmCall`.
Otherwise → map to `SpanKind::Custom` with `kind = otel_span_kind_name` (e.g., "client", "server", "internal") and all OTel attributes preserved in `attributes` HashMap.

For LlmCall spans, remaining non-gen_ai attributes are discarded (they can be added to input/output if useful later).

### Trace auto-creation

OTel has no explicit "create trace" concept — traces are implicit from shared `trace_id`. On OTLP ingest:

1. Derive the Traceway TraceId from OTel `trace_id`
2. Check if a Trace with that ID already exists in the store
3. If not → create one with:
   - `name`: the name of the root span (parentSpanId empty), or first span seen
   - `started_at`: earliest `startTimeUnixNano` in the batch for this trace
   - `tags`: `["otlp"]` (to distinguish OTel-sourced traces)
4. If yes → optionally update `ended_at` if this batch contains a later timestamp

### Input/Output mapping

OTel spans don't have a structured input/output concept. For `LlmCall` spans detected via gen_ai conventions:
- `input`: Look for `gen_ai.prompt` or message-related events; if not found, `None`
- `output`: Look for `gen_ai.completion` or message-related events; if not found, `None`

For `Custom` spans:
- `input` / `output`: `None` (attributes are in the `Custom.attributes` map)

## OTLP Request/Response Format

### Request: `ExportTraceServiceRequest`

```json
{
  "resourceSpans": [
    {
      "resource": {
        "attributes": [
          { "key": "service.name", "value": { "stringValue": "my-app" } }
        ]
      },
      "scopeSpans": [
        {
          "scope": { "name": "openai", "version": "1.0" },
          "spans": [
            {
              "traceId": "5b8aa5a2d2c872e8321cf37308d69df2",
              "spanId": "051581bf3cb55c13",
              "parentSpanId": "",
              "name": "chat.completions",
              "kind": 3,
              "startTimeUnixNano": "1544712660000000000",
              "endTimeUnixNano": "1544712661000000000",
              "attributes": [
                { "key": "gen_ai.system", "value": { "stringValue": "openai" } },
                { "key": "gen_ai.request.model", "value": { "stringValue": "gpt-4" } },
                { "key": "gen_ai.usage.input_tokens", "value": { "intValue": "150" } },
                { "key": "gen_ai.usage.output_tokens", "value": { "intValue": "50" } }
              ],
              "status": { "code": 1 }
            }
          ]
        }
      ]
    }
  ]
}
```

### Response: `ExportTraceServiceResponse`

On full success:
```json
{}
```

The OTLP spec allows an empty object for full success. No partial success handling in v1.

### OTel Attribute Value types

OTel uses a tagged union for attribute values:
```json
{ "stringValue": "foo" }
{ "intValue": "123" }        // Note: int as string
{ "doubleValue": 3.14 }
{ "boolValue": true }
{ "arrayValue": { "values": [...] } }
{ "kvlistValue": { "values": [{ "key": "k", "value": {...} }] } }
{ "bytesValue": "base64..." }
```

These must be converted to `serde_json::Value` for storage in `Custom.attributes`.

## Implementation Plan

### Phase 1: Types + Route (new file: `crates/api/src/otlp.rs`)

1. Define OTLP request/response structs:
   - `ExportTraceServiceRequest`, `ResourceSpans`, `ScopeSpans`, `OtlpSpan`
   - `OtlpResource`, `OtlpScope`, `OtlpStatus`, `OtlpKeyValue`, `OtlpAnyValue`
   - `ExportTraceServiceResponse`

2. ID conversion functions:
   - `otel_trace_id_to_uuid(hex: &str) -> Result<TraceId>`
   - `otel_span_id_to_uuid(trace_hex: &str, span_hex: &str) -> Result<SpanId>`

3. Attribute extraction helpers:
   - `extract_string_attr(attrs, key) -> Option<String>`
   - `extract_int_attr(attrs, key) -> Option<i64>`
   - `extract_double_attr(attrs, key) -> Option<f64>`
   - `otel_value_to_json(OtlpAnyValue) -> serde_json::Value`

### Phase 2: Span conversion logic

4. `convert_otlp_span(span: OtlpSpan, resource: &OtlpResource, org_id: OrgId) -> Result<Span>`:
   - Convert IDs
   - Detect gen_ai attributes → build SpanKind
   - Map status
   - Convert timestamps
   - Build Span directly (not via SpanBuilder, since we have all fields)

### Phase 3: Ingest handler

5. `POST /v1/traces` handler:
   - Extract API key from `Authorization: Bearer <key>` header
   - Validate key, resolve org_id
   - Parse `ExportTraceServiceRequest`
   - For each resource_spans → scope_spans → span:
     - Convert to Traceway Span
     - Group by trace_id
   - For each unique trace_id:
     - Check if Trace exists; if not, create it
   - Insert all Spans into store
   - Emit SystemEvents (SpanCreated, TraceCreated)
   - Run capture rules on completed spans
   - Return `ExportTraceServiceResponse {}`

### Phase 4: Route wiring

6. Mount `POST /v1/traces` on the app router:
   - NOT under `/api` — OTLP spec requires `/v1/traces` at the root
   - Auth: API key only (no cookie/session auth)
   - Should work in both local and cloud mode

## Files to create
- `crates/api/src/otlp.rs` — all OTLP types, conversion logic, and handler

## Files to modify
- `crates/api/src/lib.rs` — add `pub mod otlp;`, mount `/v1/traces` route in `build_router()`
- `crates/api/Cargo.toml` — add `uuid` features if needed (v5 generation)
- `crates/trace/src/lib.rs` — may need to add a constructor that accepts pre-built fields (for OTLP spans that arrive already completed with known IDs)

## Acceptance criteria

- [ ] `POST /v1/traces` accepts OTLP/HTTP JSON and returns `{}`
- [ ] OTel trace_id/span_id are deterministically mapped to Traceway UUIDs
- [ ] Parent-child relationships are preserved
- [ ] Traces are auto-created for new trace_ids
- [ ] `gen_ai.*` attributes are detected and mapped to `LlmCall` SpanKind
- [ ] Non-gen_ai spans map to `Custom` with OTel attributes preserved
- [ ] OTel status codes map correctly to Traceway SpanStatus
- [ ] Timestamps (nanoseconds) convert correctly to `DateTime<Utc>`
- [ ] SystemEvents are emitted (SSE subscribers see OTLP-ingested spans)
- [ ] Capture rules fire on OTLP-ingested spans
- [ ] API key authentication works (`Authorization: Bearer tw_sk_...`)
- [ ] `cargo check -p api` succeeds
- [ ] Manual test: send OTel trace data with `curl`, verify it appears in Traceway UI

## Testing approach

```bash
# Generate an API key, then:
curl -X POST http://localhost:8080/v1/traces \
  -H "Authorization: Bearer tw_sk_..." \
  -H "Content-Type: application/json" \
  -d '{
    "resourceSpans": [{
      "resource": { "attributes": [{"key":"service.name","value":{"stringValue":"test"}}] },
      "scopeSpans": [{
        "scope": {"name":"test"},
        "spans": [{
          "traceId": "5b8aa5a2d2c872e8321cf37308d69df2",
          "spanId": "051581bf3cb55c13",
          "parentSpanId": "",
          "name": "chat gpt-4",
          "kind": 3,
          "startTimeUnixNano": "1709650000000000000",
          "endTimeUnixNano": "1709650001000000000",
          "attributes": [
            {"key":"gen_ai.system","value":{"stringValue":"openai"}},
            {"key":"gen_ai.request.model","value":{"stringValue":"gpt-4"}},
            {"key":"gen_ai.usage.input_tokens","value":{"intValue":"100"}},
            {"key":"gen_ai.usage.output_tokens","value":{"intValue":"50"}}
          ],
          "status": {"code": 1}
        }]
      }]
    }]
  }'
# Should return: {}
# Then verify via: GET /api/traces and GET /api/spans
```

## Future work (out of scope for this issue)

- OTLP/gRPC support (protobuf over HTTP/2)
- OTLP/HTTP protobuf content type
- Partial success responses with per-span error reporting
- `POST /v1/metrics` and `POST /v1/logs`
- OTel resource attributes → Traceway trace metadata
- Span events (OTel events within a span) → structured data
- Span links support
