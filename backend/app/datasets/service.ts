import { and, asc, eq } from "drizzle-orm";

import { db } from "../core/database";
import { asJson } from "../core/json";
import { datapoints, datasets } from "../core/schema";
import { newId, nowIso } from "../core/utils";
import {
  CreateDatapointRequest,
  CreateDatasetRequest,
  Datapoint,
  Dataset,
  UpdateDatasetRequest,
} from "./types";

function mapDataset(row: typeof datasets.$inferSelect): Dataset {
  return {
    id: row.id,
    org_id: row.orgId,
    project_id: row.projectId,
    name: row.name,
    description: row.description ?? undefined,
    created_at: row.createdAt.toISOString(),
    updated_at: row.updatedAt.toISOString(),
  };
}

function mapDatapoint(row: typeof datapoints.$inferSelect): Datapoint {
  return {
    id: row.id,
    dataset_id: row.datasetId,
    kind: asJson(row.kind),
    source: row.source,
    source_span_id: row.sourceSpanId ?? undefined,
    created_at: row.createdAt.toISOString(),
  };
}

export const DatasetsService = {
  async list(orgId: string, projectId: string): Promise<Dataset[]> {
    const rows = await db
      .select()
      .from(datasets)
      .where(and(eq(datasets.orgId, orgId), eq(datasets.projectId, projectId)))
      .orderBy(asc(datasets.createdAt));
    return rows.map(mapDataset);
  },

  async get(orgId: string, projectId: string, id: string): Promise<Dataset | null> {
    const [row] = await db
      .select()
      .from(datasets)
      .where(and(eq(datasets.id, id), eq(datasets.orgId, orgId), eq(datasets.projectId, projectId)))
      .limit(1);
    return row ? mapDataset(row) : null;
  },

  async create(req: CreateDatasetRequest): Promise<Dataset> {
    const now = nowIso();
    const [row] = await db
      .insert(datasets)
      .values({
        id: req.id ?? newId(),
        orgId: req.org_id,
        projectId: req.project_id,
        name: req.name,
        description: req.description ?? null,
        createdAt: new Date(now),
        updatedAt: new Date(now),
      })
      .returning();
    return mapDataset(row);
  },

  async update(req: UpdateDatasetRequest): Promise<Dataset | null> {
    const patch: Partial<typeof datasets.$inferInsert> = {
      updatedAt: new Date(),
    };
    if (req.name !== undefined) {
      patch.name = req.name;
    }
    if (req.description !== undefined) {
      patch.description = req.description;
    }

    const [row] = await db
      .update(datasets)
      .set(patch)
      .where(
        and(eq(datasets.id, req.id), eq(datasets.orgId, req.org_id), eq(datasets.projectId, req.project_id))
      )
      .returning();

    return row ? mapDataset(row) : null;
  },

  async delete(orgId: string, projectId: string, id: string): Promise<boolean> {
    const deleted = await db
      .delete(datasets)
      .where(and(eq(datasets.id, id), eq(datasets.orgId, orgId), eq(datasets.projectId, projectId)))
      .returning({ id: datasets.id });
    return deleted.length > 0;
  },

  async listDatapoints(orgId: string, projectId: string, datasetId: string): Promise<Datapoint[]> {
    const rows = await db
      .select()
      .from(datapoints)
      .where(
        and(
          eq(datapoints.datasetId, datasetId),
          eq(datapoints.orgId, orgId),
          eq(datapoints.projectId, projectId)
        )
      )
      .orderBy(asc(datapoints.createdAt));
    return rows.map(mapDatapoint);
  },

  async createDatapoint(req: CreateDatapointRequest): Promise<Datapoint> {
    const [row] = await db
      .insert(datapoints)
      .values({
        id: req.id ?? newId(),
        orgId: req.org_id,
        projectId: req.project_id,
        datasetId: req.dataset_id,
        kind: req.kind,
        source: req.source,
        sourceSpanId: req.source_span_id ?? null,
        createdAt: new Date(),
      })
      .returning();
    return mapDatapoint(row);
  },
};
