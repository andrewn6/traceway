import { APIError, api } from "encore.dev/api";
import { CronJob } from "encore.dev/cron";
import { and, eq, inArray, lt } from "drizzle-orm";

import { datasets, evals } from "~encore/clients";
import { db } from "../core/database";
import { evalRuns } from "../core/schema";
import { asJson } from "../core/json";
import { evalRunCompleted } from "../evals/events";
import { evalArtifacts } from "./storage";
import { systemCallOpts } from "./system";

function scoreDatapoint(input: unknown): number {
  const text = JSON.stringify(input ?? null);
  let hash = 0;
  for (let i = 0; i < text.length; i += 1) {
    hash = (hash * 31 + text.charCodeAt(i)) % 1000;
  }
  return Math.max(0, Math.min(1, hash / 1000));
}

async function executeEvalRun(orgId: string, projectId: string, runId: string, datasetId: string) {
  const datapointsResp = await datasets.listDatapoints(
    { org_id: orgId, project_id: projectId, id: datasetId },
    systemCallOpts
  );

  const datapoints = datapointsResp.items;
  const startedAt = Date.now();
  let completed = 0;
  let failed = 0;
  let scoreTotal = 0;

  for (const dp of datapoints) {
    try {
      const score = scoreDatapoint(dp.kind);
      scoreTotal += score;
      completed += 1;

      await evals.createEvalResult(
        {
          org_id: orgId,
          project_id: projectId,
          run_id: runId,
          datapoint_id: dp.id,
          status: "completed",
          actual_output: asJson({ heuristic: "encore-workflow", score }),
          score,
          score_reason: "Deterministic placeholder score from workflow runner",
          latency_ms: 5,
        },
        systemCallOpts
      );
    } catch {
      failed += 1;
      await evals.createEvalResult(
        {
          org_id: orgId,
          project_id: projectId,
          run_id: runId,
          datapoint_id: dp.id,
          status: "failed",
          actual_output: asJson(null),
          latency_ms: 0,
          error: "Workflow execution failed",
        },
        systemCallOpts
      );
    }
  }

  const avgScore = completed > 0 ? scoreTotal / completed : 0;
  const summary = {
    total: datapoints.length,
    completed,
    failed,
    duration_ms: Date.now() - startedAt,
    scores: {
      avg: avgScore,
    },
  };

  const artifactKey = `eval-runs/${runId}/summary.json`;
  await evalArtifacts.upload(artifactKey, Buffer.from(JSON.stringify(summary, null, 2), "utf8"), {
    contentType: "application/json",
  });

  const finalStatus = failed > 0 ? "failed" : "completed";
  const completedAt = new Date().toISOString();
  await evals.updateEvalRun(
    {
      org_id: orgId,
      project_id: projectId,
      id: runId,
      status: finalStatus,
      results: asJson({ ...summary, artifact_key: artifactKey }),
      completed_at: completedAt,
      error: failed > 0 ? `${failed} datapoints failed` : undefined,
    },
    systemCallOpts
  );

  await evalRunCompleted.publish({
    org_id: orgId,
    project_id: projectId,
    run_id: runId,
    status: finalStatus,
    artifact_key: artifactKey,
    completed_at: completedAt,
  });
}

async function recoverForScope(orgId: string, projectId: string, staleMinutes: number) {
  const cutoff = new Date(Date.now() - staleMinutes * 60_000);

  const stale = await db
    .select({ id: evalRuns.id })
    .from(evalRuns)
    .where(
      and(
        eq(evalRuns.orgId, orgId),
        eq(evalRuns.projectId, projectId),
        inArray(evalRuns.status, ["pending", "running"]),
        lt(evalRuns.createdAt, cutoff)
      )
    );

  for (const run of stale) {
    await evals.updateEvalRun(
      {
        org_id: orgId,
        project_id: projectId,
        id: run.id,
        status: "failed",
        completed_at: new Date().toISOString(),
        error: "Marked stale by cron recovery",
      },
      systemCallOpts
    );

    await evalRunCompleted.publish({
      org_id: orgId,
      project_id: projectId,
      run_id: run.id,
      status: "failed",
      completed_at: new Date().toISOString(),
    });
  }

  return stale.length;
}

export const runEvalNow = api(
  { method: "POST", path: "/internal/workflows/evals/:run_id/run", auth: true, expose: true },
  async (req: { org_id: string; project_id: string; run_id: string; dataset_id: string }) => {
    await executeEvalRun(req.org_id, req.project_id, req.run_id, req.dataset_id);
    return { ok: true };
  }
);

export const recoverStaleEvalRuns = api(
  { method: "POST", path: "/internal/workflows/evals/recover", auth: true, expose: true },
  async (req: { org_id: string; project_id: string; stale_minutes?: number }) => {
    const staleMinutes = Math.max(5, req.stale_minutes ?? 60);
    const recovered = await recoverForScope(req.org_id, req.project_id, staleMinutes);
    return { recovered };
  }
);

export const recoverStaleEvalRunsCron = api(
  { method: "POST", path: "/internal/workflows/evals/recover/default", auth: false },
  async () => {
    const orgId = process.env.TRACEWAY_DEFAULT_ORG_ID;
    const projectId = process.env.TRACEWAY_DEFAULT_PROJECT_ID;
    if (!orgId || !projectId) {
      return { recovered: 0, skipped: true };
    }
    const recovered = await recoverForScope(orgId, projectId, 60);
    return { recovered, skipped: false };
  }
);

const _staleRecovery = new CronJob("recover-stale-eval-runs", {
  title: "Recover stale eval runs",
  every: "1h",
  endpoint: recoverStaleEvalRunsCron,
});

export const getEvalArtifactUrl = api(
  { method: "GET", path: "/internal/workflows/evals/:run_id/artifact-url", auth: true, expose: true },
  async (req: { run_id: string }) => {
    const key = `eval-runs/${req.run_id}/summary.json`;
    if (!(await evalArtifacts.exists(key))) {
      throw APIError.notFound("artifact not found");
    }
    const signed = await evalArtifacts.signedDownloadUrl(key, { ttl: 3600 });
    return { url: signed.url, key };
  }
);
