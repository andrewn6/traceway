from __future__ import annotations

import os
import uuid
from contextlib import contextmanager
from typing import Any, Generator
from urllib.parse import quote

import httpx

from .types import (
    CreatedSpan,
    ExportData,
    FileVersion,
    LlmCallKind,
    Span,
    SpanFilter,
    SpanKind,
    SpanList,
    SpanMetadata,
    Stats,
    Trace,
    TrackedFile,
    TraceList,
    span_kind_to_dict,
)


class SpanContext:
    """Context manager for a span that auto-completes on exit or fails on exception."""

    def __init__(self, client: "LLMTrace", span_id: str, trace_id: str):
        self._client = client
        self._span_id = span_id
        self._trace_id = trace_id
        self._output: Any = None

    @property
    def span_id(self) -> str:
        return self._span_id

    @property
    def trace_id(self) -> str:
        return self._trace_id

    def set_output(self, output: Any) -> None:
        self._output = output


class TraceContext:
    """Context manager for a trace that groups all operations under one trace ID."""

    def __init__(self, client: "LLMTrace", trace_id: str):
        self._client = client
        self._trace_id = trace_id

    @property
    def trace_id(self) -> str:
        return self._trace_id

    @contextmanager
    def span(
        self,
        name: str,
        *,
        kind: SpanKind | None = None,
        parent_id: str | None = None,
        input: Any = None,
    ) -> Generator[SpanContext, None, None]:
        """Create a span within this trace."""
        created = self._client.start_span(
            trace_id=self._trace_id,
            parent_id=parent_id,
            name=name,
            kind=kind,
            input=input,
        )
        ctx = SpanContext(self._client, created.id, self._trace_id)
        try:
            yield ctx
        except Exception as e:
            self._client.fail_span(created.id, str(e))
            raise
        else:
            self._client.complete_span(created.id, output=ctx._output)

    @contextmanager
    def llm_call(
        self,
        name: str = "llm_call",
        *,
        model: str,
        provider: str | None = None,
        parent_id: str | None = None,
        input: Any = None,
    ) -> Generator[SpanContext, None, None]:
        """Convenience: create an LlmCall span within this trace."""
        kind = LlmCallKind(model=model, provider=provider)
        with self.span(name, kind=kind, parent_id=parent_id, input=input) as ctx:
            yield ctx


