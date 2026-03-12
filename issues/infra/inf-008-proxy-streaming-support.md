# Add Streaming Support to LLM Proxy

**Labels:** `enhancement`, `backend`
**Difficulty:** Hard
**Priority:** P1

## Summary

The LLM proxy (`crates/proxy/src/lib.rs`) buffers the entire LLM response body before forwarding it to the client. For streaming responses (`stream: true` in OpenAI/Anthropic), this defeats the purpose — the client sees nothing until the full response is ready. The proxy also has a 10MB body size limit and no timeouts, both of which cause issues with large or slow responses.

## Impact

- Users cannot get streaming responses through the proxy
- Long-running LLM calls may timeout at the HTTP layer before the proxy returns
- Responses over 10MB are rejected entirely
- TTFB (time to first byte) is equal to total generation time, destroying UX for interactive use cases

## Where

`crates/proxy/src/lib.rs` — the main request handler that:
1. Reads the entire request body
2. Forwards to the upstream LLM provider
3. Reads the entire response body
4. Processes it (extract tokens, build span)
5. Returns the full response to the client

## What to do

### 1. Detect streaming requests

Check if the request body contains `"stream": true`. If so, use a streaming code path.

### 2. Stream SSE chunks through

For streaming responses, forward each SSE chunk (`data: {...}\n\n`) to the client as it arrives from the upstream provider. Use Axum's `Body::from_stream()` or equivalent.

### 3. Accumulate for span metadata

While streaming chunks through, accumulate the content pieces to build the final span metadata (output preview, token counts). The `[DONE]` sentinel or the final chunk with `usage` data provides token counts.

### 4. Build span after stream completes

Once the stream ends, create/complete the span with the accumulated metadata. The span is finalized asynchronously — the client has already received all data.

### 5. Add timeouts

- Connect timeout: 10s
- First byte timeout: 120s (LLM reasoning can be slow)
- Total request timeout: 600s (10 min for very long generations)

### 6. Remove body size limit

Replace the 10MB limit with a streaming approach that doesn't buffer the full body.

## Files to modify

- `crates/proxy/src/lib.rs` — Major refactor of the request handler

## Acceptance criteria

- [ ] `stream: true` requests receive SSE chunks in real-time
- [ ] Non-streaming requests still work as before
- [ ] Spans are created with correct token counts for streaming responses
- [ ] Timeouts are configurable
- [ ] No body size limit for streaming responses
- [ ] `cargo check -p proxy` passes
