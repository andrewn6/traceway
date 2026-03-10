import { and, asc, desc, eq, gt } from "drizzle-orm";

import { db } from "../core/database";
import { eventLog, fileContents, fileVersions, spans, traces } from "../core/schema";
import { newId } from "../core/utils";
import { mirrorSpan, mirrorTrace } from "../search/turbopuffer";

type Scope = { org_id: string; project_id: string };

export type TraceItem = {
  id: string;
  name?: string | null;
  tags?: string[];
  started_at: string;
  ended_at?: string | null;
  machine_id?: string | null;
};

export type SpanItem = {
  id: string;
  trace_id: string;
  parent_id?: string | null;
  name: string;
  kind: Record<string, unknown>;
  status: unknown;
  input?: unknown;
  output?: unknown;
  started_at: string;
  ended_at?: string | null;
};

export type SessionItem = {
  id: string;
  trace_count: number;
  span_count: number;
  total_tokens: number;
  total_cost: number;
  started_at: string;
  ended_at?: string | null;
};

function mapTrace(row: typeof traces.$inferSelect): TraceItem {
  return {
    id: row.id,
    name: row.name ?? undefined,
    tags: (row.tags as string[]) ?? [],
    started_at: row.startedAt.toISOString(),
    ended_at: row.endedAt?.toISOString(),
    machine_id: row.machineId ?? undefined,
  };
}

function mapSpan(row: typeof spans.$inferSelect): SpanItem {
  return {
    id: row.id,
    trace_id: row.traceId,
    parent_id: row.parentId ?? undefined,
    name: row.name,
    kind: row.kind as Record<string, unknown>,
    status: row.status,
    input: row.input ?? undefined,
    output: row.output ?? undefined,
    started_at: row.startedAt.toISOString(),
    ended_at: row.endedAt?.toISOString(),
  };
}

async function appendEvent(scope: Scope, eventType: string, payload: unknown): Promise<number> {
  const [row] = await db
    .insert(eventLog)
    .values({
      orgId: scope.org_id,
      projectId: scope.project_id,
      eventType,
      payload,
      createdAt: new Date(),
    })
    .returning({ id: eventLog.id });
  return row.id;
}

export async function listTraces(scope: Scope): Promise<TraceItem[]> {
  const rows = await db
    .select()
    .from(traces)
    .where(and(eq(traces.orgId, scope.org_id), eq(traces.projectId, scope.project_id)))
    .orderBy(desc(traces.startedAt));
  return rows.map(mapTrace);
}

function extractSessionId(trace: TraceItem): string | null {
  const tags = trace.tags ?? [];
  for (const tag of tags) {
    if (tag.startsWith("session_id:")) return tag.slice("session_id:".length);
    if (tag.startsWith("session:")) return tag.slice("session:".length);
  }
  return null;
}

export async function listSessions(scope: Scope): Promise<SessionItem[]> {
  const traceRows = await listTraces(scope);
  const spanRows = await listSpans(scope);
  const spanByTrace = new Map<string, SpanItem[]>();
  for (const s of spanRows) {
    const arr = spanByTrace.get(s.trace_id) ?? [];
    arr.push(s);
    spanByTrace.set(s.trace_id, arr);
  }

  const grouped = new Map<string, SessionItem>();
  for (const trace of traceRows) {
    const sessionId = extractSessionId(trace);
    if (!sessionId) continue;
    const spansForTrace = spanByTrace.get(trace.id) ?? [];
    let tokens = 0;
    let cost = 0;
    for (const s of spansForTrace) {
      const kind = s.kind as Record<string, unknown>;
      if (kind.type === "llm_call") {
        const input = typeof kind.input_tokens === "number" ? kind.input_tokens : 0;
        const output = typeof kind.output_tokens === "number" ? kind.output_tokens : 0;
        tokens += input + output;
        if (typeof kind.cost === "number") cost += kind.cost;
      }
    }

    const existing = grouped.get(sessionId);
    if (!existing) {
      grouped.set(sessionId, {
        id: sessionId,
        trace_count: 1,
        span_count: spansForTrace.length,
        total_tokens: tokens,
        total_cost: cost,
        started_at: trace.started_at,
        ended_at: trace.ended_at ?? null,
      });
      continue;
    }

    existing.trace_count += 1;
    existing.span_count += spansForTrace.length;
    existing.total_tokens += tokens;
    existing.total_cost += cost;
    if (new Date(trace.started_at).getTime() < new Date(existing.started_at).getTime()) {
      existing.started_at = trace.started_at;
    }
    if (!existing.ended_at) {
      existing.ended_at = trace.ended_at ?? existing.ended_at;
    } else if (trace.ended_at && new Date(trace.ended_at).getTime() > new Date(existing.ended_at).getTime()) {
      existing.ended_at = trace.ended_at;
    }
  }

  return [...grouped.values()].sort((a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime());
}

export async function createTrace(scope: Scope, input: { id?: string; name?: string; tags?: string[] }): Promise<TraceItem> {
  const [row] = await db
    .insert(traces)
    .values({
      id: input.id ?? newId(),
      orgId: scope.org_id,
      projectId: scope.project_id,
      name: input.name ?? null,
      tags: input.tags ?? [],
      startedAt: new Date(),
      endedAt: null,
      machineId: null,
    })
    .onConflictDoNothing({ target: traces.id })
    .returning();

  if (!row) {
    const [existing] = await db.select().from(traces).where(eq(traces.id, input.id!)).limit(1);
    if (existing) return mapTrace(existing);
    throw new Error("Failed to create trace");
  }

  const trace = mapTrace(row);
  await appendEvent(scope, "trace_created", { type: "trace_created", trace });
  void mirrorTrace({
    id: trace.id,
    kind: "trace",
    org_id: scope.org_id,
    project_id: scope.project_id,
    text: [trace.name ?? "", ...(trace.tags ?? [])].join(" ").trim(),
    metadata: trace,
  });
  return trace;
}

export async function getTraceSpans(scope: Scope, traceId: string): Promise<SpanItem[]> {
  const rows = await db
    .select()
    .from(spans)
    .where(and(eq(spans.traceId, traceId), eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)))
    .orderBy(asc(spans.startedAt));
  return rows.map(mapSpan);
}

