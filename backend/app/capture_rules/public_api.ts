import { api } from "encore.dev/api";

import { JsonValue } from "../core/json";
import { CaptureRulesService } from "./service";
import { handlePreflight, json, page, readJsonBody, requireSession, setCors } from "../shared/http";
import { pathSegments } from "../shared/request";

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
      filters: (body.filters ?? null) as JsonValue,
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
      filters: body.filters as JsonValue | undefined,
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
