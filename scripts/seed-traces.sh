#!/usr/bin/env bash
# seed-traces.sh — Generate realistic trace data against a live Traceway instance.
#
# Usage:
#   ./scripts/seed-traces.sh                                  # local, no auth
#   ./scripts/seed-traces.sh -k tw_sk_abc123                  # local with key
#   ./scripts/seed-traces.sh -u https://api.traceway.ai -k tw_sk_abc123
#
# Env vars (alternative to flags):
#   TRACEWAY_URL       Base URL (default: http://localhost:3000)
#   TRACEWAY_API_KEY   API key

set -euo pipefail

URL="${TRACEWAY_URL:-http://localhost:3000}"
KEY="${TRACEWAY_API_KEY:-}"

while getopts "u:k:" opt; do
  case $opt in
    u) URL="$OPTARG" ;;
    k) KEY="$OPTARG" ;;
    *) echo "Usage: $0 [-u url] [-k api_key]" && exit 1 ;;
  esac
done

API="$URL/api"

# ── HTTP helpers ──────────────────────────────────────────────────────

post() {
  local path="$1" body="$2"
  if [[ -n "$KEY" ]]; then
    curl -s -X POST "$API$path" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $KEY" \
      -d "$body"
  else
    curl -s -X POST "$API$path" \
      -H "Content-Type: application/json" \
      -d "$body"
  fi
}

# Extract JSON field (portable, no jq dependency)
json_val() {
  local key="$1" json="$2"
  echo "$json" | grep -o "\"$key\":\"[^\"]*\"" | head -1 | cut -d'"' -f4
}

# ── Colors ────────────────────────────────────────────────────────────

G='\033[0;32m' Y='\033[0;33m' C='\033[0;36m' D='\033[0;90m' R='\033[0m'

log()  { echo -e "${G}✓${R} $*"; }
info() { echo -e "${C}→${R} $*"; }
head() { echo -e "\n${Y}━━━ $* ━━━${R}"; }

# ── Verify connection ─────────────────────────────────────────────────

info "Connecting to ${C}$API${R}"
if [[ -n "$KEY" ]]; then
  health=$(curl -s -o /dev/null -w "%{http_code}" "$API/health" -H "Authorization: Bearer $KEY" 2>/dev/null || echo "000")
else
  health=$(curl -s -o /dev/null -w "%{http_code}" "$API/health" 2>/dev/null || echo "000")
fi
if [[ "$health" != "200" ]]; then
  echo "ERROR: Cannot reach $API/health (HTTP $health)" >&2
  exit 1
fi
log "Connected"

# ══════════════════════════════════════════════════════════════════════
#  Trace 1: RAG Chat Agent
# ══════════════════════════════════════════════════════════════════════

head "Trace 1: RAG Chat Agent"

resp=$(post "/traces" '{"name":"rag-chat: what are our refund policies?","tags":["production","chat","rag"]}')
T1=$(json_val "id" "$resp")
log "Trace $T1"

# Root: agent orchestrator
resp=$(post "/spans" "{
  \"trace_id\":\"$T1\",
  \"name\":\"agent-orchestrator\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"agent\",\"attributes\":{\"version\":\"1.4.2\",\"framework\":\"langchain\"}},
  \"input\":{\"query\":\"What are our refund policies for enterprise customers?\",\"user_id\":\"usr_8f3k2\",\"session_id\":\"sess_29x\"}
}")
S_ROOT=$(json_val "id" "$resp")
log "  root: agent-orchestrator ($S_ROOT)"
sleep 0.05

# Step 1: query rewriting LLM call
resp=$(post "/spans" "{
  \"trace_id\":\"$T1\",
  \"parent_id\":\"$S_ROOT\",
  \"name\":\"rewrite-query\",
  \"kind\":{\"type\":\"llm_call\",\"model\":\"gpt-4o-mini\",\"provider\":\"openai\",\"input_tokens\":340,\"output_tokens\":89,\"cost\":0.00012},
  \"input\":{\"messages\":[{\"role\":\"system\",\"content\":\"Rewrite the user query for optimal vector search retrieval. Output only the rewritten query.\"},{\"role\":\"user\",\"content\":\"What are our refund policies for enterprise customers?\"}]}
}")
S_REWRITE=$(json_val "id" "$resp")
sleep 0.03
post "/spans/$S_REWRITE/complete" '{"output":{"rewritten":"enterprise customer refund policy terms conditions SLA"}}'>/dev/null
log "  ├─ rewrite-query (gpt-4o-mini)"