export async function deleteTrace(scope: Scope, traceId: string): Promise<{ trace_id: string; spans_deleted: number }> {
  const deletedSpans = await db
    .delete(spans)
    .where(and(eq(spans.traceId, traceId), eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)))
    .returning({ id: spans.id });

  await db
    .delete(traces)
    .where(and(eq(traces.id, traceId), eq(traces.orgId, scope.org_id), eq(traces.projectId, scope.project_id)));

  await appendEvent(scope, "trace_deleted", { type: "trace_deleted", trace_id: traceId });
  return { trace_id: traceId, spans_deleted: deletedSpans.length };
}

export async function clearAll(scope: Scope): Promise<void> {
  await db
    .delete(spans)
    .where(and(eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)));
  await db
    .delete(traces)
    .where(and(eq(traces.orgId, scope.org_id), eq(traces.projectId, scope.project_id)));
  await appendEvent(scope, "cleared", { type: "cleared" });
}

export async function listSpans(scope: Scope): Promise<SpanItem[]> {
  const rows = await db
    .select()
    .from(spans)
    .where(and(eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)))
    .orderBy(desc(spans.startedAt));
  return rows.map(mapSpan);
}

export async function getSpan(scope: Scope, spanId: string): Promise<SpanItem | null> {
  const [row] = await db
    .select()
    .from(spans)
    .where(and(eq(spans.id, spanId), eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)))
    .limit(1);
  return row ? mapSpan(row) : null;
}

export async function createSpan(
  scope: Scope,
  input: {
    id?: string;
    trace_id: string;
    parent_id?: string | null;
    name: string;
    kind: Record<string, unknown>;
    input?: unknown;
  }
): Promise<{ id: string; trace_id: string }> {
  const [row] = await db
    .insert(spans)
    .values({
      id: input.id ?? newId(),
      orgId: scope.org_id,
      projectId: scope.project_id,
      traceId: input.trace_id,
      parentId: input.parent_id ?? null,
      name: input.name,
      kind: input.kind,
      status: "running",
      input: input.input ?? null,
      output: null,
      startedAt: new Date(),
      endedAt: null,
    })
    .onConflictDoNothing({ target: spans.id })
    .returning();

  if (!row) {
    return { id: input.id ?? "", trace_id: input.trace_id };
  }

  const span = mapSpan(row);
  await appendEvent(scope, "span_created", { type: "span_created", span });
  void mirrorSpan({
    id: span.id,
    kind: "span",
    org_id: scope.org_id,
    project_id: scope.project_id,
    trace_id: span.trace_id,
    text: `${span.name} ${JSON.stringify(span.input ?? {})} ${JSON.stringify(span.output ?? {})}`,
    metadata: span,
  });

  return { id: row.id, trace_id: row.traceId };
}

export async function completeSpan(scope: Scope, spanId: string, output?: unknown): Promise<void> {
  const [row] = await db
    .update(spans)
    .set({
      status: "completed",
      endedAt: new Date(),
      output: output ?? null,
    })
    .where(and(eq(spans.id, spanId), eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)))
    .returning();

  if (!row) return;

  const kind = row.kind as Record<string, unknown>;
  const kindType = typeof kind.type === "string" ? kind.type : "";
  if (kindType === "fs_read" || kindType === "fs_write") {
    const path = typeof kind.path === "string" ? kind.path : "";
    const hash = typeof kind.file_version === "string" ? kind.file_version : "";
    const sizeField = kindType === "fs_read" ? kind.bytes_read : kind.bytes_written;
    const size = typeof sizeField === "number" ? sizeField : 0;

    if (path && hash) {
      await db
        .insert(fileVersions)
        .values({
          id: newId(),
          orgId: scope.org_id,
          projectId: scope.project_id,
          path,
          hash,
          metadata: { size, source: kindType },
          createdBySpan: row.id,
          createdAt: row.endedAt ?? new Date(),
        })
        .onConflictDoNothing();

      const out = row.output as Record<string, unknown> | null;
      const fileContent = out && typeof out === "object"
        ? (typeof out.file_content === "string"
          ? out.file_content
          : typeof out.content === "string"
            ? out.content
            : null)
        : null;
      if (fileContent !== null) {
        await putFileContent(hash, fileContent);
      }
    }
  }

  const span = mapSpan(row);
  await appendEvent(scope, "span_completed", { type: "span_completed", span });
}

