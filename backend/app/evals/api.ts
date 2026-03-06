import { APIError, api } from "encore.dev/api";

import { ScopeQuery } from "../core/types";
import { validateScope } from "../core/utils";
import { evalRunRequested } from "./events";
import { EvalService } from "./service";
import { CreateEvalResultRequest, CreateEvalRunRequest, UpdateEvalRunRequest } from "./types";

export const listEvalRuns = api(
  { expose: true, auth: true, method: "GET", path: "/internal/eval-runs" },
  async (req: ScopeQuery & { dataset_id?: string }) => {
    validateScope(req);
    const runs = await EvalService.listRuns(req.org_id, req.project_id, req.dataset_id);
    return { runs, count: runs.length };
  }
);

export const getEvalRun = api(
  { expose: true, auth: true, method: "GET", path: "/internal/eval-runs/:id" },
  async (req: ScopeQuery & { id: string }) => {
    validateScope(req);
    const run = await EvalService.getRun(req.org_id, req.project_id, req.id);
    if (!run) {
      throw APIError.notFound("Eval run not found");
    }
    const result_items = await EvalService.listResults(req.org_id, req.project_id, req.id);
    return { ...run, result_items };
  }
);

export const createEvalRun = api(
  { expose: true, auth: true, method: "POST", path: "/internal/eval-runs" },
  async (req: CreateEvalRunRequest) => {
    validateScope(req);
    if (!req.dataset_id || !req.config) {
      throw APIError.invalidArgument("dataset_id and config are required");
    }
    const run = await EvalService.createRun(req);
    await evalRunRequested.publish({
      org_id: req.org_id,
      project_id: req.project_id,
      run_id: run.id,
      dataset_id: run.dataset_id,
      requested_at: new Date().toISOString(),
    });
    return run;
  }
);

export const updateEvalRun = api(
  { expose: true, auth: true, method: "PATCH", path: "/internal/eval-runs/:id" },
  async (req: UpdateEvalRunRequest) => {
    validateScope(req);
    const run = await EvalService.updateRun(req);
    if (!run) {
      throw APIError.notFound("Eval run not found");
    }
    return run;
  }
);

export const deleteEvalRun = api(
  { expose: true, auth: true, method: "DELETE", path: "/internal/eval-runs/:id" },
  async (req: ScopeQuery & { id: string }) => {
    validateScope(req);
    const ok = await EvalService.deleteRun(req.org_id, req.project_id, req.id);
    return { ok };
  }
);

export const cancelEvalRun = api(
  { expose: true, auth: true, method: "POST", path: "/internal/eval-runs/:id/cancel" },
  async (req: ScopeQuery & { id: string }) => {
    validateScope(req);
    const run = await EvalService.getRun(req.org_id, req.project_id, req.id);
    if (!run) {
      throw APIError.notFound("Eval run not found");
    }
    if (["completed", "failed", "cancelled"].includes(run.status)) {
      throw APIError.aborted("Eval run already terminal");
    }
    const updated = await EvalService.updateRun({
      org_id: req.org_id,
      project_id: req.project_id,
      id: req.id,
      status: "cancelled",
      completed_at: new Date().toISOString(),
    });
    if (!updated) {
      throw APIError.notFound("Eval run not found");
    }
    return updated;
  }
);

export const listEvalResults = api(
  { expose: true, auth: true, method: "GET", path: "/internal/eval-results" },
  async (req: ScopeQuery & { run_id: string }) => {
    validateScope(req);
    const items = await EvalService.listResults(req.org_id, req.project_id, req.run_id);
    return { items, count: items.length };
  }
);

export const createEvalResult = api(
  { expose: true, auth: true, method: "POST", path: "/internal/eval-results" },
  async (req: CreateEvalResultRequest) => {
    validateScope(req);
    if (!req.run_id || !req.datapoint_id) {
      throw APIError.invalidArgument("run_id and datapoint_id are required");
    }
    return EvalService.createResult(req);
  }
);
