"""Simple SDK test: create a trace, add a span, complete it, verify."""

import os
import sys

# Allow running without installing: add the SDK to sys.path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from llmtrace import LLMTrace, CustomKind

BASE = os.environ.get("LLMTRACE_URL", "http://localhost:3000")


def test_create_and_complete_span():
    """Round-trip: create trace → create span → complete → verify."""
    client = LLMTrace(BASE)

    # Clean slate
    client.clear_all()

    # Create a trace
    with client.trace("simple-test") as t:
        trace_id = t.trace_id
        assert trace_id, "trace_id should be set"

        # Create a span inside the trace
        with t.span("step-1", kind=CustomKind(kind="task", attributes={"key": "val"})) as s:
            s.set_output({"answer": 42})

    # Verify trace exists
    traces = client.get_traces()
    assert traces.count >= 1
    found = any(tr.id == trace_id for tr in traces.traces)
    assert found, f"Trace {trace_id} not in trace list"

    # Verify span
    span_list = client.get_trace(trace_id)
    assert span_list.count == 1
    span = span_list.spans[0]
    assert span.name == "step-1"
    assert span.status == "completed"
    assert span.output == {"answer": 42}
    assert isinstance(span.kind, CustomKind)
    assert span.kind.kind == "task"

    # Verify stats
    stats = client.get_stats()
    assert stats.span_count >= 1

    # Cleanup
    client.delete_trace(trace_id)
    client.close()
    print("PASS: test_create_and_complete_span")


def test_fail_span():
    """A span that raises should be marked as failed."""
    client = LLMTrace(BASE)

    with client.trace("fail-test") as t:
        trace_id = t.trace_id
        try:
            with t.span("will-fail", kind=CustomKind(kind="task", attributes={})) as s:
                raise ValueError("something broke")
        except ValueError:
            pass  # expected

    span_list = client.get_trace(trace_id)
    assert span_list.count == 1
    span = span_list.spans[0]
    assert span.status == "failed"
    assert span.error == "something broke"

    client.delete_trace(trace_id)
    client.close()
    print("PASS: test_fail_span")


if __name__ == "__main__":
    test_create_and_complete_span()
    test_fail_span()
    print("\nAll simple tests passed!")
