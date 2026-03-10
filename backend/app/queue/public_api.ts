import { api } from "encore.dev/api";

import { JsonValue } from "../core/json";
import { QueueService } from "../queue/service";
import { handlePreflight, json, page, readJsonBody, requireScope, setCors } from "../shared/http";
import { pathSegments } from "../shared/request";

export const listQueuePublic = api.raw({ expose: true, method: "GET", path: "/queue" }, async (req, res) => {
  if (handlePreflight(req, res)) return;
  const scope = await requireScope(req, res);
  if (!scope) return;
  setCors(req, res);
  const params = new URL(req.url ?? "/", "http://local").searchParams;
  const status = params.get("status") ?? undefined;
  const datasetId = params.get("dataset_id") ?? undefined;
  let items = await QueueService.list(scope.org_id, scope.project_id, datasetId);
  if (status) items = items.filter((q) => q.status === status);
  json(res, 200, page(items));
});

export const claimQueuePublic = api.raw(
  { expose: true, method: "POST", path: "/queue/:item_id/claim" },
  async (req, res) => {
    if (handlePreflight(req, res)) return;
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const itemId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ claimed_by: string }>(req);
    const item = await QueueService.claim(scope.org_id, scope.project_id, itemId, body.claimed_by);
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
    const scope = await requireScope(req, res);
    if (!scope) return;
    setCors(req, res);
    const itemId = pathSegments(req)[1] ?? "";
    const body = await readJsonBody<{ edited_data?: unknown }>(req);
    const item = await QueueService.submit(scope.org_id, scope.project_id, itemId, (body.edited_data ?? null) as JsonValue);
    if (!item) {
      json(res, 409, { error: "Queue item not submittable" });
      return;
    }
    json(res, 200, item);
  }
);
