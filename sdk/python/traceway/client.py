from __future__ import annotations

import os
import uuid
from contextlib import contextmanager
from typing import Any, Generator
from urllib.parse import quote

import httpx

from .types import (
    CreatedSpan,
    Datapoint,
    DatapointList,
    Dataset,
    DatasetList,
    ExportData,
    FileVersion,
    FsReadKind,
    FsWriteKind,
    LlmCallKind,
    QueueItem,
    QueueList,
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

    def __init__(self, client: "Traceway", span_id: str, trace_id: str, kind: SpanKind | None = None):
        self._client = client
        self._span_id = span_id
        self._trace_id = trace_id
        self._output: Any = None
        self._kind: SpanKind | None = kind

    @property
    def span_id(self) -> str:
        return self._span_id

    @property
    def trace_id(self) -> str:
        return self._trace_id

    def set_output(self, output: Any) -> None:
        self._output = output

    def set_kind(self, kind: SpanKind) -> None:
        """Update the span kind (e.g. to add token counts after an LLM call)."""
        self._kind = kind


class TraceContext:
    """Context manager for a trace that groups all operations under one trace ID."""

    def __init__(self, client: "Traceway", trace_id: str):
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
        ctx = SpanContext(self._client, created.id, self._trace_id, kind=kind)
        try:
            yield ctx
        except Exception as e:
            self._client.fail_span(created.id, str(e))
            raise
        else:
            self._client.complete_span(created.id, output=ctx._output, kind=ctx._kind)

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
        """Convenience: create an LlmCall span within this trace.
        
        After yielding, if ctx._output has 'input_tokens' and 'output_tokens' keys,
        the span kind is automatically updated with token counts.
        """
        kind = LlmCallKind(model=model, provider=provider)
        with self.span(name, kind=kind, parent_id=parent_id, input=input) as ctx:
            yield ctx
            # Auto-populate token counts from output if available
            if isinstance(ctx._output, dict):
                input_tokens = ctx._output.get("input_tokens")
                output_tokens = ctx._output.get("output_tokens")
                cost = ctx._output.get("cost")
                actual_model = ctx._output.get("model", model)
                if input_tokens is not None or output_tokens is not None:
                    ctx._kind = LlmCallKind(
                        model=actual_model,
                        provider=provider,
                        input_tokens=input_tokens,
                        output_tokens=output_tokens,
                        cost=cost,  # pass through if user provided; backend estimates if None
                    )


class Traceway:
    """Client for the Traceway daemon API."""

    def __init__(
        self,
        url: str | None = None,
        api_key: str | None = None,
        api_prefix: str | None = None,
    ):
        """Initialize the Traceway client.
        
        Args:
            url: Base URL of the Traceway server. Defaults to TRACEWAY_URL env var
                 or http://localhost:4000
            api_key: API key for authentication. Defaults to TRACEWAY_API_KEY env var.
                      Required for cloud deployments.
            api_prefix: API prefix to use for requests. Defaults to TRACEWAY_API_PREFIX
                       or "auto". In auto mode, the client tries /api first and then /
                       for compatibility across backend versions.
        """
        self._base_url = (
            url or os.environ.get("TRACEWAY_URL") or "http://localhost:4000"
        ).rstrip("/")

        env_prefix = os.environ.get("TRACEWAY_API_PREFIX")
        self._api_prefix = api_prefix if api_prefix is not None else env_prefix
        
        self._api_key = api_key or os.environ.get("TRACEWAY_API_KEY")
        
        headers = {}
        if self._api_key:
            headers["Authorization"] = f"Bearer {self._api_key}"
        
        self._client = httpx.Client(headers=headers)

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> "Traceway":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    # ─── Internal helpers ─────────────────────────────────────────────

    def _candidate_prefixes(self) -> list[str]:
        if self._api_prefix is not None and self._api_prefix != "auto":
            return [self._api_prefix]

        # If base URL already includes /api, don't prepend it again.
        if self._base_url.endswith("/api"):
            return [""]

        # Auto mode: support both legacy (/api/*) and new (/*) backends.
        return ["/api", ""]

    def _build_url(self, prefix: str, path: str) -> str:
        clean_prefix = "" if prefix in ("", "/") else (prefix if prefix.startswith("/") else f"/{prefix}")
        clean_path = path if path.startswith("/") else f"/{path}"
        return f"{self._base_url}{clean_prefix}{clean_path}"

    def _request(self, method: str, path: str, **kwargs: Any) -> Any:
        last_resp: httpx.Response | None = None
        for prefix in self._candidate_prefixes():
            resp = self._client.request(method, self._build_url(prefix, path), **kwargs)
            if resp.status_code == 404:
                last_resp = resp
                continue
            resp.raise_for_status()
            if resp.content:
                return resp.json()
            return None

        if last_resp is not None:
            last_resp.raise_for_status()
        return None

    def _request_text(self, method: str, path: str, **kwargs: Any) -> str:
        last_resp: httpx.Response | None = None
        for prefix in self._candidate_prefixes():
            resp = self._client.request(method, self._build_url(prefix, path), **kwargs)
            if resp.status_code == 404:
                last_resp = resp
                continue
            resp.raise_for_status()
            return resp.text

        if last_resp is not None:
            last_resp.raise_for_status()
        return ""

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

    def complete_span(self, span_id: str, *, output: Any = None, kind: SpanKind | None = None) -> None:
        data: dict[str, Any] = {}
        if output is not None:
            data["output"] = output
        if kind is not None:
            data["kind"] = span_kind_to_dict(kind)
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
        files = resp.get("files", []) if isinstance(resp, dict) else resp
        return [TrackedFile.from_dict(f) for f in files]

    def read_file(self, path: str) -> str:
        versions = self.file_versions(path)
        if not versions:
            raise FileNotFoundError(f"No tracked versions found for path: {path}")
        latest = versions[0]
        return self._request_text("GET", f"/files/content/{quote(latest.hash, safe='')}")

    def file_versions(self, path: str) -> list[FileVersion]:
        quoted_path = quote(path, safe='')
        try:
            resp = self._request("GET", f"/files/{quoted_path}/versions")
            versions_raw = resp if isinstance(resp, list) else resp.get("versions", [])
        except httpx.HTTPStatusError as e:
            if e.response.status_code != 404:
                raise
            resp = self._request("GET", f"/files/{quoted_path}")
            versions_raw = resp.get("versions", []) if isinstance(resp, dict) else resp

        versions = [FileVersion.from_dict(v) for v in versions_raw]
        if not versions:
            derived: list[FileVersion] = []
            for span in self.get_spans().spans:
                kind = span.kind
                if isinstance(kind, FsReadKind) and kind.path == path and kind.file_version:
                    derived.append(
                        FileVersion(
                            hash=kind.file_version,
                            path=path,
                            size=kind.bytes_read,
                            created_at=span.started_at or "",
                            created_by_span=span.id,
                            created_by_trace=span.trace_id,
                        )
                    )
                elif isinstance(kind, FsWriteKind) and kind.path == path and kind.file_version:
                    derived.append(
                        FileVersion(
                            hash=kind.file_version,
                            path=path,
                            size=kind.bytes_written,
                            created_at=span.started_at or "",
                            created_by_span=span.id,
                            created_by_trace=span.trace_id,
                        )
                    )

            dedup: dict[str, FileVersion] = {}
            for v in derived:
                if v.hash not in dedup:
                    dedup[v.hash] = v
            versions = list(dedup.values())

        versions.sort(key=lambda v: v.created_at, reverse=True)
        return versions

    def file_traces(self, path: str) -> dict[str, list[dict[str, str]]]:
        quoted_path = quote(path, safe='')
        try:
            return self._request("GET", f"/files/{quoted_path}/traces")
        except httpx.HTTPStatusError as e:
            if e.response.status_code != 404:
                raise

        links: list[dict[str, str]] = []
        for span in self.get_spans().spans:
            kind = span.kind
            if isinstance(kind, FsReadKind) and kind.path == path:
                link: dict[str, str] = {
                    "trace_id": span.trace_id,
                    "span_id": span.id,
                    "operation": "read",
                }
                if kind.file_version:
                    link["file_version"] = kind.file_version
                if span.started_at:
                    link["started_at"] = span.started_at
                links.append(link)
            elif isinstance(kind, FsWriteKind) and kind.path == path:
                link = {
                    "trace_id": span.trace_id,
                    "span_id": span.id,
                    "operation": "write",
                }
                if kind.file_version:
                    link["file_version"] = kind.file_version
                if span.started_at:
                    link["started_at"] = span.started_at
                links.append(link)

        links.sort(key=lambda l: l.get("started_at", ""), reverse=True)
        return {"traces": links}

    # ─── Dataset operations ─────────────────────────────────────────────

    def list_datasets(self) -> DatasetList:
        resp = self._request("GET", "/datasets")
        return DatasetList.from_dict(resp)

    def create_dataset(self, name: str, description: str | None = None) -> Dataset:
        data: dict[str, Any] = {"name": name}
        if description is not None:
            data["description"] = description
        resp = self._request("POST", "/datasets", json=data)
        return Dataset.from_dict(resp)

    def get_dataset(self, dataset_id: str) -> Dataset:
        resp = self._request("GET", f"/datasets/{dataset_id}")
        return Dataset.from_dict(resp)

    def update_dataset(
        self,
        dataset_id: str,
        *,
        name: str | None = None,
        description: str | None = None,
    ) -> Dataset:
        data: dict[str, Any] = {}
        if name is not None:
            data["name"] = name
        if description is not None:
            data["description"] = description
        resp = self._request("PUT", f"/datasets/{dataset_id}", json=data)
        return Dataset.from_dict(resp)

    def delete_dataset(self, dataset_id: str) -> None:
        self._request("DELETE", f"/datasets/{dataset_id}")

    # ─── Datapoint operations ─────────────────────────────────────────

    def list_datapoints(self, dataset_id: str) -> DatapointList:
        resp = self._request("GET", f"/datasets/{dataset_id}/datapoints")
        return DatapointList.from_dict(resp)

    def get_datapoint(self, dataset_id: str, datapoint_id: str) -> Datapoint:
        resp = self._request("GET", f"/datasets/{dataset_id}/datapoints/{datapoint_id}")
        return Datapoint.from_dict(resp)

    def create_datapoint(self, dataset_id: str, kind: dict[str, Any]) -> Datapoint:
        resp = self._request("POST", f"/datasets/{dataset_id}/datapoints", json={"kind": kind})
        return Datapoint.from_dict(resp)

    def delete_datapoint(self, dataset_id: str, datapoint_id: str) -> None:
        self._request("DELETE", f"/datasets/{dataset_id}/datapoints/{datapoint_id}")

    def export_span_to_dataset(self, dataset_id: str, span_id: str) -> Datapoint:
        resp = self._request("POST", f"/datasets/{dataset_id}/export-span", json={"span_id": span_id})
        return Datapoint.from_dict(resp)

    # ─── Queue operations ─────────────────────────────────────────────

    def list_queue(self, dataset_id: str) -> QueueList:
        resp = self._request("GET", f"/datasets/{dataset_id}/queue")
        return QueueList.from_dict(resp)

    def enqueue_datapoints(self, dataset_id: str, datapoint_ids: list[str]) -> list[QueueItem]:
        resp = self._request(
            "POST",
            f"/datasets/{dataset_id}/queue",
            json={"datapoint_ids": datapoint_ids},
        )
        if isinstance(resp, list):
            return [QueueItem.from_dict(q) for q in resp]
        if isinstance(resp, dict):
            if "items" in resp and isinstance(resp["items"], list):
                return [QueueItem.from_dict(q) for q in resp["items"]]
            if resp.get("ok") is True:
                return []
        return []

    def claim_queue_item(self, item_id: str, claimed_by: str | None = None) -> QueueItem:
        data: dict[str, Any] = {}
        if claimed_by is not None:
            data["claimed_by"] = claimed_by
        resp = self._request("POST", f"/queue/{item_id}/claim", json=data if data else None)
        return QueueItem.from_dict(resp)

    def submit_queue_item(self, item_id: str, edited_data: Any = None) -> QueueItem:
        data: dict[str, Any] = {}
        if edited_data is not None:
            data["edited_data"] = edited_data
        resp = self._request("POST", f"/queue/{item_id}/submit", json=data if data else None)
        return QueueItem.from_dict(resp)

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

        Calls POST /traces (or /api/traces on legacy backends) to register the trace.
        Sets TRACEWAY_TRACE_ID env var for subprocess propagation.

        Example:
            with client.trace("chat-completion") as t:
                config = client.read_file("config.json")
                with t.llm_call("inference", model="gpt-4o") as call:
                    result = openai.chat(...)
                    call.set_output(result)
        """
        trace = self.create_trace(name=name or None)
        trace_id = trace.id
        old_env = os.environ.get("TRACEWAY_TRACE_ID")
        os.environ["TRACEWAY_TRACE_ID"] = trace_id
        try:
            yield TraceContext(self, trace_id)
        finally:
            if old_env is not None:
                os.environ["TRACEWAY_TRACE_ID"] = old_env
            else:
                os.environ.pop("TRACEWAY_TRACE_ID", None)

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
        ctx = SpanContext(self, created.id, trace_id, kind=kind)
        try:
            yield ctx
        except Exception as e:
            self.fail_span(created.id, str(e))
            raise
        else:
            self.complete_span(created.id, output=ctx._output, kind=ctx._kind)
