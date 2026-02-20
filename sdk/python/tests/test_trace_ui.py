"""
Comprehensive UI stress test: populates the daemon with realistic, deeply nested
traces that exercise every feature of the trace-detail view.

Creates 4 traces:
  1. coding-agent session   – 22 spans, 4 nesting levels, LLM chat messages,
                              file reads/writes, a failed cargo-check + retry
  2. RAG pipeline           – embedding, vector search, file reads, LLM answer
  3. eval pipeline          – 3 samples x 2 models + scoring, heavy nesting
  4. live debug session     – intentionally leaves a span running

Run against a live daemon:
    python tests/test_trace_ui.py          # localhost:3000
    TRACEWAY_URL=http://host:port python tests/test_trace_ui.py
"""

import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from traceway import Traceway
from traceway.types import LlmCallKind, FsReadKind, FsWriteKind, CustomKind

BASE = os.environ.get("TRACEWAY_URL", "http://localhost:3000")


def sleep(ms: int):
    time.sleep(ms / 1000)


# ── helpers ────────────────────────────────────────────────────────────


def _make_chat(*pairs: tuple[str, str]) -> list[dict[str, str]]:
    """Build a messages array from (role, content) tuples."""
    return [{"role": r, "content": c} for r, c in pairs]


# ── Trace 1: full coding-agent session ─────────────────────────────────


