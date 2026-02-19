"""Complex SDK test: multi-span traces, nested spans, filtering, analytics, export."""

import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from llmtrace import LLMTrace, CustomKind, LlmCallKind, FsReadKind, FsWriteKind

BASE = os.environ.get("LLMTRACE_URL", "http://localhost:3000")


def test_multi_span_trace_with_kinds():
    """Simulate a realistic agent trace: read files, call LLM, write output."""
    client = LLMTrace(BASE)
    client.clear_all()

    with client.trace("agent-run") as t:
        trace_id = t.trace_id

        # 1. Read a config file
        with t.span("read-config", kind=FsReadKind(path="/config.yaml", bytes_read=1024)) as s:
            s.set_output({"config": {"model": "gpt-4", "temperature": 0.7}})

        # 2. Read source code
        with t.span("read-source", kind=FsReadKind(path="/src/main.py", bytes_read=4096)) as s:
            s.set_output({"lines": 150})

        # 3. LLM call with model info
        with t.llm_call("inference", model="gpt-4o", provider="openai") as s:
            # Simulate LLM response
            time.sleep(0.05)
            s.set_output({
                "response": "Here is the refactored code...",
                "tokens": {"input": 2000, "output": 500},
            })

        # 4. Write output file
        with t.span("write-output", kind=FsWriteKind(
            path="/src/main_refactored.py",
            file_version="sha256:abc123",
            bytes_written=3500,
        )) as s:
            s.set_output({"written": True})

        # 5. A custom span
        with t.span("post-process", kind=CustomKind(kind="validation", attributes={"linter": "ruff"})) as s:
            s.set_output({"warnings": 0, "errors": 0})

    # Verify all spans
    result = client.get_trace(trace_id)
    assert result.count == 5, f"Expected 5 spans, got {result.count}"

    span_names = {s.name for s in result.spans}
    assert span_names == {"read-config", "read-source", "inference", "write-output", "post-process"}

    # All completed
    for span in result.spans:
        assert span.status == "completed", f"Span {span.name} is {span.status}"

    # Check kind parsing
    for span in result.spans:
        if span.name == "read-config":
            assert isinstance(span.kind, FsReadKind)
            assert span.kind.path == "/config.yaml"
            assert span.kind.bytes_read == 1024
        elif span.name == "inference":
            assert isinstance(span.kind, LlmCallKind)
            assert span.kind.model == "gpt-4o"
            assert span.kind.provider == "openai"
        elif span.name == "write-output":
            assert isinstance(span.kind, FsWriteKind)
            assert span.kind.path == "/src/main_refactored.py"
            assert span.kind.bytes_written == 3500

    print("PASS: test_multi_span_trace_with_kinds")
    return trace_id


def test_nested_spans():
    """Test parent-child span relationships."""
    client = LLMTrace(BASE)

    with client.trace("nested-test") as t:
        trace_id = t.trace_id

        # Root span
        root = client.start_span(
            trace_id=trace_id,
            name="root-task",
            kind=CustomKind(kind="orchestrator", attributes={}),
        )

        # Child 1
        child1 = client.start_span(
            trace_id=trace_id,
            parent_id=root.id,
            name="child-1",
            kind=CustomKind(kind="subtask", attributes={}),
            input={"step": 1},
        )
        client.complete_span(child1.id, output={"done": True})

        # Child 2
        child2 = client.start_span(
            trace_id=trace_id,
            parent_id=root.id,
            name="child-2",
            kind=CustomKind(kind="subtask", attributes={}),
            input={"step": 2},
        )
        client.complete_span(child2.id, output={"done": True})

        # Complete root
        client.complete_span(root.id, output={"children_completed": 2})

    result = client.get_trace(trace_id)
    assert result.count == 3

    spans_by_name = {s.name: s for s in result.spans}
    root_span = spans_by_name["root-task"]
    child1_span = spans_by_name["child-1"]
    child2_span = spans_by_name["child-2"]

    assert root_span.parent_id is None
    assert child1_span.parent_id == root_span.id
    assert child2_span.parent_id == root_span.id

    print("PASS: test_nested_spans")
    return trace_id


