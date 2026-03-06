import { api } from "encore.dev/api";
import { and, eq } from "drizzle-orm";

import { CaptureRulesService } from "../capture_rules/service";
import { db } from "../core/database";
import { datapoints, datasets, providerConnections } from "../core/schema";
import { handlePreflight, json, page, readJsonBody, requireSession, setCors } from "../core/public_api";
import { DatasetsService } from "../datasets/service";
import { EvalService } from "../evals/service";
import { ProviderConnectionsService } from "../provider_connections/service";
import { QueueService } from "../queue/service";
import { stats } from "../tracing/service";

function apiKeyPreview(key: string | undefined): string | null {
  if (!key) return null;
  if (key.length <= 6) return key;
  return `${key.slice(0, 4)}...${key.slice(-2)}`;
}

function pathSegments(req: import("http").IncomingMessage): string[] {
  return new URL(req.url ?? "/", "http://local").pathname.split("/").filter(Boolean);
}

export const listDatasetsPublic = api.raw({ expose: true, method: "GET", path: "/datasets" }, async (req, res) => {
  if (handlePreflight(req, res)) return;
  const session = await requireSession(req, res);
  if (!session) return;
  setCors(req, res);

  const items = await DatasetsService.list(session.org_id, session.project_id);
  const withCount = await Promise.all(
    items.map(async (dataset) => {
      const points = await DatasetsService.listDatapoints(session.org_id, session.project_id, dataset.id);
      return { ...dataset, datapoint_count: points.length };
    })
  );
  json(res, 200, { datasets: withCount, count: withCount.length });
});

export const createDatasetPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const body = await readJsonBody<{ name: string; description?: string }>(req);
    const dataset = await DatasetsService.create({
      org_id: session.org_id,
      project_id: session.project_id,
      name: body.name,
      description: body.description,
    });
    json(res, 200, dataset);
  }
);

export const getDatasetPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);

    const datasetId = pathSegments(req)[1] ?? "";
    const dataset = await DatasetsService.get(session.org_id, session.project_id, datasetId);
    if (!dataset) {
      json(res, 404, { error: "Dataset not found" });
      return;
    }
    const points = await DatasetsService.listDatapoints(session.org_id, session.project_id, datasetId);
    json(res, 200, { ...dataset, datapoint_count: points.length });
  }
);

export const updateDatasetPublic = api.raw(
  { expose: true, method: "PUT", path: "/datasets/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);

    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ name?: string; description?: string }>(req);
    const updated = await DatasetsService.update({
      org_id: session.org_id,
      project_id: session.project_id,
      id: datasetId,
      name: body.name,
      description: body.description,
    });
    if (!updated) {
      json(res, 404, { error: "Dataset not found" });
      return;
    }
    const points = await DatasetsService.listDatapoints(session.org_id, session.project_id, datasetId);
    json(res, 200, { ...updated, datapoint_count: points.length });
  }
);

export const deleteDatasetPublic = api.raw(
  { expose: true, method: "DELETE", path: "/datasets/:id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    await DatasetsService.delete(session.org_id, session.project_id, datasetId);
    json(res, 200, { ok: true });
  }
);

export const listDatapointsPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id/datapoints" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const items = await DatasetsService.listDatapoints(session.org_id, session.project_id, datasetId);
    json(res, 200, page(items));
  }
);

export const createDatapointPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/datapoints" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ kind: unknown }>(req);
    const item = await DatasetsService.createDatapoint({
      org_id: session.org_id,
      project_id: session.project_id,
      dataset_id: datasetId,
      kind: (body.kind ?? null) as unknown as import("../core/json").JsonValue,
      source: "manual",
    });
    json(res, 200, item);
  }
);

export const deleteDatapointPublic = api.raw(
  { expose: true, method: "DELETE", path: "/datasets/:id/datapoints/:dp_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const parts = pathSegments(req);
    const datasetId = parts[1] ?? "";
    const datapointId = parts[3] ?? "";
    await db
      .delete(datapoints)
      .where(
        and(
          eq(datapoints.id, datapointId),
          eq(datapoints.datasetId, datasetId),
          eq(datapoints.orgId, session.org_id),
          eq(datapoints.projectId, session.project_id)
        )
      );
    json(res, 200, { ok: true });
  }
);

