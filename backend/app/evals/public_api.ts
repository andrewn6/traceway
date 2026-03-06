import { api } from "encore.dev/api";

import { JsonValue } from "../core/json";
import { EvalService } from "../evals/service";
import { handlePreflight, json, page, readJsonBody, requireSession, setCors } from "../shared/http";
import { pathSegments } from "../shared/request";

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
      config: (body.config ?? null) as JsonValue,
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
    const runs = (await EvalService.listRuns(session.org_id, session.project_id, datasetId)).filter((r) => ids.includes(r.id));
    json(res, 200, { runs, datapoints: [] });
  }
);