def create_coding_agent_trace(client: Traceway) -> str:
    with client.trace("coding-agent: implement user auth") as t:
        # ── root orchestrator ──────────────────────────────────────
        root = client.start_span(
            trace_id=t.trace_id,
            name="orchestrator",
            kind=CustomKind(
                kind="agent",
                attributes={"version": "2.1.0", "model": "claude-sonnet-4-20250514"},
            ),
            input={
                "task": (
                    "Implement user authentication with JWT tokens, "
                    "bcrypt password hashing, and session management"
                )
            },
        )
        sleep(50)

        # ── planning LLM call ──────────────────────────────────────
        plan = client.start_span(
            trace_id=t.trace_id,
            parent_id=root.id,
            name="planning",
            kind=LlmCallKind(
                model="claude-sonnet-4-20250514",
                provider="anthropic",
                input_tokens=2847,
                output_tokens=1523,
            ),
            input={
                "messages": _make_chat(
                    (
                        "system",
                        "You are an expert software engineer. Analyze the codebase "
                        "and create a detailed implementation plan for the requested "
                        "feature. Consider existing patterns, dependencies, and "
                        "potential edge cases.",
                    ),
                    (
                        "user",
                        "Implement user authentication with JWT tokens, bcrypt "
                        "password hashing, and session management. The app uses "
                        "Rust with axum for the backend. We need:\n"
                        "1. User model with email/password\n"
                        "2. Signup/login endpoints\n"
                        "3. JWT middleware for protected routes\n"
                        "4. Session cookie handling\n\n"
                        "Here's the current project structure:\n"
                        "- crates/api/src/lib.rs (main router)\n"
                        "- crates/auth/ (auth crate, mostly empty)\n"
                        "- crates/storage-postgres/ (database layer)",
                    ),
                    (
                        "assistant",
                        "I'll implement the authentication system in 4 phases:\n\n"
                        "## Phase 1: User Model & Storage\n"
                        "- Add `User` struct with bcrypt helpers\n"
                        "- Create `AuthStore` trait\n"
                        "- Implement `PostgresAuthStore`\n\n"
                        "## Phase 2: Auth Context & Middleware\n"
                        "- `AuthContext` with user info, org, scopes\n"
                        "- JWT generation / verification\n"
                        "- Tower middleware layer\n\n"
                        "## Phase 3: Route Handlers\n"
                        "- POST /auth/signup, /auth/login, /auth/logout\n\n"
                        "## Phase 4: Protected Routes\n"
                        "- `auth::Auth(ctx)` extractor + scope checks on all "
                        "handlers\n\nLet me start with Phase 1.",
                    ),
                )
            },
        )
        sleep(320)
        client.complete_span(
            plan.id,
            output={
                "messages": _make_chat(
                    (
                        "assistant",
                        "Plan confirmed — 4 phases, starting with Phase 1.",
                    )
                ),
                "plan_steps": 4,
            },
        )

        # ── read existing code (3 files) ──────────────────────────
        reads = [
            ("read auth/src/lib.rs", "/crates/auth/src/lib.rs", 4521),
            ("read storage-postgres/src/lib.rs", "/crates/storage-postgres/src/lib.rs", 8934),
            ("read api/src/lib.rs", "/crates/api/src/lib.rs", 12450),
        ]
        for name, path, size in reads:
            s = client.start_span(
                trace_id=t.trace_id,
                parent_id=root.id,
                name=name,
                kind=FsReadKind(path=path, bytes_read=size),
                input={"purpose": f"understand {path.split('/')[-1]}"},
            )
            sleep(15)
            client.complete_span(s.id, output={"lines": size // 30})

        # ── implementation phase ──────────────────────────────────
        impl_phase = client.start_span(
            trace_id=t.trace_id,
            parent_id=root.id,
            name="implement-auth-system",
            kind=CustomKind(kind="task", attributes={"phase": "implementation"}),
        )
        sleep(30)

        # codegen: user model
        cg1 = client.start_span(
            trace_id=t.trace_id,
            parent_id=impl_phase.id,
            name="generate user model",
            kind=LlmCallKind(
                model="claude-sonnet-4-20250514",
                provider="anthropic",
                input_tokens=4215,
                output_tokens=2890,
            ),
            input={
                "messages": _make_chat(
                    (
                        "system",
                        "You are writing Rust code. Generate production-quality "
                        "code with proper error handling, documentation, and tests.",
                    ),
                    (
                        "user",
                        "Write the User struct with bcrypt password hashing. "
                        "Add the AuthStore trait definition. Include proper error types.",
                    ),
                )
            },
        )
        sleep(450)
        client.complete_span(
            cg1.id,
            output={
                "messages": _make_chat(
                    (
                        "assistant",
                        "```rust\nuse bcrypt::{hash, verify, DEFAULT_COST};\n\n"
                        "#[derive(Debug, Clone)]\npub struct User {\n"
                        "    pub id: UserId,\n    pub email: String,\n"
                        "    pub password_hash: String,\n    pub role: Role,\n"
                        "}\n\nimpl User {\n"
                        "    pub fn with_password(email: &str, pw: &str)"
                        " -> Result<Self, AuthError> {\n"
                        "        let hash = hash(pw.as_bytes(), DEFAULT_COST)?;\n"
                        "        Ok(Self { ... })\n    }\n\n"
                        "    pub fn verify_password(&self, pw: &str) -> bool {\n"
                        "        verify(pw.as_bytes(), &self.password_hash)"
                        ".unwrap_or(false)\n    }\n}\n```",
                    )
                ),
            },
        )

        # write: user model
        w1 = client.start_span(
            trace_id=t.trace_id,
            parent_id=impl_phase.id,
            name="write auth/src/lib.rs",
            kind=FsWriteKind(
                path="/crates/auth/src/lib.rs",
                file_version="a3f8b2c1",
                bytes_written=6789,
            ),
        )
        sleep(12)
        client.complete_span(w1.id, output={"lines_written": 210})

        # codegen: middleware
        cg2 = client.start_span(
            trace_id=t.trace_id,
            parent_id=impl_phase.id,
            name="generate auth middleware",
            kind=LlmCallKind(
                model="claude-sonnet-4-20250514",
                provider="anthropic",
                input_tokens=6120,
                output_tokens=3845,
            ),
            input={
                "messages": _make_chat(
                    (
                        "system",
                        "You are writing Rust code. Generate production-quality code.",
                    ),
                    (
                        "user",
                        "Implement the Tower middleware layer for auth. Extract "
                        "Bearer token, session cookie, and API key. Verify JWT "
                        "and inject AuthContext.",
                    ),
                )
            },
        )
        sleep(580)
        client.complete_span(
            cg2.id,
            output={
                "messages": _make_chat(
                    (
                        "assistant",
                        "```rust\npub struct AuthLayer { config: AuthConfig }\n\n"
                        "impl<S> Layer<S> for AuthLayer {\n"
                        "    type Service = AuthMiddleware<S>;\n"
                        "    fn layer(&self, inner: S) -> Self::Service { ... }\n"
                        "}\n```",
                    )
                ),
            },
        )

        # write: middleware
        w2 = client.start_span(
            trace_id=t.trace_id,
            parent_id=impl_phase.id,
            name="write auth/src/middleware.rs",
            kind=FsWriteKind(
                path="/crates/auth/src/middleware.rs",
                file_version="d7e2f411",
                bytes_written=5234,
            ),
        )
        sleep(10)
        client.complete_span(w2.id, output={"lines_written": 168})

        # codegen: route handlers
        cg3 = client.start_span(
            trace_id=t.trace_id,
            parent_id=impl_phase.id,
            name="generate auth routes",
            kind=LlmCallKind(
                model="claude-sonnet-4-20250514",
                provider="anthropic",
                input_tokens=5580,
                output_tokens=4120,
            ),
            input={
                "messages": _make_chat(
                    (
                        "user",
                        "Write signup, login, and logout handlers for axum.",
                    ),
                )
            },
        )
        sleep(620)
        client.complete_span(
            cg3.id,
            output={
                "messages": _make_chat(
                    (
                        "assistant",
                        "```rust\npub async fn signup(\n"
                        "    State(state): State<AppState>,\n"
                        "    Json(req): Json<SignupRequest>,\n"
                        ") -> Result<Json<AuthResponse>, ApiError> { ... }\n```",
                    )
                ),
            },
        )

        # write: routes + update router
        for name, path, ver, size in [
            ("write api/src/auth_routes.rs", "/crates/api/src/auth_routes.rs", "91bc3e55", 7890),
            ("update api/src/lib.rs", "/crates/api/src/lib.rs", "c4f71a22", 13200),
        ]:
            w = client.start_span(
                trace_id=t.trace_id,
                parent_id=impl_phase.id,
                name=name,
                kind=FsWriteKind(path=path, file_version=ver, bytes_written=size),
            )
            sleep(14)
            client.complete_span(w.id, output={"lines_written": size // 28})

        client.complete_span(
            impl_phase.id, output={"files_written": 4, "total_lines": 1101}
        )

        # ── verification phase ────────────────────────────────────
        verify = client.start_span(
            trace_id=t.trace_id,
            parent_id=root.id,
            name="verification",
            kind=CustomKind(kind="task", attributes={"phase": "verify"}),
        )
        sleep(20)

        # cargo check — success
        cc1 = client.start_span(
            trace_id=t.trace_id,
            parent_id=verify.id,
            name="cargo check",
            kind=CustomKind(
                kind="tool_call",
                attributes={"tool": "bash", "command": "cargo check -p daemon"},
            ),
            input={"command": "cargo check -p daemon"},
        )
        sleep(1200)
        client.complete_span(
            cc1.id,
            output={
                "exit_code": 0,
                "stderr": (
                    "   Compiling auth v0.1.0\n"
                    "   Compiling api v0.1.0\n"
                    "   Compiling daemon v0.1.0\n"
                    "    Finished dev target(s) in 8.45s"
                ),
            },
        )

        # cargo check --features cloud — FAILS
        cc2 = client.start_span(
            trace_id=t.trace_id,
            parent_id=verify.id,
            name="cargo check --features cloud",
            kind=CustomKind(
                kind="tool_call",
                attributes={
                    "tool": "bash",
                    "command": "cargo check --features cloud -p daemon",
                },
            ),
            input={"command": "cargo check --features cloud -p daemon"},
        )
        sleep(800)
        client.fail_span(
            cc2.id,
            "error[E0277]: the trait bound `dyn AuthStore: Send` is not "
            "satisfied in `AppState`\n"
            "  --> crates/api/src/lib.rs:45:12\n"
            "   |\n"
            "45 |     state: AppState,\n"
            "   |            ^^^^^^^^ `dyn AuthStore` cannot be sent between "
            "threads safely",
        )

        # fix via LLM
        fix = client.start_span(
            trace_id=t.trace_id,
            parent_id=verify.id,
            name="fix Send bound error",
            kind=LlmCallKind(
                model="claude-sonnet-4-20250514",
                provider="anthropic",
                input_tokens=1890,
                output_tokens=456,
            ),
            input={
                "messages": _make_chat(
                    (
                        "user",
                        "Fix: error[E0277]: `dyn AuthStore: Send` not satisfied",
                    ),
                )
            },
        )
        sleep(180)
        client.complete_span(
            fix.id,
            output={
                "messages": _make_chat(
                    (
                        "assistant",
                        "Change `Arc<dyn AuthStore>` to "
                        "`Arc<dyn AuthStore + Send + Sync>`.",
                    )
                ),
            },
        )

        # apply fix
        wfix = client.start_span(
            trace_id=t.trace_id,
            parent_id=verify.id,
            name="apply fix to api/src/lib.rs",
            kind=FsWriteKind(
                path="/crates/api/src/lib.rs",
                file_version="e8a9c3b2",
                bytes_written=13250,
            ),
        )
        sleep(8)
        client.complete_span(wfix.id, output={"changed_lines": 3})

        # retry cargo check — success
        cc3 = client.start_span(
            trace_id=t.trace_id,
            parent_id=verify.id,
            name="cargo check --features cloud (retry)",
            kind=CustomKind(
                kind="tool_call",
                attributes={
                    "tool": "bash",
                    "command": "cargo check --features cloud -p daemon",
                },
            ),
            input={"command": "cargo check --features cloud -p daemon"},
        )
        sleep(950)
        client.complete_span(cc3.id, output={"exit_code": 0})

        client.complete_span(verify.id, output={"checks_passed": True, "retries": 1})

        # ── summary LLM call ──────────────────────────────────────
        summary = client.start_span(
            trace_id=t.trace_id,
            parent_id=root.id,
            name="generate summary",
            kind=LlmCallKind(
                model="claude-sonnet-4-20250514",
                provider="anthropic",
                input_tokens=3200,
                output_tokens=890,
            ),
            input={"messages": _make_chat(("user", "Summarize what was accomplished."))},
        )
        sleep(240)
        client.complete_span(
            summary.id,
            output={
                "messages": _make_chat(
                    (
                        "assistant",
                        "## Summary\n\n"
                        "Implemented complete user authentication system:\n\n"
                        "### Files Modified\n"
                        "- `crates/auth/src/lib.rs` — User model + bcrypt\n"
                        "- `crates/auth/src/middleware.rs` — Tower auth middleware\n"
                        "- `crates/api/src/auth_routes.rs` — Signup/login/logout\n"
                        "- `crates/api/src/lib.rs` — Router integration\n\n"
                        "### Verification\n"
                        "- `cargo check -p daemon` passes\n"
                        "- `cargo check --features cloud -p daemon` passes "
                        "(after fixing Send bound)",
                    )
                )
            },
        )

        sleep(50)
        client.complete_span(
            root.id,
            output={
                "status": "success",
                "files_modified": 4,
                "llm_calls": 5,
                "total_tokens": {"input": 23852, "output": 13724},
                "total_cost": 0.1847,
            },
        )

    return t.trace_id


# ── Trace 2: RAG pipeline ──────────────────────────────────────────────


def create_rag_trace(client: Traceway) -> str:
    with client.trace("rag-pipeline: answer question") as t:
        root = client.start_span(
            trace_id=t.trace_id,
            name="rag-pipeline",
            kind=CustomKind(kind="pipeline", attributes={"type": "rag"}),
            input={
                "question": "How does the auth middleware extract tokens from requests?"
            },
        )
        sleep(10)

        # embed
        embed = client.start_span(
            trace_id=t.trace_id,
            parent_id=root.id,
            name="embed-query",
            kind=LlmCallKind(
                model="text-embedding-3-small",
                provider="openai",
                input_tokens=24,
                output_tokens=0,
            ),
            input={
                "text": "How does the auth middleware extract tokens from requests?"
            },
        )
        sleep(45)
        client.complete_span(
            embed.id, output={"dimensions": 1536, "embedding": "[0.023, -0.041, ...]"}
        )

        # vector search
        search = client.start_span(
            trace_id=t.trace_id,
            parent_id=root.id,
            name="vector-search",
            kind=CustomKind(
                kind="tool_call", attributes={"tool": "turbopuffer", "top_k": 5}
            ),
            input={"vector": "[0.023, ...]", "top_k": 5},
        )
        sleep(30)
        client.complete_span(
            search.id,
            output={
                "results": [
                    {"path": "middleware.rs", "score": 0.92},
                    {"path": "context.rs", "score": 0.87},
                    {"path": "lib.rs", "score": 0.78},
                ],
                "total_matches": 5,
            },
        )

        # read retrieved docs
        for i, doc in enumerate(["middleware.rs", "context.rs", "lib.rs"]):
            r = client.start_span(
                trace_id=t.trace_id,
                parent_id=root.id,
                name=f"read {doc}",
                kind=FsReadKind(
                    path=f"/crates/auth/src/{doc}", bytes_read=3200 + i * 1100
                ),
            )
            sleep(8)
            client.complete_span(r.id)

        # answer generation
        answer = client.start_span(
            trace_id=t.trace_id,
            parent_id=root.id,
            name="generate-answer",
            kind=LlmCallKind(
                model="gpt-4o",
                provider="openai",
                input_tokens=4520,
                output_tokens=1280,
            ),
            input={
                "messages": _make_chat(
                    (
                        "system",
                        "You are a helpful documentation assistant. Answer "
                        "questions based on the provided source code context.",
                    ),
                    (
                        "user",
                        "Based on the source code, explain how the auth middleware "
                        "extracts tokens from requests.\n\n"
                        "### middleware.rs\n```rust\n"
                        "impl<S, B> Service<Request<B>> for AuthMiddleware<S> {\n"
                        "    fn call(&mut self, req: Request<B>) -> Self::Future {\n"
                        "        let bearer = req.headers().get(AUTHORIZATION)...;\n"
                        "        let cookie = extract_cookie(...);\n"
                        "        let api_key = req.uri().query()...;\n"
                        "    }\n}\n```",
                    ),
                )
            },
        )
        sleep(380)
        client.complete_span(
            answer.id,
            output={
                "messages": _make_chat(
                    (
                        "assistant",
                        "The auth middleware extracts tokens from three sources:\n\n"
                        "1. **Bearer Token** — `Authorization` header\n"
                        "2. **Session Cookie** — `traceway_session` cookie\n"
                        "3. **API Key** — `api_key` query parameter\n\n"
                        "The first successful extraction wins.",
                    )
                )
            },
        )

        sleep(10)
        client.complete_span(root.id, output={"sources": 3})

    return t.trace_id


# ── Trace 3: evaluation pipeline ───────────────────────────────────────


def create_eval_trace(client: Traceway) -> str:
    with client.trace("eval-pipeline: model comparison") as t:
        eval_root = client.start_span(
            trace_id=t.trace_id,
            name="evaluation-runner",
            kind=CustomKind(
                kind="pipeline",
                attributes={
                    "type": "evaluation",
                    "dataset": "auth-qa-v2",
                    "models": ["gpt-4o", "claude-sonnet-4-20250514"],
                },
            ),
            input={"dataset_id": "ds_auth_qa_v2", "num_samples": 3},
        )
        sleep(10)

        # load dataset
        load = client.start_span(
            trace_id=t.trace_id,
            parent_id=eval_root.id,
            name="load-dataset",
            kind=CustomKind(kind="tool_call", attributes={"tool": "dataset_store"}),
            input={"dataset_id": "ds_auth_qa_v2"},
        )
        sleep(25)
        client.complete_span(
            load.id,
            output={"samples": 3, "columns": ["question", "expected_answer", "context"]},
        )

        questions = [
            "What hashing algorithm is used for passwords?",
            "How are JWT tokens validated?",
            "What scopes are available for API keys?",
        ]

        for qi, question in enumerate(questions):
            sample = client.start_span(
                trace_id=t.trace_id,
                parent_id=eval_root.id,
                name=f"sample-{qi + 1}",
                kind=CustomKind(kind="eval_sample", attributes={"index": qi}),
                input={"question": question},
            )
            sleep(5)

            for model, provider, inp, out, dur in [
                ("gpt-4o", "openai", 1890, 620, 280),
                ("claude-sonnet-4-20250514", "anthropic", 1890, 710, 340),
            ]:
                m = client.start_span(
                    trace_id=t.trace_id,
                    parent_id=sample.id,
                    name=f"inference ({model.split('-')[0]})",
                    kind=LlmCallKind(
                        model=model,
                        provider=provider,
                        input_tokens=inp,
                        output_tokens=out,
                    ),
                    input={
                        "messages": _make_chat(("user", question)),
                    },
                )
                sleep(dur)
                client.complete_span(
                    m.id,
                    output={
                        "messages": _make_chat(
                            ("assistant", f"Answer from {model}: {question[:40]}...")
                        )
                    },
                )

            # scoring
            score = client.start_span(
                trace_id=t.trace_id,
                parent_id=sample.id,
                name="score-answers",
                kind=LlmCallKind(
                    model="gpt-4o-mini",
                    provider="openai",
                    input_tokens=2100,
                    output_tokens=180,
                ),
                input={"answers": 2, "rubric": "accuracy, completeness, clarity"},
            )
            sleep(120)
            scores = {
                "gpt-4o": round(0.88 - qi * 0.05, 2),
                "claude-sonnet-4-20250514": round(0.91 - qi * 0.03, 2),
            }
            client.complete_span(score.id, output={"scores": scores})
            client.complete_span(sample.id, output={"scores": scores})

        sleep(10)
        client.complete_span(
            eval_root.id,
            output={
                "summary": {
                    "gpt-4o": {"avg_score": 0.83, "total_cost": 0.0426},
                    "claude-sonnet-4-20250514": {"avg_score": 0.88, "total_cost": 0.0294},
                },
                "winner": "claude-sonnet-4-20250514",
            },
        )

    return t.trace_id


# ── Trace 4: live debug session (running span) ────────────────────────


def create_debug_trace(client: Traceway) -> str:
    trace = client.create_trace(name="debug-session: live investigation")

    root = client.start_span(
        trace_id=trace.id,
        name="investigate-bug",
        kind=CustomKind(
            kind="task", attributes={"issue": "AUTH-142", "priority": "high"}
        ),
        input={"description": "Users report 401 errors after token refresh"},
    )
    sleep(100)

    logs = client.start_span(
        trace_id=trace.id,
        parent_id=root.id,
        name="read server logs",
        kind=FsReadKind(path="/var/log/traceway/server.log", bytes_read=45600),
    )
    sleep(30)
    client.complete_span(
        logs.id,
        output={"relevant_lines": 12, "pattern": "JWT validation failed: token expired"},
    )

    # intentionally leave the LLM analysis span running
    client.start_span(
        trace_id=trace.id,
        parent_id=root.id,
        name="analyze logs",
        kind=LlmCallKind(
            model="claude-sonnet-4-20250514",
            provider="anthropic",
            input_tokens=3400,
            output_tokens=890,
        ),
        input={
            "messages": _make_chat(
                ("system", "You are debugging a production issue."),
                (
                    "user",
                    "Analyze these server logs. Users get 401 after token refresh. "
                    "Relevant log lines show 'JWT validation failed: token expired' "
                    "but the token was just refreshed 2 seconds ago.",
                ),
            )
        },
    )
    sleep(50)
    # NOTE: not completing — left running on purpose

    return trace.id


# ── main ───────────────────────────────────────────────────────────────


def populate_all():
    """Create all four test traces and return their IDs."""
    client = Traceway(BASE)

    ids: dict[str, str] = {}

    ids["coding_agent"] = create_coding_agent_trace(client)
    print(f"  Trace 1 (coding-agent)  : {ids['coding_agent']}")

    ids["rag"] = create_rag_trace(client)
    print(f"  Trace 2 (rag-pipeline)  : {ids['rag']}")

    ids["eval"] = create_eval_trace(client)
    print(f"  Trace 3 (eval-pipeline) : {ids['eval']}")

    ids["debug"] = create_debug_trace(client)
    print(f"  Trace 4 (debug-session) : {ids['debug']}")

    client.close()
    return ids


def verify_traces(ids: dict[str, str]):
    """Quick sanity checks on the created traces."""
    client = Traceway(BASE)

    # Trace 1: should have 20 spans
    t1 = client.get_trace(ids["coding_agent"])
    assert t1.count == 20, f"coding-agent: expected 20 spans, got {t1.count}"
    # Should have one failed span
    failed = [s for s in t1.spans if s.status == "failed"]
    assert len(failed) == 1, f"coding-agent: expected 1 failed span, got {len(failed)}"
    assert "E0277" in (failed[0].error or ""), "failed span should have compile error"

    # Trace 2: should have 7 spans
    t2 = client.get_trace(ids["rag"])
    assert t2.count == 7, f"rag-pipeline: expected 7 spans, got {t2.count}"

    # Trace 3: should have 14 spans (1 root + 1 load + 3*(1 sample + 2 infer + 1 score))
    t3 = client.get_trace(ids["eval"])
    assert t3.count == 14, f"eval-pipeline: expected 14 spans, got {t3.count}"

    # Trace 4: should have 3 spans, one still running
    t4 = client.get_trace(ids["debug"])
    assert t4.count == 3, f"debug-session: expected 3 spans, got {t4.count}"
    running = [s for s in t4.spans if s.status == "running"]
    assert len(running) >= 1, "debug-session: expected at least 1 running span"

    client.close()
    print("  All assertions passed.")


if __name__ == "__main__":
    print("Creating test traces...")
    ids = populate_all()
    print("\nVerifying...")
    verify_traces(ids)
    print(f"\nDone! Open {BASE}/traces to see the results.")
