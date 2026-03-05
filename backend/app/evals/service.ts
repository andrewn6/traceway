import { and, asc, desc, eq } from "drizzle-orm";

import { db } from "../core/database";
import { asJson } from "../core/json";
import { evalResults, evalRuns } from "../core/schema";
import { newId } from "../core/utils";
import {
  CreateEvalResultRequest,
  CreateEvalRunRequest,
  EvalResult,
  EvalRun,
  UpdateEvalRunRequest,
} from "./types";

function mapRun(row: typeof evalRuns.$inferSelect): EvalRun {
  return {
    id: row.id,
    dataset_id: row.datasetId,
    name: row.name ?? undefined,
    config: asJson(row.config),
    scoring: row.scoring,
    status: row.status,
    results: asJson(row.results),
    trace_id: row.traceId ?? undefined,
    created_at: row.createdAt.toISOString(),
    completed_at: row.completedAt?.toISOString(),
    error: row.error ?? undefined,
  };
}

function mapResult(row: typeof evalResults.$inferSelect): EvalResult {
  return {
    id: row.id,
    run_id: row.runId,
    datapoint_id: row.datapointId,
    status: row.status,
    actual_output: asJson(row.actualOutput),
    score: row.score ?? undefined,
    score_reason: row.scoreReason ?? undefined,
    latency_ms: row.latencyMs,
    input_tokens: row.inputTokens ?? undefined,
    output_tokens: row.outputTokens ?? undefined,
    error: row.error ?? undefined,
    span_id: row.spanId ?? undefined,
    created_at: row.createdAt.toISOString(),
  };
}

export const EvalService = {
  async listRuns(orgId: string, projectId: string, datasetId?: string): Promise<EvalRun[]> {
    const rows = await db
      .select()
      .from(evalRuns)
      .where(
        datasetId
          ? and(
              eq(evalRuns.orgId, orgId),
              eq(evalRuns.projectId, projectId),
              eq(evalRuns.datasetId, datasetId)
            )
          : and(eq(evalRuns.orgId, orgId), eq(evalRuns.projectId, projectId))
      )
      .orderBy(desc(evalRuns.createdAt));
    return rows.map(mapRun);
  },

  async getRun(orgId: string, projectId: string, id: string): Promise<EvalRun | null> {
    const [row] = await db
      .select()
      .from(evalRuns)
      .where(and(eq(evalRuns.id, id), eq(evalRuns.orgId, orgId), eq(evalRuns.projectId, projectId)))
      .limit(1);
    return row ? mapRun(row) : null;
  },

  async createRun(req: CreateEvalRunRequest): Promise<EvalRun> {
    const [row] = await db
      .insert(evalRuns)
      .values({
        id: req.id ?? newId(),
        orgId: req.org_id,
        projectId: req.project_id,
        datasetId: req.dataset_id,
        name: req.name ?? null,
        config: req.config,
        scoring: req.scoring ?? "none",
        status: "pending",
        results: { total: 0, completed: 0, failed: 0, scores: {} },
        createdAt: new Date(),
      })
      .returning();
    return mapRun(row);
  },

  async updateRun(req: UpdateEvalRunRequest): Promise<EvalRun | null> {
    const patch: Partial<typeof evalRuns.$inferInsert> = {};
    if (req.status !== undefined) patch.status = req.status;
    if (req.results !== undefined) patch.results = req.results;
    if (req.trace_id !== undefined) patch.traceId = req.trace_id;
    if (req.completed_at !== undefined) patch.completedAt = new Date(req.completed_at);
    if (req.error !== undefined) patch.error = req.error;

    const [row] = await db
      .update(evalRuns)
      .set(patch)
      .where(and(eq(evalRuns.id, req.id), eq(evalRuns.orgId, req.org_id), eq(evalRuns.projectId, req.project_id)))
      .returning();
    return row ? mapRun(row) : null;
  },

  async deleteRun(orgId: string, projectId: string, id: string): Promise<boolean> {
    const deleted = await db
      .delete(evalRuns)
      .where(and(eq(evalRuns.id, id), eq(evalRuns.orgId, orgId), eq(evalRuns.projectId, projectId)))
      .returning({ id: evalRuns.id });
    return deleted.length > 0;
  },

  async createResult(req: CreateEvalResultRequest): Promise<EvalResult> {
    const [row] = await db
      .insert(evalResults)
      .values({
        id: req.id ?? newId(),
        orgId: req.org_id,
        projectId: req.project_id,
        runId: req.run_id,
        datapointId: req.datapoint_id,
        status: req.status ?? "skipped",
        actualOutput: req.actual_output ?? null,
        score: req.score ?? null,
        scoreReason: req.score_reason ?? null,
        latencyMs: req.latency_ms ?? 0,
        inputTokens: req.input_tokens ?? null,
        outputTokens: req.output_tokens ?? null,
        error: req.error ?? null,
        spanId: req.span_id ?? null,
        createdAt: new Date(),
      })
      .returning();
    return mapResult(row);
  },

  async listResults(orgId: string, projectId: string, runId: string): Promise<EvalResult[]> {
    const rows = await db
      .select()
      .from(evalResults)
      .where(
        and(eq(evalResults.runId, runId), eq(evalResults.orgId, orgId), eq(evalResults.projectId, projectId))
      )
      .orderBy(asc(evalResults.createdAt));
    return rows.map(mapResult);
  },
};