class LLMTrace:
    """Client for the llmtrace daemon API."""

    def __init__(
        self,
        url: str | None = None,
        api_key: str | None = None,
    ):
        """Initialize the LLMTrace client.
        
        Args:
            url: Base URL of the llmtrace server. Defaults to LLMTRACE_URL env var
                 or http://localhost:3000
            api_key: API key for authentication. Defaults to LLMTRACE_API_KEY env var.
                     Required for cloud deployments.
        """
        self._base_url = (
            url or os.environ.get("LLMTRACE_URL") or "http://localhost:3000"
        ).rstrip("/")
        
        self._api_key = api_key or os.environ.get("LLMTRACE_API_KEY")
        
        headers = {}
        if self._api_key:
            headers["Authorization"] = f"Bearer {self._api_key}"
        
        self._client = httpx.Client(base_url=self._base_url, headers=headers)

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> "LLMTrace":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    # ─── Internal helpers ─────────────────────────────────────────────

    def _request(self, method: str, path: str, **kwargs: Any) -> Any:
        resp = self._client.request(method, f"/api{path}", **kwargs)
        resp.raise_for_status()
        if resp.content:
            return resp.json()
        return None

    def _request_text(self, method: str, path: str, **kwargs: Any) -> str:
        resp = self._client.request(method, f"/api{path}", **kwargs)
        resp.raise_for_status()
        return resp.text

    def _qs(self, params: dict[str, str | None]) -> dict[str, str]:
        return {k: v for k, v in params.items() if v is not None}

    # ─── Trace operations ─────────────────────────────────────────────

    def create_trace(self, name: str | None = None, tags: list[str] | None = None) -> Trace:
        data: dict[str, Any] = {}
        if name is not None:
            data["name"] = name
        if tags is not None:
            data["tags"] = tags
        resp = self._request("POST", "/traces", json=data)
        return Trace.from_dict(resp)

    # ─── Span operations ──────────────────────────────────────────────

    def start_span(
        self,
        *,
        trace_id: str,
        parent_id: str | None = None,
        name: str,
        kind: SpanKind | None = None,
        input: Any = None,
        metadata: dict[str, Any] | None = None,
    ) -> CreatedSpan:
        data: dict[str, Any] = {
            "trace_id": trace_id,
            "parent_id": parent_id,
            "name": name,
        }
        if kind is not None:
            data["kind"] = span_kind_to_dict(kind)
        if input is not None:
            data["input"] = input
        if metadata is not None:
            data["metadata"] = metadata
        resp = self._request("POST", "/spans", json=data)
        return CreatedSpan.from_dict(resp)

    def complete_span(self, span_id: str, *, output: Any = None) -> None:
        data: dict[str, Any] = {}
        if output is not None:
            data["output"] = output
        self._request("POST", f"/spans/{span_id}/complete", json=data if data else None)

    def fail_span(self, span_id: str, error: str) -> None:
        self._request("POST", f"/spans/{span_id}/fail", json={"error": error})

    # ─── Read operations ──────────────────────────────────────────────

    def get_traces(self) -> TraceList:
        resp = self._request("GET", "/traces")
        return TraceList.from_dict(resp)

    def get_trace(self, trace_id: str) -> SpanList:
        resp = self._request("GET", f"/traces/{trace_id}")
        return SpanList.from_dict(resp)

    def get_spans(self, **filters: str | None) -> SpanList:
        params = self._qs(filters)
        resp = self._request("GET", "/spans", params=params)
        return SpanList.from_dict(resp)

    def get_span(self, span_id: str) -> Span:
        resp = self._request("GET", f"/spans/{span_id}")
        return Span.from_dict(resp)

    def get_stats(self) -> Stats:
        resp = self._request("GET", "/stats")
        return Stats.from_dict(resp)

    # ─── File operations ──────────────────────────────────────────────

    def list_files(self, path_prefix: str | None = None) -> list[TrackedFile]:
        params = self._qs({"path_prefix": path_prefix})
        resp = self._request("GET", "/files", params=params)
        return [TrackedFile.from_dict(f) for f in resp]

    def read_file(self, path: str) -> str:
        return self._request_text("GET", f"/files/{quote(path, safe='')}")

    def file_versions(self, path: str) -> list[FileVersion]:
        resp = self._request("GET", f"/files/{quote(path, safe='')}/versions")
        return [FileVersion.from_dict(v) for v in resp]

    def file_traces(self, path: str) -> dict[str, list[dict[str, str]]]:
        return self._request("GET", f"/files/{quote(path, safe='')}/traces")

    # ─── Delete operations ────────────────────────────────────────────

    def delete_trace(self, trace_id: str) -> None:
        self._request("DELETE", f"/traces/{trace_id}")

    def delete_span(self, span_id: str) -> None:
        self._request("DELETE", f"/spans/{span_id}")

    def clear_all(self) -> None:
        self._request("DELETE", "/traces")

    # ─── Export ───────────────────────────────────────────────────────

    def export_json(self, trace_id: str | None = None) -> ExportData:
        params = self._qs({"trace_id": trace_id})
        resp = self._request("GET", "/export/json", params=params)
        return ExportData.from_dict(resp)

    # ─── Context managers ─────────────────────────────────────────────

    @contextmanager
    def trace(self, name: str = "") -> Generator[TraceContext, None, None]:
        """Create a trace context. All spans created within will share the same trace ID.

        Calls POST /api/traces to register the trace on the server.
        Sets LLMTRACE_TRACE_ID env var for subprocess propagation.

        Example:
            with client.trace("chat-completion") as t:
                config = client.read_file("config.json")
                with t.llm_call("inference", model="gpt-4o") as call:
                    result = openai.chat(...)
                    call.set_output(result)
        """
        trace = self.create_trace(name=name or None)
        trace_id = trace.id
        old_env = os.environ.get("LLMTRACE_TRACE_ID")
        os.environ["LLMTRACE_TRACE_ID"] = trace_id
        try:
            yield TraceContext(self, trace_id)
        finally:
            if old_env is not None:
                os.environ["LLMTRACE_TRACE_ID"] = old_env
            else:
                os.environ.pop("LLMTRACE_TRACE_ID", None)

    @contextmanager
    def span(
        self,
        name: str,
        *,
        trace_id: str,
        parent_id: str | None = None,
        kind: SpanKind | None = None,
        input: Any = None,
        model: str | None = None,
    ) -> Generator[SpanContext, None, None]:
        """Standalone span context manager (legacy + convenience).

        Example:
            with client.span("llm-call", trace_id=tid, model="gpt-4") as span:
                result = call_llm(...)
                span.set_output(result)
        """
        if model and kind is None:
            kind = LlmCallKind(model=model)

        created = self.start_span(
            trace_id=trace_id,
            parent_id=parent_id,
            name=name,
            kind=kind,
            input=input,
        )
        ctx = SpanContext(self, created.id, trace_id)
        try:
            yield ctx
        except Exception as e:
            self.fail_span(created.id, str(e))
            raise
        else:
            self.complete_span(created.id, output=ctx._output)
