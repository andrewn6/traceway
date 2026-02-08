from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Literal


# ─── SpanKind ─────────────────────────────────────────────────────────

@dataclass
class FsReadKind:
    path: str
    file_version: str | None = None
    bytes_read: int = 0

@dataclass
class FsWriteKind:
    path: str
    file_version: str = ""
    bytes_written: int = 0

@dataclass
class LlmCallKind:
    model: str
    provider: str | None = None
    input_tokens: int | None = None
    output_tokens: int | None = None
    input_preview: str | None = None
    output_preview: str | None = None

@dataclass
class CustomKind:
    kind: str
    attributes: dict[str, Any] = field(default_factory=dict)


SpanKind = FsReadKind | FsWriteKind | LlmCallKind | CustomKind


def span_kind_from_dict(d: dict[str, Any]) -> SpanKind | None:
    if d is None:
        return None
    if "FsRead" in d:
        v = d["FsRead"]
        return FsReadKind(path=v["path"], file_version=v.get("file_version"), bytes_read=v.get("bytes_read", 0))
    if "FsWrite" in d:
        v = d["FsWrite"]
        return FsWriteKind(path=v["path"], file_version=v.get("file_version", ""), bytes_written=v.get("bytes_written", 0))
    if "LlmCall" in d:
        v = d["LlmCall"]
        return LlmCallKind(
            model=v["model"], provider=v.get("provider"),
            input_tokens=v.get("input_tokens"), output_tokens=v.get("output_tokens"),
            input_preview=v.get("input_preview"), output_preview=v.get("output_preview"),
        )
    if "Custom" in d:
        v = d["Custom"]
        return CustomKind(kind=v["kind"], attributes=v.get("attributes", {}))
    return None


def span_kind_to_dict(kind: SpanKind) -> dict[str, Any]:
    if isinstance(kind, FsReadKind):
        return {"FsRead": {"path": kind.path, "file_version": kind.file_version, "bytes_read": kind.bytes_read}}
    if isinstance(kind, FsWriteKind):
        return {"FsWrite": {"path": kind.path, "file_version": kind.file_version, "bytes_written": kind.bytes_written}}
    if isinstance(kind, LlmCallKind):
        d: dict[str, Any] = {"model": kind.model}
        if kind.provider is not None:
            d["provider"] = kind.provider
        if kind.input_tokens is not None:
            d["input_tokens"] = kind.input_tokens
        if kind.output_tokens is not None:
            d["output_tokens"] = kind.output_tokens
        return {"LlmCall": d}
    if isinstance(kind, CustomKind):
        return {"Custom": {"kind": kind.kind, "attributes": kind.attributes}}
    return {}


# ─── Legacy SpanMetadata (backward compat) ────────────────────────────

@dataclass
class SpanMetadata:
    model: str | None = None
    input_tokens: int | None = None
    output_tokens: int | None = None

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "SpanMetadata":
        return cls(
            model=d.get("model"),
            input_tokens=d.get("input_tokens"),
            output_tokens=d.get("output_tokens"),
        )


# ─── SpanStatus ───────────────────────────────────────────────────────

@dataclass
class SpanStatus:
    kind: Literal["running", "completed", "failed"]
    started_at: str
    ended_at: str | None = None
    error: str | None = None

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "SpanStatus":
        if "Running" in d:
            return cls(kind="running", started_at=d["Running"]["started_at"])
        elif "Completed" in d:
            return cls(
                kind="completed",
                started_at=d["Completed"]["started_at"],
                ended_at=d["Completed"]["ended_at"],
            )
        elif "Failed" in d:
            return cls(
                kind="failed",
                started_at=d["Failed"]["started_at"],
                ended_at=d["Failed"]["ended_at"],
                error=d["Failed"]["error"],
            )
        raise ValueError(f"Unknown status: {d}")


# ─── Span ─────────────────────────────────────────────────────────────

@dataclass
class Span:
    id: str
    trace_id: str
    parent_id: str | None
    name: str
    status: SpanStatus
    metadata: SpanMetadata
    kind: SpanKind | None = None
    input: Any = None
    output: Any = None
    started_at: str | None = None
    ended_at: str | None = None

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "Span":
        return cls(
            id=d["id"],
            trace_id=d["trace_id"],
            parent_id=d.get("parent_id"),
            name=d["name"],
            status=SpanStatus.from_dict(d["status"]),
            metadata=SpanMetadata.from_dict(d.get("metadata", {})),
            kind=span_kind_from_dict(d["kind"]) if d.get("kind") else None,
            input=d.get("input"),
            output=d.get("output"),
            started_at=d.get("started_at"),
            ended_at=d.get("ended_at"),
        )


# ─── Collections ──────────────────────────────────────────────────────

@dataclass
class TraceList:
    traces: list[str]
    count: int

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "TraceList":
        return cls(traces=d["traces"], count=d["count"])


@dataclass
class SpanList:
    spans: list[Span]
    count: int

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "SpanList":
        return cls(
            spans=[Span.from_dict(s) for s in d["spans"]],
            count=d["count"],
        )


@dataclass
class Stats:
    trace_count: int
    span_count: int

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "Stats":
        return cls(trace_count=d["trace_count"], span_count=d["span_count"])


@dataclass
class ExportData:
    traces: dict[str, list[Span]]

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "ExportData":
        return cls(
            traces={
                tid: [Span.from_dict(s) for s in spans]
                for tid, spans in d["traces"].items()
            }
        )


# ─── Filters ──────────────────────────────────────────────────────────

@dataclass
class SpanFilter:
    model: str | None = None
    status: str | None = None
    since: str | None = None
    until: str | None = None
    name_contains: str | None = None
    kind: str | None = None
    path: str | None = None
    trace_id: str | None = None


# ─── File Types ───────────────────────────────────────────────────────

@dataclass
class TrackedFile:
    path: str
    current_hash: str
    version_count: int
    created_at: str
    updated_at: str

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "TrackedFile":
        return cls(
            path=d["path"],
            current_hash=d["current_hash"],
            version_count=d.get("version_count", 0),
            created_at=d["created_at"],
            updated_at=d["updated_at"],
        )


@dataclass
class FileVersion:
    hash: str
    path: str
    size: int
    created_at: str
    created_by_span: str | None = None
    created_by_trace: str | None = None

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "FileVersion":
        return cls(
            hash=d["hash"],
            path=d.get("path", ""),
            size=d.get("size", 0),
            created_at=d["created_at"],
            created_by_span=d.get("created_by_span"),
            created_by_trace=d.get("created_by_trace"),
        )


# ─── Response Types ───────────────────────────────────────────────────

@dataclass
class CreatedSpan:
    id: str
    trace_id: str

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "CreatedSpan":
        return cls(id=d["id"], trace_id=d["trace_id"])


@dataclass
class SpanEvent:
    type: str
    span: Span | None = None
    span_id: str | None = None
    trace_id: str | None = None

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "SpanEvent":
        event_type = d["type"]
        span = Span.from_dict(d["span"]) if "span" in d else None
        return cls(
            type=event_type,
            span=span,
            span_id=d.get("span_id"),
            trace_id=d.get("trace_id"),
        )