# Step 2: embedding
resp=$(post "/spans" "{
  \"trace_id\":\"$T1\",
  \"parent_id\":\"$S_ROOT\",
  \"name\":\"embed-query\",
  \"kind\":{\"type\":\"llm_call\",\"model\":\"text-embedding-3-small\",\"provider\":\"openai\",\"input_tokens\":12,\"cost\":0.000002},
  \"input\":{\"text\":\"enterprise customer refund policy terms conditions SLA\"}
}")
S_EMBED=$(json_val "id" "$resp")
sleep 0.02
post "/spans/$S_EMBED/complete" '{"output":{"dimensions":1536,"model":"text-embedding-3-small"}}'>/dev/null
log "  ├─ embed-query (text-embedding-3-small)"

# Step 3: vector search
resp=$(post "/spans" "{
  \"trace_id\":\"$T1\",
  \"parent_id\":\"$S_ROOT\",
  \"name\":\"vector-search\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"retrieval\",\"attributes\":{\"backend\":\"turbopuffer\",\"namespace\":\"docs\",\"top_k\":5}},
  \"input\":{\"vector_dim\":1536,\"top_k\":5,\"filter\":{\"category\":\"policy\"}}
}")
S_SEARCH=$(json_val "id" "$resp")
sleep 0.04
post "/spans/$S_SEARCH/complete" '{"output":{"results":[{"id":"doc_391","score":0.94,"title":"Enterprise Refund Policy v2.3"},{"id":"doc_127","score":0.87,"title":"SLA Terms - Enterprise Tier"},{"id":"doc_445","score":0.82,"title":"Billing FAQ"},{"id":"doc_203","score":0.76,"title":"General Refund Guidelines"},{"id":"doc_512","score":0.71,"title":"Enterprise Onboarding"}]}}'>/dev/null
log "  ├─ vector-search (turbopuffer, 5 results)"

# Step 4: read retrieved docs
for i in 1 2 3; do
  paths=("/policies/enterprise-refund-v2.3.md" "/legal/sla-enterprise.md" "/support/billing-faq.md")
  bytes=(4823 3291 2104)
  idx=$((i-1))
  resp=$(post "/spans" "{
    \"trace_id\":\"$T1\",
    \"parent_id\":\"$S_ROOT\",
    \"name\":\"fetch-document-$i\",
    \"kind\":{\"type\":\"fs_read\",\"path\":\"${paths[$idx]}\",\"bytes_read\":${bytes[$idx]}}
  }")
  S_DOC=$(json_val "id" "$resp")
  sleep 0.01
  post "/spans/$S_DOC/complete" "{\"output\":{\"path\":\"${paths[$idx]}\",\"chars\":${bytes[$idx]}}}">/dev/null
done
log "  ├─ fetch-document x3"

# Step 5: synthesis LLM call (the big one)
resp=$(post "/spans" "{
  \"trace_id\":\"$T1\",
  \"parent_id\":\"$S_ROOT\",
  \"name\":\"synthesize-answer\",
  \"kind\":{\"type\":\"llm_call\",\"model\":\"gpt-4o\",\"provider\":\"openai\",\"input_tokens\":6230,\"output_tokens\":847,\"cost\":0.042},
  \"input\":{\"messages\":[{\"role\":\"system\",\"content\":\"You are a helpful customer support agent. Answer the user's question based on the retrieved context documents. Cite specific policy sections.\"},{\"role\":\"user\",\"content\":\"What are our refund policies for enterprise customers?\"},{\"role\":\"assistant\",\"content\":null}],\"context_docs\":3,\"temperature\":0.3}
}")
S_SYNTH=$(json_val "id" "$resp")
sleep 0.08
post "/spans/$S_SYNTH/complete" '{"output":{"response":"Enterprise customers are eligible for full refunds within 30 days of purchase under Policy §4.2. After 30 days, pro-rated refunds are available for annual contracts per SLA §7.1. To initiate a refund, contact your account manager or submit a request through the enterprise portal. Processing takes 5-10 business days.","citations":["§4.2 Enterprise Refund Policy v2.3","§7.1 SLA Terms - Enterprise Tier"]}}'>/dev/null
log "  └─ synthesize-answer (gpt-4o, 847 tokens)"