export const exportSpanToDatasetPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/export-span" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ span_id: string }>(req);
    const dp = await DatasetsService.createDatapoint({
      org_id: session.org_id,
      project_id: session.project_id,
      dataset_id: datasetId,
      source: "span_export",
      source_span_id: body.span_id,
      kind: { type: "generic", input: null, expected_output: null, actual_output: null } as unknown as import("../core/json").JsonValue,
    });
    json(res, 200, dp);
  }
);

export const listQueueByDatasetPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id/queue" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const items = await QueueService.list(session.org_id, session.project_id, datasetId);
    json(res, 200, page(items));
  }
);

export const enqueueDatapointsPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/queue" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ datapoint_ids: string[] }>(req);
    await QueueService.enqueue(session.org_id, session.project_id, datasetId, body.datapoint_ids ?? []);
    json(res, 200, { ok: true });
  }
);

export const listQueuePublic = api.raw({ expose: true, method: "GET", path: "/queue" }, async (req, res) => {
  if (handlePreflight(req, res)) return;
  const session = await requireSession(req, res);
  if (!session) return;
  setCors(req, res);
  const status = new URL(req.url ?? "/", "http://local").searchParams.get("status") ?? undefined;
  let items = await QueueService.list(session.org_id, session.project_id);
  if (status) items = items.filter((q) => q.status === status);
  json(res, 200, page(items));
});

export const claimQueuePublic = api.raw(
  { expose: true, method: "POST", path: "/queue/:item_id/claim" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const itemId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ claimed_by: string }>(req);
    const item = await QueueService.claim(session.org_id, session.project_id, itemId, body.claimed_by);
    if (!item) {
      json(res, 409, { error: "Queue item not claimable" });
      return;
    }
    json(res, 200, item);
  }
);

export const submitQueuePublic = api.raw(
  { expose: true, method: "POST", path: "/queue/:item_id/submit" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const itemId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ edited_data?: unknown }>(req);
    const item = await QueueService.submit(
      session.org_id,
      session.project_id,
      itemId,
      (body.edited_data ?? null) as unknown as import("../core/json").JsonValue
    );
    if (!item) {
      json(res, 409, { error: "Queue item not submittable" });
      return;
    }
    json(res, 200, item);
  }
);

export const exportAndEnqueuePublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/export-span-and-enqueue" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ span_id: string }>(req);
    const dp = await DatasetsService.createDatapoint({
      org_id: session.org_id,
      project_id: session.project_id,
      dataset_id: datasetId,
      source: "span_export",
      source_span_id: body.span_id,
      kind: { type: "generic", input: null, expected_output: null, actual_output: null } as unknown as import("../core/json").JsonValue,
    });
    const [item] = await QueueService.enqueue(session.org_id, session.project_id, datasetId, [dp.id]);
    json(res, 200, item ?? null);
  }
);

export const listEvalRunsPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id/eval" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const runs = await EvalService.listRuns(session.org_id, session.project_id, datasetId);
    json(res, 200, page(runs));
  }
);

export const createEvalRunPublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/eval" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ name?: string; config: unknown; scoring?: string }>(req);
    const run = await EvalService.createRun({
      org_id: session.org_id,
      project_id: session.project_id,
      dataset_id: datasetId,
      name: body.name,
      config: (body.config ?? null) as unknown as import("../core/json").JsonValue,
      scoring: body.scoring,
    });
    json(res, 200, run);
  }
);

export const getEvalRunPublic = api.raw(
  { expose: true, method: "GET", path: "/eval/:run_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const runId = pathSegments(req)[1] ?? "";
    const run = await EvalService.getRun(session.org_id, session.project_id, runId);
    if (!run) {
      json(res, 404, { error: "Run not found" });
      return;
    }
    const result_items = await EvalService.listResults(session.org_id, session.project_id, runId);
    json(res, 200, { ...run, result_items });
  }
);

