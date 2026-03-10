import { api } from "encore.dev/api";

import { handlePreflight, json, page, query, readJsonBody, requireScope, setCors } from "../shared/http";
import { pathSegments } from "../shared/request";
import { analyticsQuery, analyticsSummary } from "./analytics";
import {
  clearAll,
  completeSpan,
  createSpan,
  createTrace,
  deleteSpan,
  deleteTrace,
  exportJson,
  failSpan,
  getFileContent,
  getSpan,
  getTraceSpans,
  listEvents,
  listSessions,
  listFileVersions,
  listSpans,
  listTraces,
  listVersionsForPath,
  stats,
} from "./service";

export const listTracesEndpoint = api.raw(
  { expose: true, method: "GET", path: "/traces" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const items = await listTraces(session);
    json(res, 200, page(items));
  }
);

export const createTraceEndpoint = api.raw(
  { expose: true, method: "POST", path: "/traces" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const body = await readJsonBody<{ id?: string; name?: string; tags?: string[] }>(req);
    const trace = await createTrace(session, body);
    json(res, 200, trace);
  }
);

export const getTraceEndpoint = api.raw(
  { expose: true, method: "GET", path: "/traces/:trace_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const traceId = pathSegments(req)[1] ?? "";
    const spans = await getTraceSpans(session, traceId);
    json(res, 200, { spans, count: spans.length });
  }
);

export const deleteTraceEndpoint = api.raw(
  { expose: true, method: "DELETE", path: "/traces/:trace_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const traceId = pathSegments(req)[1] ?? "";
    const deleted = await deleteTrace(session, traceId);
    json(res, 200, deleted);
  }
);

export const clearAllEndpoint = api.raw(
  { expose: true, method: "DELETE", path: "/traces" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    await clearAll(session);
    json(res, 200, { message: "cleared" });
  }
);

export const listSpansEndpoint = api.raw(
  { expose: true, method: "GET", path: "/spans" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const params = query(req);
    let items = await listSpans(session);
    const traceId = params.get("trace_id");
    if (traceId) items = items.filter((s) => s.trace_id === traceId);
    const status = params.get("status");
    if (status) {
      if (status === "failed") {
        items = items.filter((s) => typeof s.status === "object" && s.status !== null && "failed" in (s.status as Record<string, unknown>));
      } else {
        items = items.filter((s) => s.status === status);
      }
    }
    const kind = params.get("kind");
    if (kind) items = items.filter((s) => s.kind?.type === kind);
    const nameContains = params.get("name_contains");
    if (nameContains) {
      const q = nameContains.toLowerCase();
      items = items.filter((s) => s.name.toLowerCase().includes(q));
    }
    json(res, 200, page(items));
  }
);

export const listSessionsEndpoint = api.raw(
  { expose: true, method: "GET", path: "/sessions" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const items = await listSessions(session);
    json(res, 200, page(items));
  }
);

export const createSpanEndpoint = api.raw(
  { expose: true, method: "POST", path: "/spans" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const body = await readJsonBody<{
      id?: string;
      trace_id: string;
      parent_id?: string | null;
      name: string;
      kind: Record<string, unknown>;
      input?: unknown;
    }>(req);
    const created = await createSpan(session, body);
    json(res, 200, created);
  }
);

export const getSpanEndpoint = api.raw(
  { expose: true, method: "GET", path: "/spans/:span_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const spanId = pathSegments(req)[1] ?? "";
    const span = await getSpan(session, spanId);
    if (!span) {
      json(res, 404, { error: "Span not found" });
      return;
    }
    json(res, 200, span);
  }
);

export const completeSpanEndpoint = api.raw(
  { expose: true, method: "POST", path: "/spans/:span_id/complete" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const spanId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ output?: unknown }>(req);
    await completeSpan(session, spanId, body.output);
    json(res, 200, { ok: true });
  }
);

export const failSpanEndpoint = api.raw(
  { expose: true, method: "POST", path: "/spans/:span_id/fail" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const spanId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ error?: string }>(req);
    await failSpan(session, spanId, body.error ?? "Unknown error");
    json(res, 200, { ok: true });
  }
);

export const deleteSpanEndpoint = api.raw(
  { expose: true, method: "DELETE", path: "/spans/:span_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const spanId = pathSegments(req)[1] ?? "";
    await deleteSpan(session, spanId);
    json(res, 200, { ok: true });
  }
);

export const statsEndpoint = api.raw(
  { expose: true, method: "GET", path: "/stats" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    json(res, 200, await stats(session));
  }
);

export const filesEndpoint = api.raw(
  { expose: true, method: "GET", path: "/files" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const pathPrefix = query(req).get("path_prefix") ?? undefined;
    const files = await listFileVersions(session, pathPrefix);
    json(res, 200, { files, count: files.length });
  }
);

export const fileVersionsEndpoint = api.raw(
  { expose: true, method: "GET", path: "/files/:path" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const decoded = decodeURIComponent(pathSegments(req)[1] ?? "");
    const versions = await listVersionsForPath(session, decoded);
    json(res, 200, { path: decoded, versions, count: versions.length });
  }
);

export const fileContentEndpoint = api.raw(
  { expose: true, method: "GET", path: "/files/content/:hash" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const hash = pathSegments(req)[2] ?? "";
    const content = await getFileContent(hash);
    if (content === null) {
      res.statusCode = 404;
      res.end("Not found");
      return;
    }
    res.statusCode = 200;
    res.setHeader("content-type", "text/plain; charset=utf-8");
    res.end(content);
  }
);

export const analyticsSummaryEndpoint = api.raw(
  { expose: true, method: "GET", path: "/analytics/summary" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    json(res, 200, await analyticsSummary(session));
  }
);

export const analyticsQueryEndpoint = api.raw(
  { expose: true, method: "POST", path: "/analytics" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const body = await readJsonBody(req);
    json(res, 200, await analyticsQuery(session, body));
  }
);

export const exportJsonEndpoint = api.raw(
  { expose: true, method: "GET", path: "/export/json" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);
    const traceId = query(req).get("trace_id") ?? undefined;
    const traces = await exportJson(session, traceId);
    json(res, 200, { traces });
  }
);

export const eventsEndpoint = api.raw(
  { expose: true, method: "GET", path: "/events" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireScope(req, res);
    if (!session) return;
    setCors(req, res);

    const since = Number(query(req).get("since") ?? "0") || 0;
    let cursor = since;

    res.setHeader("content-type", "text/event-stream");
    res.setHeader("cache-control", "no-cache");
    res.setHeader("connection", "keep-alive");

    const flush = async () => {
      const events = await listEvents(session, cursor > 0 ? cursor : undefined);
      for (const event of events) {
        cursor = event.id;
        res.write(`id: ${event.id}\n`);
        res.write(`data: ${JSON.stringify(event.payload)}\n\n`);
      }
    };

    await flush();
    const interval = setInterval(() => {
      void flush();
    }, 1500);

    req.on("close", () => {
      clearInterval(interval);
      res.end();
    });
  }
);