# Complete root
post "/spans/$S_ROOT/complete" '{"output":{"response_length":847,"sources":3,"latency_ms":312}}'>/dev/null
log "Trace 1 complete ✓"

# ══════════════════════════════════════════════════════════════════════
#  Trace 2: Coding Agent Session
# ══════════════════════════════════════════════════════════════════════

head "Trace 2: Coding Agent — fix auth middleware"

resp=$(post "/traces" '{"name":"coding-agent: fix CORS auth middleware","tags":["development","agent","debugging"]}')
T2=$(json_val "id" "$resp")
log "Trace $T2"

# Root
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"name\":\"agent-loop\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"agent\",\"attributes\":{\"model\":\"claude-sonnet-4-20250514\",\"max_iterations\":10}},
  \"input\":{\"task\":\"Fix CORS middleware to support cross-origin credentials. Currently returns 403 on preflight from platform.traceway.ai\"}
}")
S_ROOT2=$(json_val "id" "$resp")
sleep 0.03

# Step 1: read the current CORS config
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"parent_id\":\"$S_ROOT2\",
  \"name\":\"read-api-source\",
  \"kind\":{\"type\":\"fs_read\",\"path\":\"crates/api/src/lib.rs\",\"bytes_read\":48230}
}")
S_READ1=$(json_val "id" "$resp")
sleep 0.02
post "/spans/$S_READ1/complete" '{"output":{"lines":2450,"found_cors_config":true,"line_range":"2342-2345"}}'>/dev/null
log "  ├─ read crates/api/src/lib.rs"

# Step 2: planning call
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"parent_id\":\"$S_ROOT2\",
  \"name\":\"analyze-and-plan\",
  \"kind\":{\"type\":\"llm_call\",\"model\":\"claude-sonnet-4-20250514\",\"provider\":\"anthropic\",\"input_tokens\":12400,\"output_tokens\":1890,\"cost\":0.068},
  \"input\":{\"messages\":[{\"role\":\"user\",\"content\":\"I see the CORS config at line 2342 uses allow_origin(Any). This won't work with credentials: include from the browser. I need to fix this.\"}]}
}")
S_PLAN2=$(json_val "id" "$resp")
sleep 0.06
post "/spans/$S_PLAN2/complete" '{"output":{"plan":["1. Replace Any origin with explicit AllowOrigin::list","2. Add allow_credentials(true)","3. Read origins from ALLOWED_ORIGINS env var","4. Update session cookie SameSite=None for cross-origin"]}}'>/dev/null
log "  ├─ analyze-and-plan (claude-sonnet-4-20250514)"

# Step 3: edit CORS config
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"parent_id\":\"$S_ROOT2\",
  \"name\":\"edit-cors-config\",
  \"kind\":{\"type\":\"fs_write\",\"path\":\"crates/api/src/lib.rs\",\"file_version\":\"sha256:a8f3e2\",\"bytes_written\":892}
}")
S_WRITE1=$(json_val "id" "$resp")
sleep 0.02
post "/spans/$S_WRITE1/complete" '{"output":{"lines_changed":18,"description":"Replace Any CORS with explicit origin list + credentials"}}'>/dev/null
log "  ├─ edit crates/api/src/lib.rs (CORS)"

# Step 4: edit cookie config
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"parent_id\":\"$S_ROOT2\",
  \"name\":\"edit-cookie-config\",
  \"kind\":{\"type\":\"fs_write\",\"path\":\"crates/api/src/auth_routes.rs\",\"file_version\":\"sha256:b2c4d1\",\"bytes_written\":645}
}")
S_WRITE2=$(json_val "id" "$resp")
sleep 0.02
post "/spans/$S_WRITE2/complete" '{"output":{"lines_changed":14,"description":"SameSite=None; Secure when ALLOWED_ORIGINS is set"}}'>/dev/null
log "  ├─ edit auth_routes.rs (cookies)"