export const deleteEvalRunPublic = api.raw(
  { expose: true, method: "DELETE", path: "/eval/:run_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const runId = pathSegments(req)[1] ?? "";
    await EvalService.deleteRun(session.org_id, session.project_id, runId);
    json(res, 200, undefined);
  }
);

export const cancelEvalRunPublic = api.raw(
  { expose: true, method: "POST", path: "/eval/:run_id/cancel" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const runId = pathSegments(req)[1] ?? "";
    await EvalService.updateRun({
      org_id: session.org_id,
      project_id: session.project_id,
      id: runId,
      status: "cancelled",
      completed_at: new Date().toISOString(),
    });
    json(res, 200, undefined);
  }
);

export const compareEvalRunsPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id/compare" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const runsParam = new URL(req.url ?? "/", "http://local").searchParams.get("runs") ?? "";
    const ids = runsParam.split(",").filter(Boolean);
    const runs = (await EvalService.listRuns(session.org_id, session.project_id, datasetId)).filter((r) =>
      ids.includes(r.id)
    );
    json(res, 200, { runs, datapoints: [] });
  }
);

export const listRulesPublic = api.raw(
  { expose: true, method: "GET", path: "/datasets/:id/rules" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const rules = await CaptureRulesService.list(session.org_id, session.project_id, datasetId);
    json(res, 200, page(rules));
  }
);

export const createRulePublic = api.raw(
  { expose: true, method: "POST", path: "/datasets/:id/rules" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const datasetId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ name: string; filters?: unknown; sample_rate?: number }>(req);
    const rule = await CaptureRulesService.create({
      org_id: session.org_id,
      project_id: session.project_id,
      dataset_id: datasetId,
      name: body.name,
      filters: (body.filters ?? null) as unknown as import("../core/json").JsonValue,
      sample_rate: body.sample_rate,
    });
    json(res, 200, rule);
  }
);

export const updateRulePublic = api.raw(
  { expose: true, method: "PUT", path: "/rules/:rule_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const ruleId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ name?: string; filters?: unknown; sample_rate?: number }>(req);
    const rule = await CaptureRulesService.update({
      org_id: session.org_id,
      project_id: session.project_id,
      id: ruleId,
      name: body.name,
      filters: body.filters as unknown as import("../core/json").JsonValue | undefined,
      sample_rate: body.sample_rate,
    });
    if (!rule) {
      json(res, 404, { error: "Rule not found" });
      return;
    }
    json(res, 200, rule);
  }
);

export const deleteRulePublic = api.raw(
  { expose: true, method: "DELETE", path: "/rules/:rule_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const ruleId = pathSegments(req)[1] ?? "";
    await CaptureRulesService.delete(session.org_id, session.project_id, ruleId);
    json(res, 200, undefined);
  }
);

export const toggleRulePublic = api.raw(
  { expose: true, method: "POST", path: "/rules/:rule_id/toggle" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const ruleId = pathSegments(req)[1] ?? "";
    const rule = await CaptureRulesService.toggle(session.org_id, session.project_id, ruleId);
    if (!rule) {
      json(res, 404, { error: "Rule not found" });
      return;
    }
    json(res, 200, rule);
  }
);

export const listProviderConnectionsPublic = api.raw(
  { expose: true, method: "GET", path: "/provider-connections" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);

    const connections = await ProviderConnectionsService.list(session.org_id, session.project_id);
    json(res, 200, {
      connections: connections.map((c) => ({
        id: c.id,
        name: c.name,
        provider: c.provider,
        base_url: c.base_url,
        api_key_preview: apiKeyPreview(c.api_key),
        default_model: c.default_model,
        created_at: c.created_at,
        updated_at: c.updated_at,
      })),
      count: connections.length,
    });
  }
);

