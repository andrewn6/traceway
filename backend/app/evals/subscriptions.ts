import { Subscription } from "encore.dev/pubsub";

import { queue, workflows } from "~encore/clients";
import { EvalService } from "./service";
import { evalRunCompleted, evalRunRequested } from "./events";
import { systemCallOpts } from "../workflows/system";

const _executeEvalRun = new Subscription(evalRunRequested, "execute-eval-run", {
  handler: async (event) => {
    await workflows.runEvalNow(
      {
        org_id: event.org_id,
        project_id: event.project_id,
        run_id: event.run_id,
        dataset_id: event.dataset_id,
      },
      systemCallOpts
    );
  },
});

const _autoEnqueueLowScores = new Subscription(evalRunCompleted, "auto-enqueue-low-scores", {
  handler: async (event) => {
    if (event.status !== "completed") {
      return;
    }

    const run = await EvalService.getRun(event.org_id, event.project_id, event.run_id);
    if (!run) {
      return;
    }

    const results = await EvalService.listResults(event.org_id, event.project_id, event.run_id);
    const lowScoreIds = results
      .filter((r) => r.status === "completed" && (r.score ?? 1) < 0.5)
      .map((r) => r.datapoint_id);

    if (lowScoreIds.length === 0) {
      return;
    }

    await queue.enqueue(
      {
        org_id: event.org_id,
        project_id: event.project_id,
        dataset_id: run.dataset_id,
        datapoint_ids: lowScoreIds,
      },
      systemCallOpts
    );
  },
});