# Step 5: cargo check — FAILS first time
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"parent_id\":\"$S_ROOT2\",
  \"name\":\"cargo-check\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"tool_call\",\"attributes\":{\"tool\":\"bash\",\"command\":\"cargo check -p api\"}},
  \"input\":{\"command\":\"cargo check -p api\"}
}")
S_CHECK1=$(json_val "id" "$resp")
sleep 0.1
post "/spans/$S_CHECK1/fail" '{"error":"error[E0433]: failed to resolve: use of unresolved module or unlinked crate `http`\n  --> crates/api/src/lib.rs:2346:26\n   |\n2346 |         let origins: Vec<http::HeaderValue> = origins\n   |                          ^^^^ use of unresolved module"}'>/dev/null
log "  ├─ cargo check (FAILED — unresolved import)"

# Step 6: fix the error
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"parent_id\":\"$S_ROOT2\",
  \"name\":\"fix-import\",
  \"kind\":{\"type\":\"fs_write\",\"path\":\"crates/api/src/lib.rs\",\"file_version\":\"sha256:c3d5e2\",\"bytes_written\":42}
}")
S_FIX=$(json_val "id" "$resp")
sleep 0.01
post "/spans/$S_FIX/complete" '{"output":{"fix":"Changed http::HeaderValue to axum::http::HeaderValue"}}'>/dev/null
log "  ├─ fix-import (quick edit)"

# Step 7: cargo check — succeeds
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"parent_id\":\"$S_ROOT2\",
  \"name\":\"cargo-check-retry\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"tool_call\",\"attributes\":{\"tool\":\"bash\",\"command\":\"cargo check -p api\"}},
  \"input\":{\"command\":\"cargo check -p api\"}
}")
S_CHECK2=$(json_val "id" "$resp")
sleep 0.08
post "/spans/$S_CHECK2/complete" '{"output":{"status":"success","warnings":7,"errors":0}}'>/dev/null
log "  ├─ cargo check retry (OK, 7 warnings)"

# Step 8: summary LLM call
resp=$(post "/spans" "{
  \"trace_id\":\"$T2\",
  \"parent_id\":\"$S_ROOT2\",
  \"name\":\"summarize-changes\",
  \"kind\":{\"type\":\"llm_call\",\"model\":\"claude-sonnet-4-20250514\",\"provider\":\"anthropic\",\"input_tokens\":3200,\"output_tokens\":512,\"cost\":0.019},
  \"input\":{\"messages\":[{\"role\":\"user\",\"content\":\"Summarize what was changed and why\"}]}
}")
S_SUM=$(json_val "id" "$resp")
sleep 0.04
post "/spans/$S_SUM/complete" '{"output":{"summary":"Fixed cross-origin auth by: (1) CORS now reads ALLOWED_ORIGINS env var for explicit origin list with credentials support, (2) session cookies use SameSite=None; Secure when cross-origin is configured. Fallback to permissive Any for local mode."}}'>/dev/null
log "  └─ summarize-changes (claude-sonnet-4-20250514)"

post "/spans/$S_ROOT2/complete" '{"output":{"iterations":3,"files_modified":2,"result":"success"}}'>/dev/null
log "Trace 2 complete ✓"

# ══════════════════════════════════════════════════════════════════════
#  Trace 3: Multi-model Eval Pipeline
# ══════════════════════════════════════════════════════════════════════

head "Trace 3: Model Evaluation Pipeline"

resp=$(post "/traces" '{"name":"eval: summarization benchmark Q2","tags":["eval","benchmark","summarization"]}')
T3=$(json_val "id" "$resp")
log "Trace $T3"

