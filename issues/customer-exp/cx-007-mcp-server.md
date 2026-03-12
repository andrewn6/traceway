# MCP Server for Traceway

**Labels:** `enhancement`, `backend`
**Difficulty:** Hard
**Priority:** High — Edge Feature

## Summary

Ship a Traceway MCP server so AI coding agents (Claude Code, Cursor, Codex, etc.) can query trace data directly. The unique angle: Traceway supports **local mode** — the MCP server can run as a local sidecar against `localhost:3000` with zero cloud dependency, making it the only LLM observability MCP server that works fully offline.

## Context

Laminar has an MCP server that exposes two tools: `query_laminar_sql` (ClickHouse SQL) and `get_trace_context` (trace summary). Traceway can do this better by leveraging:

1. **Local-first**: The MCP server connects to the local daemon — no API key needed, no cloud latency, works offline
2. **Traceway's query DSL**: Instead of requiring SQL, expose the existing DSL (`kind:llm_call model:gpt-4o duration:>1s`) which is simpler to use from an AI agent
3. **Richer tools**: Beyond search and summary, expose tools that let the agent interact with Traceway (create annotations, add to datasets, tag traces)

## What to do

### 1. MCP server binary

Create a new crate `crates/mcp/` (or add to the daemon) that implements the MCP protocol:

- **Transport**: HTTP (`/v1/mcp`) for cloud mode (authenticated with API key), and stdio for local mode (no auth needed)
- Implement the MCP JSON-RPC protocol for `tools/list` and `tools/call`

### 2. Tools to expose

**Read tools:**

- `search_traces` — Takes a Traceway query DSL string, returns matching traces with summary metadata (name, status, duration, tokens, cost, timestamp). Example: `kind:llm_call status:failed since:1h`
- `get_trace` — Takes a trace ID, returns the full span tree with inputs/outputs formatted for LLM consumption (truncate very long I/O to keep context manageable)
- `get_span` — Takes a span ID, returns the full span detail including input/output
- `list_recent_traces` — Returns the N most recent traces with summary info (default N=10)

**Write tools (stretch):**

- `tag_trace` — Add tags to a trace by ID
- `add_to_dataset` — Add a span's input/output to a dataset

### 3. Local mode integration

When running via `cargo run -p daemon`, the MCP server should be available at `localhost:3000/v1/mcp` (same port as the API). No authentication required in local mode.

Users configure it in Claude Code:
```bash
# Local mode — no API key needed
claude mcp add --transport http traceway http://localhost:3000/v1/mcp

# Cloud mode
claude mcp add --transport http traceway https://api.traceway.ai/v1/mcp \
  --header "Authorization: Bearer <API_KEY>"
```

### 4. Trace context formatting

The `get_trace` tool response should be optimized for LLM consumption:
- Span tree rendered as indented text, not raw JSON
- Long inputs/outputs truncated to ~2000 chars with a note ("truncated, use get_span for full content")
- Error messages shown prominently
- Token counts and costs summarized at the top

## Design notes

- The MCP protocol is JSON-RPC 2.0 over HTTP or stdio. See the [MCP specification](https://spec.modelcontextprotocol.io/)
- For HTTP transport, the endpoint receives POST requests with JSON-RPC payloads
- Keep the tool descriptions concise — they become part of the AI agent's context

## Files to create/modify

- New: `crates/mcp/` or new module in `crates/api/src/mcp.rs`
- `crates/api/src/lib.rs` — Register MCP route
- `Cargo.toml` — Add MCP dependencies if needed

## Acceptance criteria

- [ ] MCP server exposes `search_traces`, `get_trace`, `get_span`, `list_recent_traces` tools
- [ ] Works in local mode without authentication
- [ ] Works in cloud mode with API key auth
- [ ] `search_traces` accepts Traceway's query DSL
- [ ] `get_trace` returns a human-readable trace summary optimized for LLM context
- [ ] Claude Code can successfully connect and query traces
- [ ] `cargo check` passes