export const createProviderConnectionPublic = api.raw(
  { expose: true, method: "POST", path: "/provider-connections" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const body = await readJsonBody<{
      name: string;
      provider: string;
      base_url?: string;
      api_key?: string;
      default_model?: string;
    }>(req);
    const now = new Date().toISOString();
    const id = crypto.randomUUID();
    await ProviderConnectionsService.upsert(session.org_id, session.project_id, {
      id,
      name: body.name,
      provider: body.provider,
      base_url: body.base_url,
      api_key: body.api_key,
      default_model: body.default_model,
      created_at: now,
      updated_at: now,
    });
    const conn = await ProviderConnectionsService.get(session.org_id, session.project_id, id);
    json(res, 200, {
      id,
      name: conn?.name ?? body.name,
      provider: conn?.provider ?? body.provider,
      base_url: conn?.base_url,
      api_key_preview: apiKeyPreview(conn?.api_key),
      default_model: conn?.default_model,
      created_at: conn?.created_at ?? now,
      updated_at: conn?.updated_at ?? now,
    });
  }
);

export const updateProviderConnectionPublic = api.raw(
  { expose: true, method: "PUT", path: "/provider-connections/:conn_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const connId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{
      name?: string;
      provider?: string;
      base_url?: string;
      api_key?: string;
      default_model?: string;
    }>(req);
    const current = await db
      .select()
      .from(providerConnections)
      .where(
        and(
          eq(providerConnections.id, connId),
          eq(providerConnections.orgId, session.org_id),
          eq(providerConnections.projectId, session.project_id)
        )
      )
      .limit(1);
    if (current.length === 0) {
      json(res, 404, { error: "Connection not found" });
      return;
    }

    const existing = current[0];
    const now = new Date().toISOString();
    await ProviderConnectionsService.upsert(session.org_id, session.project_id, {
      id: connId,
      name: body.name ?? existing.name,
      provider: body.provider ?? existing.provider,
      base_url: body.base_url ?? existing.baseUrl ?? undefined,
      api_key: body.api_key ?? existing.apiKey ?? undefined,
      default_model: body.default_model ?? existing.defaultModel ?? undefined,
      created_at: existing.createdAt.toISOString(),
      updated_at: now,
    });
    const conn = await ProviderConnectionsService.get(session.org_id, session.project_id, connId);
    json(res, 200, {
      id: conn?.id ?? connId,
      name: conn?.name ?? existing.name,
      provider: conn?.provider ?? existing.provider,
      base_url: conn?.base_url,
      api_key_preview: apiKeyPreview(conn?.api_key),
      default_model: conn?.default_model,
      created_at: conn?.created_at ?? existing.createdAt.toISOString(),
      updated_at: conn?.updated_at ?? now,
    });
  }
);

export const deleteProviderConnectionPublic = api.raw(
  { expose: true, method: "DELETE", path: "/provider-connections/:conn_id" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    const connId = pathSegments(req)[1] ?? "";
    await ProviderConnectionsService.delete(session.org_id, session.project_id, connId);
    json(res, 200, undefined);
  }
);

export const testProviderConnectionPublic = api.raw(
  { expose: true, method: "POST", path: "/provider-connections/test" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    json(res, 200, { ok: true, models: [] });
  }
);

export const listProviderModelsPublic = api.raw(
  { expose: true, method: "GET", path: "/provider-connections/:conn_id/models" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const session = await requireSession(req, res);
    if (!session) return;
    setCors(req, res);
    json(res, 200, { ok: true, models: [] });
  }
);

export const healthPublic = api.raw({ expose: true, method: "GET", path: "/health" }, async (req, res) => {
  if (handlePreflight(req, res)) return;
  setCors(req, res);
  const session = await requireSession(req, res);
  if (!session) return;
  const count = await stats(session);
  json(res, 200, {
    status: "ok",
    version: "encore",
    uptime_secs: Math.floor(process.uptime()),
    storage: {
      backend: "postgres",
      trace_count: count.trace_count,
      span_count: count.span_count,
    },
  });
});