# Root
resp=$(post "/spans" "{
  \"trace_id\":\"$T3\",
  \"name\":\"eval-pipeline\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"pipeline\",\"attributes\":{\"dataset\":\"summarization-v3\",\"samples\":3,\"models\":[\"gpt-4o\",\"claude-sonnet-4-20250514\",\"llama-3.1-70b\"]}},
  \"input\":{\"config\":{\"temperature\":0.0,\"max_tokens\":500,\"scoring\":[\"rouge-l\",\"bertscore\",\"human-pref\"]}}
}")
S_EVAL_ROOT=$(json_val "id" "$resp")
sleep 0.02

models=("gpt-4o" "claude-sonnet-4-20250514" "llama-3.1-70b")
providers=("openai" "anthropic" "ollama")
costs=(0.028 0.032 0.0)
input_toks=(2100 2100 2100)
output_toks=(380 420 510)
scores=(0.89 0.92 0.78)

for i in 0 1 2; do
  model="${models[$i]}"
  provider="${providers[$i]}"

  # Model group span
  resp=$(post "/spans" "{
    \"trace_id\":\"$T3\",
    \"parent_id\":\"$S_EVAL_ROOT\",
    \"name\":\"eval-${model}\",
    \"kind\":{\"type\":\"custom\",\"kind\":\"model-eval\",\"attributes\":{\"model\":\"$model\"}},
    \"input\":{\"model\":\"$model\",\"samples\":3}
  }")
  S_MODEL=$(json_val "id" "$resp")

  # 3 samples per model
  for s in 1 2 3; do
    resp=$(post "/spans" "{
      \"trace_id\":\"$T3\",
      \"parent_id\":\"$S_MODEL\",
      \"name\":\"sample-$s\",
      \"kind\":{\"type\":\"llm_call\",\"model\":\"$model\",\"provider\":\"$provider\",\"input_tokens\":${input_toks[$i]},\"output_tokens\":${output_toks[$i]},\"cost\":${costs[$i]}},
      \"input\":{\"messages\":[{\"role\":\"system\",\"content\":\"Summarize the following article in 2-3 sentences.\"},{\"role\":\"user\",\"content\":\"[Article $s content — approximately 2000 tokens of news article text...]\"}]}
    }")
    S_SAMPLE=$(json_val "id" "$resp")
    sleep 0.03
    post "/spans/$S_SAMPLE/complete" "{\"output\":{\"summary\":\"[Generated summary for article $s by $model]\",\"rouge_l\":${scores[$i]},\"tokens\":${output_toks[$i]}}}">/dev/null
  done

  post "/spans/$S_MODEL/complete" "{\"output\":{\"avg_rouge_l\":${scores[$i]},\"avg_latency_ms\":$((200 + i * 150)),\"total_cost\":$(echo "${costs[$i]} * 3" | bc)}}">/dev/null
  log "  ├─ eval-$model (3 samples, avg rouge-l: ${scores[$i]})"
done

# Scoring span
resp=$(post "/spans" "{
  \"trace_id\":\"$T3\",
  \"parent_id\":\"$S_EVAL_ROOT\",
  \"name\":\"compute-rankings\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"scoring\",\"attributes\":{\"method\":\"weighted-average\"}},
  \"input\":{\"models\":3,\"metrics\":[\"rouge-l\",\"bertscore\",\"human-pref\"]}
}")
S_SCORE=$(json_val "id" "$resp")
sleep 0.02
post "/spans/$S_SCORE/complete" '{"output":{"rankings":[{"model":"claude-sonnet-4-20250514","score":0.92,"rank":1},{"model":"gpt-4o","score":0.89,"rank":2},{"model":"llama-3.1-70b","score":0.78,"rank":3}],"winner":"claude-sonnet-4-20250514"}}'>/dev/null
log "  └─ compute-rankings (winner: claude-sonnet-4-20250514)"

post "/spans/$S_EVAL_ROOT/complete" '{"output":{"winner":"claude-sonnet-4-20250514","models_evaluated":3,"total_samples":9}}'>/dev/null
log "Trace 3 complete ✓"

# ══════════════════════════════════════════════════════════════════════
#  Trace 4: Multi-step Tool-use Agent (partially failed)
# ══════════════════════════════════════════════════════════════════════

