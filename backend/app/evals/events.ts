import { Attribute, Topic } from "encore.dev/pubsub";

export interface EvalRunRequestedEvent {
  org_id: string;
  project_id: string;
  run_id: Attribute<string>;
  dataset_id: string;
  requested_at: string;
}

export const evalRunRequested = new Topic<EvalRunRequestedEvent>("eval-run-requested", {
  deliveryGuarantee: "exactly-once",
  orderingAttribute: "run_id",
});

export interface EvalRunCompletedEvent {
  org_id: string;
  project_id: string;
  run_id: Attribute<string>;
  status: "completed" | "failed" | "cancelled";
  artifact_key?: string;
  completed_at: string;
}

export const evalRunCompleted = new Topic<EvalRunCompletedEvent>("eval-run-completed", {
  deliveryGuarantee: "at-least-once",
  orderingAttribute: "run_id",
});
