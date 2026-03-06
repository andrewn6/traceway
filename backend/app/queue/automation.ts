import { Subscription } from "encore.dev/pubsub";

import { evals, queue } from "~encore/clients";
import { evalRunCompleted } from "../evals/events";
import { systemCallOpts } from "../workflows/system";

const _autoEnqueueLowScores = new Subscription(evalRunCompleted, "auto-enqueue-low-scores", {
  handler: async (event) => {
    if (event.status !== "completed") {
      return;
    }

    const run = await evals.getEvalRun(
      {
        org_id: event.org_id,
        project_id: event.project_id,
        id: event.run_id,
      },
      systemCallOpts
    );

    const lowScoreIds = run.result_items
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