head "Trace 4: Data Pipeline Agent (partial failure)"

resp=$(post "/traces" '{"name":"data-pipeline: daily user metrics ETL","tags":["production","etl","scheduled"]}')
T4=$(json_val "id" "$resp")
log "Trace $T4"

# Root
resp=$(post "/spans" "{
  \"trace_id\":\"$T4\",
  \"name\":\"etl-orchestrator\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"pipeline\",\"attributes\":{\"schedule\":\"0 2 * * *\",\"dag\":\"user-metrics-daily\"}},
  \"input\":{\"date\":\"2026-02-22\",\"source\":\"postgres\",\"destination\":\"s3+parquet\"}
}")
S_ETL=$(json_val "id" "$resp")
sleep 0.02

# Extract
resp=$(post "/spans" "{
  \"trace_id\":\"$T4\",
  \"parent_id\":\"$S_ETL\",
  \"name\":\"extract-user-events\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"sql-query\",\"attributes\":{\"database\":\"analytics-primary\",\"table\":\"user_events\"}},
  \"input\":{\"query\":\"SELECT * FROM user_events WHERE date = '2026-02-22' AND event_type IN ('page_view','click','signup','purchase')\"}
}")
S_EXTRACT=$(json_val "id" "$resp")
sleep 0.06
post "/spans/$S_EXTRACT/complete" '{"output":{"rows":284930,"bytes":142465000,"duration_ms":3420}}'>/dev/null
log "  ├─ extract-user-events (284,930 rows)"

# Transform with LLM classification
resp=$(post "/spans" "{
  \"trace_id\":\"$T4\",
  \"parent_id\":\"$S_ETL\",
  \"name\":\"classify-intent\",
  \"kind\":{\"type\":\"llm_call\",\"model\":\"gpt-4o-mini\",\"provider\":\"openai\",\"input_tokens\":45000,\"output_tokens\":12000,\"cost\":0.018},
  \"input\":{\"task\":\"Classify user intents from event sequences\",\"batch_size\":500}
}")
S_CLASSIFY=$(json_val "id" "$resp")
sleep 0.1
post "/spans/$S_CLASSIFY/complete" '{"output":{"classified":284930,"categories":{"browsing":142000,"purchasing":48200,"support":31400,"churning":8930,"other":54400}}}'>/dev/null
log "  ├─ classify-intent (gpt-4o-mini, 284k events)"

# Aggregate
resp=$(post "/spans" "{
  \"trace_id\":\"$T4\",
  \"parent_id\":\"$S_ETL\",
  \"name\":\"aggregate-metrics\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"transform\",\"attributes\":{\"engine\":\"polars\"}},
  \"input\":{\"operations\":[\"group_by user_id\",\"compute session_duration\",\"compute conversion_rate\",\"join with user_profiles\"]}
}")
S_AGG=$(json_val "id" "$resp")
sleep 0.04
post "/spans/$S_AGG/complete" '{"output":{"unique_users":42891,"avg_session_min":8.3,"conversion_rate":0.034}}'>/dev/null
log "  ├─ aggregate-metrics (42,891 users)"

# Load to S3 — FAILS
resp=$(post "/spans" "{
  \"trace_id\":\"$T4\",
  \"parent_id\":\"$S_ETL\",
  \"name\":\"upload-to-s3\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"s3-upload\",\"attributes\":{\"bucket\":\"traceway-analytics\",\"key\":\"metrics/2026/02/22/user_metrics.parquet\"}},
  \"input\":{\"format\":\"parquet\",\"compression\":\"snappy\",\"estimated_size_mb\":89}
}")
S_UPLOAD=$(json_val "id" "$resp")
sleep 0.05
post "/spans/$S_UPLOAD/fail" '{"error":"AWS S3 PutObject failed: AccessDenied — The bucket policy does not allow s3:PutObject for role arn:aws:iam::123456789:role/etl-runner. Check IAM permissions and bucket policy."}'>/dev/null
log "  ├─ upload-to-s3 (FAILED — AccessDenied)"

