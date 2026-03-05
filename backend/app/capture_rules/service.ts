import { and, asc, eq } from "drizzle-orm";

import { db } from "../core/database";
import { asJson } from "../core/json";
import { captureRules } from "../core/schema";
import { newId } from "../core/utils";
import { CaptureRule, CreateCaptureRuleRequest, UpdateCaptureRuleRequest } from "./types";

function mapRule(row: typeof captureRules.$inferSelect): CaptureRule {
  return {
    id: row.id,
    dataset_id: row.datasetId,
    name: row.name,
    enabled: row.enabled,
    filters: asJson(row.filters),
    sample_rate: row.sampleRate,
    captured_count: row.capturedCount,
    created_at: row.createdAt.toISOString(),
  };
}

export const CaptureRulesService = {
  async list(orgId: string, projectId: string, datasetId: string): Promise<CaptureRule[]> {
    const rows = await db
      .select()
      .from(captureRules)
      .where(
        and(
          eq(captureRules.orgId, orgId),
          eq(captureRules.projectId, projectId),
          eq(captureRules.datasetId, datasetId)
        )
      )
      .orderBy(asc(captureRules.createdAt));
    return rows.map(mapRule);
  },

  async create(req: CreateCaptureRuleRequest): Promise<CaptureRule> {
    const [row] = await db
      .insert(captureRules)
      .values({
        id: req.id ?? newId(),
        orgId: req.org_id,
        projectId: req.project_id,
        datasetId: req.dataset_id,
        name: req.name,
        enabled: true,
        filters: req.filters ?? {},
        sampleRate: req.sample_rate ?? 1.0,
        capturedCount: 0,
        createdAt: new Date(),
      })
      .returning();
    return mapRule(row);
  },

  async update(req: UpdateCaptureRuleRequest): Promise<CaptureRule | null> {
    const patch: Partial<typeof captureRules.$inferInsert> = {};
    if (req.name !== undefined) patch.name = req.name;
    if (req.filters !== undefined) patch.filters = req.filters;
    if (req.sample_rate !== undefined) patch.sampleRate = req.sample_rate;

    const [row] = await db
      .update(captureRules)
      .set(patch)
      .where(
        and(
          eq(captureRules.id, req.id),
          eq(captureRules.orgId, req.org_id),
          eq(captureRules.projectId, req.project_id)
        )
      )
      .returning();
    return row ? mapRule(row) : null;
  },

  async toggle(orgId: string, projectId: string, id: string): Promise<CaptureRule | null> {
    const [current] = await db
      .select()
      .from(captureRules)
      .where(and(eq(captureRules.id, id), eq(captureRules.orgId, orgId), eq(captureRules.projectId, projectId)))
      .limit(1);
    if (!current) return null;

    const [updated] = await db
      .update(captureRules)
      .set({ enabled: !current.enabled })
      .where(and(eq(captureRules.id, id), eq(captureRules.orgId, orgId), eq(captureRules.projectId, projectId)))
      .returning();
    return updated ? mapRule(updated) : null;
  },

  async delete(orgId: string, projectId: string, id: string): Promise<boolean> {
    const deleted = await db
      .delete(captureRules)
      .where(and(eq(captureRules.id, id), eq(captureRules.orgId, orgId), eq(captureRules.projectId, projectId)))
      .returning({ id: captureRules.id });
    return deleted.length > 0;
  },
};
