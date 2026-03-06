import { api } from "encore.dev/api";

import { JsonValue } from "../core/json";
import { QueueService } from "../queue/service";
import { handlePreflight, json, page, readJsonBody, requireSession, setCors } from "../shared/http";
import { pathSegments } from "../shared/request";

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
    const item = await QueueService.submit(session.org_id, session.project_id, itemId, (body.edited_data ?? null) as JsonValue);
    if (!item) {
      json(res, 409, { error: "Queue item not submittable" });
      return;
    }
    json(res, 200, item);
  }
);