# Root fails too
post "/spans/$S_ETL/fail" '{"error":"Pipeline failed at load stage: S3 upload denied. 3/4 stages completed. Data extracted and transformed but not persisted."}'>/dev/null
log "  └─ etl-orchestrator (FAILED)"
log "Trace 4 complete (with failure) ✓"

# ══════════════════════════════════════════════════════════════════════
#  Trace 5: Live streaming agent (left running)
# ══════════════════════════════════════════════════════════════════════

head "Trace 5: Live Agent Session (still running)"

resp=$(post "/traces" '{"name":"live-debug: investigate memory leak","tags":["development","live","debugging"]}')
T5=$(json_val "id" "$resp")
log "Trace $T5"

resp=$(post "/spans" "{
  \"trace_id\":\"$T5\",
  \"name\":\"debug-session\",
  \"kind\":{\"type\":\"custom\",\"kind\":\"agent\",\"attributes\":{\"model\":\"claude-sonnet-4-20250514\",\"streaming\":true}},
  \"input\":{\"task\":\"Memory usage keeps climbing in the proxy crate. Investigate and fix the leak.\"}
}")
S_DEBUG=$(json_val "id" "$resp")
sleep 0.02

resp=$(post "/spans" "{
  \"trace_id\":\"$T5\",
  \"parent_id\":\"$S_DEBUG\",
  \"name\":\"read-proxy-source\",
  \"kind\":{\"type\":\"fs_read\",\"path\":\"crates/proxy/src/lib.rs\",\"bytes_read\":15200}
}")
S_DREAD=$(json_val "id" "$resp")
sleep 0.02
post "/spans/$S_DREAD/complete" '{"output":{"lines":420,"suspicious":["line 187: clone of request body","line 243: unbounded channel"]}}'>/dev/null
log "  ├─ read proxy source (completed)"

resp=$(post "/spans" "{
  \"trace_id\":\"$T5\",
  \"parent_id\":\"$S_DEBUG\",
  \"name\":\"analyze-memory-patterns\",
  \"kind\":{\"type\":\"llm_call\",\"model\":\"claude-sonnet-4-20250514\",\"provider\":\"anthropic\",\"input_tokens\":8400,\"output_tokens\":2100,\"cost\":0.052},
  \"input\":{\"messages\":[{\"role\":\"user\",\"content\":\"Analyze this Rust proxy code for memory leaks. Focus on request body cloning and channel usage.\"}]}
}")
S_DLLM=$(json_val "id" "$resp")
sleep 0.05
post "/spans/$S_DLLM/complete" '{"output":{"findings":["Request body is cloned for logging but never dropped","Unbounded channel grows when downstream is slow","Arc<Vec<u8>> buffers accumulate in trace collector"],"severity":"high"}}'>/dev/null
log "  ├─ analyze-memory-patterns (completed)"

# This span is intentionally left running
resp=$(post "/spans" "{
  \"trace_id\":\"$T5\",
  \"parent_id\":\"$S_DEBUG\",
  \"name\":\"applying-fix\",
  \"kind\":{\"type\":\"fs_write\",\"path\":\"crates/proxy/src/lib.rs\",\"file_version\":\"sha256:d4e6f7\",\"bytes_written\":0},
  \"input\":{\"planned_changes\":[\"Use Bytes instead of Vec<u8>\",\"Add bounded channel with backpressure\",\"Drop request body after trace extraction\"]}
}")
S_DFIX=$(json_val "id" "$resp")
log "  └─ applying-fix (STILL RUNNING)"
log "Trace 5 left with running spans ✓"

# ── Summary ───────────────────────────────────────────────────────────

echo ""
echo -e "${Y}═══ Done ═══${R}"
echo ""
echo "Created 5 traces with ~35 spans total:"
echo "  1. RAG Chat Agent         — 9 spans, retrieval + synthesis"
echo "  2. Coding Agent           — 9 spans, failed build + retry"
echo "  3. Eval Pipeline          — 13 spans, 3 models × 3 samples"
echo "  4. Data Pipeline (failed) — 5 spans, S3 permission error"
echo "  5. Live Debug (running)   — 4 spans, 1 still in progress"
echo ""
echo -e "View at: ${C}${URL}${R}"