export async function failSpan(scope: Scope, spanId: string, error: string): Promise<void> {
  const [row] = await db
    .update(spans)
    .set({
      status: { failed: { error } },
      endedAt: new Date(),
    })
    .where(and(eq(spans.id, spanId), eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)))
    .returning();
  if (!row) return;
  const span = mapSpan(row);
  await appendEvent(scope, "span_failed", { type: "span_failed", span });
}

export async function deleteSpan(scope: Scope, spanId: string): Promise<void> {
  await db
    .delete(spans)
    .where(and(eq(spans.id, spanId), eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)));
  await appendEvent(scope, "span_deleted", { type: "span_deleted", span_id: spanId });
}

export async function stats(scope: Scope): Promise<{ trace_count: number; span_count: number }> {
  const traceRows = await db
    .select({ id: traces.id })
    .from(traces)
    .where(and(eq(traces.orgId, scope.org_id), eq(traces.projectId, scope.project_id)));

  const spanRows = await db
    .select({ id: spans.id })
    .from(spans)
    .where(and(eq(spans.orgId, scope.org_id), eq(spans.projectId, scope.project_id)));

  return { trace_count: traceRows.length, span_count: spanRows.length };
}

export async function listEvents(scope: Scope, since?: number): Promise<Array<{ id: number; payload: unknown }>> {
  const where = since
    ? and(eq(eventLog.orgId, scope.org_id), eq(eventLog.projectId, scope.project_id), gt(eventLog.id, since))
    : and(eq(eventLog.orgId, scope.org_id), eq(eventLog.projectId, scope.project_id));
  const rows = await db
    .select({ id: eventLog.id, payload: eventLog.payload })
    .from(eventLog)
    .where(where)
    .orderBy(asc(eventLog.id));
  return rows;
}

export async function listFileVersions(scope: Scope, pathPrefix?: string): Promise<
  Array<{
    path: string;
    hash: string;
    created_at: string;
    created_by_span?: string | null;
    size: number;
  }>
> {
  const rows = await db
    .select()
    .from(fileVersions)
    .where(and(eq(fileVersions.orgId, scope.org_id), eq(fileVersions.projectId, scope.project_id)))
    .orderBy(desc(fileVersions.createdAt));

  return rows
    .filter((row) => (pathPrefix ? row.path.startsWith(pathPrefix) : true))
    .map((row) => {
      const metadata = row.metadata as Record<string, unknown>;
      const size = typeof metadata?.size === "number" ? metadata.size : 0;
      return {
        path: row.path,
        hash: row.hash,
        created_at: row.createdAt.toISOString(),
        created_by_span: row.createdBySpan,
        size,
      };
    });
}

export async function listVersionsForPath(
  scope: Scope,
  path: string
): Promise<Array<{ path: string; hash: string; created_at: string; created_by_span?: string | null; size: number }>> {
  const rows = await db
    .select()
    .from(fileVersions)
    .where(
      and(
        eq(fileVersions.orgId, scope.org_id),
        eq(fileVersions.projectId, scope.project_id),
        eq(fileVersions.path, path)
      )
    )
    .orderBy(asc(fileVersions.createdAt));
  return rows.map((row) => {
    const metadata = row.metadata as Record<string, unknown>;
    const size = typeof metadata?.size === "number" ? metadata.size : 0;
    return {
      path: row.path,
      hash: row.hash,
      created_at: row.createdAt.toISOString(),
      created_by_span: row.createdBySpan,
      size,
    };
  });
}

export async function getFileContent(hash: string): Promise<string | null> {
  const [row] = await db.select().from(fileContents).where(eq(fileContents.hash, hash)).limit(1);
  return row?.content ?? null;
}

export async function putFileContent(hash: string, content: string): Promise<void> {
  await db
    .insert(fileContents)
    .values({ hash, content, createdAt: new Date() })
    .onConflictDoNothing();
}

export async function exportJson(scope: Scope, traceId?: string): Promise<Record<string, SpanItem[]>> {
  const traceRows = traceId ? [{ id: traceId }] : await db
    .select({ id: traces.id })
    .from(traces)
    .where(and(eq(traces.orgId, scope.org_id), eq(traces.projectId, scope.project_id)));

  const out: Record<string, SpanItem[]> = {};
  for (const t of traceRows) {
    out[t.id] = await getTraceSpans(scope, t.id);
  }
  return out;
}