def test_span_filtering():
    """Test querying spans with filters."""
    client = LLMTrace(BASE)
    client.clear_all()

    # Create two traces with different span kinds
    with client.trace("trace-a") as t:
        tid_a = t.trace_id
        with t.llm_call("call-gpt4", model="gpt-4") as s:
            s.set_output({"text": "hello"})
        with t.span("read-file", kind=FsReadKind(path="/a.txt", bytes_read=100)) as s:
            s.set_output({})

    with client.trace("trace-b") as t:
        tid_b = t.trace_id
        with t.llm_call("call-claude", model="claude-3") as s:
            s.set_output({"text": "hi"})

    # Filter by kind
    llm_spans = client.get_spans(kind="llm_call")
    assert llm_spans.count == 2, f"Expected 2 LLM spans, got {llm_spans.count}"

    fs_spans = client.get_spans(kind="fs_read")
    assert fs_spans.count == 1, f"Expected 1 fs_read span, got {fs_spans.count}"

    # Filter by trace
    trace_a_spans = client.get_spans(trace_id=tid_a)
    assert trace_a_spans.count == 2, f"Expected 2 spans in trace A, got {trace_a_spans.count}"

    # Filter by name
    gpt_spans = client.get_spans(name_contains="gpt4")
    assert gpt_spans.count == 1

    # Filter by status
    completed = client.get_spans(status="completed")
    assert completed.count == 3

    print("PASS: test_span_filtering")


def test_export_and_stats():
    """Test data export and stats endpoints."""
    client = LLMTrace(BASE)
    client.clear_all()

    # Create some data
    with client.trace("export-test") as t:
        trace_id = t.trace_id
        with t.span("s1", kind=CustomKind(kind="task", attributes={})) as s:
            s.set_output({"v": 1})
        with t.span("s2", kind=CustomKind(kind="task", attributes={})) as s:
            s.set_output({"v": 2})

    # Stats
    stats = client.get_stats()
    assert stats.trace_count >= 1
    assert stats.span_count >= 2

    # Export all
    export = client.export_json()
    assert trace_id in export.traces
    assert len(export.traces[trace_id]) == 2

    # Export single trace
    export_single = client.export_json(trace_id=trace_id)
    assert trace_id in export_single.traces

    print("PASS: test_export_and_stats")


def test_analytics_summary():
    """Test the analytics summary endpoint."""
    import httpx

    client = LLMTrace(BASE)
    client.clear_all()

    with client.trace("analytics-test") as t:
        with t.llm_call("call-1", model="gpt-4", provider="openai") as s:
            s.set_output({"text": "response"})
        with t.llm_call("call-2", model="claude-3", provider="anthropic") as s:
            s.set_output({"text": "response2"})
        with t.span("read", kind=FsReadKind(path="/f.txt", bytes_read=50)) as s:
            s.set_output({})

    # Call analytics summary via raw HTTP (SDK doesn't wrap this yet)
    resp = httpx.get(f"{BASE}/api/analytics/summary")
    assert resp.status_code == 200
    data = resp.json()
    assert data["total_spans"] >= 3
    assert data["total_llm_calls"] >= 2
    assert len(data["models_used"]) >= 2
    assert "gpt-4" in data["models_used"]
    assert "claude-3" in data["models_used"]

    # Flexible analytics query
    resp = httpx.post(f"{BASE}/api/analytics", json={
        "metrics": ["span_count", "total_tokens"],
        "group_by": ["model"],
        "filter": {"kind": "llm_call"},
    })
    assert resp.status_code == 200
    analytics = resp.json()
    assert "groups" in analytics
    assert "totals" in analytics

    print("PASS: test_analytics_summary")


def test_trace_lifecycle():
    """Test full CRUD: create, read, delete, verify gone."""
    client = LLMTrace(BASE)

    # Create
    with client.trace("lifecycle-test") as t:
        trace_id = t.trace_id
        with t.span("temp", kind=CustomKind(kind="task", attributes={})) as s:
            s.set_output({})

    # Read
    result = client.get_trace(trace_id)
    assert result.count == 1

    # Get individual span
    span = client.get_span(result.spans[0].id)
    assert span.name == "temp"

    # Delete
    client.delete_trace(trace_id)

    # Verify gone
    try:
        client.get_trace(trace_id)
        assert False, "Should have raised for deleted trace"
    except Exception:
        pass  # Expected — 404

    print("PASS: test_trace_lifecycle")


if __name__ == "__main__":
    test_multi_span_trace_with_kinds()
    test_nested_spans()
    test_span_filtering()
    test_export_and_stats()
    test_analytics_summary()
    test_trace_lifecycle()
    print("\nAll complex tests passed!")
